//! Application-owned read-only message with explicit provenance and optional actions.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, relative,
};
use gpui_design::{ObservedElement, TestSupportExt as _, TestTextExt as _};

use super::{super::Palette, Paragraph};

/// A static message whose author, time, body, and actions belong to the application.
///
/// The native Comment contains one named Label for each metadata value and one
/// Paragraph for the complete body. The frame has no redundant name, focus stop,
/// click handler, live announcement, or retained message state. Time is displayed
/// exactly as supplied; the caller owns its meaning, formatting, and timezone.
/// Unlike Marker this is content, not a boundary; unlike Item it describes a
/// message's provenance and body, not a generic titled row.
///
/// Give the containing layout a definite width. Stacked metadata and the body
/// wrap normally and grow vertically without truncation or an inner viewport.
/// The optional action slot is below the body, so controls do not steal its
/// width. Supplied Buttons retain their own names, focus, activation, and disabled
/// behavior; a container can supply multiple controls with its own layout.
/// Styled customizes the frame and metadata; Paragraph keeps its palette style.
#[derive(IntoElement)]
pub(crate) struct Message {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    author: SharedString,
    time: SharedString,
    body: SharedString,
    action: Option<AnyElement>,
    palette: Palette,
}

impl Message {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        author: impl Into<SharedString>,
        time: impl Into<SharedString>,
        body: impl Into<SharedString>,
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
            author: author.into(),
            time: time.into(),
            body: body.into(),
            action: None,
            palette,
        }
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl Styled for Message {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Message {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let metadata = |part, text: SharedString| {
            div()
                .id((self.id.clone(), part))
                .test_support()
                .role(Role::Label)
                .aria_label(text.clone())
                .min_w_0()
                .max_w_full()
                .flex_shrink_0()
                .whitespace_normal()
                .child(text.clone().test_text(text))
        };
        let author = metadata("author", self.author).font_weight(FontWeight::SEMIBOLD);
        let time = metadata("time", self.time).text_color(self.palette.muted);
        let body = Paragraph::new((self.id.clone(), "body"), self.palette).text(self.body);
        self.base
            .role(Role::Comment)
            .child(author)
            .child(time)
            .child(body)
            .when_some(self.action, |message, action| {
                message.child(
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
        author: SharedString,
        time: SharedString,
        body: SharedString,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(self.width).flex().flex_col().child(Message::new(
                "sample-message",
                self.author.clone(),
                self.time.clone(),
                self.body.clone(),
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
                author: "You".into(),
                time: "Edited 2026-11-10 08:30 UTC".into(),
                body: "A retained local note.".into(),
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
    fn message_provenance_and_body_wrap_once_without_extra_semantics(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let author = format!("작성자 이름 {}", "long_author_name_0123456789".repeat(3));
        let time = "Edited 2026-11-10 08:30 UTC · 작성자가 제공한 시각과 시간대";
        let body = format!(
            "첫 번째 줄은 한국어입니다.\nEnglish text keeps its line break.\n\n{} {}\n",
            "공백없는한글문장".repeat(12),
            "unbroken_english_message_content_0123456789".repeat(6),
        );
        let id = ElementId::from("sample-message");
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (cx, sample) = harness(cx, appearance);
            cx.simulate_resize(size(px(500.), px(2500.)));
            sample.update(cx, |sample, cx| {
                sample.author = author.clone().into();
                sample.time = time.into();
                sample.body = body.clone().into();
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
                    let message = test_support::find(window, &[], &id).unwrap();
                    assert_eq!(message.role(), Some(Role::Comment));
                    assert_eq!(message.label(), None);
                    assert_eq!(message.description(), None);
                    assert_eq!(message.focused(), None);
                    assert!(message.bounds().size.width <= px(width) + px(1.));
                    let semantics: Vec<_> = test_support::snapshots(window)
                        .into_iter()
                        .filter(|node| node.path().contains(&id) && node.role().is_some())
                        .collect();
                    assert_eq!(semantics.len(), 4);
                    for (part, role, expected) in [
                        ("author", Role::Label, author.as_str()),
                        ("time", Role::Label, time),
                        ("body", Role::Paragraph, body.as_str()),
                    ] {
                        let part_id = (id.clone(), part).into();
                        let node = test_support::find(window, &[], &part_id).unwrap();
                        assert_eq!(node.role(), Some(role));
                        assert_eq!(node.label(), Some(expected));
                        assert_eq!(node.focused(), None);
                        assert!(node.bounds().left() >= message.bounds().left());
                        assert!(node.bounds().right() <= message.bounds().right() + px(1.));
                        assert!(node.bounds().top() >= message.bounds().top());
                        assert!(node.bounds().bottom() <= message.bounds().bottom() + px(1.));
                        assert_eq!(
                            semantics
                                .iter()
                                .filter(|node| node.label() == Some(expected))
                                .count(),
                            1,
                        );
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
                    let height = message.bounds().size.height;
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
