//! A static, named boundary with decorative separator rules.

use gpui::{
    App, Div, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, px, relative,
};
use gpui_design::{ObservedElement, Separator, TestSupportExt as _, TestTextExt as _};

use super::super::Palette;

/// A label between two decorative horizontal rules.
///
/// The label is the marker's only native semantic node: its Label role exposes
/// the complete supplied text once. Both existing Separator elements are
/// decorative, and the outer layout has no role or name. The marker adds no
/// focus stop, action, live announcement, date formatting, or retained state.
///
/// Give the parent a definite available width. The centered label uses at most
/// three quarters of that width and wraps normally; the rules share the rest.
/// Height follows the complete text without truncation. Styled customizes the
/// outer frame and inherited text style; the palette supplies the rule color.
#[derive(IntoElement)]
pub(crate) struct Marker {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    label: SharedString,
    rule_color: Hsla,
}

impl Marker {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
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
                .items_center()
                .gap_2()
                .text_size(palette.font_size)
                .text_color(palette.muted)
                .line_height(relative(1.5)),
            id,
            label: label.into(),
            rule_color: palette.border,
        }
    }
}

impl Styled for Marker {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Marker {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let rule = |part| {
            Separator::new((self.id.clone(), part))
                .decorative(true)
                .flex_1()
                .min_w_0()
                .h(px(1.))
                .bg(self.rule_color)
        };
        let label = div()
            .id((self.id.clone(), "label"))
            .test_support()
            .role(Role::Label)
            .aria_label(self.label.clone())
            .min_w_0()
            .max_w(relative(0.75))
            .flex_shrink_0()
            .text_center()
            .whitespace_normal()
            .child(self.label.clone().test_text(self.label));
        self.base
            .child(rule("before"))
            .child(label)
            .child(rule("after"))
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use gpui::{AppContext as _, Context, FocusHandle, Pixels, Render, TestAppContext, size};
    use gpui_design::{Button, Root, test_support};

    use super::super::super::Appearance;
    use super::*;

    struct Sample {
        appearance: Appearance,
        label: SharedString,
        width: Pixels,
        before: FocusHandle,
        after: FocusHandle,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(self.width)
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    Button::new("before-marker")
                        .track_focus(&self.before)
                        .h(px(32.))
                        .child("Before"),
                )
                .child(Marker::new(
                    "sample-marker",
                    self.label.clone(),
                    self.appearance.palette(),
                ))
                .child(
                    Button::new("after-marker")
                        .track_focus(&self.after)
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    #[gpui::test]
    fn long_labels_wrap_with_one_semantic_name_and_no_focus_stop(cx: &mut TestAppContext) {
        let source = format!(
            "선택한 날짜와 준비 상태를 긴 문장으로 표시합니다. Recording boundary for {} {}",
            "공백없는긴한글경계이름".repeat(5),
            "unbroken_recording_configuration_identifier_0123456789".repeat(3),
        );
        let marker_id = ElementId::from("sample-marker");
        let label_id = (marker_id.clone(), "label").into();
        cx.update(gpui_design::init);
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (root, cx) = cx.add_window_view(|window, cx| {
                let sample = cx.new(|cx| Sample {
                    appearance,
                    label: source.clone().into(),
                    width: px(360.),
                    before: cx.focus_handle(),
                    after: cx.focus_handle(),
                });
                Root::new(sample, window, cx)
            });
            let sample = root
                .read_with(cx, |root, _| root.view().clone())
                .downcast::<Sample>()
                .unwrap();
            cx.simulate_resize(size(px(500.), px(1800.)));
            let mut wide_height = None;
            for width in [360., 160., 360.] {
                sample.update(cx, |sample, cx| {
                    sample.width = px(width);
                    cx.notify();
                });
                cx.update(|window, cx| window.draw(cx).clear(cx));
                cx.run_until_parked();
                cx.update(|window, cx| window.draw(cx).clear(cx));
                cx.update(|window, _| {
                    let frame = test_support::find(window, &[], &marker_id).unwrap();
                    assert_eq!(frame.role(), None);
                    assert_eq!(frame.label(), None);
                    assert_eq!(frame.focused(), None);
                    assert!(frame.bounds().size.width <= px(width) + px(1.));
                    let label = test_support::find(window, &[], &label_id).unwrap();
                    assert_eq!(label.role(), Some(Role::Label));
                    assert_eq!(label.label(), Some(source.as_str()));
                    assert_eq!(label.focused(), None);
                    assert!(label.bounds().left() >= frame.bounds().left());
                    assert!(label.bounds().right() <= frame.bounds().right() + px(1.));
                    for part in ["before", "after"] {
                        let rule =
                            test_support::find(window, &[], &(marker_id.clone(), part).into())
                                .unwrap();
                        assert_eq!(rule.role(), None);
                        assert_eq!(rule.label(), None);
                        assert_eq!(rule.focused(), None);
                        assert!(rule.bounds().size.width > px(0.));
                        assert!(rule.bounds().left() >= frame.bounds().left());
                        assert!(rule.bounds().right() <= frame.bounds().right() + px(1.));
                        if part == "before" {
                            assert!(rule.bounds().right() <= label.bounds().left());
                        } else {
                            assert!(rule.bounds().left() >= label.bounds().right());
                        }
                    }
                    let semantics: Vec<_> = test_support::snapshots(window)
                        .into_iter()
                        .filter(|item| item.path().contains(&marker_id) && item.role().is_some())
                        .collect();
                    assert_eq!(semantics.len(), 1);
                    assert_eq!(semantics[0].role(), Some(Role::Label));
                    assert_eq!(semantics[0].label(), Some(source.as_str()));
                    let texts: Vec<_> = test_support::texts(window)
                        .into_iter()
                        .filter(|text| text.path().contains(&marker_id))
                        .collect();
                    assert_eq!(texts.len(), 1);
                    let text = &texts[0];
                    assert_eq!(text.text(), source);
                    assert!(text.visible());
                    assert!(text.bounds().size.width <= label.bounds().size.width + px(1.));
                    let height = text.bounds().size.height;
                    assert!(height > px(0.));
                    match wide_height {
                        None => wide_height = Some(height),
                        Some(wide) if width == 360. => assert_eq!(height, wide),
                        Some(wide) => assert!(height > wide),
                    }
                });
            }

            let updated = "Updated boundary · 2026-12-10";
            sample.update(cx, |sample, cx| {
                sample.label = updated.into();
                cx.notify();
            });
            cx.update(|window, cx| {
                let before = sample.read(cx).before.clone();
                before.focus(window, cx);
                window.draw(cx).clear(cx);
            });
            cx.update(|window, _| {
                let semantics: Vec<_> = test_support::snapshots(window)
                    .into_iter()
                    .filter(|item| item.path().contains(&marker_id) && item.role().is_some())
                    .collect();
                assert_eq!(semantics.len(), 1);
                assert_eq!(semantics[0].label(), Some(updated));
                assert_eq!(semantics[0].focused(), None);
            });
            cx.simulate_keystrokes("tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(sample.read(cx).after.is_focused(window));
            });
            cx.simulate_keystrokes("shift-tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(sample.read(cx).before.is_focused(window));
            });
        }
    }
}
