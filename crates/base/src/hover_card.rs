//! Noninteractive rich previews with retained hover and keyboard-focus timing.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    Anchor, AnyElement, App, Context, ElementId, Entity, FocusHandle, HoverListenerMode,
    InteractiveElement as _, IntoElement, MouseDownEvent, ParentElement as _, Pixels, RenderOnce,
    Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription,
    Task, Window, canvas, div, prelude::FluentBuilder as _, px,
};

use crate::{Button, ElementExt as _, Popup, StyledExt as _, TestSupportExt as _};

const OPEN_DELAY: Duration = Duration::from_millis(500);
const CLOSE_DELAY: Duration = Duration::from_millis(200);

/// Retained visibility for a rich, noninteractive preview of a Button's action.
///
/// Pointer entry waits before opening; leaving both trigger and card waits
/// before closing, so the pointer can cross their gap. Keyboard focus opens
/// immediately. The preview never moves focus and adds no Tab stops. Escape,
/// pointer presses, trigger activation keys, focus changes, and window
/// deactivation dismiss it. Explicit dismissal suppresses reopening until a
/// fresh trigger entry or keyboard focus.
///
/// Keep the matching HoverCard mounted with its trigger and observe this state
/// to redraw its consumer. The mount cancels delayed work when removed, even
/// if the application retains the state for later reuse.
pub struct HoverCardState {
    focus: FocusHandle,
    open: bool,
    disabled: bool,
    mounted: bool,
    trigger_hovered: bool,
    content_hovered: bool,
    keyboard_focused: bool,
    suppressed: bool,
    open_delay: Duration,
    close_delay: Duration,
    open_task: Option<Task<()>>,
    close_task: Option<Task<()>>,
    focus_when_open: Option<FocusHandle>,
    focus_watch: Option<Subscription>,
    _subscriptions: Vec<Subscription>,
}

impl HoverCardState {
    /// Uses the same focus handle as the underlying Button and application.
    pub fn new(focus: &FocusHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.on_focus(focus, window, |state, window, cx| {
                if window.last_input_was_keyboard() {
                    state.keyboard_focused = true;
                    state.suppressed = false;
                    state.show(window, cx);
                }
            }),
            cx.on_blur(focus, window, |state, window, cx| {
                state.keyboard_focused = false;
                state.close(window, cx);
                if !state.trigger_hovered {
                    state.suppressed = false;
                }
            }),
            cx.observe_window_activation(window, |state, window, cx| {
                if !window.is_window_active() {
                    state.close(window, cx);
                }
            }),
        ];
        Self {
            focus: focus.clone(),
            open: false,
            disabled: false,
            mounted: false,
            trigger_hovered: false,
            content_hovered: false,
            keyboard_focused: false,
            suppressed: false,
            open_delay: OPEN_DELAY,
            close_delay: CLOSE_DELAY,
            open_task: None,
            close_task: None,
            focus_when_open: None,
            focus_watch: None,
            _subscriptions: subscriptions,
        }
    }

    /// Configures pointer entry and exit delays before mounting.
    /// Keyboard focus remains immediate. Defaults are 500 ms and 200 ms.
    pub fn with_delays(mut self, open: Duration, close: Duration) -> Self {
        self.open_delay = open;
        self.close_delay = close;
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// Coordinates disabled state for the preview and its native Button.
    /// Enabling waits for another pointer entry or keyboard focus.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.close(window, cx);
            self.disabled = disabled;
            cx.notify();
            window.refresh();
        }
    }

    /// Dismisses without moving focus, suppressing the current hover/focus entry.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hide(true, window, cx);
    }

    fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_task = None;
        self.close_task = None;
        if self.open
            || self.disabled
            || self.suppressed
            || !self.mounted
            || !window.is_window_active()
        {
            return;
        }
        self.open = true;
        self.focus_when_open = window.focused(cx);
        // A pointer preview can open while another control owns focus. Close
        // when that owner changes, before a new menu or dialog receives keys.
        self.focus_watch = self.focus_when_open.as_ref().map(|focus| {
            let expected = focus.clone();
            cx.on_blur(focus, window, move |state, window, cx| {
                if state.focus_when_open.as_ref() == Some(&expected) {
                    state.close(window, cx);
                }
            })
        });
        cx.notify();
        window.refresh();
    }

    fn hide(&mut self, suppressed: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open_task = None;
        self.close_task = None;
        self.focus_watch = None;
        self.focus_when_open = None;
        self.content_hovered = false;
        self.keyboard_focused = false;
        self.suppressed = suppressed;
        if self.open {
            self.open = false;
            cx.notify();
            window.refresh();
        }
    }

    fn hover(&mut self, content: bool, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        let previous = if content {
            &mut self.content_hovered
        } else {
            &mut self.trigger_hovered
        };
        if *previous == hovered {
            return;
        }
        *previous = hovered;
        if hovered {
            self.close_task = None;
            if !content {
                self.suppressed = false;
                if !self.open && !self.disabled && self.open_task.is_none() {
                    let delay = self.open_delay;
                    self.open_task = Some(cx.spawn_in(window, async move |state, cx| {
                        cx.background_executor().timer(delay).await;
                        let _ = state.update_in(cx, |state, window, cx| {
                            if state.trigger_hovered {
                                state.show(window, cx);
                            }
                        });
                    }));
                }
            }
        } else if !self.trigger_hovered && !self.content_hovered && !self.keyboard_focused {
            self.schedule_close(window, cx);
        }
    }

    fn schedule_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_task = None;
        if !self.open || self.close_task.is_some() {
            return;
        }
        let delay = self.close_delay;
        self.close_task = Some(cx.spawn_in(window, async move |state, cx| {
            cx.background_executor().timer(delay).await;
            let _ = state.update_in(cx, |state, window, cx| {
                if !state.trigger_hovered && !state.content_hovered && !state.keyboard_focused {
                    state.hide(false, window, cx);
                }
            });
        }));
    }

    fn unmount(&mut self, cx: &mut Context<Self>) {
        self.mounted = false;
        self.open_task = None;
        self.close_task = None;
        self.focus_watch = None;
        self.focus_when_open = None;
        self.trigger_hovered = false;
        self.content_hovered = false;
        self.keyboard_focused = false;
        self.suppressed = false;
        if self.open {
            self.open = false;
            cx.notify();
        }
    }
}

struct HoverCardMount {
    visible: Rc<Cell<bool>>,
    _keys: Subscription,
    _release: Subscription,
}

/// A native Button with an application-owned, noninteractive rich preview.
///
/// The trigger retains its normal action; the card only adds preview behavior.
/// Supply noninteractive content and keep essential information available in
/// the trigger's accessible description or its ordinary destination. The card
/// is a supplementary tooltip surface with no focus stop. `accessibility_label`
/// names that surface; name the Button separately in its builder.
///
/// [`Styled`] applies to the host frame. Style the content's surface yourself.
/// [`Popup`] supplies measured positioning, window-edge clamping, deferred
/// painting, and occlusion. Interactive popup content belongs in Popover.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    state: Entity<HoverCardState>,
    trigger: Button,
    content: Option<AnyElement>,
    label: Option<SharedString>,
    anchor: Anchor,
    offset: Pixels,
    style: StyleRefinement,
}

impl HoverCard {
    pub fn new(id: impl Into<ElementId>, state: &Entity<HoverCardState>) -> Self {
        let id = id.into();
        Self {
            trigger: Button::new(id.clone()),
            id,
            state: state.clone(),
            content: None,
            label: None,
            anchor: Anchor::TopLeft,
            offset: px(0.),
            style: StyleRefinement::default(),
        }
    }

    pub fn trigger(mut self, build: impl FnOnce(Button) -> Button) -> Self {
        self.trigger = build(self.trigger);
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    pub fn offset(mut self, offset: Pixels) -> Self {
        self.offset = offset;
        self
    }
}

impl Styled for HoverCard {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let weak_state = self.state.downgrade();
        let mount =
            window.use_keyed_state((self.id.clone(), "hover-card-mount"), cx, |window, cx| {
                let _ = weak_state.update(cx, |state, _| state.mounted = true);
                let visible = Rc::new(Cell::new(false));
                let escape_visible = visible.clone();
                let release_visible = visible.clone();
                let release_state = weak_state.clone();
                let owner_window = window.window_handle();
                let keys = cx.intercept_keystrokes(move |event, window, cx| {
                    if window.window_handle() != owner_window {
                        return;
                    }
                    let _ = weak_state.update(cx, |state, cx| match event.keystroke.key.as_str() {
                        "escape" if state.open || state.open_task.is_some() => {
                            let consume = state.open
                                && escape_visible.get()
                                && state.focus_when_open == window.focused(cx);
                            state.close(window, cx);
                            if consume {
                                cx.stop_propagation();
                            }
                        }
                        "enter" | "space" if state.focus.is_focused(window) => {
                            state.close(window, cx);
                        }
                        _ => {}
                    });
                });
                let release = cx.on_release(move |_, cx| {
                    release_visible.set(false);
                    let _ = release_state.update(cx, |state, cx| state.unmount(cx));
                });
                HoverCardMount {
                    visible,
                    _keys: keys,
                    _release: release,
                }
            });
        let visible = mount.read(cx).visible.clone();
        visible.set(false);
        if self.state.read(cx).open && self.state.read(cx).focus_when_open != window.focused(cx) {
            self.state.update(cx, |state, cx| state.close(window, cx));
        }
        let model = self.state.read(cx);
        let open = model.open;
        let disabled = model.disabled;
        let focus = model.focus.clone();
        let trigger_state = self.state.clone();
        let content_state = self.state.clone();
        let trigger_visible = Rc::new(Cell::new(false));
        let content_trigger_visible = trigger_visible.clone();
        let trigger = self
            .trigger
            .track_focus(&focus)
            .disabled(disabled)
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                trigger_state.update(cx, |state, cx| state.hover(false, *hovered, window, cx));
            });
        let mut popup = Popup::new((self.id.clone(), "hover-card-host"), trigger)
            .anchor(self.anchor)
            .offset(self.offset);
        if open && let Some(content) = self.content {
            popup = popup.content(
                div()
                    .id((self.id.clone(), "hover-card-content"))
                    .test_support()
                    .role(Role::Tooltip)
                    .when_some(self.label, |this, label| this.aria_label(label))
                    .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
                    .on_hover(move |hovered, window, cx| {
                        content_state
                            .update(cx, |state, cx| state.hover(true, *hovered, window, cx));
                    })
                    .on_prepaint(move |bounds, window, _| {
                        visible.set(
                            content_trigger_visible.get()
                                && bounds.intersects(&window.content_mask().bounds),
                        );
                    })
                    .child(content),
            );
        }
        let geometry_state = self.state.clone();
        let press_state = self.state;
        div()
            .refine_style(&self.style)
            .relative()
            .child(popup)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let visible = bounds.intersects(&window.content_mask().bounds);
                        trigger_visible.set(visible);
                        if !visible && geometry_state.read(cx).open {
                            window.defer(cx, move |window, cx| {
                                geometry_state.update(cx, |state, cx| state.close(window, cx));
                            });
                        }
                    },
                    move |_, _, window, _| {
                        // Closing a preview never consumes the original click.
                        window.on_mouse_event(move |_: &MouseDownEvent, phase, window, cx| {
                            if phase.capture() {
                                press_state.update(cx, |state, cx| state.close(window, cx));
                            }
                        });
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Root, actions::Cancel};
    use gpui::{
        AppContext as _, KeyBinding, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render,
        TestAppContext, VisualTestContext, point,
    };

    struct Harness {
        state: Entity<HoverCardState>,
        before: FocusHandle,
        trigger: FocusHandle,
        after: FocusHandle,
        show_card: bool,
        activations: Rc<Cell<usize>>,
        escapes: Rc<Cell<usize>>,
        _observation: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let activations = self.activations.clone();
            let escapes = self.escapes.clone();
            div()
                .id("hover-card-test")
                .key_context("HoverCardTest")
                .size_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .on_action(move |_: &Cancel, _, _| escapes.set(escapes.get() + 1))
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .w(px(160.))
                        .h(px(32.))
                        .child("Before"),
                )
                .when(self.show_card, |this| {
                    this.child(
                        HoverCard::new("preview", &self.state)
                            .w(px(160.))
                            .offset(px(8.))
                            .accessibility_label("Recording preview")
                            .trigger(move |button| {
                                button
                                    .debug_selector(|| "preview-trigger".into())
                                    .w_full()
                                    .h(px(32.))
                                    .accessibility_label("Preset details")
                                    .child("Preset details")
                                    .on_click(move |_, _, _| activations.set(activations.get() + 1))
                            })
                            .content(
                                div()
                                    .debug_selector(|| "preview-content".into())
                                    .w(px(220.))
                                    .h(px(72.))
                                    .child("Balanced recording preset"),
                            ),
                    )
                })
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .w(px(160.))
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    fn settle(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn harness(cx: &mut TestAppContext) -> (&mut VisualTestContext, Entity<Harness>) {
        cx.update(|cx| {
            crate::init(cx);
            cx.bind_keys([KeyBinding::new("escape", Cancel, Some("HoverCardTest"))]);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let trigger = cx.focus_handle();
                let state = cx.new(|cx| HoverCardState::new(&trigger, window, cx));
                let observation = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    before: cx.focus_handle(),
                    trigger,
                    after: cx.focus_handle(),
                    show_card: true,
                    activations: Rc::default(),
                    escapes: Rc::default(),
                    _observation: observation,
                }
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            view.read(cx).after.clone().focus(window, cx);
        });
        settle(cx);
        (cx, view)
    }

    fn enter(cx: &mut VisualTestContext) {
        let keystroke =
            cx.update(|_, cx| Keystroke::parse_with_os("enter", cx.platform_info().os).unwrap());
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
        settle(cx);
    }

    #[gpui::test]
    fn hover_delays_crossing_the_gap_and_clicks_preserve_focus(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx);
        let trigger_bounds = cx.debug_bounds("preview-trigger").unwrap();
        let trigger = trigger_bounds.center();
        let outside = point(px(4.), px(4.));
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY / 2);
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));

        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.state.read(cx).is_open());
            assert!(view.after.is_focused(window));
            #[cfg(feature = "test-support")]
            assert!(
                crate::test_support::snapshots(window)
                    .iter()
                    .any(|item| item.role() == Some(Role::Tooltip)
                        && item.label() == Some("Recording preview")
                        && item.visible())
            );
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.simulate_mouse_move(trigger + point(px(1.), px(0.)), None, Modifiers::default());
        cx.executor().advance_clock(OPEN_DELAY * 2);
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        let content = cx.debug_bounds("preview-content").unwrap().center();
        let gap = point(trigger.x, trigger_bounds.bottom_left().y + px(4.));
        cx.simulate_mouse_move(gap, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(CLOSE_DELAY / 2);
        cx.simulate_mouse_move(content, None, Modifiers::default());
        cx.executor().advance_clock(CLOSE_DELAY * 2);
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(
                view.state.read(cx).is_open(),
                "entering the card cancels its close delay"
            );
            assert!(view.after.is_focused(window));
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.state.read(cx).is_open());
            assert!(view.after.is_focused(window));
            assert_eq!(view.escapes.get(), 0);
        });
        // The removed card's old hover flag must not retain a later preview.
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(CLOSE_DELAY / 2);
        cx.update(|_, cx| assert!(view.read(cx).state.read(cx).is_open()));
        cx.executor().advance_clock(CLOSE_DELAY);
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));

        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        cx.simulate_click(trigger, Modifiers::default());
        settle(cx);
        cx.executor().advance_clock(OPEN_DELAY * 2);
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert_eq!(view.activations.get(), 1);
            assert!(view.trigger.is_focused(window));
            assert!(!view.state.read(cx).is_open());
        });
    }

    #[gpui::test]
    fn keyboard_dismissal_disabled_and_retained_mounts_keep_their_owners(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx);
        cx.simulate_keystrokes("shift-tab");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.trigger.is_focused(window));
            assert!(view.state.read(cx).is_open());
        });
        cx.simulate_mouse_move(point(px(4.), px(4.)), None, Modifiers::default());
        cx.executor().advance_clock(CLOSE_DELAY * 2);
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.trigger.is_focused(window));
            assert!(
                view.state.read(cx).is_open(),
                "pointer movement does not blur keyboard focus"
            );
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.trigger.is_focused(window));
            assert!(!view.state.read(cx).is_open());
            assert_eq!(view.escapes.get(), 0);
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).escapes.get(), 1));
        cx.simulate_keystrokes("tab shift-tab");
        settle(cx);
        enter(cx);
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert!(!view.state.read(cx).is_open());
            assert_eq!(view.activations.get(), 1);
        });
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        settle(cx);
        cx.simulate_keystrokes("tab shift-tab");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.before.is_focused(window));
            assert!(!view.state.read(cx).is_open());
        });
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(false, window, cx));
            view.read(cx).after.clone().focus(window, cx);
        });
        settle(cx);

        let trigger = cx.debug_bounds("preview-trigger").unwrap().center();
        let outside = point(px(4.), px(4.));
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY / 2);
        view.update(cx, |view, cx| {
            view.show_card = false;
            cx.notify();
        });
        settle(cx);
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        view.update(cx, |view, cx| {
            view.show_card = true;
            cx.notify();
        });
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).state.read(cx).is_open()));

        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(OPEN_DELAY);
        settle(cx);
        cx.update(|window, cx| view.read(cx).before.clone().focus(window, cx));
        // A new focus owner receives Escape even before the next paint.
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.before.is_focused(window));
            assert!(!view.state.read(cx).is_open());
            assert_eq!(view.escapes.get(), 2);
        });
    }
}
