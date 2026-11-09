//! Noninteractive text tooltips using GPUI's native tooltip layer.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    AnyTooltip, App, AppContext as _, Context, ElementId, Entity, FocusHandle,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render, RenderOnce,
    Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription,
    Task, Window, canvas, div, point, px,
};

use crate::{Button, StyledExt as _, TestSupportExt as _};

const HOVER_DELAY: Duration = Duration::from_millis(500);

/// Retained visibility for a tooltip attached to an existing button focus handle.
///
/// Keyboard focus reveals the description immediately; pointer hover waits
/// 500 ms. Leaving the trigger cancels a pending hover. Blur, activation, and
/// Escape dismiss the tooltip without moving focus. Escape suppresses reopening
/// until another focus or pointer entry.
///
/// Keep the matching [`Tooltip`] mounted with its trigger. Its text and visual
/// styles belong to the application; this state does not own a second focus stop.
pub struct TooltipState {
    focus: FocusHandle,
    hovered: bool,
    open: bool,
    suppressed: bool,
    hover_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl TooltipState {
    pub fn new(focus: &FocusHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.on_focus(focus, window, |state, window, cx| {
                // A pointer press dismisses before Button gives itself focus.
                // Do not immediately reopen during that same activation.
                if state.suppressed && !window.last_input_was_keyboard() {
                    return;
                }
                state.suppressed = false;
                state.show(window, cx);
            }),
            cx.on_blur(focus, window, |state, window, cx| {
                state.dismiss(window, cx);
                if !state.hovered {
                    state.suppressed = false;
                }
            }),
            cx.observe_window_activation(window, |state, window, cx| {
                if !window.is_window_active() {
                    state.dismiss(window, cx);
                }
            }),
        ];
        Self {
            focus: focus.clone(),
            hovered: false,
            open: false,
            suppressed: false,
            hover_task: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The same focus handle used by the application and underlying Button.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hover_task = None;
        if self.open || !window.is_window_active() {
            return;
        }
        self.open = true;
        cx.notify();
        window.refresh();
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hover_task = None;
        self.suppressed = true;
        if !self.open {
            return;
        }
        self.open = false;
        cx.notify();
        window.refresh();
    }

    fn hover(&mut self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.hovered = hovered;
        self.hover_task = None;
        if !hovered {
            if !self.focus.is_focused(window) {
                self.dismiss(window, cx);
                self.suppressed = false;
            }
            return;
        }
        self.suppressed = false;
        if self.focus.is_focused(window) {
            self.show(window, cx);
        } else {
            self.hover_task = Some(cx.spawn_in(window, async move |state, cx| {
                cx.background_executor().timer(HOVER_DELAY).await;
                let _ = state.update_in(cx, |state, window, cx| {
                    if state.hovered && !state.suppressed {
                        state.show(window, cx);
                    }
                });
            }));
        }
    }
}

struct TooltipSurface {
    id: ElementId,
    description: SharedString,
    style: StyleRefinement,
}

impl Render for TooltipSurface {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .test_support()
            .role(Role::Tooltip)
            .aria_label(self.description.clone())
            .refine_style(&self.style)
            .child(self.description.clone())
    }
}

// GPUI element state owns the interceptor: removing the trigger drops it even
// when an application retains TooltipState for a later mount.
struct TooltipMount {
    surface: Entity<TooltipSurface>,
    visible: Rc<Cell<bool>>,
    _escape: Subscription,
    _release: Subscription,
}

/// A Button with a noninteractive text description on hover and keyboard focus.
///
/// The trigger builder keeps Button's normal activation behavior and supplies
/// its appearance. [`Styled`] configures the tooltip surface, not the button.
/// The description is always exposed through the trigger's native accessibility
/// description; the visible surface has [`Role::Tooltip`] and no focus stop.
/// GPUI's native tooltip layer owns placement, viewport clamping, and drawing
/// above other content. Rich or interactive popup content belongs in Popover.
#[derive(IntoElement)]
pub struct Tooltip {
    id: ElementId,
    state: Entity<TooltipState>,
    description: SharedString,
    trigger: Button,
    style: StyleRefinement,
}

impl Tooltip {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<TooltipState>,
        description: impl Into<SharedString>,
    ) -> Self {
        let id = id.into();
        Self {
            trigger: Button::new(id.clone()),
            id,
            state: state.clone(),
            description: description.into(),
            style: StyleRefinement::default(),
        }
    }

    pub fn trigger(mut self, build: impl FnOnce(Button) -> Button) -> Self {
        self.trigger = build(self.trigger);
        self
    }
}

impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = self.state.read(cx).focus.clone();
        let weak_state = self.state.downgrade();
        let surface_id = (self.id.clone(), "tooltip-content").into();
        let mount = window.use_keyed_state((self.id.clone(), "tooltip-mount"), cx, |window, cx| {
            let visible = Rc::new(Cell::new(false));
            let escape_visible = visible.clone();
            let release_visible = visible.clone();
            let release_state = weak_state.clone();
            let owner_window = window.window_handle();
            let surface = cx.new(|_| TooltipSurface {
                id: surface_id,
                description: self.description.clone(),
                style: self.style.clone(),
            });
            let escape = cx.intercept_keystrokes(move |event, window, cx| {
                if window.window_handle() != owner_window
                    || !matches!(event.keystroke.key.as_str(), "escape" | "enter" | "space")
                {
                    return;
                }
                let _ = weak_state.update(cx, |state, cx| match event.keystroke.key.as_str() {
                    "escape" if state.open || state.hover_task.is_some() => {
                        let was_visible = escape_visible.get();
                        state.dismiss(window, cx);
                        if was_visible {
                            cx.stop_propagation();
                        }
                    }
                    "enter" | "space" if state.focus.is_focused(window) => {
                        state.dismiss(window, cx);
                    }
                    _ => {}
                });
            });
            let release = cx.on_release(move |_, cx| {
                release_visible.set(false);
                let _ = release_state.update(cx, |state, cx| {
                    state.hover_task = None;
                    state.hovered = false;
                    state.suppressed = false;
                    if state.open {
                        state.open = false;
                        cx.notify();
                    }
                });
            });
            TooltipMount {
                surface,
                visible,
                _escape: escape,
                _release: release,
            }
        });
        let surface = mount.read(cx).surface.clone();
        surface.update(cx, |surface, _| {
            surface.description = self.description.clone();
            surface.style = self.style;
        });
        let visible = mount.read(cx).visible.clone();
        let hover_state = self.state.clone();
        let press_state = self.state.clone();
        let trigger = self
            .trigger
            .track_focus(&focus)
            .aria_description(self.description)
            .on_hover(move |hovered, window, cx| {
                hover_state.update(cx, |state, cx| state.hover(*hovered, window, cx));
            })
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                press_state.update(cx, |state, cx| state.dismiss(window, cx));
            });
        let state = self.state;

        div()
            .id((self.id, "tooltip-host"))
            .relative()
            .flex()
            .child(trigger)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        visible.set(false);
                        if !state.read(cx).is_open()
                            || !bounds.intersects(&window.content_mask().bounds)
                        {
                            return;
                        }
                        let state = state.downgrade();
                        let visible = visible.clone();
                        window.set_tooltip(AnyTooltip {
                            view: surface.clone().into(),
                            mouse_position: bounds.bottom_left() + point(px(0.), px(4.)),
                            check_visible_and_update: Rc::new(move |_, window, cx| {
                                let show = window.is_window_active()
                                    && state
                                        .read_with(cx, |state, _| state.is_open())
                                        .unwrap_or(false);
                                visible.set(show);
                                show
                            }),
                        });
                    },
                    |_, _, _, _| {},
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
        KeyBinding, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, TestAppContext,
        VisualTestContext, prelude::FluentBuilder as _,
    };

    const DESCRIPTION: &str = "Change the session settings.";

    struct Harness {
        tooltip: Entity<TooltipState>,
        trigger: FocusHandle,
        after: FocusHandle,
        activations: Rc<Cell<usize>>,
        escapes: Rc<Cell<usize>>,
        show_tooltip: bool,
        _observation: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let activations = self.activations.clone();
            let escapes = self.escapes.clone();
            div()
                .id("tooltip-test")
                .key_context("TooltipTest")
                .size_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .on_action(move |_: &Cancel, _, _| escapes.set(escapes.get() + 1))
                .when(self.show_tooltip, |this| {
                    this.child(
                        Tooltip::new("tooltip-trigger", &self.tooltip, DESCRIPTION)
                            .w(px(220.))
                            .p(px(8.))
                            .trigger(move |button| {
                                button
                                    .debug_selector(|| "tooltip-trigger".into())
                                    .w(px(160.))
                                    .h(px(32.))
                                    .accessibility_label("Edit settings")
                                    .child("Edit settings")
                                    .on_click(move |_, _, _| activations.set(activations.get() + 1))
                            }),
                    )
                })
                .child(
                    Button::new("after-tooltip")
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
            cx.bind_keys([KeyBinding::new("escape", Cancel, Some("TooltipTest"))]);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let trigger = cx.focus_handle();
                let tooltip = cx.new(|cx| TooltipState::new(&trigger, window, cx));
                let observation = cx.observe(&tooltip, |_, _, cx| cx.notify());
                Harness {
                    tooltip,
                    trigger,
                    after: cx.focus_handle(),
                    activations: Rc::default(),
                    escapes: Rc::default(),
                    show_tooltip: true,
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
            let after = view.read(cx).after.clone();
            after.focus(window, cx);
        });
        settle(cx);
        (cx, view)
    }

    fn activate(cx: &mut VisualTestContext) {
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
    fn focus_escape_and_activation_preserve_the_trigger_contract(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx);
        cx.simulate_keystrokes("shift-tab");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.trigger.is_focused(window));
            assert!(view.tooltip.read(cx).is_open());
            #[cfg(feature = "test-support")]
            {
                let trigger =
                    crate::test_support::find(window, &[], &"tooltip-trigger".into()).unwrap();
                assert_eq!(trigger.description(), Some(DESCRIPTION));
                assert!(crate::test_support::snapshots(window).iter().any(|item| {
                    item.role() == Some(Role::Tooltip) && item.label() == Some(DESCRIPTION)
                }));
            }
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.tooltip.read(cx).is_open());
            assert!(view.trigger.is_focused(window));
            assert_eq!(view.activations.get(), 0);
            assert_eq!(view.escapes.get(), 0);
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).escapes.get(), 1));
        cx.simulate_keystrokes("tab");
        settle(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        settle(cx);
        cx.update(|_, cx| assert!(view.read(cx).tooltip.read(cx).is_open()));
        activate(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert_eq!(view.activations.get(), 1);
            assert!(!view.tooltip.read(cx).is_open());
            assert!(view.trigger.is_focused(window));
        });
    }

    #[gpui::test]
    fn hover_delay_leave_escape_and_click_do_not_steal_focus(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx);
        let trigger = cx.debug_bounds("tooltip-trigger").unwrap().center();
        let outside = point(px(4.), px(4.));
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(HOVER_DELAY / 2);
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.executor().advance_clock(HOVER_DELAY);
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).tooltip.read(cx).is_open()));

        // A retained state must not finish its hover delay after its trigger
        // unmounts, then reveal stale content when the trigger returns.
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(HOVER_DELAY / 2);
        view.update(cx, |view, cx| {
            view.show_tooltip = false;
            cx.notify();
        });
        settle(cx);
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.executor().advance_clock(HOVER_DELAY);
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).tooltip.read(cx).is_open()));
        view.update(cx, |view, cx| {
            view.show_tooltip = true;
            cx.notify();
        });
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).tooltip.read(cx).is_open()));
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).escapes.get(), 1));

        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(HOVER_DELAY);
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.tooltip.read(cx).is_open());
            assert!(view.after.is_focused(window));
            assert_eq!(view.activations.get(), 0);
        });
        cx.simulate_keystrokes("escape");
        settle(cx);
        cx.executor().advance_clock(HOVER_DELAY * 2);
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.tooltip.read(cx).is_open());
            assert_eq!(view.escapes.get(), 1);
            assert!(view.after.is_focused(window));
        });

        cx.simulate_mouse_move(outside, None, Modifiers::default());
        cx.simulate_mouse_move(trigger, None, Modifiers::default());
        cx.run_until_parked();
        cx.executor().advance_clock(HOVER_DELAY);
        settle(cx);
        cx.update(|_, cx| assert!(view.read(cx).tooltip.read(cx).is_open()));
        cx.simulate_click(trigger, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert_eq!(view.activations.get(), 1);
            assert!(!view.tooltip.read(cx).is_open());
            assert!(view.trigger.is_focused(window));
        });
    }
}
