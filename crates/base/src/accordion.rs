//! A single-open, collapsible accordion with native header activation.
//! Derived from GPUI Kit v0.9.0's accordion primitive; GPUI Design adds retained
//! state, keyboard navigation, focus ownership, and active-only panels.

use gpui::{
    AnyElement, App, Context, ElementId, Entity, EntityId, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
};

use crate::{
    StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectUp},
};

const CONTEXT: &str = "Accordion";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
    ]);
}

/// One item in a fixed, ordered accordion. Values must be unique.
#[derive(Clone, Debug)]
pub struct AccordionItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl AccordionItem {
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

/// Emitted once when the open item changes. `None` means all items are closed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccordionEvent {
    Change(Option<SharedString>),
}

/// Retained state for one single-open accordion in one window.
///
/// All items start collapsed. Every enabled header is a Tab stop; moving focus
/// with Up, Down, Home, or End never changes the open item. The application keeps
/// panel data in its own retained entities, independently of panel mounting.
pub struct AccordionState {
    items: Vec<AccordionItem>,
    open: Option<usize>,
    root_focus: FocusHandle,
    trigger_focus: Vec<FocusHandle>,
    panel_focus: Vec<FocusHandle>,
}

impl AccordionState {
    pub fn new(items: impl IntoIterator<Item = AccordionItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.value != item.value),
                "AccordionItem values must be unique"
            );
        }
        let trigger_focus = items.iter().map(|_| cx.focus_handle()).collect();
        let panel_focus = items.iter().map(|_| cx.focus_handle()).collect();
        Self {
            items,
            open: None,
            root_focus: cx.focus_handle(),
            trigger_focus,
            panel_focus,
        }
    }

    /// Opens an enabled value before the state is first rendered.
    ///
    /// # Panics
    /// Panics if `value` does not identify an enabled item in this collection.
    pub fn default_open(mut self, value: impl AsRef<str>) -> Self {
        self.open = Some(
            self.items
                .iter()
                .position(|item| item.value.as_ref() == value.as_ref() && !item.disabled)
                .expect("AccordionState default_open must identify an enabled AccordionItem"),
        );
        self
    }

    pub fn items(&self) -> &[AccordionItem] {
        &self.items
    }

    pub fn open_index(&self) -> Option<usize> {
        self.open
    }

    pub fn open_item(&self) -> Option<&AccordionItem> {
        self.open.map(|index| &self.items[index])
    }

    /// The stable focus handle of a header's native button.
    pub fn trigger_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.trigger_focus[index]
    }

    /// The stable focus scope of a panel, mounted only while that item is open.
    pub fn panel_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.panel_focus[index]
    }

    /// Opens a closed item, closing the previous item, or collapses an open item.
    /// Disabled items ignore requests. `index` must identify an item.
    ///
    /// If this removes the focused panel, focus moves to the requested header.
    /// Focus elsewhere is preserved, including a header already focused by a click.
    pub fn toggle(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.items[index].disabled {
            return;
        }
        self.set_open((self.open != Some(index)).then_some(index), window, cx);
    }

    /// Collapses the open panel and returns its focus to the still-mounted header.
    /// Focus outside the panel is preserved. Removing the whole accordion remains
    /// the application's responsibility, including its choice of focus target.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_open(None, window, cx);
    }

    fn set_open(&mut self, next: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        if self.open == next {
            return;
        }
        if let Some(previous) = self.open {
            let target = &self.trigger_focus[next.unwrap_or(previous)];
            // A detached panel may leave its scope focused. Only restore a
            // header that is still inside the rendered accordion's focus tree.
            if self.panel_focus[previous].contains_focused(window, cx)
                && self.root_focus.contains(target, window)
            {
                target.focus(window, cx);
            }
        }
        self.open = next;
        cx.emit(AccordionEvent::Change(
            self.open_item().map(|item| item.value.clone()),
        ));
        cx.notify();
        window.refresh();
    }

    fn focused_index(&self, window: &Window) -> Option<usize> {
        self.trigger_focus
            .iter()
            .position(|focus| focus.is_focused(window))
    }

    fn move_focus(&self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
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
            self.trigger_focus[next].focus(window, cx);
        }
    }

    fn focus_edge(&self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
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
            self.trigger_focus[next].focus(window, cx);
        }
    }
}

impl EventEmitter<AccordionEvent> for AccordionState {}

fn part_id(state: EntityId, part: &'static str, index: usize) -> ElementId {
    (ElementId::from((part, state)), index.to_string()).into()
}

/// An unstyled accordion root. Render each header followed by its matching panel
/// in the state's item order; applications own item wrappers and all layout.
#[derive(IntoElement)]
pub struct Accordion {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<AccordionState>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Accordion {
    pub fn new(id: impl Into<ElementId>, state: &Entity<AccordionState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for Accordion {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Accordion {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Accordion {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Accordion {}

impl RenderOnce for Accordion {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let root_focus = self.state.read(cx).root_focus.clone();
        let previous = self.state.clone();
        let next = self.state.clone();
        let first = self.state.clone();
        let last = self.state;
        self.base
            .role(Role::Group)
            .key_context(CONTEXT)
            .track_focus(&root_focus.tab_stop(false))
            .on_action(move |_: &SelectUp, window, cx| {
                previous.update(cx, |state, cx| state.move_focus(false, window, cx));
            })
            .on_action(move |_: &SelectDown, window, cx| {
                next.update(cx, |state, cx| state.move_focus(true, window, cx));
            })
            .on_action(move |_: &SelectFirst, window, cx| {
                first.update(cx, |state, cx| state.focus_edge(false, window, cx));
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| state.focus_edge(true, window, cx));
            })
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// A heading containing a native button for one item.
///
/// Styles, interactive refinements, and children apply to the button. The heading
/// adds structure without decorative geometry. Supply only non-interactive button
/// content; application actions belong outside the header button.
#[derive(IntoElement)]
pub struct AccordionHeader {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<AccordionState>,
    index: usize,
    heading_level: usize,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl AccordionHeader {
    /// `index` must identify an item in the enclosing accordion's state.
    pub fn new(state: &Entity<AccordionState>, index: usize) -> Self {
        Self {
            base: div()
                .id(part_id(state.entity_id(), "accordion-trigger", index))
                .test_support(),
            state: state.clone(),
            index,
            heading_level: 3,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// The native heading level, defaulting to 3. `level` must be positive.
    pub fn heading_level(mut self, level: usize) -> Self {
        assert!(level > 0, "AccordionHeader heading_level must be positive");
        self.heading_level = level;
        self
    }
}

impl Styled for AccordionHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for AccordionHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for AccordionHeader {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for AccordionHeader {}

impl RenderOnce for AccordionHeader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("AccordionHeader index must identify an item");
        let disabled = item.disabled;
        let expanded = model.open == Some(self.index);
        let label = item.label.clone();
        let focus = model.trigger_focus_handle(self.index).clone();
        let heading_id = part_id(self.state.entity_id(), "accordion-heading", self.index);
        let index = self.index;
        let click_state = self.state;
        let button = self
            .base
            .role(Role::Button)
            .aria_label(label)
            .aria_expanded(expanded)
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(!disabled))
            })
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .when(!disabled, |this| {
                this.on_click(move |_, window, cx| {
                    click_state.update(cx, |state, cx| state.toggle(index, window, cx));
                })
            })
            .children(self.children)
            .refine_style(&self.style)
            .a11y_disabled(disabled);

        div()
            .id(heading_id)
            .test_support()
            .role(Role::Heading)
            .aria_level(self.heading_level)
            .child(button)
    }
}

/// An item's named region, mounted only while that item is open.
///
/// Its accessible name is the item's label. The pinned GPUI API cannot express
/// explicit button/panel accessibility relationships; element IDs are not native
/// accessibility node IDs. Keep panel data in application-owned retained entities.
///
/// By default Tab reaches the first focusable child. Set [`Self::tab_stop`] for a
/// panel whose leading content should itself receive focus, such as plain text.
#[derive(IntoElement)]
pub struct AccordionPanel {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<AccordionState>,
    index: usize,
    tab_stop: bool,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl AccordionPanel {
    /// `index` must identify the matching header in this state's collection.
    pub fn new(state: &Entity<AccordionState>, index: usize) -> Self {
        Self {
            base: div()
                .id(part_id(state.entity_id(), "accordion-panel", index))
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

impl Styled for AccordionPanel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for AccordionPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for AccordionPanel {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for AccordionPanel {}

impl RenderOnce for AccordionPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("AccordionPanel index must identify an item");
        if model.open != Some(self.index) {
            return gpui::Empty.into_any_element();
        }
        let focus = model.panel_focus_handle(self.index).clone();
        self.base
            .role(Role::Region)
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
        AppContext as _, Element as _, Focusable as _, KeyDownEvent, KeyUpEvent, Keystroke,
        Modifiers, Render, Subscription, TestAppContext, VisualTestContext, accesskit, canvas, px,
    };

    use super::*;
    use crate::{Button, Input, InputBase, InputState, Root};

    struct Harness {
        state: Entity<AccordionState>,
        input: Entity<InputState>,
        before: FocusHandle,
        after: FocusHandle,
        show_host: bool,
        unhandled_keys: Rc<RefCell<Vec<String>>>,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let unhandled_keys = self.unhandled_keys.clone();
            div()
                .w(px(400.))
                .flex()
                .flex_col()
                .on_key_down(move |event, _, _| {
                    unhandled_keys
                        .borrow_mut()
                        .push(event.keystroke.key.clone());
                })
                .child(
                    Button::new("before")
                        .debug_selector(|| "before".into())
                        .track_focus(&self.before)
                        .h(px(32.))
                        .child("Before"),
                )
                .when(self.show_host, |this| {
                    this.child(
                        Accordion::new("accordion", &self.state)
                            .flex()
                            .flex_col()
                            .children(self.state.read(cx).items().iter().enumerate().map(
                                |(index, item)| {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .child(
                                            AccordionHeader::new(&self.state, index)
                                                .debug_selector(move || {
                                                    format!("header-{index}").into()
                                                })
                                                .h(px(32.))
                                                .child(item.label().clone()),
                                        )
                                        .child(
                                            AccordionPanel::new(&self.state, index)
                                                .tab_stop(index == 2)
                                                .h(px(40.))
                                                .child(if index == 0 {
                                                    InputBase::new("panel-input")
                                                        .debug_selector(|| "panel-input".into())
                                                        .h(px(32.))
                                                        .child(Input::new(&self.input))
                                                        .into_any_element()
                                                } else {
                                                    div().child("Panel text").into_any_element()
                                                }),
                                        )
                                },
                            )),
                    )
                })
                .child(
                    Button::new("after")
                        .debug_selector(|| "after".into())
                        .track_focus(&self.after)
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<Option<SharedString>>>>;

    fn items() -> Vec<AccordionItem> {
        vec![
            AccordionItem::new("general", "General"),
            AccordionItem::new("disabled", "Unavailable").disabled(true),
            AccordionItem::new("help", "Help"),
            AccordionItem::new("advanced", "Advanced"),
        ]
    }

    fn harness(
        cx: &mut TestAppContext,
        items: Vec<AccordionItem>,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| AccordionState::new(items, cx));
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    input: cx.new(|cx| InputState::new(window, cx)),
                    before: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
                    show_host: true,
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
            cx.subscribe(&state, move |state, event: &AccordionEvent, cx| {
                let AccordionEvent::Change(value) = event;
                assert_eq!(
                    state.read(cx).open_item().map(|item| item.value().clone()),
                    *value,
                );
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

    fn click_header(cx: &mut VisualTestContext, index: usize) {
        let selector = ["header-0", "header-1", "header-2", "header-3"][index];
        let position = cx.debug_bounds(selector).unwrap().center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
    }

    fn assert_header(cx: &mut VisualTestContext, view: &Entity<Harness>, index: usize) {
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .state
                    .read(cx)
                    .trigger_focus_handle(index)
                    .is_focused(window)
            );
        });
    }

    fn assert_open(cx: &mut VisualTestContext, view: &Entity<Harness>, open: Option<usize>) {
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).open_index(), open));
    }

    #[gpui::test]
    fn every_enabled_header_is_a_tab_stop_and_arrow_focus_does_not_expand(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, items());
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_header(cx, &view, 0);
        for (key, index) in [
            ("up", 3),
            ("home", 0),
            ("down", 2),
            ("end", 3),
            ("down", 0),
            ("tab", 2),
            ("tab", 3),
        ] {
            cx.simulate_keystrokes(key);
            draw(cx);
            assert_header(cx, &view, index);
            assert_open(cx, &view, None);
        }
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab shift-tab shift-tab");
        draw(cx);
        assert_header(cx, &view, 0);
        cx.update(|_, cx| view.read(cx).unhandled_keys.borrow_mut().clear());
        cx.simulate_keystrokes("left right");
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(*view.read(cx).unhandled_keys.borrow(), ["left", "right"]);
        });
        assert!(changes.borrow().is_empty());
    }

    #[gpui::test]
    fn native_pointer_enter_and_space_toggle_once_and_ignore_disabled_items(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness(cx, items());
        let collapsed_after = cx.debug_bounds("after").unwrap().origin.y;
        let first_header = cx.debug_bounds("header-0").unwrap();
        assert_eq!(collapsed_after - first_header.origin.y, px(128.));
        click_header(cx, 0);
        assert_header(cx, &view, 0);
        assert_open(cx, &view, Some(0));
        assert_eq!(
            cx.debug_bounds("after").unwrap().origin.y - collapsed_after,
            px(48.)
        );
        press(cx, "enter");
        assert_open(cx, &view, None);
        assert_eq!(cx.debug_bounds("after").unwrap().origin.y, collapsed_after);
        press(cx, "space");
        assert_open(cx, &view, Some(0));
        click_header(cx, 1);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.toggle(1, window, cx));
        });
        assert_header(cx, &view, 0);
        assert_open(cx, &view, Some(0));
        click_header(cx, 2);
        assert_open(cx, &view, Some(2));
        press(cx, "space");
        assert_open(cx, &view, None);
        assert_eq!(
            *changes.borrow(),
            [Some("general"), None, Some("general"), Some("help"), None]
                .map(|value| value.map(SharedString::from))
        );
    }

    #[gpui::test]
    fn panel_data_survives_unmount_and_collapsing_focused_content_restores_header(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, _) = harness(cx, items());
        cx.simulate_keystrokes("tab");
        draw(cx);
        press(cx, "enter");
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).input.focus_handle(cx).is_focused(window));
        });
        cx.simulate_input("retained");
        draw(cx);
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
            let input = &view.read(cx).input;
            assert!(input.focus_handle(cx).is_focused(window));
            assert_eq!(input.read(cx).selected_range(), 8..8);
        });
        assert_open(cx, &view, Some(0));
        let restored_before_event = Rc::new(RefCell::new(Vec::new()));
        let subscription = cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                let restored_before_event = restored_before_event.clone();
                cx.subscribe_in(
                    &view.state,
                    window,
                    move |_, state, _: &AccordionEvent, window, cx| {
                        restored_before_event
                            .borrow_mut()
                            .push(state.read(cx).trigger_focus_handle(2).is_focused(window));
                    },
                )
            })
        });
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.toggle(2, window, cx));
        });
        draw(cx);
        assert_header(cx, &view, 2);
        assert!(cx.debug_bounds("panel-input").is_none());
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
            view.read(cx).unhandled_keys.borrow_mut().clear();
        });
        cx.simulate_keystrokes("up down home end");
        draw(cx);
        assert_open(cx, &view, Some(2));
        cx.update(|window, cx| {
            assert_eq!(
                *view.read(cx).unhandled_keys.borrow(),
                ["up", "down", "home", "end"]
            );
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.close(window, cx));
        });
        draw(cx);
        assert_header(cx, &view, 2);
        assert_eq!(*restored_before_event.borrow(), [true, true]);
        drop(subscription);
        cx.update(|window, cx| {
            let after = view.read(cx).after.clone();
            after.focus(window, cx);
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.toggle(0, window, cx));
            assert!(view.read(cx).after.is_focused(window));
            assert_eq!(view.read(cx).input.read(cx).value(), "retained");
        });
        draw(cx);
        assert!(cx.debug_bounds("panel-input").is_some());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.close(window, cx));
            assert!(view.read(cx).after.is_focused(window));
        });
    }

    #[gpui::test]
    fn closing_after_host_removal_does_not_focus_a_detached_header(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, items());
        click_header(cx, 2);
        cx.simulate_keystrokes("tab");
        draw(cx);
        view.update(cx, |view, cx| {
            view.show_host = false;
            cx.notify();
        });
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.close(window, cx);
                state.close(window, cx);
            });
            assert!(!state.read(cx).trigger_focus_handle(2).is_focused(window));
        });
        assert_eq!(*changes.borrow(), [Some(SharedString::from("help")), None]);
    }

    #[gpui::test]
    fn empty_disabled_and_single_enabled_collections_preserve_tab_navigation(
        cx: &mut TestAppContext,
    ) {
        for items in [
            Vec::new(),
            vec![AccordionItem::new("disabled", "Unavailable").disabled(true)],
        ] {
            let (cx, view, changes) = harness(cx, items);
            cx.simulate_keystrokes("tab");
            draw(cx);
            cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
            assert_open(cx, &view, None);
            assert!(changes.borrow().is_empty());
        }
        let (cx, view, changes) = harness(cx, vec![AccordionItem::new("only", "Only")]);
        cx.simulate_keystrokes("tab up down home end");
        draw(cx);
        assert_header(cx, &view, 0);
        assert_open(cx, &view, None);
        assert!(changes.borrow().is_empty());
    }

    #[gpui::test]
    fn heading_level_is_written_to_native_metadata(cx: &mut TestAppContext) {
        struct Probe {
            state: Entity<AccordionState>,
            nodes: Rc<RefCell<Vec<accesskit::Node>>>,
        }
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let state = self.state.clone();
                let nodes = self.nodes.clone();
                canvas(
                    move |_, window, cx| {
                        let mut captured = nodes.borrow_mut();
                        captured.clear();
                        let heading = AccordionHeader::new(&state, 0)
                            .heading_level(2)
                            .render(window, cx)
                            .into_element();
                        let mut node = accesskit::Node::new(heading.a11y_role().unwrap());
                        heading.write_a11y_info(&mut node);
                        captured.push(node);
                    },
                    |_, _, _, _| {},
                )
            }
        }
        let nodes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let nodes = nodes.clone();
            move |_, cx| Probe {
                state: cx
                    .new(|cx| AccordionState::new([AccordionItem::new("general", "General")], cx)),
                nodes,
            }
        });
        draw(cx);
        let nodes = nodes.borrow();
        assert_eq!(nodes[0].role(), Role::Heading);
        assert_eq!(nodes[0].level(), Some(2));
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn accessibility_reports_expanded_disabled_and_only_the_mounted_region(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, _) = harness(cx, items());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            for (index, item) in state.read(cx).items().iter().enumerate() {
                let heading = crate::test_support::find(
                    window,
                    &[],
                    &part_id(state.entity_id(), "accordion-heading", index),
                )
                .unwrap();
                assert_eq!(heading.role(), Some(Role::Heading));
                let button = crate::test_support::find(
                    window,
                    &[],
                    &part_id(state.entity_id(), "accordion-trigger", index),
                )
                .unwrap();
                assert_eq!(button.role(), Some(Role::Button));
                assert_eq!(button.label(), Some(item.label().as_ref()));
                assert_eq!(button.expanded(), Some(false));
                assert_eq!(button.disabled(), (index == 1).then_some(true));
            }
            assert!(
                crate::test_support::snapshots(window)
                    .iter()
                    .all(|node| node.role() != Some(Role::Region))
            );
            assert!(crate::test_support::find(window, &[], &"panel-input".into()).is_none());
        });
        click_header(cx, 0);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let button = crate::test_support::find(
                window,
                &[],
                &part_id(state.entity_id(), "accordion-trigger", 0),
            )
            .unwrap();
            assert_eq!(button.expanded(), Some(true));
            let regions: Vec<_> = crate::test_support::snapshots(window)
                .into_iter()
                .filter(|node| node.role() == Some(Role::Region))
                .collect();
            assert_eq!(regions.len(), 1);
            assert_eq!(regions[0].label(), Some("General"));
            assert!(crate::test_support::find(window, &[], &"panel-input".into()).is_some());
        });
        click_header(cx, 2);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert!(
                crate::test_support::find(
                    window,
                    &[],
                    &part_id(state.entity_id(), "accordion-panel", 0),
                )
                .is_none()
            );
            assert!(crate::test_support::find(window, &[], &"panel-input".into()).is_none());
            let regions: Vec<_> = crate::test_support::snapshots(window)
                .into_iter()
                .filter(|node| node.role() == Some(Role::Region))
                .collect();
            assert_eq!(regions.len(), 1);
            assert_eq!(regions[0].label(), Some("Help"));
        });
    }
}
