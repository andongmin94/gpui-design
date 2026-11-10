//! Exclusive selection with one tab stop and application-owned presentation.
//! Derived from GPUI Kit v0.9.0's radio primitives; GPUI Design adds retained
//! group selection, roving focus, keyboard navigation, and group-level semantics.

use gpui::{
    AnyElement, App, Axis, Context, ElementId, Entity, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Toggled,
    Window, accesskit, div, prelude::FluentBuilder as _,
};

use crate::{
    StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{SelectDown, SelectLeft, SelectRight, SelectUp},
};

const CONTEXT: &str = "RadioGroup";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

/// One option in a radio group's fixed, ordered collection. Values must be unique.
#[derive(Clone, Debug)]
pub struct RadioItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl RadioItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn value(&self) -> &SharedString {
        &self.value
    }

    pub fn label(&self) -> &SharedString {
        &self.label
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Emitted once when a user selects a different value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RadioGroupEvent {
    Change(SharedString),
}

/// Retained exclusive selection and focus for one rendered radio group.
///
/// Tab enters at the selected option, or the first enabled option when nothing
/// is selected. Arrow keys select and focus the next enabled option, wrapping at
/// either end. Space/Enter or a pointer click select without deselecting an
/// already selected option. Disabled options are skipped.
pub struct RadioGroupState {
    items: Vec<RadioItem>,
    selected: Option<usize>,
    disabled: bool,
    focus: Vec<FocusHandle>,
}

impl RadioGroupState {
    pub fn new(items: impl IntoIterator<Item = RadioItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (ix, item) in items.iter().enumerate() {
            assert!(
                items[..ix].iter().all(|other| other.value != item.value),
                "RadioItem values must be unique"
            );
        }
        let focus = items.iter().map(|_| cx.focus_handle()).collect();
        Self {
            items,
            selected: None,
            disabled: false,
            focus,
        }
    }

    /// Selects an enabled value before the state is first rendered.
    /// An unknown or disabled value leaves the group unselected.
    pub fn default_value(mut self, value: impl AsRef<str>) -> Self {
        self.selected = self
            .items
            .iter()
            .position(|item| item.value.as_ref() == value.as_ref() && !item.disabled);
        self
    }

    pub fn items(&self) -> &[RadioItem] {
        &self.items
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&RadioItem> {
        self.selected.map(|ix| &self.items[ix])
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// The stable focus handle of an option in this state's fixed collection.
    pub fn option_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.focus[index]
    }

    /// Disables the group while retaining its value. An option already holding
    /// focus stays in the focus tree so Tab/Shift-Tab can leave the group.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
        window.refresh();
    }

    fn tab_stop_index(&self) -> Option<usize> {
        if self.disabled {
            return None;
        }
        self.selected
            .or_else(|| self.items.iter().position(|item| !item.disabled))
    }

    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.items[index].disabled {
            return;
        }
        self.focus[index].focus(window, cx);
        if self.selected == Some(index) {
            return;
        }
        self.selected = Some(index);
        cx.emit(RadioGroupEvent::Change(self.items[index].value.clone()));
        cx.notify();
        window.refresh();
    }

    fn move_selection(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        // Only the option holding focus owns this command. A consumer may put
        // other content in the group without giving it radio keyboard behavior.
        let Some(current) = self.focus.iter().position(|focus| focus.is_focused(window)) else {
            cx.propagate();
            return;
        };
        let len = self.items.len();
        if let Some(next) = (1..=len)
            .map(|step| {
                if forward {
                    (current + step) % len
                } else {
                    (current + len - step) % len
                }
            })
            .find(|&ix| !self.items[ix].disabled)
        {
            self.select(next, window, cx);
        }
    }
}

impl EventEmitter<RadioGroupEvent> for RadioGroupState {}

/// An unstyled radio group. Render one [`RadioOption`] for every item, in order,
/// using this group's state. Children supply presentation; the group supplies
/// exclusive selection, keyboard navigation, and accessibility semantics.
#[derive(IntoElement)]
pub struct RadioGroup {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<RadioGroupState>,
    label: Option<SharedString>,
    axis: Axis,
    direction: Option<crate::Direction>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl RadioGroup {
    pub fn new(id: impl Into<ElementId>, state: &Entity<RadioGroupState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
            axis: Axis::Vertical,
            direction: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets semantic orientation, without imposing layout. Both pairs of arrow
    /// keys navigate the fixed option order in either orientation.
    /// Overrides the inherited logical direction for horizontal arrows.
    pub fn direction(mut self, direction: crate::Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for RadioGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for RadioGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for RadioGroup {}

impl RenderOnce for RadioGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let direction = self
            .direction
            .unwrap_or_else(|| crate::Direction::current(cx));
        let disabled = self.state.read(cx).disabled;
        let up_state = self.state.clone();
        let down_state = self.state.clone();
        let left_state = self.state.clone();
        let right_state = self.state;

        self.base
            .role(Role::RadioGroup)
            .aria_orientation(match self.axis {
                Axis::Horizontal => accesskit::Orientation::Horizontal,
                Axis::Vertical => accesskit::Orientation::Vertical,
            })
            .when_some(self.label, |this, label| this.aria_label(label))
            .key_context(CONTEXT)
            .on_action(move |_: &SelectUp, window, cx| {
                up_state.update(cx, |state, cx| state.move_selection(false, window, cx));
            })
            .on_action(move |_: &SelectDown, window, cx| {
                down_state.update(cx, |state, cx| state.move_selection(true, window, cx));
            })
            .on_action(move |_: &SelectLeft, window, cx| {
                left_state.update(cx, |state, cx| {
                    state.move_selection(direction.horizontal_forward(false), window, cx)
                });
            })
            .on_action(move |_: &SelectRight, window, cx| {
                right_state.update(cx, |state, cx| {
                    state.move_selection(direction.horizontal_forward(true), window, cx)
                });
            })
            .children(self.children)
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}

/// One unstyled radio option. Its state supplies the label, checked state, focus,
/// and activation; applications provide the visual indicator and other content.
#[derive(IntoElement)]
pub struct RadioOption {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<RadioGroupState>,
    index: usize,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl RadioOption {
    /// `index` must identify an item in the enclosing group's state.
    pub fn new(state: &Entity<RadioGroupState>, index: usize) -> Self {
        Self {
            base: div().id(("radio-option", index)).test_support(),
            state: state.clone(),
            index,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for RadioOption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for RadioOption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for RadioOption {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for RadioOption {}

impl RenderOnce for RadioOption {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("RadioOption index must identify an item");
        let disabled = model.disabled || item.disabled;
        let checked = model.selected == Some(self.index);
        let tab_stop = model.tab_stop_index() == Some(self.index);
        let focus = model.option_focus_handle(self.index).clone();
        let label = item.label.clone();
        let size = model.items.len();
        let index = self.index;
        let click_state = self.state;

        self.base
            .role(Role::RadioButton)
            .aria_label(label)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .aria_position_in_set(index + 1)
            .aria_size_of_set(size)
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(tab_stop))
            })
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .when(!disabled, |this| {
                this.on_click(move |_, window, cx| {
                    click_state.update(cx, |state, cx| state.select(index, window, cx));
                })
            })
            .children(self.children)
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, Subscription,
        TestAppContext, VisualTestContext, point, px,
    };

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        state: Entity<RadioGroupState>,
        before: FocusHandle,
        after: FocusHandle,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let options = self
                .state
                .read(cx)
                .items()
                .iter()
                .enumerate()
                .map(|(ix, item)| {
                    RadioOption::new(&self.state, ix)
                        .w_full()
                        .h(px(32.))
                        .child(item.label().clone())
                });
            div()
                .w(px(240.))
                .flex()
                .flex_col()
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .w_full()
                        .h(px(32.))
                        .child("Before"),
                )
                .child(
                    RadioGroup::new("group", &self.state)
                        .accessibility_label("Size")
                        .flex()
                        .flex_col()
                        .children(options),
                )
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .w_full()
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    fn items() -> Vec<RadioItem> {
        vec![
            RadioItem::new("small", "Small"),
            RadioItem::new("unavailable", "Unavailable").disabled(true),
            RadioItem::new("medium", "Medium"),
            RadioItem::new("large", "Large"),
        ]
    }

    type Changes = Rc<RefCell<Vec<SharedString>>>;

    fn harness<'a>(
        cx: &'a mut TestAppContext,
        items: Vec<RadioItem>,
        value: Option<&'static str>,
    ) -> (&'a mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| {
                    let state = RadioGroupState::new(items, cx);
                    if let Some(value) = value {
                        state.default_value(value)
                    } else {
                        state
                    }
                });
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    before: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
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
            cx.subscribe(&state, move |_, event: &RadioGroupEvent, _| {
                let RadioGroupEvent::Change(value) = event;
                changes.borrow_mut().push(value.clone());
            })
            .detach();
            window.activate_window();
            let before = view.read(cx).before.clone();
            before.focus(window, cx);
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

    fn assert_option(cx: &mut VisualTestContext, view: &Entity<Harness>, index: usize) {
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(state.selected_index(), Some(index));
            assert!(state.option_focus_handle(index).is_focused(window));
        });
    }

    #[gpui::test]
    fn arrows_wrap_skip_disabled_and_keep_one_tab_stop(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, items(), Some("medium"));
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_option(cx, &view, 2);
        assert!(changes.borrow().is_empty());

        for (key, index) in [
            ("right", 3),
            ("down", 0),
            ("right", 2),
            ("left", 0),
            ("up", 3),
        ] {
            cx.simulate_keystrokes(key);
            draw(cx);
            assert_option(cx, &view, index);
        }
        assert_eq!(
            *changes.borrow(),
            ["large", "small", "medium", "small", "large"].map(SharedString::from)
        );

        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        assert_option(cx, &view, 3);
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).before.is_focused(window)));
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_option(cx, &view, 3);
        assert_eq!(changes.borrow().len(), 5);
    }

    #[gpui::test]
    fn pointer_and_native_activation_select_once_without_deselecting(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, items(), None);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(state.selected_index(), None);
            assert!(state.option_focus_handle(0).is_focused(window));
        });
        press(cx, "space");
        assert_option(cx, &view, 0);
        press(cx, "space");
        press(cx, "enter");
        assert_eq!(*changes.borrow(), [SharedString::from("small")]);

        // Each row is 32px high, following the 32px "Before" button.
        cx.simulate_click(point(px(12.), px(112.)), Modifiers::default());
        draw(cx);
        assert_option(cx, &view, 2);
        cx.simulate_click(point(px(12.), px(112.)), Modifiers::default());
        draw(cx);
        cx.simulate_click(point(px(12.), px(80.)), Modifiers::default());
        draw(cx);
        assert_option(cx, &view, 2);
        assert_eq!(
            *changes.borrow(),
            [SharedString::from("small"), SharedString::from("medium")]
        );
    }

    #[gpui::test]
    fn disabling_a_focused_group_preserves_exit_and_skips_it_on_reentry(cx: &mut TestAppContext) {
        for backwards in [false, true] {
            let (cx, view, changes) = harness(cx, items(), Some("small"));
            cx.simulate_keystrokes("tab");
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.clone();
                state.update(cx, |state, cx| state.set_disabled(true, window, cx));
            });
            draw(cx);
            press(cx, "space");
            press(cx, "enter");
            cx.simulate_keystrokes("right");
            draw(cx);
            cx.simulate_click(point(px(12.), px(112.)), Modifiers::default());
            draw(cx);
            assert_option(cx, &view, 0);
            assert!(changes.borrow().is_empty());

            cx.simulate_keystrokes(if backwards { "shift-tab" } else { "tab" });
            draw(cx);
            cx.update(|window, cx| {
                let view = view.read(cx);
                let expected = if backwards { &view.before } else { &view.after };
                assert!(expected.is_focused(window));
            });
            cx.simulate_keystrokes(if backwards { "tab" } else { "shift-tab" });
            draw(cx);
            cx.update(|window, cx| {
                let view = view.read(cx);
                let expected = if backwards { &view.after } else { &view.before };
                assert!(expected.is_focused(window));
                let state = view.state.clone();
                state.update(cx, |state, cx| state.set_disabled(false, window, cx));
            });
            draw(cx);
            cx.simulate_keystrokes(if backwards { "shift-tab" } else { "tab" });
            draw(cx);
            assert_option(cx, &view, 0);
        }
    }

    #[gpui::test]
    fn empty_and_all_disabled_groups_have_no_tab_stop(cx: &mut TestAppContext) {
        for items in [
            Vec::new(),
            vec![RadioItem::new("only", "Only").disabled(true)],
        ] {
            let (cx, view, changes) = harness(cx, items, None);
            cx.simulate_keystrokes("tab");
            draw(cx);
            cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
            cx.simulate_keystrokes("shift-tab");
            draw(cx);
            cx.update(|window, cx| {
                assert!(view.read(cx).before.is_focused(window));
                assert!(view.read(cx).state.read(cx).selected_item().is_none());
            });
            assert!(changes.borrow().is_empty());
        }
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn group_and_options_expose_names_checked_and_disabled_state(cx: &mut TestAppContext) {
        let (cx, view, _) = harness(cx, items(), Some("medium"));
        cx.update(|window, _| {
            let group = crate::test_support::find(window, &[], &"group".into()).unwrap();
            assert_eq!(group.role(), Some(Role::RadioGroup));
            assert_eq!(group.label(), Some("Size"));
            for (index, label) in ["Small", "Unavailable", "Medium", "Large"]
                .iter()
                .enumerate()
            {
                let option =
                    crate::test_support::find(window, &[], &("radio-option", index).into())
                        .unwrap();
                assert_eq!(option.role(), Some(Role::RadioButton));
                assert_eq!(option.label(), Some(*label));
                assert_eq!(option.checked(), Some(index == 2));
                assert_eq!(option.disabled(), (index == 1).then_some(true));
            }
        });
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(
                crate::test_support::find(window, &[], &"group".into())
                    .unwrap()
                    .disabled(),
                Some(true)
            );
            for index in 0usize..4 {
                assert_eq!(
                    crate::test_support::find(window, &[], &("radio-option", index).into())
                        .unwrap()
                        .disabled(),
                    Some(true)
                );
            }
        });
    }
}
