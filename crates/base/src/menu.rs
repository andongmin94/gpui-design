//! Shared commands, navigation, activation, and semantics for menu triggers.

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle,
    InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent, ParentElement, Pixels, Point,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Subscription, Toggled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    PopoverState, StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{Confirm, SelectDown, SelectFirst, SelectLast, SelectUp},
};

const CONTEXT: &str = "Menu";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
    ]);
}

/// One command in a menu's fixed, ordered collection. Values must be unique.
#[derive(Clone, Debug)]
pub struct MenuItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl MenuItem {
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

/// Emitted once per command activation, after closing and restoring focus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuEvent {
    Action(SharedString),
}

/// Retained commands and cursor for one non-modal menu in one window.
///
/// The nested [`PopoverState`] exclusively owns open, disabled, dismissal, and
/// focus-restoration behavior. The menu adds enabled-item navigation and command
/// events. It never stores a selected value: repeated activation of the same
/// command emits an event each time.
///
/// Keep the matching [`crate::DropdownMenu`] or [`crate::ContextMenu`] mounted while closed and host the app in
/// [`crate::Root`]. Empty and all-disabled collections may open, but cannot emit
/// commands. Menu options are not individual Tab stops.
pub struct MenuState {
    items: Vec<MenuItem>,
    highlighted: Option<usize>,
    pub(crate) popover: Entity<PopoverState>,
    position: Option<Point<Pixels>>,
    focus: FocusHandle,
    content_focus: FocusHandle,
    _popover_subscription: Subscription,
}

impl MenuState {
    pub fn new(items: impl IntoIterator<Item = MenuItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.value != item.value),
                "MenuItem values must be unique"
            );
        }
        let content_focus = cx.focus_handle();
        let popover = cx.new(|cx| PopoverState::new(cx).with_initial_focus(&content_focus));
        let focus = popover.read(cx).focus_handle().clone();
        let subscription = cx.observe(&popover, |state, popover, cx| {
            if popover.read(cx).is_open() {
                // Native Button activation opens Popover directly. Preserve a
                // cursor already chosen by ArrowUp/Down on the trigger.
                if state.highlighted.is_none() {
                    state.highlighted = state.items.iter().position(|item| !item.disabled);
                }
            } else {
                state.highlighted = None;
                state.position = None;
            }
            cx.notify();
        });
        Self {
            items,
            highlighted: None,
            popover,
            position: None,
            focus,
            content_focus,
            _popover_subscription: subscription,
        }
    }

    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    pub fn highlighted_index(&self) -> Option<usize> {
        self.highlighted
    }

    pub fn is_open(&self, cx: &App) -> bool {
        self.popover.read(cx).is_open()
    }

    pub fn is_disabled(&self, cx: &App) -> bool {
        self.popover.read(cx).is_disabled()
    }

    /// The button or context target's focus handle.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// The menu's focus handle; keyboard navigation moves its active descendant.
    pub fn content_focus_handle(&self) -> &FocusHandle {
        &self.content_focus
    }

    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_disabled(cx) == disabled {
            return;
        }
        if disabled {
            self.highlighted = None;
        }
        self.popover
            .update(cx, |state, cx| state.set_disabled(disabled, window, cx));
        cx.notify();
    }

    /// Opens with the first enabled command highlighted.
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_at(false, window, cx);
    }

    /// Closes and restores the trigger while focus still belongs to the menu.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open(cx) {
            return;
        }
        self.highlighted = None;
        self.position = None;
        self.popover.update(cx, |state, cx| state.close(window, cx));
        cx.notify();
    }

    pub(crate) fn open_at(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_disabled(cx) || self.is_open(cx) {
            return;
        }
        self.position = None;
        self.highlighted = self.edge(last);
        self.popover.update(cx, |state, cx| state.open(window, cx));
        cx.notify();
    }

    pub(crate) fn position(&self) -> Option<Point<Pixels>> {
        self.position
    }

    // A context request can move an already open menu without toggling it off.
    pub(crate) fn open_at_position(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_disabled(cx) {
            return;
        }
        self.position = Some(position);
        self.highlighted = self.edge(false);
        self.popover.update(cx, |state, cx| state.open(window, cx));
        cx.notify();
        window.refresh();
    }

    fn edge(&self, last: bool) -> Option<usize> {
        if last {
            self.items.iter().rposition(|item| !item.disabled)
        } else {
            self.items.iter().position(|item| !item.disabled)
        }
    }

    fn move_cursor(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_disabled(cx) || !self.is_open(cx) || self.items.is_empty() {
            return;
        }
        let count = self.items.len();
        let start = self
            .highlighted
            .unwrap_or(if forward { count - 1 } else { 0 });
        for distance in 1..=count {
            let index = if forward {
                (start + distance) % count
            } else {
                (start + count - distance) % count
            };
            if !self.items[index].disabled {
                self.highlight(index, window, cx);
                break;
            }
        }
    }

    fn move_to_edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.edge(last) {
            self.highlight(index, window, cx);
        }
    }

    fn highlight(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open(cx)
            || self.is_disabled(cx)
            || self.items.get(index).is_none_or(|item| item.disabled)
            || self.highlighted == Some(index)
        {
            return;
        }
        self.highlighted = Some(index);
        cx.notify();
        window.refresh();
    }

    fn typeahead(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
            return;
        }
        let text = event
            .keystroke
            .key_char
            .as_deref()
            .unwrap_or(&event.keystroke.key);
        let mut characters = text.chars();
        let Some(character) = characters.next() else {
            return;
        };
        if characters.next().is_some() || character.is_control() || character.is_whitespace() {
            return;
        }
        if !self.is_open(cx) || self.is_disabled(cx) || self.items.is_empty() {
            return;
        }
        let prefix = character.to_lowercase().collect::<String>();
        let count = self.items.len();
        let start = self.highlighted.unwrap_or(count - 1);
        for distance in 1..=count {
            let index = (start + distance) % count;
            let item = &self.items[index];
            if !item.disabled && item.label.to_lowercase().starts_with(&prefix) {
                self.highlight(index, window, cx);
                cx.stop_propagation();
                break;
            }
        }
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open(cx) || self.is_disabled(cx) {
            return;
        }
        let Some(item) = self.items.get(index).filter(|item| !item.disabled) else {
            return;
        };
        let value = item.value.clone();
        // Subscribers may immediately focus another view or open a Dialog.
        // Close first so this menu cannot steal that new focus afterward.
        self.close(window, cx);
        cx.emit(MenuEvent::Action(value));
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.highlighted {
            self.activate(index, window, cx);
        }
    }
}

impl EventEmitter<MenuEvent> for MenuState {}

pub(crate) fn menu_content(
    menu: &Entity<MenuState>,
    label: Option<SharedString>,
    children: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement + use<> {
    let state = menu.read(cx);
    let focus = state.content_focus.clone();
    let up = menu.clone();
    let down = menu.clone();
    let first = menu.clone();
    let last = menu.clone();
    let confirm = menu.clone();
    let typeahead = menu.clone();

    div()
        .id(("menu", menu.entity_id()))
        .test_support()
        .role(Role::Menu)
        .when_some(label, |this, label| this.aria_label(label))
        .track_focus(&focus)
        .key_context(CONTEXT)
        .on_action(move |_: &SelectUp, window, cx| {
            up.update(cx, |state, cx| state.move_cursor(false, window, cx));
        })
        .on_action(move |_: &SelectDown, window, cx| {
            down.update(cx, |state, cx| state.move_cursor(true, window, cx));
        })
        .on_action(move |_: &SelectFirst, window, cx| {
            first.update(cx, |state, cx| state.move_to_edge(false, window, cx));
        })
        .on_action(move |_: &SelectLast, window, cx| {
            last.update(cx, |state, cx| state.move_to_edge(true, window, cx));
        })
        .on_action(move |_: &Confirm, window, cx| {
            confirm.update(cx, |state, cx| state.confirm(window, cx));
        })
        .on_key_down(move |event, window, cx| {
            typeahead.update(cx, |state, cx| state.typeahead(event, window, cx));
        })
        .children(children)
}

/// One command's application-styled surface, with library-owned interaction.
///
/// Highlighted options expose the menu's active descendant without becoming
/// individual Tab stops. Read [`MenuState::highlighted_index`] to style
/// the keyboard/pointer cursor; disabled state comes from its fixed item.
#[derive(IntoElement)]
pub struct MenuOption {
    state: Entity<MenuState>,
    index: usize,
    checked: Option<bool>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MenuOption {
    pub fn new(state: &Entity<MenuState>, index: usize) -> Self {
        Self {
            state: state.clone(),
            index,
            checked: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Presents this command as a checkbox menu item with an application-owned value.
    ///
    /// Without this method the option is an ordinary menu item with no checked
    /// metadata. The value is rendered only; activation still closes the menu,
    /// restores focus, and emits [`MenuEvent::Action`]. Update application state
    /// in that handler and render the new value back here. This adds no item
    /// focus stop and does not change the menu's highlighted cursor.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
}

impl Styled for MenuOption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for MenuOption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MenuOption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let item = state
            .items
            .get(self.index)
            .expect("MenuOption index is out of bounds");
        let disabled = state.is_disabled(cx) || item.disabled;
        let highlighted = state.is_open(cx) && state.highlighted == Some(self.index);
        let label = item.label.clone();
        let index = self.index;
        let hover = self.state.clone();
        let activate = self.state.clone();
        div()
            .id(("menu-option", index))
            .test_support()
            .role(if self.checked.is_some() {
                Role::MenuItemCheckBox
            } else {
                Role::MenuItem
            })
            .aria_label(label)
            .when_some(self.checked, |this, checked| {
                this.aria_toggled(if checked {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .when(highlighted, |this| this.aria_active_descendant())
            .when(!disabled, |this| {
                this.on_hover(move |hovered, window, cx| {
                    if *hovered {
                        hover.update(cx, |state, cx| state.highlight(index, window, cx));
                    }
                })
                .on_click(move |_, window, cx| {
                    activate.update(cx, |state, cx| state.activate(index, window, cx));
                })
            })
            .children(self.children)
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}
