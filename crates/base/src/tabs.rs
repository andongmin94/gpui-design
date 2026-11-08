//! Automatically activated tabs with roving focus and active-only panels.
//! Derived from GPUI Kit v0.9.0's tab primitive; GPUI Design adds retained
//! selection, keyboard navigation, focus activation, and panel lifecycle.

use gpui::{
    AnyElement, App, Axis, Context, ElementId, Entity, EntityId, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Subscription, Window, accesskit, div, prelude::FluentBuilder as _,
};

use crate::{
    StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
};

const CONTEXT: &str = "TabList";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some("TabList && axis == horizontal")),
        KeyBinding::new("right", SelectRight, Some("TabList && axis == horizontal")),
        KeyBinding::new("up", SelectUp, Some("TabList && axis == vertical")),
        KeyBinding::new("down", SelectDown, Some("TabList && axis == vertical")),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
    ]);
}

/// One tab in a fixed, ordered collection. Values must be unique.
#[derive(Clone, Debug)]
pub struct TabItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl TabItem {
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

/// Emitted once after a different enabled tab becomes selected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TabsEvent {
    Change(SharedString),
}

/// Retained selection and focus for one tab list and its panels in one window.
///
/// The first enabled tab starts selected. Moving focus to another enabled tab
/// activates it, including arrow-key and accessibility/programmatic focus.
/// The application keeps panel data in its own retained entities; an inactive
/// [`TabPanel`] contributes no element, focus stop, or accessibility node.
pub struct TabsState {
    items: Vec<TabItem>,
    selected: Option<usize>,
    tab_focus: Vec<FocusHandle>,
    panel_focus: Vec<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
}

impl TabsState {
    pub fn new(items: impl IntoIterator<Item = TabItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (ix, item) in items.iter().enumerate() {
            assert!(
                items[..ix].iter().all(|other| other.value != item.value),
                "TabItem values must be unique"
            );
        }
        let selected = items.iter().position(|item| !item.disabled);
        let tab_focus = items.iter().map(|_| cx.focus_handle()).collect();
        let panel_focus = items.iter().map(|_| cx.focus_handle()).collect();
        Self {
            items,
            selected,
            tab_focus,
            panel_focus,
            focus_subscriptions: Vec::new(),
        }
    }

    /// Selects an enabled value before the state is first rendered.
    ///
    /// # Panics
    /// Panics if `value` does not identify an enabled item in this collection.
    pub fn default_value(mut self, value: impl AsRef<str>) -> Self {
        self.selected = Some(
            self.items
                .iter()
                .position(|item| item.value.as_ref() == value.as_ref() && !item.disabled)
                .expect("TabsState default_value must identify an enabled TabItem"),
        );
        self
    }

    pub fn items(&self) -> &[TabItem] {
        &self.items
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&TabItem> {
        self.selected.map(|index| &self.items[index])
    }

    /// The stable focus handle of a tab in the fixed collection.
    pub fn tab_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.tab_focus[index]
    }

    /// The stable scope handle of a panel. Only the selected panel is rendered.
    pub fn panel_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.panel_focus[index]
    }

    fn bind_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus_subscriptions.is_empty() {
            return;
        }
        self.focus_subscriptions = self
            .tab_focus
            .iter()
            .enumerate()
            .map(|(index, focus)| {
                cx.on_focus(focus, window, move |_, window, cx| {
                    // Focus notifications occur while drawing. Commit after the
                    // draw so a second frame can mount the newly active panel.
                    cx.defer_in(window, move |state, window, cx| {
                        if state.tab_focus[index].is_focused(window) {
                            state.select(index, window, cx);
                        }
                    });
                })
            })
            .collect();
    }

    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.items[index].disabled {
            return;
        }
        self.tab_focus[index].focus(window, cx);
        if self.selected == Some(index) {
            return;
        }
        self.selected = Some(index);
        cx.emit(TabsEvent::Change(self.items[index].value.clone()));
        cx.notify();
        window.refresh();
    }

    fn focused_index(&self, window: &Window) -> Option<usize> {
        self.tab_focus
            .iter()
            .position(|focus| focus.is_focused(window))
    }

    fn move_selection(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
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
            self.select(next, window, cx);
        }
    }

    fn select_edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.focused_index(window).is_none() {
            cx.propagate();
            return;
        }
        let index = if last {
            self.items.iter().rposition(|item| !item.disabled)
        } else {
            self.items.iter().position(|item| !item.disabled)
        };
        if let Some(index) = index {
            self.select(index, window, cx);
        }
    }
}

impl EventEmitter<TabsEvent> for TabsState {}

// Parts remain distinct when multiple tab states render as siblings without a
// dedicated wrapper. These are GPUI element IDs, not accessibility relation IDs.
fn part_id(state: EntityId, part: &'static str, index: usize) -> ElementId {
    (ElementId::from((part, state)), index.to_string()).into()
}

/// The unstyled list of tab triggers. Render one [`Tab`] per item in order.
///
/// The default horizontal list uses Left/Right. A vertical list uses Up/Down.
/// Home/End select the first/last enabled tab. Other-axis arrows are left to the
/// enclosing application. Axis sets semantics and keyboard behavior, not layout.
#[derive(IntoElement)]
pub struct TabList {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<TabsState>,
    label: Option<SharedString>,
    axis: Axis,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TabList {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TabsState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
            axis: Axis::Horizontal,
            style: StyleRefinement::default(),
            children: Vec::new(),
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

impl Styled for TabList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TabList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for TabList {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for TabList {}

impl RenderOnce for TabList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, cx| state.bind_focus(window, cx));
        let previous_state = self.state.clone();
        let next_state = self.state.clone();
        let first_state = self.state.clone();
        let last_state = self.state;
        let base = self
            .base
            .role(Role::TabList)
            .when_some(self.label, |this, label| this.aria_label(label))
            .on_action(move |_: &SelectFirst, window, cx| {
                first_state.update(cx, |state, cx| state.select_edge(false, window, cx));
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last_state.update(cx, |state, cx| state.select_edge(true, window, cx));
            });

        match self.axis {
            Axis::Horizontal => base
                .aria_orientation(accesskit::Orientation::Horizontal)
                .key_context("TabList axis=horizontal")
                .on_action(move |_: &SelectLeft, window, cx| {
                    previous_state.update(cx, |state, cx| state.move_selection(false, window, cx));
                })
                .on_action(move |_: &SelectRight, window, cx| {
                    next_state.update(cx, |state, cx| state.move_selection(true, window, cx));
                }),
            Axis::Vertical => base
                .aria_orientation(accesskit::Orientation::Vertical)
                .key_context("TabList axis=vertical")
                .on_action(move |_: &SelectUp, window, cx| {
                    previous_state.update(cx, |state, cx| state.move_selection(false, window, cx));
                })
                .on_action(move |_: &SelectDown, window, cx| {
                    next_state.update(cx, |state, cx| state.move_selection(true, window, cx));
                }),
        }
        .children(self.children)
        .refine_style(&self.style)
    }
}

/// An unstyled trigger for one tab. The state owns its label, selection and
/// focus. Applications supply its visual content and state styling.
#[derive(IntoElement)]
pub struct Tab {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<TabsState>,
    index: usize,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Tab {
    /// `index` must identify an item in the enclosing tab list's state.
    pub fn new(state: &Entity<TabsState>, index: usize) -> Self {
        Self {
            base: div()
                .id(part_id(state.entity_id(), "tab", index))
                .test_support(),
            state: state.clone(),
            index,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for Tab {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl RenderOnce for Tab {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("Tab index must identify an item");
        let disabled = item.disabled;
        let selected = model.selected == Some(self.index);
        let focus = model.tab_focus_handle(self.index).clone();
        let label = item.label.clone();
        let size = model.items.len();
        let index = self.index;
        let click_state = self.state;

        self.base
            .role(Role::Tab)
            .aria_label(label)
            .aria_selected(selected)
            .aria_position_in_set(index + 1)
            .aria_size_of_set(size)
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(selected && !disabled))
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

/// Content for one tab, mounted only while that tab is selected.
///
/// The panel's accessible name is its tab label. The pinned GPUI API cannot
/// express explicit tab/panel accessibility relationships; element IDs are not
/// substituted for native accessibility node IDs.
///
/// By default the panel scope is not a Tab stop, allowing Tab to reach its first
/// focusable child. Set [`Self::tab_stop`] to `true` for a panel whose leading content
/// should itself receive keyboard focus, such as a text-only help panel.
#[derive(IntoElement)]
pub struct TabPanel {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<TabsState>,
    index: usize,
    tab_stop: bool,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl TabPanel {
    /// `index` must identify the matching tab in this state's fixed collection.
    pub fn new(state: &Entity<TabsState>, index: usize) -> Self {
        Self {
            base: div()
                .id(part_id(state.entity_id(), "tab-panel", index))
                .test_support(),
            state: state.clone(),
            index,
            tab_stop: false,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }
}

impl Styled for TabPanel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TabPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for TabPanel {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for TabPanel {}

impl RenderOnce for TabPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("TabPanel index must identify an item");
        if model.selected != Some(self.index) {
            return gpui::Empty.into_any_element();
        }
        let focus = model.panel_focus_handle(self.index).clone();
        self.base
            .role(Role::TabPanel)
            .aria_label(item.label.clone())
            .track_focus(&focus.tab_stop(self.tab_stop))
            .children(self.children)
            .refine_style(&self.style)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext,
        VisualTestContext, point, px,
    };

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        state: Entity<TabsState>,
        axis: Axis,
        before: FocusHandle,
        panel_button: FocusHandle,
        last_button: FocusHandle,
        after: FocusHandle,
        unhandled_keys: Rc<RefCell<Vec<String>>>,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let unhandled_keys = self.unhandled_keys.clone();
            let tabs = self
                .state
                .read(cx)
                .items()
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    Tab::new(&self.state, index)
                        .w(px(120.))
                        .h(px(32.))
                        .child(item.label().clone())
                });
            div()
                .w(px(480.))
                .flex()
                .flex_col()
                .on_key_down(move |event, _, _| {
                    unhandled_keys
                        .borrow_mut()
                        .push(event.keystroke.key.clone());
                })
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .h(px(32.))
                        .child("Before"),
                )
                .child(
                    TabList::new("tabs", &self.state)
                        .axis(self.axis)
                        .accessibility_label("Sections")
                        .flex()
                        .when(self.axis == Axis::Vertical, |this| this.flex_col())
                        .children(tabs),
                )
                .children((0..self.state.read(cx).items().len()).map(|index| {
                    TabPanel::new(&self.state, index)
                        .tab_stop(index == 2)
                        .h(px(40.))
                        .child(match index {
                            0 => Button::new("panel-button")
                                .accessibility_label("Panel action")
                                .track_focus(&self.panel_button)
                                .h(px(32.))
                                .child("Panel action")
                                .into_any_element(),
                            3 => Button::new("last-button")
                                .track_focus(&self.last_button)
                                .h(px(32.))
                                .child("Last action")
                                .into_any_element(),
                            _ => div().child("Panel text").into_any_element(),
                        })
                }))
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<SharedString>>>;

    fn items() -> Vec<TabItem> {
        vec![
            TabItem::new("general", "General"),
            TabItem::new("disabled", "Unavailable").disabled(true),
            TabItem::new("help", "Help"),
            TabItem::new("advanced", "Advanced"),
        ]
    }

    fn harness(
        cx: &mut TestAppContext,
        axis: Axis,
        items: Vec<TabItem>,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| TabsState::new(items, cx));
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    axis,
                    before: cx.focus_handle().tab_stop(true),
                    panel_button: cx.focus_handle().tab_stop(true),
                    last_button: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
                    unhandled_keys: Rc::new(RefCell::new(Vec::new())),
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
            cx.subscribe(&state, move |_, event: &TabsEvent, _| {
                let TabsEvent::Change(value) = event;
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

    fn assert_tab(cx: &mut VisualTestContext, view: &Entity<Harness>, index: usize) {
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(state.selected_index(), Some(index));
            assert!(state.tab_focus_handle(index).is_focused(window));
        });
    }

    #[gpui::test]
    fn horizontal_navigation_wraps_skips_disabled_and_preserves_other_axis(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal, items());
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_tab(cx, &view, 0);

        for (key, index) in [
            ("left", 3),
            ("home", 0),
            ("right", 2),
            ("end", 3),
            ("right", 0),
        ] {
            cx.simulate_keystrokes(key);
            draw(cx);
            assert_tab(cx, &view, index);
        }
        assert_eq!(
            *changes.borrow(),
            ["advanced", "general", "help", "advanced", "general"].map(SharedString::from)
        );
        cx.update(|_, cx| view.read(cx).unhandled_keys.borrow_mut().clear());
        cx.simulate_keystrokes("up down");
        draw(cx);
        assert_tab(cx, &view, 0);
        cx.update(|_, cx| {
            assert_eq!(*view.read(cx).unhandled_keys.borrow(), ["up", "down"]);
        });

        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).panel_button.is_focused(window)));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab shift-tab");
        draw(cx);
        assert_tab(cx, &view, 0);
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).before.is_focused(window)));
    }

    #[gpui::test]
    fn vertical_navigation_and_text_panel_focus_follow_their_contract(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Vertical, items());
        cx.simulate_keystrokes("tab down");
        draw(cx);
        assert_tab(cx, &view, 2);
        cx.update(|_, cx| view.read(cx).unhandled_keys.borrow_mut().clear());
        cx.simulate_keystrokes("left right");
        draw(cx);
        assert_tab(cx, &view, 2);
        cx.update(|_, cx| {
            assert_eq!(*view.read(cx).unhandled_keys.borrow(), ["left", "right"]);
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .panel_focus_handle(2)
                    .is_focused(window)
            );
            assert!(!view.read(cx).panel_button.is_focused(window));
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab shift-tab");
        draw(cx);
        assert_tab(cx, &view, 2);
        cx.simulate_keystrokes("up");
        draw(cx);
        assert_tab(cx, &view, 0);
        assert_eq!(
            *changes.borrow(),
            [SharedString::from("help"), SharedString::from("general")]
        );
    }

    #[gpui::test]
    fn pointer_and_native_activation_do_not_duplicate_selection(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal, items());
        cx.simulate_keystrokes("tab");
        draw(cx);
        press(cx, "space");
        press(cx, "enter");
        assert!(changes.borrow().is_empty());
        // Horizontal tabs occupy 120px each below the 32px "Before" button.
        cx.simulate_click(point(px(250.), px(48.)), Modifiers::default());
        draw(cx);
        assert_tab(cx, &view, 2);
        cx.simulate_click(point(px(250.), px(48.)), Modifiers::default());
        draw(cx);
        cx.simulate_click(point(px(130.), px(48.)), Modifiers::default());
        draw(cx);
        press(cx, "space");
        press(cx, "enter");
        assert_tab(cx, &view, 2);
        assert_eq!(*changes.borrow(), [SharedString::from("help")]);
    }

    #[gpui::test]
    fn direct_focus_activates_and_unmounts_previous_panel(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal, items());
        cx.simulate_keystrokes("tab tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).panel_button.is_focused(window));
            let focus = view.read(cx).state.read(cx).tab_focus_handle(3).clone();
            focus.focus(window, cx);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        assert_tab(cx, &view, 3);
        assert_eq!(*changes.borrow(), [SharedString::from("advanced")]);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).last_button.is_focused(window));
            assert!(!view.read(cx).panel_button.is_focused(window));
        });
    }

    #[gpui::test]
    fn empty_all_disabled_and_single_enabled_collections_are_well_defined(cx: &mut TestAppContext) {
        for items in [
            Vec::new(),
            vec![TabItem::new("disabled", "Unavailable").disabled(true)],
        ] {
            let (cx, view, changes) = harness(cx, Axis::Horizontal, items);
            cx.simulate_keystrokes("tab");
            draw(cx);
            cx.update(|window, cx| {
                assert!(view.read(cx).after.is_focused(window));
                assert!(view.read(cx).state.read(cx).selected_index().is_none());
            });
            cx.simulate_keystrokes("shift-tab");
            draw(cx);
            cx.update(|window, cx| assert!(view.read(cx).before.is_focused(window)));
            assert!(changes.borrow().is_empty());
        }
        let (cx, view, changes) = harness(cx, Axis::Horizontal, vec![TabItem::new("only", "Only")]);
        cx.simulate_keystrokes("tab right left home end");
        draw(cx);
        assert_tab(cx, &view, 0);
        assert!(changes.borrow().is_empty());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn accessibility_exposes_only_active_panel_and_selected_tab(cx: &mut TestAppContext) {
        let (cx, view, _) = harness(cx, Axis::Horizontal, items());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let list = crate::test_support::find(window, &[], &"tabs".into()).unwrap();
            assert_eq!(list.role(), Some(Role::TabList));
            assert_eq!(list.label(), Some("Sections"));
            for (index, item) in state.read(cx).items().iter().enumerate() {
                let tab = crate::test_support::find(
                    window,
                    &[],
                    &part_id(state.entity_id(), "tab", index),
                )
                .unwrap();
                assert_eq!(tab.role(), Some(Role::Tab));
                assert_eq!(tab.label(), Some(item.label().as_ref()));
                assert_eq!(tab.selected(), Some(index == 0));
                assert_eq!(tab.disabled(), (index == 1).then_some(true));
            }
            let panels: Vec<_> = crate::test_support::snapshots(window)
                .into_iter()
                .filter(|item| item.role() == Some(Role::TabPanel))
                .collect();
            assert_eq!(panels.len(), 1);
            assert_eq!(panels[0].label(), Some("General"));
        });
        cx.simulate_keystrokes("tab right");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert!(
                crate::test_support::find(window, &[], &part_id(state.entity_id(), "tab-panel", 0))
                    .is_none()
            );
            assert!(crate::test_support::find(window, &[], &"panel-button".into()).is_none());
            let panel =
                crate::test_support::find(window, &[], &part_id(state.entity_id(), "tab-panel", 2))
                    .unwrap();
            assert_eq!(panel.role(), Some(Role::TabPanel));
            assert_eq!(panel.label(), Some("Help"));
            let tab = crate::test_support::find(window, &[], &part_id(state.entity_id(), "tab", 2))
                .unwrap();
            assert_eq!(tab.selected(), Some(true));
        });
    }
}
