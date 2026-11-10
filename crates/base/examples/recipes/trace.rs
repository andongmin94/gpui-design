//! Application-owned disclosure for a summary and its inspectable details.

use gpui::{
    AnyElement, App, ElementId, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, relative,
};
use gpui_design::{
    Collapsible, CollapsibleContent, CollapsibleState, CollapsibleTrigger, TestSupportExt as _,
    TestTextExt as _,
};

use super::{super::Palette, Paragraph};

/// A summary disclosure over the application's existing CollapsibleState.
///
/// The trigger exposes the complete summary and Collapsible's expanded/disabled
/// metadata. Its visible summary and plus/minus indicator add no duplicate names.
/// While open, an explicitly named Region contains one Paragraph and the optional
/// action slot. Closing removes that content and its actions. The existing root
/// and content retain their non-Tab focus scopes; the recipe adds no focus stops,
/// key handlers, event types, or state controller.
///
/// The caller retains and observes the state and owns all text and actions.
/// Changing text does not change disclosure state. Collapsible owns activation
/// and focus return on closure, including its root target when the trigger is
/// disabled. Disabling the trigger leaves the details and their controls intact;
/// supplied controls have their own disabled state. Before removing a focused
/// action or the entire trace, the caller moves focus to a surviving target.
///
/// Give the containing layout a definite width. Summary and details wrap and grow
/// vertically without truncation or an inner viewport. Styled customizes the
/// outer frame; the palette styles the trigger and Paragraph. This recipe does
/// not infer execution, completion, or history from the supplied content.
#[derive(IntoElement)]
pub(crate) struct Trace {
    base: Collapsible,
    id: ElementId,
    state: Entity<CollapsibleState>,
    summary: SharedString,
    details_label: SharedString,
    detail_text: SharedString,
    action: Option<AnyElement>,
    palette: Palette,
}

impl Trace {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        state: &Entity<CollapsibleState>,
        summary: impl Into<SharedString>,
        details_label: impl Into<SharedString>,
        detail_text: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: Collapsible::new(id.clone(), state)
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .bg(palette.field)
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            id,
            state: state.clone(),
            summary: summary.into(),
            details_label: details_label.into(),
            detail_text: detail_text.into(),
            action: None,
            palette,
        }
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl Styled for Trace {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Trace {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let indicator: SharedString = if state.is_open() { "−" } else { "+" }.into();
        let trigger = CollapsibleTrigger::new(&self.state)
            .accessibility_label(self.summary.clone())
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .items_start()
            .justify_start()
            .gap_2()
            .p_3()
            .border_1()
            .border_color(self.palette.field)
            .rounded(self.palette.radius)
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(relative(1.5))
            .hover(|style| style.bg(self.palette.highlight))
            .focus_visible(|style| style.border_color(self.palette.accent))
            .when(state.is_disabled(), |trigger| trigger.opacity(0.5))
            .child(
                div()
                    .id((self.id.clone(), "summary"))
                    .test_support()
                    .flex_1()
                    .min_w_0()
                    .whitespace_normal()
                    .text_left()
                    .child(self.summary.clone().test_text(self.summary)),
            )
            .child(
                div()
                    .id((self.id.clone(), "indicator"))
                    .test_support()
                    .flex_shrink_0()
                    .child(indicator.clone().test_text(indicator)),
            );
        let content = CollapsibleContent::new(&self.state)
            .accessibility_label(self.details_label)
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .pb_3()
            .child(Paragraph::new((self.id.clone(), "body"), self.palette).text(self.detail_text))
            .when_some(self.action, |content, action| {
                content.child(
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
            });
        self.base.child(trigger).child(content)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use gpui::{
        AppContext as _, Context, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
        Pixels, Render, Role, Subscription, TestAppContext, VisualTestContext, px, size,
    };
    use gpui_design::{Button, Root, test_support};

    use super::super::super::Appearance;
    use super::*;

    struct Sample {
        appearance: Appearance,
        width: Pixels,
        state: Entity<CollapsibleState>,
        summary: SharedString,
        details: SharedString,
        action: bool,
        action_disabled: bool,
        activations: usize,
        focus: [FocusHandle; 3],
        _subscription: Subscription,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let trace = Trace::new(
                "sample-trace",
                &self.state,
                self.summary.clone(),
                "Configuration details",
                self.details.clone(),
                self.appearance.palette(),
            )
            .when(self.action, |trace| {
                trace.action(
                    Button::new("trace-action")
                        .track_focus(&self.focus[1])
                        .accessibility_label("Close details")
                        .disabled(self.action_disabled)
                        .min_w_0()
                        .h(px(32.))
                        .px_2()
                        .child("Close")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.activations += 1;
                            this.state
                                .update(cx, |state, cx| state.set_open(false, window, cx));
                            cx.notify();
                        })),
                )
            });
            div()
                .w(self.width)
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    Button::new("before-trace")
                        .track_focus(&self.focus[0])
                        .h(px(32.))
                        .child("Before"),
                )
                .child(trace)
                .child(
                    Button::new("after-trace")
                        .track_focus(&self.focus[2])
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        appearance: Appearance,
    ) -> (&mut VisualTestContext, Entity<Sample>) {
        let (root, cx) = cx.add_window_view(|window, cx| {
            let sample = cx.new(|cx| {
                let state = cx.new(CollapsibleState::new);
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Sample {
                    appearance,
                    width: px(360.),
                    state,
                    summary: "Inspect configuration".into(),
                    details: "The current session has a retained note.".into(),
                    action: false,
                    action_disabled: false,
                    activations: 0,
                    focus: std::array::from_fn(|_| cx.focus_handle()),
                    _subscription: subscription,
                }
            });
            Root::new(sample, window, cx)
        });
        let sample = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Sample>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            let before = sample.read(cx).focus[0].clone();
            before.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (cx, sample)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn activate_key(cx: &mut VisualTestContext, key: &str) {
        let keystroke =
            cx.update(|_, cx| Keystroke::parse_with_os(key, cx.platform_info().os).unwrap());
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
        draw(cx);
    }

    fn part_ids(sample: &Entity<Sample>, cx: &VisualTestContext) -> (ElementId, ElementId) {
        sample.read_with(cx, |sample, _| {
            (
                ("collapsible-trigger", sample.state.entity_id()).into(),
                ("collapsible-content", sample.state.entity_id()).into(),
            )
        })
    }

    #[gpui::test]
    fn trace_summary_and_details_wrap_with_disclosure_semantics(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let summary = format!(
            "설정 상세 보기 {} {}",
            "공백없는긴한글요약".repeat(4),
            "unbroken_english_summary_0123456789".repeat(3),
        );
        let details = format!(
            "한국어 상세 내용입니다.\nEnglish details preserve newlines.\n\n{} {}\n",
            "공백없는한글상세내용".repeat(8),
            "unbroken_english_detail_0123456789".repeat(5),
        );
        let id = ElementId::from("sample-trace");
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (cx, sample) = harness(cx, appearance);
            cx.simulate_resize(size(px(500.), px(2500.)));
            sample.update(cx, |sample, cx| {
                sample.summary = summary.clone().into();
                sample.details = details.clone().into();
                cx.notify();
            });
            cx.update(|window, cx| {
                let state = sample.read(cx).state.clone();
                state.update(cx, |state, cx| state.set_open(true, window, cx));
            });
            let (trigger_id, content_id) = part_ids(&sample, cx);
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
                    let trace = test_support::find(window, &[], &id).unwrap();
                    assert_eq!(trace.role(), Some(Role::Group));
                    assert_eq!(trace.label(), None);
                    assert!(trace.bounds().size.width <= px(width) + px(1.));
                    let trigger = test_support::find(window, &[], &trigger_id).unwrap();
                    assert_eq!(trigger.role(), Some(Role::Button));
                    assert_eq!(trigger.label(), Some(summary.as_str()));
                    assert_eq!(trigger.expanded(), Some(true));
                    let content = test_support::find(window, &[], &content_id).unwrap();
                    assert_eq!(content.role(), Some(Role::Region));
                    assert_eq!(content.label(), Some("Configuration details"));
                    assert!(content.bounds().top() >= trigger.bounds().bottom());
                    for node in [&trigger, &content] {
                        assert!(node.bounds().left() >= trace.bounds().left());
                        assert!(node.bounds().right() <= trace.bounds().right() + px(1.));
                    }
                    for (part, expected, role) in [
                        ("summary", summary.as_str(), None),
                        ("indicator", "−", None),
                        ("body", details.as_str(), Some(Role::Paragraph)),
                    ] {
                        let part_id = (id.clone(), part).into();
                        let node = test_support::find(window, &[], &part_id).unwrap();
                        assert_eq!(node.role(), role);
                        assert_eq!(node.label(), role.map(|_| expected));
                        assert_eq!(node.focused(), None);
                        assert!(node.bounds().left() >= trace.bounds().left());
                        assert!(node.bounds().right() <= trace.bounds().right() + px(1.));
                        assert!(node.bounds().bottom() <= trace.bounds().bottom() + px(1.));
                        let texts = test_support::texts(window);
                        let text = texts
                            .iter()
                            .find(|text| text.path().contains(&part_id))
                            .unwrap();
                        assert_eq!(text.text(), expected);
                        assert!(text.visible());
                        assert!(text.bounds().size.width <= node.bounds().size.width + px(1.));
                    }
                    let named: Vec<_> = test_support::snapshots(window)
                        .into_iter()
                        .filter(|node| node.path().contains(&id) && node.label().is_some())
                        .collect();
                    assert_eq!(named.len(), 3);
                    assert!(
                        test_support::find(window, &[], &(id.clone(), "action").into()).is_none()
                    );
                    let height = trace.bounds().size.height;
                    match wide_height {
                        None => wide_height = Some(height),
                        Some(wide) if width == 360. => assert_eq!(height, wide),
                        Some(wide) => assert!(height > wide),
                    }
                });
            }
            cx.update(|window, cx| {
                let state = sample.read(cx).state.clone();
                state.update(cx, |state, cx| state.set_open(false, window, cx));
            });
            draw(cx);
            cx.update(|window, _| {
                let trigger = test_support::find(window, &[], &trigger_id).unwrap();
                assert_eq!(trigger.expanded(), Some(false));
                assert!(test_support::find(window, &[], &content_id).is_none());
                assert!(test_support::find(window, &[], &(id.clone(), "body").into()).is_none());
            });
        }
    }

    #[gpui::test]
    fn trace_actions_and_replacement_preserve_collapsible_focus(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let (cx, sample) = harness(cx, Appearance::Settings);
        let (trigger_id, content_id) = part_ids(&sample, cx);
        cx.update(|window, _| assert!(test_support::find(window, &[], &content_id).is_none()));
        cx.simulate_keystrokes("tab");
        activate_key(cx, "enter");
        cx.update(|window, cx| {
            let state = sample.read(cx).state.read(cx);
            assert!(state.is_open());
            assert!(state.trigger_focus_handle().is_focused(window));
        });
        // Static body and Region add no Tab stop.
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(sample.read(cx).focus[2].is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        activate_key(cx, "space");
        cx.update(|window, cx| {
            assert!(!sample.read(cx).state.read(cx).is_open());
            assert!(test_support::find(window, &[], &content_id).is_none());
        });
        let position = cx.update(|window, _| {
            test_support::find(window, &[], &trigger_id)
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
        sample.update(cx, |sample, cx| {
            sample.action = true;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(sample.read(cx).focus[1].is_focused(window)));
        sample.update(cx, |sample, cx| {
            sample.summary = "Updated configuration summary".into();
            sample.details = "Updated retained details.\n다시 구성한 본문입니다.".into();
            cx.notify();
        });
        draw(cx);
        cx.update(|window, cx| {
            let sample = sample.read(cx);
            assert!(sample.state.read(cx).is_open());
            assert!(sample.focus[1].is_focused(window));
            let trigger = test_support::find(window, &[], &trigger_id).unwrap();
            assert_eq!(trigger.label(), Some("Updated configuration summary"));
            let body = test_support::find(
                window,
                &[],
                &(ElementId::from("sample-trace"), "body").into(),
            )
            .unwrap();
            assert_eq!(body.label(), Some(sample.details.as_ref()));
        });
        activate_key(cx, "enter");
        cx.update(|window, cx| {
            let sample = sample.read(cx);
            assert_eq!(sample.activations, 1);
            assert!(!sample.state.read(cx).is_open());
            assert!(
                sample
                    .state
                    .read(cx)
                    .trigger_focus_handle()
                    .is_focused(window)
            );
            assert!(test_support::find(window, &[], &content_id).is_none());
            assert!(test_support::find(window, &[], &"trace-action".into()).is_none());
        });
        activate_key(cx, "space");
        cx.simulate_keystrokes("tab shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                sample
                    .read(cx)
                    .state
                    .read(cx)
                    .trigger_focus_handle()
                    .is_focused(window)
            );
        });
        sample.update(cx, |sample, cx| {
            sample.action = false;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(sample.read(cx).focus[2].is_focused(window));
            assert!(sample.read(cx).state.read(cx).is_open());
            assert!(test_support::find(window, &[], &"trace-action".into()).is_none());
        });
    }

    #[gpui::test]
    fn disabled_trace_trigger_keeps_content_and_existing_closure_target(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let (cx, sample) = harness(cx, Appearance::Compact);
        let (trigger_id, content_id) = part_ids(&sample, cx);
        sample.update(cx, |sample, cx| {
            sample.width = px(160.);
            sample.action = true;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        activate_key(cx, "enter");
        cx.update(|window, cx| {
            let state = sample.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        draw(cx);
        let trigger_position = cx.update(|window, _| {
            let trigger = test_support::find(window, &[], &trigger_id).unwrap();
            assert_eq!(trigger.disabled(), Some(true));
            assert_eq!(trigger.expanded(), Some(true));
            let action = test_support::find(window, &[], &"trace-action".into()).unwrap();
            // The snapshot exposes AccessKit's set flag; enabled has no flag.
            assert_eq!(action.disabled(), None);
            trigger.bounds().center()
        });
        activate_key(cx, "enter");
        activate_key(cx, "space");
        cx.simulate_click(trigger_position, Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            assert!(sample.read(cx).state.read(cx).is_open());
            assert!(test_support::find(window, &[], &content_id).is_some());
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        sample.update(cx, |sample, cx| {
            sample.action_disabled = true;
            cx.notify();
        });
        draw(cx);
        let action_position = cx.update(|window, cx| {
            assert!(sample.read(cx).focus[1].is_focused(window));
            let action = test_support::find(window, &[], &"trace-action".into()).unwrap();
            assert_eq!(action.disabled(), Some(true));
            action.bounds().center()
        });
        activate_key(cx, "enter");
        activate_key(cx, "space");
        cx.simulate_click(action_position, Modifiers::default());
        draw(cx);
        assert_eq!(sample.read_with(cx, |sample, _| sample.activations), 0);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(sample.read(cx).focus[2].is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(sample.read(cx).focus[0].is_focused(window)));
        sample.update(cx, |sample, cx| {
            sample.action_disabled = false;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(sample.read(cx).focus[1].is_focused(window));
        });
        // The content action still works while the disclosure trigger is disabled.
        activate_key(cx, "enter");
        cx.update(|window, cx| {
            assert_eq!(sample.read(cx).activations, 1);
            assert!(!sample.read(cx).state.read(cx).is_open());
            let trace = test_support::find(window, &[], &"sample-trace".into()).unwrap();
            assert_eq!(trace.focused(), Some(true));
            assert!(test_support::find(window, &[], &content_id).is_none());
            assert!(test_support::find(window, &[], &"trace-action".into()).is_none());
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(sample.read(cx).focus[2].is_focused(window)));
    }
}
