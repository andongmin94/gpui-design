//! Timed notifications that preserve time while a user is interacting with them.

use std::time::Duration;

use gpui::{
    AnyElement, App, Bounds, Context, Element, ElementId, Entity, EventEmitter, FocusHandle,
    GlobalElementId, HoverListenerMode, InspectorElementId, InteractiveElement as _, IntoElement,
    KeyBinding, LayoutId, ParentElement, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Task, WeakFocusHandle,
    Window, accesskit, div, prelude::FluentBuilder as _,
};
use web_time::Instant;

use crate::{Button, StyledExt as _, TestSupportExt as _, actions::Cancel};

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some("Toast"))]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastDismissReason {
    Timeout,
    CloseButton,
    Escape,
    Imperative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastEvent {
    Opened,
    Dismissed(ToastDismissReason),
}

/// One retained notification. Applications own its message, actions and stacking.
///
/// Showing a toast never takes focus. Its countdown starts when mounted and
/// pauses while hovered, while it or a descendant owns focus, and while the
/// window is inactive. Resuming spends only the remaining duration. Explicitly
/// dismissing a focused toast restores the focus captured when it was shown;
/// closing an unfocused toast never moves another control's focus.
pub struct ToastState {
    focus: FocusHandle,
    previous_focus: Option<WeakFocusHandle>,
    duration: Duration,
    remaining: Duration,
    running_since: Option<Instant>,
    timer: Option<Task<()>>,
    open: bool,
    mounted: bool,
    hovered: bool,
    focused: bool,
    _subscriptions: Vec<Subscription>,
}

impl ToastState {
    /// Uses a five-second lifetime; configure it before mounting with
    /// [`Self::with_duration`]. Keep the matching [`Toast`] mounted even when closed.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(false);
        let subscriptions = vec![
            cx.on_focus_in(&focus, window, |state, _, cx| {
                state.focused = true;
                state.pause(cx);
            }),
            cx.on_focus_out(&focus, window, |state, _, window, cx| {
                state.focused = false;
                state.resume(window, cx);
            }),
            cx.observe_window_activation(window, |state, window, cx| {
                if window.is_window_active() {
                    state.resume(window, cx);
                } else {
                    state.pause(cx);
                }
            }),
        ];
        let duration = Duration::from_secs(5);
        Self {
            focus,
            previous_focus: None,
            duration,
            remaining: duration,
            running_since: None,
            timer: None,
            open: false,
            mounted: false,
            hovered: false,
            focused: false,
            _subscriptions: subscriptions,
        }
    }

    /// A nonzero amount of active, unattended time before dismissal.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        assert!(!duration.is_zero(), "Toast duration must be nonzero");
        self.duration = duration;
        self.remaining = duration;
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The non-Tab-stop notification scope. Its action buttons keep their own
    /// ordinary Tab stops; applications may use this handle to announce/focus
    /// the notification on an explicit user request.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// Shows or replaces the notification and restarts its full lifetime.
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.timer = None;
        self.running_since = None;
        self.remaining = self.duration;
        if !self.open {
            self.previous_focus = window.focused(cx).map(|focus| focus.downgrade());
            self.open = true;
            cx.emit(ToastEvent::Opened);
        }
        self.resume(window, cx);
        cx.notify();
        window.refresh();
    }

    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_with_reason(ToastDismissReason::Imperative, window, cx);
    }

    fn owns_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.is_focused(window) || self.focus.contains_focused(window, cx)
    }

    fn pause(&mut self, cx: &mut Context<Self>) {
        self.timer = None;
        if let Some(started) = self.running_since.take() {
            self.remaining = self
                .remaining
                .saturating_sub(cx.background_executor().now().duration_since(started));
        }
    }

    fn resume(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open
            || !self.mounted
            || self.hovered
            || self.focused
            || self.owns_focus(window, cx)
            || !window.is_window_active()
            || self.timer.is_some()
        {
            return;
        }
        self.running_since = Some(cx.background_executor().now());
        let remaining = self.remaining;
        self.timer = Some(cx.spawn_in(window, async move |state, cx| {
            cx.background_executor().timer(remaining).await;
            let _ = state.update_in(cx, |state, window, cx| {
                state.timer = None;
                state.running_since = None;
                state.remaining = Duration::ZERO;
                if state.hovered || state.focused || state.owns_focus(window, cx) {
                    return;
                }
                state.dismiss_with_reason(ToastDismissReason::Timeout, window, cx);
            });
        }));
    }

    fn hover(&mut self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.hovered = hovered;
        if hovered {
            self.pause(cx);
        } else {
            self.resume(window, cx);
        }
    }

    fn dismiss_with_reason(
        &mut self,
        reason: ToastDismissReason,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.timer = None;
        self.running_since = None;
        if !self.open {
            return;
        }
        self.open = false;
        let owns_focus = self.owns_focus(window, cx);
        let restore = self.previous_focus.take().and_then(|focus| focus.upgrade());
        self.hovered = false;
        self.focused = false;
        if owns_focus {
            if let Some(focus) = restore {
                focus.focus(window, cx);
            } else {
                window.blur(cx);
            }
        }
        cx.emit(ToastEvent::Dismissed(reason));
        cx.notify();
        window.refresh();
    }
}

impl EventEmitter<ToastEvent> for ToastState {}

struct ToastMount {
    _release: Subscription,
}

// GPUI has no live-region builder. Project native metadata onto the same
// surface, without a second layout, focus or accessibility node.
struct ToastLive<E> {
    inner: E,
    live: Option<accesskit::Live>,
}

impl<E: Element> IntoElement for ToastLive<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for ToastLive<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.inner
            .prepaint(id, inspector, bounds, layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }

    fn a11y_role(&self) -> Option<Role> {
        self.inner.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if let Some(live) = self.live {
            node.set_live(live);
            node.set_live_atomic();
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        Element::a11y_synthetic_children(&mut self.inner, prepaint, builder);
    }
}

/// A nonmodal status surface. The consumer chooses its location and content.
///
/// The surface has a native Status role (Alert with [`Self::urgent`]), a focus
/// scope for its actions, and no automatic focus or modal trap. Escape dismisses
/// only while focus is inside this toast. Hover uses either input modality.
#[derive(IntoElement)]
pub struct Toast {
    id: ElementId,
    state: Entity<ToastState>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    label: Option<SharedString>,
    urgent: bool,
}

impl Toast {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ToastState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            label: None,
            urgent: false,
        }
    }

    /// The complete notification text announced as an atomic live update.
    /// Supplies both native name and value for platform announcement adapters.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn urgent(mut self, urgent: bool) -> Self {
        self.urgent = urgent;
        self
    }
}

impl Styled for Toast {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Toast {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for Toast {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let weak = self.state.downgrade();
        let owner_window = window.window_handle();
        window.with_element_namespace(self.id.clone(), |window| {
            window.use_keyed_state(("toast-mount", self.state.entity_id()), cx, |_, cx| {
                ToastMount {
                    _release: cx.on_release(move |_, cx| {
                        let _ = owner_window.update(cx, |_, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.mounted = false;
                                // Removing the host can also remove its return target.
                                state.previous_focus = None;
                                state.dismiss(window, cx);
                            });
                        });
                    }),
                }
            })
        });
        self.state.update(cx, |state, cx| {
            state.mounted = true;
            state.resume(window, cx);
        });
        let model = self.state.read(cx);
        let base = div().id(self.id).test_support();
        if !model.open {
            return ToastLive {
                inner: base,
                live: None,
            };
        }
        let focus = model.focus.clone();
        let hover = self.state.clone();
        let dismiss = self.state;
        let surface = base
            .role(if self.urgent {
                Role::Alert
            } else {
                Role::Status
            })
            .when_some(self.label, |this, label| {
                this.aria_label(label.clone()).aria_value(label)
            })
            .key_context("Toast")
            .track_focus(&focus)
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                hover.update(cx, |state, cx| state.hover(*hovered, window, cx));
            })
            .on_action(move |_: &Cancel, window, cx| {
                dismiss.update(cx, |state, cx| {
                    state.dismiss_with_reason(ToastDismissReason::Escape, window, cx);
                });
            })
            .children(self.children)
            .refine_style(&self.style);
        ToastLive {
            inner: surface,
            live: Some(if self.urgent {
                accesskit::Live::Assertive
            } else {
                accesskit::Live::Polite
            }),
        }
    }
}

/// A normal Button whose activation safely dismisses this notification.
#[derive(IntoElement)]
pub struct ToastClose {
    button: Button,
}

impl ToastClose {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ToastState>) -> Self {
        let state = state.clone();
        Self {
            button: Button::new(id)
                .accessibility_label("Dismiss notification")
                .on_click(move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.dismiss_with_reason(ToastDismissReason::CloseButton, window, cx);
                    });
                }),
        }
    }

    pub fn trigger(mut self, build: impl FnOnce(Button) -> Button) -> Self {
        self.button = build(self.button);
        self
    }
}

impl RenderOnce for ToastClose {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.button
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, Modifiers, Render, TestAppContext, point, px};

    use super::*;

    struct Harness {
        state: Entity<ToastState>,
        trigger: FocusHandle,
        close: FocusHandle,
        urgent: bool,
        show_host: bool,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            if !self.show_host {
                return div().into_any_element();
            }
            let close = self.close.clone();
            let surface = Toast::new("notice", &self.state)
                .accessibility_label("Saved locally")
                .urgent(self.urgent)
                .w(px(240.))
                .h(px(60.))
                .child("Saved locally")
                .child(
                    ToastClose::new("close-notice", &self.state)
                        .trigger(|button| button.track_focus(&close).child("Dismiss")),
                )
                .render(window, cx)
                .into_element();
            if self.state.read(cx).is_open() {
                let role = if self.urgent {
                    Role::Alert
                } else {
                    Role::Status
                };
                let mut node = accesskit::Node::new(role);
                surface.write_a11y_info(&mut node);
                assert_eq!(surface.a11y_role(), Some(role));
                assert_eq!(node.label(), Some("Saved locally"));
                assert_eq!(node.value(), Some("Saved locally"));
                assert_eq!(
                    node.live(),
                    Some(if self.urgent {
                        accesskit::Live::Assertive
                    } else {
                        accesskit::Live::Polite
                    })
                );
                assert!(node.is_live_atomic());
            }
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(
                    Button::new("trigger")
                        .track_focus(&self.trigger)
                        .h(px(30.))
                        .child("Show"),
                )
                .child(
                    div()
                        .debug_selector(|| "notice".into())
                        .w(px(240.))
                        .h(px(60.))
                        .child(surface),
                )
                .into_any_element()
        }
    }

    #[gpui::test]
    fn lifetime_pauses_for_hover_and_focus_and_dismissal_restores_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let state =
                    cx.new(|cx| ToastState::new(window, cx).with_duration(Duration::from_secs(4)));
                Harness {
                    _subscription: cx.observe(&state, |_, _, cx| cx.notify()),
                    state,
                    trigger: cx.focus_handle(),
                    close: cx.focus_handle(),
                    urgent: false,
                    show_host: true,
                }
            });
            crate::Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            view.read(cx).trigger.clone().focus(window, cx);
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.show(window, cx));
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(1));
        let notice = cx.debug_bounds("notice").unwrap().center();
        cx.simulate_mouse_move(notice, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(8));
        cx.run_until_parked();
        cx.update(|_, cx| assert!(view.read(cx).state.read(cx).is_open()));
        cx.simulate_mouse_move(point(px(600.), px(400.)), None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(view.read(cx).state.read(cx).is_open());
            view.read(cx).close.clone().focus(window, cx);
            view.update(cx, |view, cx| {
                view.urgent = true;
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(8));
        cx.run_until_parked();
        cx.update(|_, cx| assert!(view.read(cx).state.read(cx).is_open()));
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!view.read(cx).state.read(cx).is_open());
            assert!(view.read(cx).trigger.is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.show(window, cx));
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(4));
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!view.read(cx).state.read(cx).is_open());
            assert!(view.read(cx).trigger.is_focused(window));
        });
        let replaced = cx.update(|window, cx| {
            let previous = view.read(cx).state.clone();
            previous.update(cx, |state, cx| state.show(window, cx));
            window.draw(cx).clear(cx);
            let replacement =
                cx.new(|cx| ToastState::new(window, cx).with_duration(Duration::from_secs(4)));
            replacement.update(cx, |state, cx| state.show(window, cx));
            view.update(cx, |view, cx| {
                view._subscription = cx.observe(&replacement, |_, _, cx| cx.notify());
                view.state = replacement;
                cx.notify();
            });
            window.draw(cx).clear(cx);
            previous
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(
                !replaced.read(cx).is_open(),
                "replacing the state at the same UI ID must release the old notification"
            );
            assert!(view.read(cx).state.read(cx).is_open());
        });
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.show(window, cx));
            state.read(cx).focus.clone().focus(window, cx);
            window.draw(cx).clear(cx);
            view.update(cx, |view, cx| {
                view.show_host = false;
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!view.read(cx).state.read(cx).is_open());
            assert!(
                !view.read(cx).trigger.is_focused(window),
                "unmounting a notification must not focus its hidden trigger"
            );
        });
    }
}
