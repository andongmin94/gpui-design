//! Application-owned file metadata with optional consumer actions.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, relative,
};
use gpui_design::{ObservedElement, TestSupportExt as _, TestTextExt as _};

use super::super::Palette;

/// A static file summary whose metadata and actions belong to the application.
///
/// The native Group exposes the complete file name once and describes it with
/// the visible `media_type · size_label` text. Its text children have no separate
/// roles or names. This identifies file metadata, rather than Item's generic
/// titled row or Message's authored body. It adds no focus stop, whole-row
/// action, link destination, file access, transfer state, or retained data.
///
/// The caller supplies the name, media type, and formatted size, including their
/// accuracy and localization. Nothing is parsed, inferred, or loaded. Give the
/// containing layout a definite width: both lines wrap and grow vertically
/// without truncation or an inner viewport. The optional action slot is below
/// the metadata; supplied controls keep their own naming, keyboard, focus, and
/// disabled behavior. A container can supply multiple controls and their layout.
/// Before removing a focused action, the caller moves focus to a retained target.
/// Styled customizes the frame and inherited text style.
#[derive(IntoElement)]
pub(crate) struct Attachment {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    file_name: SharedString,
    metadata: SharedString,
    action: Option<AnyElement>,
    palette: Palette,
}

impl Attachment {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        file_name: impl Into<SharedString>,
        media_type: impl Into<SharedString>,
        size_label: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .bg(palette.field)
                .text_size(palette.font_size)
                .text_color(palette.foreground)
                .line_height(relative(1.5)),
            id,
            file_name: file_name.into(),
            metadata: format!("{} · {}", media_type.into(), size_label.into()).into(),
            action: None,
            palette,
        }
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl Styled for Attachment {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Attachment {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let text = |part, text: SharedString| {
            div()
                .id((self.id.clone(), part))
                .test_support()
                .min_w_0()
                .max_w_full()
                .flex_shrink_0()
                .whitespace_normal()
                .child(text.clone().test_text(text))
        };
        let file_name = text("file-name", self.file_name.clone()).font_weight(FontWeight::SEMIBOLD);
        let metadata = text("metadata", self.metadata.clone()).text_color(self.palette.muted);
        self.base
            .role(Role::Group)
            .aria_label(self.file_name)
            .aria_description(self.metadata)
            .child(file_name)
            .child(metadata)
            .when_some(self.action, |attachment, action| {
                attachment.child(
                    div()
                        .id((self.id, "action"))
                        .test_support()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(action),
                )
            })
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use gpui::{
        AppContext as _, Context, Entity, Pixels, Render, TestAppContext, VisualTestContext, px,
        size,
    };
    use gpui_design::{Root, test_support};

    use super::super::super::Appearance;
    use super::*;

    struct Sample {
        appearance: Appearance,
        width: Pixels,
        file_name: SharedString,
        media_type: SharedString,
        size_label: SharedString,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(self.width).flex().flex_col().child(Attachment::new(
                "sample-attachment",
                self.file_name.clone(),
                self.media_type.clone(),
                self.size_label.clone(),
                self.appearance.palette(),
            ))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        appearance: Appearance,
    ) -> (&mut VisualTestContext, Entity<Sample>) {
        let (root, cx) = cx.add_window_view(|window, cx| {
            let sample = cx.new(|_| Sample {
                appearance,
                width: px(360.),
                file_name: "session.json".into(),
                media_type: "application/json".into(),
                size_label: "64 bytes".into(),
            });
            Root::new(sample, window, cx)
        });
        let sample = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Sample>()
            .unwrap();
        (cx, sample)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[gpui::test]
    fn attachment_metadata_wraps_with_one_name_and_description(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let file_name = format!(
            "회의 설정 {}{}.json",
            "공백없는한글파일이름".repeat(8),
            "unbroken_english_file_name_0123456789".repeat(4),
        );
        let media_type = "application/vnd.example.configuration+json; 한글과 English metadata";
        let size_label = "123456789 bytes · 사용자 제공 크기";
        let metadata = format!("{media_type} · {size_label}");
        let id = ElementId::from("sample-attachment");
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (cx, sample) = harness(cx, appearance);
            cx.simulate_resize(size(px(500.), px(2000.)));
            sample.update(cx, |sample, cx| {
                sample.file_name = file_name.clone().into();
                sample.media_type = media_type.into();
                sample.size_label = size_label.into();
                cx.notify();
            });
            let mut wide_height = None;
            for width in [360., 160., 360.] {
                sample.update(cx, |sample, cx| {
                    sample.width = px(width);
                    cx.notify();
                });
                draw(cx);
                cx.run_until_parked();
                draw(cx);
                cx.update(|window, _| {
                    let attachment = test_support::find(window, &[], &id).unwrap();
                    assert_eq!(attachment.role(), Some(Role::Group));
                    assert_eq!(attachment.label(), Some(file_name.as_str()));
                    assert_eq!(attachment.description(), Some(metadata.as_str()));
                    assert_eq!(attachment.focused(), None);
                    assert!(attachment.bounds().size.width <= px(width) + px(1.));
                    let semantics: Vec<_> = test_support::snapshots(window)
                        .into_iter()
                        .filter(|node| node.path().contains(&id))
                        .collect();
                    assert_eq!(
                        semantics
                            .iter()
                            .filter(|node| node.role().is_some())
                            .count(),
                        1
                    );
                    assert_eq!(
                        semantics
                            .iter()
                            .filter(|node| node.label().is_some())
                            .count(),
                        1
                    );
                    assert_eq!(
                        semantics
                            .iter()
                            .filter(|node| node.description().is_some())
                            .count(),
                        1
                    );
                    for (part, expected) in [
                        ("file-name", file_name.as_str()),
                        ("metadata", metadata.as_str()),
                    ] {
                        let part_id = (id.clone(), part).into();
                        let node = test_support::find(window, &[], &part_id).unwrap();
                        assert_eq!(node.role(), None);
                        assert_eq!(node.label(), None);
                        assert_eq!(node.description(), None);
                        assert_eq!(node.focused(), None);
                        assert!(node.bounds().left() >= attachment.bounds().left());
                        assert!(node.bounds().right() <= attachment.bounds().right() + px(1.));
                        assert!(node.bounds().top() >= attachment.bounds().top());
                        assert!(node.bounds().bottom() <= attachment.bounds().bottom() + px(1.));
                        let texts = test_support::texts(window);
                        let text = texts
                            .iter()
                            .find(|text| text.path().contains(&part_id))
                            .unwrap();
                        assert_eq!(text.text(), expected);
                        assert!(text.visible());
                        assert!(text.bounds().size.width <= node.bounds().size.width + px(1.));
                    }
                    assert!(
                        test_support::find(window, &[], &(id.clone(), "action").into()).is_none()
                    );
                    let height = attachment.bounds().size.height;
                    match wide_height {
                        None => wide_height = Some(height),
                        Some(wide) if width == 360. => assert_eq!(height, wide),
                        Some(wide) => assert!(height > wide),
                    }
                });
            }
        }
    }
}
