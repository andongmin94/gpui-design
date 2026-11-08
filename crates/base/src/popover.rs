//! Retained, non-modal popover behavior with application-owned presentation.
// Modified for GPUI Design from GPUI Kit v0.9.0: use a native Button trigger,
// retained focus lifecycle, and non-modal traversal with no styled facade.

use std::{cell::Cell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Bounds, ClickEvent, Context, ElementId, Entity, FocusHandle,
    HitboxBehavior, InteractiveElement as _, IntoElement, KeyBinding, MouseButton, MouseDownEvent,
    ParentElement as _, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Subscription, Window, anchored, canvas, deferred, div, point,
    prelude::FluentBuilder as _, px,
};

use crate::{
    Button, POPUP_PRIORITY, Popup, StyledExt as _, TestSupportExt as _,
    actions::Cancel,
    global_state::{DeferredPopover, GlobalState},
};

const CONTEXT: &str = "Popover";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))]);
}

/// Owns one popover's open state, disabled state, trigger, and content focus.
///
/// Keep its matching [`Popover`] mounted while closed. Opening focuses the
/// supplied initial control, or the content scope. Escape and imperative close
/// restore the trigger while focus belongs to the popup. Leaving the popup by
/// pointer or keyboard preserves the new focus target.
pub struct PopoverState {
    open: bool,
    disabled: bool,
    focus: FocusHandle,
    content_focus: FocusHandle,
    initial_focus: Option<FocusHandle>,
    open_on_pointer_down: bool,
    deferred_context: Option<DeferredPopover>,
    dismiss_subscription: Option<Subscription>,
}

impl PopoverState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            disabled: false,
            focus: cx.focus_handle().tab_stop(true),
            content_focus: cx.focus_handle(),
            initial_focus: None,
            open_on_pointer_down: false,
            deferred_context: None,
            dismiss_subscription: None,
        }
    }

    /// Chooses a control rendered inside the content for initial focus.
    pub fn with_initial_focus(mut self, focus: &FocusHandle) -> Self {
        self.initial_focus = Some(focus.clone());
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// The trigger's focus handle.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// The content's non-modal focus scope, excluded from ordinary Tab order.
    pub fn content_focus_handle(&self) -> &FocusHandle {
        &self.content_focus
    }

    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        if disabled {
            self.close(window, cx);
        }
        self.disabled = disabled;
        cx.notify();
        window.refresh();
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.open {
            return;
        }
        self.open = true;
        self.deferred_context = Some(GlobalState::register_deferred_popover(cx));
        self.dismiss_subscription =
            Some(
                cx.on_focus_out(&self.content_focus, window, |_, event, window, cx| {
                    // Focus notifications run while drawing. A detached scope can
                    // leave its old handle focused even though it is no longer in
                    // the tree; that case must not refocus a hidden trigger.
                    cx.defer_in(window, move |state, window, cx| {
                        let detached = event
                            .blurred
                            .upgrade()
                            .is_some_and(|focus| focus.is_focused(window));
                        if !window.is_window_active()
                            || detached
                            || !state.content_focus.contains_focused(window, cx)
                        {
                            state.dismiss(window, cx);
                        }
                    });
                }),
            );
        self.initial_focus
            .as_ref()
            .unwrap_or(&self.content_focus)
            .focus(window, cx);
        cx.notify();
        window.refresh();
    }

    /// Closes the popup and restores its trigger only while it owns focus.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        if self.content_focus.contains_focused(window, cx)
            || self
                .initial_focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window))
        {
            self.focus.focus(window, cx);
        }
        self.dismiss(window, cx);
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.deferred_context = None;
        self.dismiss_subscription = None;
        cx.notify();
        window.refresh();
    }

    fn activate(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Focus-out can dismiss between native mouse down and up. Preserve the
        // press's intent so clicking an open trigger cannot reopen its popup.
        let was_open = match event {
            ClickEvent::Mouse(_) => self.open_on_pointer_down,
            ClickEvent::Keyboard(_) => self.open,
            // A direct recognized tap has no preceding mouse-down snapshot.
            // This GPUI snapshot currently routes native taps through Mouse.
            ClickEvent::Touch(_) => self.open,
        };
        if was_open {
            self.close(window, cx);
        } else {
            self.open(window, cx);
        }
    }

    fn tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let trap = crate::active_focus_trap(window, cx);
        let previous = window.focused(cx);
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
        if window.focused(cx) != previous && self.content_focus.contains_focused(window, cx) {
            return;
        }

        // Deferred content is painted after ordinary controls, so its native
        // Tab order is not adjacent to the trigger. At the boundary, leave from
        // the trigger's position. The closing content still exists in this
        // frame's tab map, so skip its stops and honor an enclosing modal.
        self.focus.focus(window, cx);
        self.dismiss(window, cx);
        if forward {
            let mut visited = Vec::new();
            loop {
                window.focus_next(cx);
                let focused = window.focused(cx);
                if visited.contains(&focused) {
                    self.focus.focus(window, cx);
                    break;
                }
                visited.push(focused);
                if !self.content_focus.contains_focused(window, cx)
                    && trap
                        .as_ref()
                        .is_none_or(|trap| trap.contains_focused(window, cx))
                {
                    break;
                }
            }
        }
    }
}

/// An unstyled trigger and non-modal dialog anchored by [`Popup`].
///
/// The trigger builder supplies the Button's content and presentation; focus,
/// activation, disabled state, and expanded semantics belong to the popover.
/// `accessibility_label` names the content. Name the trigger separately in its
/// builder. Host the app in [`crate::Root`] for Tab and enclosing modal behavior.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    state: Entity<PopoverState>,
    trigger: Button,
    content: Option<AnyElement>,
    label: Option<SharedString>,
    anchor: Anchor,
    offset: Pixels,
    style: StyleRefinement,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>, state: &Entity<PopoverState>) -> Self {
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

    /// Sets the accessible name of the non-modal dialog content.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Sets the gap between the trigger and the anchored content.
    pub fn offset(mut self, offset: Pixels) -> Self {
        self.offset = offset;
        self
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let open = model.open;
        let disabled = model.disabled;
        let focus = model.focus.clone();
        let content_focus = model.content_focus.clone();
        let press_state = self.state.clone();
        let trigger_state = self.state.clone();
        let trigger = self
            .trigger
            .role(Role::Button)
            .track_focus(&focus)
            .focusable(true)
            .tab_stop(true)
            .disabled(disabled)
            .aria_expanded(open)
            .when(!disabled, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    press_state.update(cx, |state, _| {
                        state.open_on_pointer_down = state.open;
                    });
                })
            })
            .on_click(move |event, window, cx| {
                trigger_state.update(cx, |state, cx| state.activate(event, window, cx));
            });
        let trigger_bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
        let position_bounds = trigger_bounds.clone();
        let mut popup = Popup::new((self.id, "popover-host"), trigger)
            .anchor(self.anchor)
            .offset(self.offset)
            .on_position(move |_, bounds| position_bounds.set(bounds));

        let mut host = div().refine_style(&self.style);
        if open {
            let cancel_state = self.state.clone();
            let tab_state = self.state.clone();
            let previous_state = self.state.clone();
            popup = popup.content(
                div()
                    .id(("popover-content", self.state.entity_id()))
                    .test_support()
                    .role(Role::Dialog)
                    .when_some(self.label, |this, label| this.aria_label(label))
                    .track_focus(&content_focus)
                    .key_context(CONTEXT)
                    .on_action(move |_: &Cancel, window, cx| {
                        cx.stop_propagation();
                        cancel_state.update(cx, |state, cx| state.close(window, cx));
                    })
                    .on_action(move |_: &crate::root::Tab, window, cx| {
                        tab_state.update(cx, |state, cx| state.tab(true, window, cx));
                    })
                    .on_action(move |_: &crate::root::TabPrev, window, cx| {
                        previous_state.update(cx, |state, cx| state.tab(false, window, cx));
                    })
                    .children(self.content),
            );

            let viewport = window.viewport_size();
            let outside_state = self.state;
            // A geometric capture-phase "outside" callback would also close
            // this popover when clicking a descendant Select's anchored list.
            // This normal hitbox is below popup occlusion and does not block
            // the underlying target's click or focus. Descendant popup surfaces
            // suppress it through the same hit testing as ordinary controls.
            host = host.child(
                deferred(
                    anchored().position(point(px(0.), px(0.))).child(
                        canvas(
                            |bounds, window, _| {
                                window.insert_hitbox(bounds, HitboxBehavior::Normal)
                            },
                            move |_, hitbox, window, _| {
                                window.on_mouse_event(
                                    move |event: &MouseDownEvent, phase, window, cx| {
                                        if phase.bubble()
                                            && hitbox.is_hovered(window)
                                            && !trigger_bounds.get().contains(&event.position)
                                        {
                                            outside_state.update(cx, |state, cx| {
                                                state.close(window, cx);
                                            });
                                        }
                                    },
                                );
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(viewport.width)
                        .h(viewport.height),
                    ),
                )
                .with_priority(POPUP_PRIORITY - 1),
            );
        }
        host.child(popup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Dialog, DialogBackdrop, DialogPopup, DialogState, Input, InputBase, InputState, Root,
        Select, SelectItem, SelectOption, SelectState,
    };
    use gpui::{
        AppContext as _, Focusable as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render,
        TestAppContext, VisualTestContext,
    };

    struct Harness {
        popover: Entity<PopoverState>,
        dialog: Entity<DialogState>,
        input: Entity<InputState>,
        select: Entity<SelectState>,
        outside: FocusHandle,
        first: FocusHandle,
        last: FocusHandle,
        after: FocusHandle,
        show_host: bool,
        show_select: bool,
        trailing: bool,
        activations: usize,
        _subscriptions: Vec<Subscription>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let content = div()
                .debug_selector(|| "popover-panel".into())
                .w(px(180.))
                .p(px(8.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    Button::new("inside-first")
                        .track_focus(&self.first)
                        .h(px(28.))
                        .child("First")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.activations += 1;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("inside-last")
                        .track_focus(&self.last)
                        .h(px(28.))
                        .child("Last"),
                )
                .when(self.show_select, |this| {
                    this.child(
                        Select::new("nested-select", &self.select)
                            .accessibility_label("Nested choice")
                            .trigger(div().h(px(28.)).child("Choose"))
                            .content(
                                div().w(px(164.)).flex().flex_col().children(
                                    ["One", "Two"]
                                        .into_iter()
                                        .enumerate()
                                        .map(|(index, label)| {
                                            SelectOption::new(&self.select, index).h(px(32.)).child(
                                                div()
                                                    .debug_selector(move || {
                                                        format!("nested-option-{index}").into()
                                                    })
                                                    .size_full()
                                                    .child(label),
                                            )
                                        }),
                                ),
                            ),
                    )
                });
            let popup = DialogPopup::new()
                .w(px(360.))
                .h(px(350.))
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    InputBase::new("outside-input")
                        .debug_selector(|| "outside-input".into())
                        .h(px(32.))
                        .child(Input::new(&self.input)),
                )
                .when(self.show_host, |this| {
                    this.child(
                        Popover::new("popover-trigger", &self.popover)
                            .accessibility_label("Quick tools")
                            .trigger(|button| {
                                button
                                    .debug_selector(|| "popover-trigger".into())
                                    .accessibility_label("Open quick tools")
                                    .h(px(32.))
                                    .child("Quick tools")
                            })
                            .content(content),
                    )
                })
                .when(self.trailing, |this| {
                    this.child(
                        Button::new("after-popover")
                            .track_focus(&self.after)
                            .h(px(28.))
                            .child("After"),
                    )
                });

            div()
                .size_full()
                .child(
                    Button::new("outside-dialog")
                        .track_focus(&self.outside)
                        .w(px(120.))
                        .h(px(28.))
                        .child("Outside dialog"),
                )
                .child(
                    Dialog::new(&self.dialog)
                        .accessibility_label("Parent dialog")
                        .backdrop(DialogBackdrop::new().size_full())
                        .popup(popup),
                )
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn settle(cx: &mut VisualTestContext) {
        draw(cx);
        cx.run_until_parked();
        draw(cx);
    }

    fn press(cx: &mut VisualTestContext, key: &str) {
        let keystroke =
            cx.update(|_, cx| Keystroke::parse_with_os(key, cx.platform_info().os).unwrap());
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
        settle(cx);
    }

    fn harness(
        cx: &mut TestAppContext,
        initial_focus: bool,
    ) -> (&mut VisualTestContext, Entity<Harness>) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let controls = cx.new(|cx| {
                let first = cx.focus_handle();
                let popover = cx.new(|cx| {
                    let state = PopoverState::new(cx);
                    if initial_focus {
                        state.with_initial_focus(&first)
                    } else {
                        state
                    }
                });
                let input = cx.new(|cx| InputState::new(window, cx).default_value("Outside"));
                let dialog =
                    cx.new(|cx| DialogState::new(cx).with_initial_focus(&input.focus_handle(cx)));
                let select = cx.new(|cx| {
                    SelectState::new(
                        [SelectItem::new("one", "One"), SelectItem::new("two", "Two")],
                        cx,
                    )
                    .default_value("one")
                });
                let subscriptions = vec![
                    cx.observe(&popover, |_, _, cx| cx.notify()),
                    cx.observe(&dialog, |_, _, cx| cx.notify()),
                    cx.observe(&select, |_, _, cx| cx.notify()),
                ];
                Harness {
                    popover,
                    dialog,
                    input,
                    select,
                    outside: cx.focus_handle(),
                    first,
                    last: cx.focus_handle(),
                    after: cx.focus_handle(),
                    show_host: true,
                    show_select: false,
                    trailing: true,
                    activations: 0,
                    _subscriptions: subscriptions,
                }
            });
            Root::new(controls, window, cx)
        });
        let controls = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
            let outside = controls.read(cx).outside.clone();
            outside.focus(window, cx);
            let dialog = controls.read(cx).dialog.clone();
            dialog.update(cx, |state, cx| state.open(window, cx));
        });
        settle(cx);
        (cx, controls)
    }

    fn open(cx: &mut VisualTestContext, controls: &Entity<Harness>) {
        cx.update(|window, cx| {
            let focus = controls.read(cx).popover.read(cx).focus_handle().clone();
            focus.focus(window, cx);
        });
        draw(cx);
        press(cx, "enter");
        cx.update(|_, cx| assert!(controls.read(cx).popover.read(cx).is_open()));
    }

    #[gpui::test]
    fn native_trigger_and_content_activation_remain_separate(cx: &mut TestAppContext) {
        let (cx, controls) = harness(cx, true);
        open(cx, &controls);
        cx.update(|window, cx| assert!(controls.read(cx).first.is_focused(window)));
        press(cx, "enter");
        press(cx, "space");
        cx.update(|_, cx| {
            assert_eq!(controls.read(cx).activations, 2);
            assert!(controls.read(cx).popover.read(cx).is_open());
        });

        press(cx, "escape");
        cx.update(|window, cx| {
            let controls = controls.read(cx);
            assert!(!controls.popover.read(cx).is_open());
            assert!(controls.dialog.read(cx).is_open());
            assert!(controls.popover.read(cx).focus_handle().is_focused(window));
        });
        press(cx, "space");
        cx.update(|_, cx| assert!(controls.read(cx).popover.read(cx).is_open()));

        // A real pointer press can render after down and before up. Focus-out
        // during that frame must not turn a closing trigger click into reopen.
        let trigger = cx.debug_bounds("popover-trigger").unwrap().center();
        cx.simulate_mouse_down(trigger, MouseButton::Left, Modifiers::default());
        settle(cx);
        cx.simulate_mouse_up(trigger, MouseButton::Left, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let controls = controls.read(cx);
            assert!(!controls.popover.read(cx).is_open());
            assert!(controls.popover.read(cx).focus_handle().is_focused(window));
            assert!(controls.dialog.read(cx).is_open());
        });
        cx.simulate_click(trigger, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert!(controls.read(cx).popover.read(cx).is_open()));
    }

    #[gpui::test]
    fn last_trigger_in_a_modal_skips_closed_content_and_background(cx: &mut TestAppContext) {
        let (cx, controls) = harness(cx, true);
        controls.update(cx, |controls, cx| {
            controls.trailing = false;
            cx.notify();
        });
        settle(cx);
        open(cx, &controls);
        press(cx, "tab");
        cx.update(|window, cx| assert!(controls.read(cx).last.is_focused(window)));
        press(cx, "tab");
        cx.update(|window, cx| {
            let controls = controls.read(cx);
            assert!(!controls.popover.read(cx).is_open());
            assert!(controls.input.focus_handle(cx).is_focused(window));
            assert!(
                controls
                    .dialog
                    .read(cx)
                    .focus_handle()
                    .contains_focused(window, cx)
            );
        });
    }

    #[gpui::test]
    fn modal_backdrop_receives_the_outside_press(cx: &mut TestAppContext) {
        let (cx, controls) = harness(cx, false);
        open(cx, &controls);
        cx.simulate_click(point(px(2.), px(2.)), Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let controls = controls.read(cx);
            assert!(!controls.popover.read(cx).is_open());
            assert!(!controls.dialog.read(cx).is_open());
            assert!(controls.outside.is_focused(window));
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }

    #[gpui::test]
    fn nested_select_pointer_and_escape_preserve_outer_scopes(cx: &mut TestAppContext) {
        let (cx, controls) = harness(cx, false);
        controls.update(cx, |controls, cx| {
            controls.show_select = true;
            cx.notify();
        });
        settle(cx);
        open(cx, &controls);
        cx.update(|window, cx| {
            let focus = controls.read(cx).select.read(cx).focus_handle().clone();
            focus.focus(window, cx);
        });
        draw(cx);
        press(cx, "down");
        let option = cx.debug_bounds("nested-option-1").unwrap().center();
        assert!(!cx.debug_bounds("popover-panel").unwrap().contains(&option));
        cx.simulate_click(option, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let controls = controls.read(cx);
            assert!(controls.popover.read(cx).is_open());
            assert!(!controls.select.read(cx).is_open());
            assert_eq!(controls.select.read(cx).selected_index(), Some(1));
            assert!(controls.select.read(cx).focus_handle().is_focused(window));
        });

        press(cx, "down");
        press(cx, "escape");
        cx.update(|_, cx| {
            let controls = controls.read(cx);
            assert!(!controls.select.read(cx).is_open());
            assert!(controls.popover.read(cx).is_open());
            assert!(controls.dialog.read(cx).is_open());
        });
        press(cx, "escape");
        cx.update(|_, cx| {
            assert!(!controls.read(cx).popover.read(cx).is_open());
            assert!(controls.read(cx).dialog.read(cx).is_open());
        });
        press(cx, "escape");
        cx.update(|window, cx| {
            assert!(!controls.read(cx).dialog.read(cx).is_open());
            assert!(controls.read(cx).outside.is_focused(window));
        });
    }

    #[gpui::test]
    fn disabled_popover_closes_and_keeps_traversal_available(cx: &mut TestAppContext) {
        let (cx, controls) = harness(cx, false);
        open(cx, &controls);
        cx.update(|window, cx| {
            let state = controls.read(cx).popover.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        settle(cx);
        cx.update(|window, cx| {
            let state = controls.read(cx).popover.read(cx);
            assert!(!state.is_open());
            assert!(state.is_disabled());
            assert!(state.focus_handle().is_focused(window));
            #[cfg(feature = "test-support")]
            {
                let trigger =
                    crate::test_support::find(window, &[], &"popover-trigger".into()).unwrap();
                assert_eq!(trigger.role(), Some(Role::Button));
                assert_eq!(trigger.label(), Some("Open quick tools"));
                assert_eq!(trigger.expanded(), Some(false));
                assert_eq!(trigger.disabled(), Some(true));
            }
        });
        press(cx, "enter");
        press(cx, "space");
        let trigger = cx.debug_bounds("popover-trigger").unwrap().center();
        cx.simulate_click(trigger, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let state = controls.read(cx).popover.clone();
            state.update(cx, |state, cx| state.open(window, cx));
            assert!(!state.read(cx).is_open());
        });
        press(cx, "tab");
        cx.update(|window, cx| assert!(controls.read(cx).after.is_focused(window)));
    }

    #[gpui::test]
    fn external_focus_parent_close_and_deactivation_dismiss_retained_state(
        cx: &mut TestAppContext,
    ) {
        let (cx, controls) = harness(cx, false);
        open(cx, &controls);
        cx.update(|window, cx| controls.read(cx).input.focus_handle(cx).focus(window, cx));
        settle(cx);
        cx.update(|window, cx| {
            assert!(!controls.read(cx).popover.read(cx).is_open());
            assert!(controls.read(cx).input.focus_handle(cx).is_focused(window));
        });

        open(cx, &controls);
        cx.update(|window, cx| {
            let dialog = controls.read(cx).dialog.clone();
            dialog.update(cx, |state, cx| state.close(window, cx));
        });
        settle(cx);
        cx.update(|window, cx| {
            assert!(!controls.read(cx).popover.read(cx).is_open());
            assert!(controls.read(cx).outside.is_focused(window));
            let dialog = controls.read(cx).dialog.clone();
            dialog.update(cx, |state, cx| state.open(window, cx));
        });
        settle(cx);
        open(cx, &controls);
        cx.deactivate_window();
        settle(cx);
        cx.update(|_, cx| {
            assert!(!controls.read(cx).popover.read(cx).is_open());
            assert!(controls.read(cx).dialog.read(cx).is_open());
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }

    #[gpui::test]
    fn unmounting_a_focused_host_never_refocuses_its_hidden_trigger(cx: &mut TestAppContext) {
        for initial in [false, true] {
            let (cx, controls) = harness(cx, initial);
            open(cx, &controls);
            controls.update(cx, |controls, cx| {
                controls.show_host = false;
                cx.notify();
            });
            settle(cx);
            cx.update(|window, cx| {
                let state = controls.read(cx).popover.read(cx);
                assert!(!state.is_open());
                assert!(!state.focus_handle().is_focused(window));
                assert!(!GlobalState::is_in_deferred_context(cx));
            });
        }
    }

    #[gpui::test]
    fn immediate_close_restores_the_trigger_before_initial_focus_is_painted(
        cx: &mut TestAppContext,
    ) {
        let (cx, controls) = harness(cx, true);
        cx.update(|window, cx| {
            let state = controls.read(cx).popover.clone();
            state.update(cx, |state, cx| {
                state.open(window, cx);
                state.close(window, cx);
            });
            assert!(!state.read(cx).is_open());
            assert!(state.read(cx).focus_handle().is_focused(window));
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn accessibility_exposes_button_expansion_and_named_nonmodal_content(cx: &mut TestAppContext) {
        let (cx, controls) = harness(cx, false);
        cx.update(|window, _| {
            let trigger =
                crate::test_support::find(window, &[], &"popover-trigger".into()).unwrap();
            assert_eq!(trigger.role(), Some(Role::Button));
            assert_eq!(trigger.label(), Some("Open quick tools"));
            assert_eq!(trigger.expanded(), Some(false));
        });
        open(cx, &controls);
        cx.update(|window, cx| {
            let trigger =
                crate::test_support::find(window, &[], &"popover-trigger".into()).unwrap();
            assert_eq!(trigger.expanded(), Some(true));
            let id = ("popover-content", controls.read(cx).popover.entity_id()).into();
            let content = crate::test_support::find(window, &[], &id).unwrap();
            assert_eq!(content.role(), Some(Role::Dialog));
            assert_eq!(content.label(), Some("Quick tools"));
            assert_eq!(content.focused(), Some(true));
        });
    }
}
