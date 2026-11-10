//! Shared commands, navigation, activation, and semantics for menu triggers.

use gpui::{
    Anchor, AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle,
    InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent, ParentElement, Pixels, Point,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Subscription, Toggled, WeakEntity, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    CheckboxState, Direction, Popover, PopoverState, StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{
        Cancel, Confirm, SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp,
    },
};

const CONTEXT: &str = "Menu";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
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
    submenu: Option<Entity<MenuState>>,
}

impl MenuItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            submenu: None,
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

    /// Makes this row open a child menu. A child belongs to exactly one parent;
    /// its leaf command values are emitted by the top-level [`MenuState`].
    /// Render this row with [`MenuSubmenu`] so its child content stays mounted.
    pub fn submenu(mut self, submenu: &Entity<MenuState>) -> Self {
        self.submenu = Some(submenu.clone());
        self
    }

    pub fn submenu_state(&self) -> Option<&Entity<MenuState>> {
        self.submenu.as_ref()
    }
}

/// Emitted once by the top-level menu after a leaf command closes the entire
/// menu chain and restores focus. Submenu trigger rows never emit commands.
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
/// Keep its matching [`crate::DropdownMenu`], [`crate::ContextMenu`],
/// [`crate::MenubarMenu`] or [`MenuSubmenu`] mounted while closed and host the app
/// in [`crate::Root`]. Empty and all-disabled collections may open, but cannot
/// emit commands. Menu options are not individual Tab stops.
pub struct MenuState {
    items: Vec<MenuItem>,
    highlighted: Option<usize>,
    pub(crate) popover: Entity<PopoverState>,
    position: Option<Point<Pixels>>,
    focus: FocusHandle,
    content_focus: FocusHandle,
    _popover_subscription: Subscription,
    _submenu_subscriptions: Vec<Subscription>,
    parent: Option<WeakEntity<MenuState>>,
    direction: Direction,
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
        let parent = cx.weak_entity();
        let submenu_subscriptions = items
            .iter()
            .filter_map(|item| item.submenu.as_ref())
            .map(|submenu| {
                submenu.update(cx, |state, _| {
                    assert!(
                        state
                            .parent
                            .as_ref()
                            .and_then(WeakEntity::upgrade)
                            .is_none(),
                        "A submenu can belong to only one MenuItem"
                    );
                    state.parent = Some(parent.clone());
                });
                cx.observe(submenu, |_, _, cx| cx.notify())
            })
            .collect();
        Self {
            items,
            highlighted: None,
            popover,
            position: None,
            focus,
            content_focus,
            _popover_subscription: subscription,
            _submenu_subscriptions: submenu_subscriptions,
            parent: None,
            direction: Direction::Ltr,
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
            self.close_descendants(None, window, cx);
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
        self.close_descendants(None, window, cx);
        self.close_local(window, cx);
    }

    fn close_local(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open(cx) {
            return;
        }
        self.highlighted = None;
        self.position = None;
        self.popover.update(cx, |state, cx| state.close(window, cx));
        cx.notify();
    }

    fn close_descendants(
        &self,
        keep: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut closed = false;
        for (index, item) in self.items.iter().enumerate() {
            if keep != Some(index)
                && let Some(submenu) = &item.submenu
                && submenu.read(cx).is_open(cx)
            {
                submenu.update(cx, |menu, cx| menu.close(window, cx));
                closed = true;
            }
        }
        closed
    }

    fn open_submenu(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.is_open(cx) || self.is_disabled(cx) {
            return false;
        }
        let Some(item) = self.items.get(index).filter(|item| !item.disabled) else {
            return false;
        };
        let Some(submenu) = item.submenu.clone() else {
            return false;
        };
        if submenu.read(cx).is_disabled(cx) {
            return false;
        }
        self.close_descendants(Some(index), window, cx);
        self.highlighted = Some(index);
        submenu.update(cx, |menu, cx| menu.open(window, cx));
        let focus = submenu.read(cx).content_focus_handle().clone();
        focus.focus(window, cx);
        cx.notify();
        window.refresh();
        true
    }

    fn close_to_parent(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(parent) = self.parent.as_ref().and_then(WeakEntity::upgrade) else {
            return false;
        };
        self.close(window, cx);
        let focus = parent.read(cx).content_focus.clone();
        focus.focus(window, cx);
        true
    }

    fn horizontal(&mut self, expand: bool, window: &mut Window, cx: &mut Context<Self>) {
        // Nested popups share the dispatch ancestry. Only the focused menu
        // handles a submenu key; an unhandled root key can reach Menubar.
        if !self.owns_focus(window, cx) {
            cx.propagate();
            return;
        }
        let handled = if expand {
            self.highlighted
                .is_some_and(|index| self.open_submenu(index, window, cx))
        } else {
            self.close_to_parent(window, cx)
        };
        if !handled {
            cx.propagate();
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.close_to_parent(window, cx) {
            self.close(window, cx);
        }
    }

    fn tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(parent) = self.parent.as_ref().and_then(WeakEntity::upgrade) {
            self.close_local(window, cx);
            parent.update(cx, |menu, cx| menu.tab(forward, window, cx));
        } else {
            self.popover
                .update(cx, |popover, cx| popover.tab(forward, window, cx));
        }
    }

    fn complete_action(
        &mut self,
        value: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The active descendant branch has already closed. Closing just this
        // level avoids updating the child currently reporting its command.
        self.close_local(window, cx);
        if let Some(parent) = self.parent.as_ref().and_then(WeakEntity::upgrade) {
            parent.update(cx, |menu, cx| menu.complete_action(value, window, cx));
        } else {
            cx.emit(MenuEvent::Action(value));
        }
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
        if self.close_descendants(Some(index), window, cx) {
            self.content_focus.focus(window, cx);
        }
        cx.notify();
        window.refresh();
    }

    fn typeahead(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.owns_focus(window, cx) {
            return;
        }
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
        if item.submenu.is_some() {
            self.open_submenu(index, window, cx);
            return;
        }
        // Subscribers may immediately focus another view or open a Dialog.
        // Close first so this menu cannot steal that new focus afterward.
        self.close_descendants(None, window, cx);
        self.complete_action(value, window, cx);
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.highlighted {
            self.activate(index, window, cx);
        }
    }

    fn owns_focus(&self, window: &Window, cx: &App) -> bool {
        self.content_focus.is_focused(window)
            || self
                .items
                .iter()
                .filter_map(|item| item.submenu.as_ref())
                .any(|submenu| submenu.read(cx).focus.is_focused(window))
    }
}

impl EventEmitter<MenuEvent> for MenuState {}

pub(crate) fn menu_content(
    menu: &Entity<MenuState>,
    label: Option<SharedString>,
    children: Option<AnyElement>,
    cx: &mut App,
) -> impl IntoElement + use<> {
    menu_content_with_direction(menu, label, children, Direction::current(cx), cx)
}

pub(crate) fn menu_content_with_direction(
    menu: &Entity<MenuState>,
    label: Option<SharedString>,
    children: Option<AnyElement>,
    direction: Direction,
    cx: &mut App,
) -> impl IntoElement + use<> {
    menu.update(cx, |state, _| state.direction = direction);
    let state = menu.read(cx);
    let focus = state.content_focus.clone();
    let up = menu.clone();
    let down = menu.clone();
    let first = menu.clone();
    let last = menu.clone();
    let confirm = menu.clone();
    let typeahead = menu.clone();
    let left = menu.clone();
    let right = menu.clone();
    let cancel = menu.clone();
    let tab = menu.clone();
    let previous = menu.clone();

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
        .on_action(move |_: &SelectLeft, window, cx| {
            left.update(cx, |state, cx| {
                state.horizontal(direction.is_rtl(), window, cx)
            });
        })
        .on_action(move |_: &SelectRight, window, cx| {
            right.update(cx, |state, cx| {
                state.horizontal(!direction.is_rtl(), window, cx)
            });
        })
        .on_action(move |_: &Cancel, window, cx| {
            cancel.update(cx, |state, cx| state.cancel(window, cx));
        })
        .on_action(move |_: &crate::root::Tab, window, cx| {
            tab.update(cx, |state, cx| state.tab(true, window, cx));
        })
        .on_action(move |_: &crate::root::TabPrev, window, cx| {
            previous.update(cx, |state, cx| state.tab(false, window, cx));
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
    checked: Option<Toggled>,
    role: Role,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MenuOption {
    pub fn new(state: &Entity<MenuState>, index: usize) -> Self {
        Self {
            state: state.clone(),
            index,
            checked: None,
            role: Role::MenuItem,
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
    pub fn checked(self, checked: bool) -> Self {
        self.check_state(if checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        })
    }

    /// Exposes a controlled checkbox value, including a mixed selection.
    pub fn check_state(mut self, state: CheckboxState) -> Self {
        self.role = Role::MenuItemCheckBox;
        self.checked = Some(match state {
            CheckboxState::Unchecked => Toggled::False,
            CheckboxState::Checked => Toggled::True,
            CheckboxState::Indeterminate => Toggled::Mixed,
        });
        self
    }

    /// Exposes a radio menu item. Render each peer from one application-owned
    /// value, for example `option.radio_checked(current == item.value())`, and
    /// update that value from [`MenuEvent::Action`]. Group related peers in a
    /// named `Role::Group` surface; this adds no independent focus or selection.
    pub fn radio_checked(mut self, checked: bool) -> Self {
        self.role = Role::MenuItemRadio;
        self.checked = Some(if checked {
            Toggled::True
        } else {
            Toggled::False
        });
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
        let submenu = item.submenu.clone();
        let submenu_focus = submenu
            .as_ref()
            .map(|submenu| submenu.read(cx).focus.clone().tab_stop(false));
        let expanded = submenu.as_ref().map(|submenu| submenu.read(cx).is_open(cx));
        let index = self.index;
        let hover = self.state.clone();
        let activate = self.state.clone();
        div()
            .id(("menu-option", index))
            .test_support()
            .role(if submenu.is_some() {
                Role::MenuItem
            } else {
                self.role
            })
            .aria_label(label)
            .when_some(
                self.checked.filter(|_| submenu.is_none()),
                |this, checked| this.aria_toggled(checked),
            )
            .when_some(submenu_focus, |this, focus| this.track_focus(&focus))
            .when_some(expanded, |this, expanded| this.aria_expanded(expanded))
            .when(highlighted, |this| this.aria_active_descendant())
            .when(!disabled, |this| {
                this.on_hover(move |hovered, window, cx| {
                    if *hovered {
                        hover.update(cx, |state, cx| {
                            state.highlight(index, window, cx);
                            state.open_submenu(index, window, cx);
                        });
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

/// One submenu row and its nested command surface.
///
/// The corresponding [`MenuItem`] must declare its child with
/// [`MenuItem::submenu`]. The trigger is a normal active-descendant menu option;
/// hovering or activating it opens the child. The expanding horizontal arrow
/// opens it, and the opposite arrow or Escape returns to the parent's cursor.
/// A leaf command closes the entire chain before emitting one root event.
#[derive(IntoElement)]
pub struct MenuSubmenu {
    parent: Entity<MenuState>,
    index: usize,
    trigger: MenuOption,
    content: Option<AnyElement>,
    anchor: Option<Anchor>,
    offset: Pixels,
    style: StyleRefinement,
}

impl MenuSubmenu {
    pub fn new(parent: &Entity<MenuState>, index: usize) -> Self {
        Self {
            parent: parent.clone(),
            index,
            trigger: MenuOption::new(parent, index),
            content: None,
            anchor: None,
            offset: px(0.),
            style: StyleRefinement::default(),
        }
    }

    pub fn trigger(mut self, build: impl FnOnce(MenuOption) -> MenuOption) -> Self {
        self.trigger = build(self.trigger);
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Overrides the default side anchor (right in LTR, left in RTL).
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    pub fn offset(mut self, offset: Pixels) -> Self {
        self.offset = offset;
        self
    }
}

impl Styled for MenuSubmenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MenuSubmenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = self.parent.read(cx);
        let item = parent
            .items
            .get(self.index)
            .expect("MenuSubmenu index is out of bounds");
        let submenu = item
            .submenu
            .as_ref()
            .expect("MenuSubmenu requires a MenuItem with a submenu")
            .clone();
        let label = item.label.clone();
        let direction = parent.direction;
        let popover = submenu.read(cx).popover.clone();
        let anchor = self.anchor.unwrap_or_else(|| {
            if direction.is_rtl() {
                Anchor::RightCenter
            } else {
                Anchor::LeftCenter
            }
        });
        Popover::new(("menu-submenu", self.index), &popover)
            .supplied_trigger(self.trigger)
            .content_role(None)
            .anchor(anchor)
            .offset(self.offset)
            .content(menu_content_with_direction(
                &submenu,
                Some(label),
                self.content,
                direction,
                cx,
            ))
            .refine_style(&self.style)
    }
}
