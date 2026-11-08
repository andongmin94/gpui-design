//! Multiple selection composed from Toggle, with one roving Tab stop.
//! GPUI Kit v0.9.0 supplied an unstyled toggle-group container; GPUI Design adds
//! retained multiple selection and independent, orientation-aware focus movement.

use gpui::{
    AnyElement, App, Axis, Context, ElementId, Entity, EntityId, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window,
    accesskit, div, prelude::FluentBuilder as _,
};

use crate::{
    TestSupportExt as _, Toggle,
    a11y_disabled::A11yDisabledExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
};

const CONTEXT: &str = "ToggleGroup";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(
            "left",
            SelectLeft,
            Some("ToggleGroup && axis == horizontal"),
        ),
        KeyBinding::new(
            "right",
            SelectRight,
            Some("ToggleGroup && axis == horizontal"),
        ),
        KeyBinding::new("up", SelectUp, Some("ToggleGroup && axis == vertical")),
        KeyBinding::new("down", SelectDown, Some("ToggleGroup && axis == vertical")),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
    ]);
}

/// An item in a fixed, ordered group. Values must be unique.
#[derive(Clone, Debug)]
pub struct ToggleItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl ToggleItem {
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

/// The selected values after one option changes, in the original item order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToggleGroupEvent {
    Change(Vec<SharedString>),
}

/// Retained multiple selection and focus for one group in one window.
///
/// Tab enters the last focused enabled option, initially the first enabled item.
/// Arrow keys move focus without changing pressed values. Activation toggles just
/// the focused or clicked option; any number of items can be pressed, including none.
pub struct ToggleGroupState {
    items: Vec<ToggleItem>,
    pressed: Vec<bool>,
    cursor: Option<usize>,
    disabled: bool,
    focus: Vec<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
}

impl ToggleGroupState {
    pub fn new(items: impl IntoIterator<Item = ToggleItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.value != item.value),
                "ToggleItem values must be unique"
            );
        }
        let cursor = items.iter().position(|item| !item.disabled);
        let focus = items.iter().map(|_| cx.focus_handle()).collect();
        Self {
            pressed: vec![false; items.len()],
            items,
            cursor,
            disabled: false,
            focus,
            focus_subscriptions: Vec::new(),
        }
    }

    /// Sets initially pressed values without emitting an event.
    ///
    /// # Panics
    /// Every value must identify an enabled item in the fixed collection.
    pub fn default_values<V: AsRef<str>>(mut self, values: impl IntoIterator<Item = V>) -> Self {
        self.pressed.fill(false);
        for value in values {
            let index = self
                .items
                .iter()
                .position(|item| item.value.as_ref() == value.as_ref() && !item.disabled)
                .expect("ToggleGroupState default_values must identify enabled ToggleItems");
            self.pressed[index] = true;
        }
        self
    }

    pub fn items(&self) -> &[ToggleItem] {
        &self.items
    }

    pub fn is_pressed(&self, index: usize) -> bool {
        self.pressed[index]
    }

    pub fn selected_values(&self) -> Vec<SharedString> {
        self.items
            .iter()
            .zip(&self.pressed)
            .filter_map(|(item, pressed)| pressed.then(|| item.value.clone()))
            .collect()
    }

    pub fn option_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.focus[index]
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Disables user changes and normal Tab entry while retaining pressed values.
    /// An already-focused option keeps Button's focus binding so Tab can leave.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
        window.refresh();
    }

    fn bind_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus_subscriptions.is_empty() {
            return;
        }
        self.focus_subscriptions = self
            .focus
            .iter()
            .enumerate()
            .map(|(index, focus)| {
                cx.on_focus(focus, window, move |_, window, cx| {
                    // Native focus notifications happen during drawing. Remember
                    // the new Tab entry after that draw, without changing selection.
                    cx.defer_in(window, move |state, window, cx| {
                        if state.focus[index].is_focused(window) {
                            state.remember_focus(index, window, cx);
                        }
                    });
                })
            })
            .collect();
    }

    fn remember_focus(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.items[index].disabled || self.cursor == Some(index) {
            return;
        }
        self.cursor = Some(index);
        cx.notify();
        window.refresh();
    }

    fn focus_option(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.items[index].disabled {
            return;
        }
        self.focus[index].focus(window, cx);
        self.remember_focus(index, window, cx);
    }

    fn set_pressed(
        &mut self,
        index: usize,
        pressed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.items[index].disabled {
            return;
        }
        self.focus_option(index, window, cx);
        if self.pressed[index] == pressed {
            return;
        }
        self.pressed[index] = pressed;
        cx.emit(ToggleGroupEvent::Change(self.selected_values()));
        cx.notify();
        window.refresh();
    }

    fn focused_index(&self, window: &Window) -> Option<usize> {
        self.focus.iter().position(|focus| focus.is_focused(window))
    }

    fn move_focus(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let Some(current) = self.focused_index(window) else {
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
            .find(|&index| !self.items[index].disabled)
        {
            self.focus_option(next, window, cx);
        }
    }

    fn focus_edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.focused_index(window).is_none() {
            cx.propagate();
            return;
        }
        let next = if last {
            self.items.iter().rposition(|item| !item.disabled)
        } else {
            self.items.iter().position(|item| !item.disabled)
        };
        if let Some(next) = next {
            self.focus_option(next, window, cx);
        }
    }
}

impl EventEmitter<ToggleGroupEvent> for ToggleGroupState {}

fn option_id(state: EntityId, index: usize) -> ElementId {
    (ElementId::from(("toggle-option", state)), index.to_string()).into()
}

/// An unstyled multiple-selection group composed from [`ToggleOption`] children.
/// Render one option per item in collection order; axis changes keyboard and
/// accessibility orientation, while application styles choose the actual layout.
#[derive(IntoElement)]
pub struct ToggleGroup {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ToggleGroupState>,
    label: Option<SharedString>,
    axis: Axis,
}

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ToggleGroupState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
            axis: Axis::Horizontal,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

impl Styled for ToggleGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ToggleGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for ToggleGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ToggleGroup {}

impl RenderOnce for ToggleGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, cx| state.bind_focus(window, cx));
        let disabled = self.state.read(cx).disabled;
        let previous = self.state.clone();
        let next = self.state.clone();
        let first = self.state.clone();
        let last = self.state;
        let base = self
            .base
            .role(Role::Group)
            .when_some(self.label, |this, label| this.aria_label(label))
            .on_action(move |_: &SelectFirst, window, cx| {
                first.update(cx, |state, cx| state.focus_edge(false, window, cx));
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| state.focus_edge(true, window, cx));
            });
        match self.axis {
            Axis::Horizontal => base
                .aria_orientation(accesskit::Orientation::Horizontal)
                .key_context("ToggleGroup axis=horizontal")
                .on_action(move |_: &SelectLeft, window, cx| {
                    previous.update(cx, |state, cx| state.move_focus(false, window, cx));
                })
                .on_action(move |_: &SelectRight, window, cx| {
                    next.update(cx, |state, cx| state.move_focus(true, window, cx));
                }),
            Axis::Vertical => base
                .aria_orientation(accesskit::Orientation::Vertical)
                .key_context("ToggleGroup axis=vertical")
                .on_action(move |_: &SelectUp, window, cx| {
                    previous.update(cx, |state, cx| state.move_focus(false, window, cx));
                })
                .on_action(move |_: &SelectDown, window, cx| {
                    next.update(cx, |state, cx| state.move_focus(true, window, cx));
                }),
        }
        .a11y_disabled(disabled)
    }
}

/// One group option, composed from [`Toggle`] with group-owned state and focus.
/// Supply non-interactive content; its label and pressed value come from the state.
#[derive(IntoElement)]
pub struct ToggleOption {
    toggle: Toggle,
    state: Entity<ToggleGroupState>,
    index: usize,
}

impl ToggleOption {
    pub fn new(state: &Entity<ToggleGroupState>, index: usize) -> Self {
        Self {
            toggle: Toggle::new(option_id(state.entity_id(), index)),
            state: state.clone(),
            index,
        }
    }
}

impl Styled for ToggleOption {
    fn style(&mut self) -> &mut StyleRefinement {
        self.toggle.style()
    }
}

impl ParentElement for ToggleOption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.toggle.extend(elements);
    }
}

impl InteractiveElement for ToggleOption {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.toggle.interactivity()
    }
}

impl StatefulInteractiveElement for ToggleOption {}

impl RenderOnce for ToggleOption {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("ToggleOption index must identify an item");
        let disabled = model.disabled || item.disabled;
        let pressed = model.is_pressed(self.index);
        let tab_stop = model.cursor == Some(self.index);
        let focus = model.option_focus_handle(self.index).clone();
        let label = item.label.clone();
        let index = self.index;
        let state = self.state;
        self.toggle
            .accessibility_label(label)
            .pressed(pressed)
            .disabled(disabled)
            .track_focus(&focus)
            .tab_stop(tab_stop)
            .on_change(move |pressed, _, window, cx| {
                state.update(cx, |state, cx| {
                    state.set_pressed(index, pressed, window, cx)
                });
            })
            .render(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render,
        TestAppContext, Toggled, VisualTestContext, px,
    };

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        state: Entity<ToggleGroupState>,
        axis: Axis,
        before: FocusHandle,
        after: FocusHandle,
        unhandled: Rc<RefCell<Vec<String>>>,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let unhandled = self.unhandled.clone();
            div()
                .w(px(400.))
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
                .child(
                    ToggleGroup::new("group", &self.state)
                        .axis(self.axis)
                        .accessibility_label("Text emphasis")
                        .flex()
                        .when(self.axis == Axis::Vertical, |this| this.flex_col())
                        .children(self.state.read(cx).items().iter().enumerate().map(
                            |(index, item)| {
                                ToggleOption::new(&self.state, index)
                                    .debug_selector(move || format!("option-{index}"))
                                    .w(px(100.))
                                    .h(px(32.))
                                    .child(item.label().clone())
                            },
                        )),
                )
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<Vec<SharedString>>>>;

    fn harness(
        cx: &mut TestAppContext,
        axis: Axis,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| {
                    ToggleGroupState::new(
                        [
                            ToggleItem::new("bold", "Bold"),
                            ToggleItem::new("underline", "Unavailable").disabled(true),
                            ToggleItem::new("italic", "Italic"),
                            ToggleItem::new("strike", "Strike"),
                        ],
                        cx,
                    )
                    .default_values(["bold"])
                });
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    axis,
                    before: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
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
            cx.subscribe(&state, move |_, event: &ToggleGroupEvent, _| {
                let ToggleGroupEvent::Change(values) = event;
                changes.borrow_mut().push(values.clone());
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
    fn focus_navigation_is_independent_of_multiple_selection(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal);
        cx.simulate_keystrokes("tab right");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert!(state.option_focus_handle(2).is_focused(window));
            assert_eq!(state.selected_values(), [SharedString::from("bold")]);
        });
        assert!(changes.borrow().is_empty());
        press(cx, "space");
        press(cx, "enter");
        cx.simulate_keystrokes("end");
        draw(cx);
        press(cx, "enter");
        cx.simulate_keystrokes("right");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .option_focus_handle(0)
                    .is_focused(window)
            );
        });
        let bold = cx.debug_bounds("option-0").unwrap().center();
        cx.simulate_click(bold, Modifiers::default());
        draw(cx);
        let disabled = cx.debug_bounds("option-1").unwrap().center();
        cx.simulate_click(disabled, Modifiers::default());
        draw(cx);
        assert_eq!(
            *changes.borrow(),
            [
                vec!["bold", "italic"],
                vec!["bold"],
                vec!["bold", "strike"],
                vec!["strike"]
            ]
            .map(|values| values
                .into_iter()
                .map(SharedString::from)
                .collect::<Vec<_>>())
        );
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert!(state.option_focus_handle(0).is_focused(window));
            assert_eq!(state.selected_values(), [SharedString::from("strike")]);
        });
    }

    #[gpui::test]
    fn vertical_and_programmatic_focus_preserve_one_tab_entry_and_disabled_values(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness(cx, Axis::Vertical);
        cx.simulate_keystrokes("tab down");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .option_focus_handle(2)
                    .is_focused(window)
            );
            view.read(cx).unhandled.borrow_mut().clear();
        });
        cx.simulate_keystrokes("left right");
        draw(cx);
        cx.update(|window, cx| {
            assert_eq!(*view.read(cx).unhandled.borrow(), ["left", "right"]);
            let focus = view.read(cx).state.read(cx).option_focus_handle(3).clone();
            focus.focus(window, cx);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .option_focus_handle(3)
                    .is_focused(window)
            );
        });
        assert!(changes.borrow().is_empty());
        press(cx, "space");
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
            let group = ToggleGroup::new("native-group", &state)
                .axis(Axis::Vertical)
                .accessibility_label("Text emphasis")
                .render(window, cx)
                .into_element();
            assert_eq!(group.a11y_role(), Some(Role::Group));
            let mut node = accesskit::Node::new(Role::Group);
            group.write_a11y_info(&mut node);
            assert_eq!(node.label(), Some("Text emphasis"));
            assert_eq!(node.orientation(), Some(accesskit::Orientation::Vertical));
            assert!(node.is_disabled());
            let option = ToggleOption::new(&state, 3)
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(option.a11y_role().unwrap());
            option.write_a11y_info(&mut node);
            assert_eq!(node.role(), Role::Button);
            assert_eq!(node.toggled(), Some(Toggled::True));
            assert!(node.is_disabled());
        });
        draw(cx);
        press(cx, "space");
        cx.simulate_keystrokes("down home tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert!(view.read(cx).before.is_focused(window));
            assert_eq!(
                state.read(cx).selected_values(),
                ["bold", "strike"].map(SharedString::from)
            );
            state.update(cx, |state, cx| state.set_disabled(false, window, cx));
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .option_focus_handle(3)
                    .is_focused(window)
            );
        });
        assert_eq!(changes.borrow().len(), 1);
    }
}
