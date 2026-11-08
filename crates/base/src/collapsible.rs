//! A single disclosure composed from Button and an active-only content region.
//! Derived from GPUI Kit v0.9.0's controlled Collapsible; GPUI Design adds retained
//! open state, native trigger activation, and focused-content closure.

use gpui::{
    AnyElement, App, Context, ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement,
    Interactivity, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{Button, TestSupportExt as _};

/// Emitted after the open state changes. Repeated requests emit no event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapsibleEvent {
    Change(bool),
}

/// Retained disclosure state and focus for one host in one window.
///
/// The trigger toggles the content without moving focus into it. Closing focused
/// content returns focus to its still-mounted trigger, or the root scope while
/// that trigger is disabled. Focus elsewhere is kept.
/// Keep child data in application-owned entities so it survives unmounting.
pub struct CollapsibleState {
    open: bool,
    disabled: bool,
    root_focus: FocusHandle,
    trigger_focus: FocusHandle,
    content_focus: FocusHandle,
}

impl CollapsibleState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            disabled: false,
            root_focus: cx.focus_handle(),
            trigger_focus: cx.focus_handle(),
            content_focus: cx.focus_handle(),
        }
    }

    /// Chooses the initial open state without emitting a change event.
    pub fn default_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn trigger_focus_handle(&self) -> &FocusHandle {
        &self.trigger_focus
    }

    pub fn content_focus_handle(&self) -> &FocusHandle {
        &self.content_focus
    }

    /// Changes visibility programmatically, including while the trigger is disabled.
    ///
    /// If closing removes the focused content, focus returns to the live trigger.
    /// While that trigger is disabled, the live non-Tab root receives focus so
    /// normal Tab traversal can leave the closed disclosure.
    /// Removing the entire host remains the application's responsibility, including
    /// moving focus to a surviving control before removal.
    pub fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        if !open && self.content_focus.contains_focused(window, cx) {
            if !self.disabled && self.root_focus.contains(&self.trigger_focus, window) {
                self.trigger_focus.focus(window, cx);
            } else if self.disabled && self.root_focus.contains(&self.content_focus, window) {
                self.root_focus.focus(window, cx);
            }
        }
        self.open = open;
        cx.emit(CollapsibleEvent::Change(open));
        cx.notify();
        window.refresh();
    }

    /// Toggles an enabled disclosure. Disabled triggers ignore this request.
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled {
            self.set_open(!self.open, window, cx);
        }
    }

    /// Disables user toggling without closing or disabling existing content.
    /// A focused disabled trigger keeps Button's focus binding so Tab can leave.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
        window.refresh();
    }
}

impl EventEmitter<CollapsibleEvent> for CollapsibleState {}

/// An unstyled disclosure root containing one trigger and its content.
///
/// Keep the root and trigger mounted while closed. This scope imposes no layout,
/// heading, arrow navigation, or focus trap. Applications supply all presentation.
#[derive(IntoElement)]
pub struct Collapsible {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CollapsibleState>,
}

impl Collapsible {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CollapsibleState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
        }
    }
}

impl Styled for Collapsible {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for Collapsible {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Collapsible {}

impl RenderOnce for Collapsible {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = self.state.read(cx).root_focus.clone();
        self.base
            .role(Role::Group)
            .track_focus(&focus.tab_stop(false))
    }
}

/// A native Button that toggles its disclosure and exposes the expanded state.
/// Supply an accessible name and non-interactive children. Button owns activation,
/// disabled handling, focus, and geometry; application styles remain unrestricted.
#[derive(IntoElement)]
pub struct CollapsibleTrigger {
    button: Button,
    state: Entity<CollapsibleState>,
}

impl CollapsibleTrigger {
    pub fn new(state: &Entity<CollapsibleState>) -> Self {
        Self {
            button: Button::new(("collapsible-trigger", state.entity_id())),
            state: state.clone(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.button = self.button.accessibility_label(label);
        self
    }
}

impl Styled for CollapsibleTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}

impl ParentElement for CollapsibleTrigger {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(elements);
    }
}

impl InteractiveElement for CollapsibleTrigger {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.button.interactivity()
    }
}

impl StatefulInteractiveElement for CollapsibleTrigger {}

impl RenderOnce for CollapsibleTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let open = state.open;
        let disabled = state.disabled;
        let focus = state.trigger_focus.clone();
        let state = self.state;
        self.button
            .aria_expanded(open)
            .disabled(disabled)
            .track_focus(&focus)
            .on_click(move |_, window, cx| {
                state.update(cx, |state, cx| state.toggle(window, cx));
            })
            .render(window, cx)
    }
}

/// A named region mounted only while its disclosure is open.
///
/// Closed content contributes no layout, focus stops, or accessibility nodes.
/// Name the region with [`Self::accessibility_label`]. Use [`Self::tab_stop`] when
/// static content should receive focus itself; otherwise Tab enters its children.
/// The pinned GPUI API does not expose native trigger/content relationship IDs.
#[derive(IntoElement)]
pub struct CollapsibleContent {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CollapsibleState>,
    label: Option<SharedString>,
    tab_stop: bool,
}

impl CollapsibleContent {
    pub fn new(state: &Entity<CollapsibleState>) -> Self {
        Self {
            base: div()
                .id(("collapsible-content", state.entity_id()))
                .test_support(),
            state: state.clone(),
            label: None,
            tab_stop: false,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }
}

impl Styled for CollapsibleContent {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for CollapsibleContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for CollapsibleContent {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for CollapsibleContent {}

impl RenderOnce for CollapsibleContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        if !state.open {
            return gpui::Empty.into_any_element();
        }
        let focus = state.content_focus.clone();
        self.base
            .role(Role::Region)
            .when_some(self.label, |this, label| this.aria_label(label))
            .track_focus(&focus.tab_stop(self.tab_stop))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, Focusable as _, KeyDownEvent, KeyUpEvent, Keystroke, Render, Subscription,
        TestAppContext, VisualTestContext, px,
    };

    use super::*;
    use crate::{Input, InputBase, InputState, Root};

    struct Harness {
        state: Entity<CollapsibleState>,
        input: Entity<InputState>,
        before: FocusHandle,
        after: FocusHandle,
        show_host: bool,
        unhandled: Rc<RefCell<Vec<String>>>,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let unhandled = self.unhandled.clone();
            div()
                .w(px(320.))
                .flex()
                .flex_col()
                .on_key_down(move |event, _, _| {
                    unhandled.borrow_mut().push(event.keystroke.key.clone());
                })
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .h(px(32.))
                        .child("Before"),
                )
                .when(self.show_host, |this| {
                    this.child(
                        Collapsible::new("disclosure", &self.state)
                            .flex()
                            .flex_col()
                            .child(
                                CollapsibleTrigger::new(&self.state)
                                    .accessibility_label("Options")
                                    .debug_selector(|| "trigger".into())
                                    .h(px(32.))
                                    .child("Options"),
                            )
                            .child(
                                CollapsibleContent::new(&self.state)
                                    .accessibility_label("Options")
                                    .debug_selector(|| "content".into())
                                    .h(px(40.))
                                    .child(
                                        InputBase::new("option-input")
                                            .h(px(32.))
                                            .child(Input::new(&self.input)),
                                    ),
                            ),
                    )
                })
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .debug_selector(|| "after".into())
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<bool>>>;

    fn harness(
        cx: &mut TestAppContext,
        open: bool,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| CollapsibleState::new(cx).default_open(open));
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    input: cx.new(|cx| InputState::new(window, cx)),
                    before: cx.focus_handle(),
                    after: cx.focus_handle(),
                    show_host: true,
                    unhandled: Rc::new(RefCell::new(Vec::new())),
                    _subscription: subscription,
                }
            });
            Root::new(content, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        let changes = Rc::new(RefCell::new(Vec::new()));
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let changes = changes.clone();
            cx.subscribe(&state, move |state, event: &CollapsibleEvent, cx| {
                let CollapsibleEvent::Change(open) = event;
                assert_eq!(state.read(cx).is_open(), *open);
                changes.borrow_mut().push(*open);
            })
            .detach();
            window.activate_window();
            view.read(cx).before.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        (cx, view, changes)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
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
        draw(cx);
    }

    #[gpui::test]
    fn disclosure_mounting_preserves_normal_focus_and_unbound_keys(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, false);
        let collapsed_after = cx.debug_bounds("after").unwrap().origin.y;
        assert!(cx.debug_bounds("content").is_none());
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|_, cx| view.read(cx).unhandled.borrow_mut().clear());
        cx.simulate_keystrokes("up down home end");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert!(state.trigger_focus_handle().is_focused(window));
            assert!(!state.is_open());
            assert_eq!(
                *view.read(cx).unhandled.borrow(),
                ["up", "down", "home", "end"]
            );
            #[cfg(feature = "test-support")]
            {
                let trigger = crate::test_support::find(
                    window,
                    &[],
                    &("collapsible-trigger", view.read(cx).state.entity_id()).into(),
                )
                .unwrap();
                assert_eq!(trigger.role(), Some(Role::Button));
                assert_eq!(trigger.label(), Some("Options"));
                assert_eq!(trigger.expanded(), Some(false));
                assert!(
                    crate::test_support::snapshots(window)
                        .iter()
                        .all(|node| node.role() != Some(Role::Region))
                );
            }
        });
        assert!(changes.borrow().is_empty());
        press(cx, "space");
        assert_eq!(
            cx.debug_bounds("after").unwrap().origin.y - collapsed_after,
            px(40.)
        );
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert!(state.is_open());
            assert!(state.trigger_focus_handle().is_focused(window));
            #[cfg(feature = "test-support")]
            {
                let id = view.read(cx).state.entity_id();
                let trigger =
                    crate::test_support::find(window, &[], &("collapsible-trigger", id).into())
                        .unwrap();
                assert_eq!(trigger.expanded(), Some(true));
                let region =
                    crate::test_support::find(window, &[], &("collapsible-content", id).into())
                        .unwrap();
                assert_eq!(region.role(), Some(Role::Region));
                assert_eq!(region.label(), Some("Options"));
            }
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).input.focus_handle(cx).is_focused(window)));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).after.is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.set_open(false, window, cx);
                state.set_open(false, window, cx);
            });
            assert!(view.read(cx).after.is_focused(window));
        });
        draw(cx);
        assert!(cx.debug_bounds("content").is_none());
        assert_eq!(cx.debug_bounds("after").unwrap().origin.y, collapsed_after);
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .trigger_focus_handle()
                    .is_focused(window)
            );
        });
        assert_eq!(*changes.borrow(), [true, false]);
    }

    #[gpui::test]
    fn imperative_visibility_retains_content_and_restores_only_a_live_trigger(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness(cx, true);
        cx.simulate_keystrokes("tab tab");
        draw(cx);
        cx.simulate_input("retained");
        cx.simulate_keystrokes("home");
        draw(cx);
        cx.update(|window, cx| {
            let input = &view.read(cx).input;
            assert!(input.focus_handle(cx).is_focused(window));
            assert_eq!(input.read(cx).selected_range(), 0..0);
        });
        cx.simulate_keystrokes("end");
        draw(cx);
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).input.read(cx).selected_range(), 8..8);
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.set_open(false, window, cx);
                assert!(state.trigger_focus_handle().is_focused(window));
                state.set_disabled(true, window, cx);
            });
        });
        draw(cx);
        press(cx, "space");
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert!(!state.is_open());
            assert!(state.is_disabled());
            assert!(state.trigger_focus_handle().is_focused(window));
            #[cfg(feature = "test-support")]
            {
                let trigger = crate::test_support::find(
                    window,
                    &[],
                    &("collapsible-trigger", view.read(cx).state.entity_id()).into(),
                )
                .unwrap();
                assert_eq!(trigger.disabled(), Some(true));
            }
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).before.is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.set_open(true, window, cx);
                state.toggle(window, cx);
            });
            assert!(state.read(cx).is_open());
            assert!(view.read(cx).before.is_focused(window));
            assert_eq!(view.read(cx).input.read(cx).value(), "retained");
        });
        draw(cx);
        assert!(cx.debug_bounds("content").is_some());
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).input.focus_handle(cx).is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_open(false, window, cx));
            assert!(state.read(cx).root_focus.is_focused(window));
        });
        draw(cx);
        assert!(cx.debug_bounds("content").is_none());
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).after.is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.set_disabled(false, window, cx);
                state.set_open(true, window, cx);
            });
        });
        draw(cx);
        cx.update(|window, cx| {
            view.read(cx).input.focus_handle(cx).focus(window, cx);
        });
        draw(cx);
        view.update(cx, |view, cx| {
            view.show_host = false;
            cx.notify();
        });
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_open(false, window, cx));
            assert!(!state.read(cx).trigger_focus_handle().is_focused(window));
        });
        assert_eq!(*changes.borrow(), [false, true, false, true, false]);
    }
}
