//! Retained command-menu behavior with application-owned presentation.
//! GPUI Design composes its Popover lifecycle and positioning with a menu
//! cursor, command activation, and active-descendant accessibility semantics.

use gpui::{
    Anchor, AnyElement, App, AppContext as _, Context, ElementId, Entity, EventEmitter,
    FocusHandle, InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    Button, Popover, PopoverState, StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{Confirm, SelectDown, SelectFirst, SelectLast, SelectUp},
};

const CONTEXT: &str = "DropdownMenu";
const TRIGGER_CONTEXT: &str = "DropdownMenuTrigger";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectDown, Some(TRIGGER_CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(TRIGGER_CONTEXT)),
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
pub struct DropdownMenuItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl DropdownMenuItem {
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
pub enum DropdownMenuEvent {
    Action(SharedString),
}

/// Retained commands and cursor for one non-modal menu in one window.
///
/// The nested [`PopoverState`] exclusively owns open, disabled, dismissal, and
/// focus-restoration behavior. The menu adds enabled-item navigation and command
/// events. It never stores a selected value: repeated activation of the same
/// command emits an event each time.
///
/// Keep the matching [`DropdownMenu`] mounted while closed and host the app in
/// [`crate::Root`]. Empty and all-disabled collections may open, but cannot emit
/// commands. Menu options are not individual Tab stops.
pub struct DropdownMenuState {
    items: Vec<DropdownMenuItem>,
    highlighted: Option<usize>,
    popover: Entity<PopoverState>,
    focus: FocusHandle,
    content_focus: FocusHandle,
    _popover_subscription: Subscription,
}

impl DropdownMenuState {
    pub fn new(items: impl IntoIterator<Item = DropdownMenuItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.value != item.value),
                "DropdownMenuItem values must be unique"
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
            }
            cx.notify();
        });
        Self {
            items,
            highlighted: None,
            popover,
            focus,
            content_focus,
            _popover_subscription: subscription,
        }
    }

    pub fn items(&self) -> &[DropdownMenuItem] {
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

    /// The native trigger's focus handle.
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
        self.popover.update(cx, |state, cx| state.close(window, cx));
        cx.notify();
    }

    fn open_at(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_disabled(cx) || self.is_open(cx) {
            return;
        }
        self.highlighted = self.edge(last);
        self.popover.update(cx, |state, cx| state.open(window, cx));
        cx.notify();
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
        cx.emit(DropdownMenuEvent::Action(value));
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.highlighted {
            self.activate(index, window, cx);
        }
    }
}

impl EventEmitter<DropdownMenuEvent> for DropdownMenuState {}

/// A native button trigger and anchored, unstyled command menu.
///
/// `trigger` supplies the Button's label and presentation; the menu owns focus,
/// disabled state, expansion, and activation. `accessibility_label` names the
/// menu itself. Supply [`DropdownMenuOption`] children in collection order and
/// keep interactive controls outside the menu's command content.
#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    state: Entity<DropdownMenuState>,
    trigger: Button,
    content: Option<AnyElement>,
    label: Option<SharedString>,
    anchor: Anchor,
    offset: Pixels,
    style: StyleRefinement,
}

impl DropdownMenu {
    pub fn new(id: impl Into<ElementId>, state: &Entity<DropdownMenuState>) -> Self {
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

    /// Names the command menu. Name its Button separately in `trigger`.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    pub fn offset(mut self, offset: Pixels) -> Self {
        self.offset = offset;
        self
    }
}

impl Styled for DropdownMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let popover = state.popover.clone();
        let focus = state.content_focus.clone();
        let trigger_down = self.state.clone();
        let trigger_up = self.state.clone();
        let up = self.state.clone();
        let down = self.state.clone();
        let first = self.state.clone();
        let last = self.state.clone();
        let confirm = self.state.clone();
        let typeahead = self.state.clone();

        let content = div()
            .id(("dropdown-menu", self.state.entity_id()))
            .test_support()
            .role(Role::Menu)
            .when_some(self.label, |this, label| this.aria_label(label))
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
            .children(self.content);

        Popover::new(self.id, &popover)
            .content_role(None)
            .anchor(self.anchor)
            .offset(self.offset)
            .trigger(|_| {
                self.trigger
                    .key_context(TRIGGER_CONTEXT)
                    .on_action(move |_: &SelectDown, window, cx| {
                        trigger_down.update(cx, |state, cx| state.open_at(false, window, cx));
                    })
                    .on_action(move |_: &SelectUp, window, cx| {
                        trigger_up.update(cx, |state, cx| state.open_at(true, window, cx));
                    })
            })
            .content(content)
            .refine_style(&self.style)
    }
}

/// One command's application-styled surface, with library-owned interaction.
///
/// Highlighted options expose the menu's active descendant without becoming
/// individual Tab stops. Read [`DropdownMenuState::highlighted_index`] to style
/// the keyboard/pointer cursor; disabled state comes from its fixed item.
#[derive(IntoElement)]
pub struct DropdownMenuOption {
    state: Entity<DropdownMenuState>,
    index: usize,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DropdownMenuOption {
    pub fn new(state: &Entity<DropdownMenuState>, index: usize) -> Self {
        Self {
            state: state.clone(),
            index,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for DropdownMenuOption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DropdownMenuOption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for DropdownMenuOption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let item = state
            .items
            .get(self.index)
            .expect("DropdownMenuOption index is out of bounds");
        let disabled = state.is_disabled(cx) || item.disabled;
        let highlighted = state.is_open(cx) && state.highlighted == Some(self.index);
        let label = item.label.clone();
        let index = self.index;
        let hover = self.state.clone();
        let activate = self.state.clone();
        div()
            .id(("dropdown-menu-option", index))
            .test_support()
            .role(Role::MenuItem)
            .aria_label(label)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dialog, DialogBackdrop, DialogPopup, DialogState, Root};
    use gpui::{
        KeyUpEvent, Keystroke, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    };

    struct Harness {
        menu: Entity<DropdownMenuState>,
        dialog: Entity<DialogState>,
        outer: Entity<PopoverState>,
        before: FocusHandle,
        after: FocusHandle,
        outside: FocusHandle,
        nested: bool,
        trailing: bool,
        commands: Vec<SharedString>,
        _subscriptions: Vec<Subscription>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let options = self
                .menu
                .read(cx)
                .items()
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    DropdownMenuOption::new(&self.menu, index).h(px(28.)).child(
                        div()
                            .debug_selector(move || format!("command-{index}").into())
                            .size_full()
                            .child(item.label().clone()),
                    )
                });
            let controls = div()
                .debug_selector(|| "outer-panel".into())
                .w(px(180.))
                .p(px(8.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    Button::new("before-menu")
                        .debug_selector(|| "before-menu".into())
                        .track_focus(&self.before)
                        .h(px(28.))
                        .child("Before"),
                )
                .child(
                    DropdownMenu::new("menu-trigger", &self.menu)
                        .accessibility_label("Commands")
                        .trigger(|button| {
                            button
                                .debug_selector(|| "menu-trigger".into())
                                .accessibility_label("Open commands")
                                .h(px(28.))
                                .child("Commands")
                        })
                        .content(
                            div()
                                .w(px(164.))
                                .p(px(4.))
                                .flex()
                                .flex_col()
                                .children(options),
                        ),
                )
                .when(self.trailing, |this| {
                    this.child(
                        Button::new("after-menu")
                            .track_focus(&self.after)
                            .h(px(28.))
                            .child("After"),
                    )
                });
            let controls = if self.nested {
                Popover::new("outer-trigger", &self.outer)
                    .accessibility_label("Parent popover")
                    .trigger(|button| button.h(px(28.)).child("Parent"))
                    .content(controls)
                    .into_any_element()
            } else {
                controls.into_any_element()
            };
            div()
                .size_full()
                .child(
                    Button::new("outside-dialog")
                        .track_focus(&self.outside)
                        .h(px(28.))
                        .child("Outside"),
                )
                .child(
                    Dialog::new(&self.dialog)
                        .accessibility_label("Parent dialog")
                        .backdrop(DialogBackdrop::new().size_full())
                        .popup(
                            DialogPopup::new()
                                .w(px(360.))
                                .h(px(380.))
                                .p(px(16.))
                                .child(controls),
                        ),
                )
        }
    }

    fn items() -> Vec<DropdownMenuItem> {
        vec![
            DropdownMenuItem::new("alpha", "Alpha").disabled(true),
            DropdownMenuItem::new("open", "Open"),
            DropdownMenuItem::new("save", "Save").disabled(true),
            DropdownMenuItem::new("share", "Share"),
            DropdownMenuItem::new("search", "Search"),
            DropdownMenuItem::new("quit", "Quit"),
            DropdownMenuItem::new("zip", "Zip").disabled(true),
        ]
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
        items: Vec<DropdownMenuItem>,
    ) -> (&mut VisualTestContext, Entity<Harness>) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                let before = cx.focus_handle();
                let menu = cx.new(|cx| DropdownMenuState::new(items, cx));
                let dialog = cx.new(|cx| DialogState::new(cx).with_initial_focus(&before));
                let outer = cx.new(PopoverState::new);
                let subscriptions = vec![
                    cx.observe(&menu, |_, _, cx| cx.notify()),
                    cx.observe(&dialog, |_, _, cx| cx.notify()),
                    cx.observe(&outer, |_, _, cx| cx.notify()),
                    cx.subscribe_in(
                        &menu,
                        window,
                        |this: &mut Harness, menu, event, window, cx| {
                            let DropdownMenuEvent::Action(value) = event;
                            assert!(!menu.read(cx).is_open(cx));
                            assert_eq!(menu.read(cx).highlighted_index(), None);
                            assert!(menu.read(cx).focus_handle().is_focused(window));
                            this.commands.push(value.clone());
                            cx.notify();
                        },
                    ),
                ];
                Harness {
                    menu,
                    dialog,
                    outer,
                    before,
                    after: cx.focus_handle(),
                    outside: cx.focus_handle(),
                    nested: false,
                    trailing: true,
                    commands: Vec::new(),
                    _subscriptions: subscriptions,
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
            window.draw(cx).clear(cx);
            let outside = view.read(cx).outside.clone();
            outside.focus(window, cx);
            let dialog = view.read(cx).dialog.clone();
            dialog.update(cx, |state, cx| state.open(window, cx));
        });
        settle(cx);
        (cx, view)
    }

    fn open(cx: &mut VisualTestContext, view: &Entity<Harness>, key: &str) {
        cx.update(|window, cx| {
            let focus = view.read(cx).menu.read(cx).focus_handle().clone();
            focus.focus(window, cx);
        });
        draw(cx);
        press(cx, key);
        cx.update(|window, cx| {
            let state = view.read(cx).menu.read(cx);
            assert!(state.is_open(cx));
            assert!(state.content_focus_handle().is_focused(window));
        });
    }

    fn assert_highlight(cx: &mut VisualTestContext, view: &Entity<Harness>, index: usize) {
        cx.update(|window, cx| {
            let state = view.read(cx).menu.read(cx);
            assert_eq!(state.highlighted_index(), Some(index));
            assert!(state.content_focus_handle().is_focused(window));
        });
    }

    #[gpui::test]
    fn native_trigger_and_repeated_command_activation_close_before_events(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx, items());
        open(cx, &view, "enter");
        assert_highlight(cx, &view, 1);
        press(cx, "enter");
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert_eq!(view.commands, vec![SharedString::from("open")]);
            assert!(!view.menu.read(cx).is_open(cx));
        });
        open(cx, &view, "space");
        assert_highlight(cx, &view, 1);
        press(cx, "space");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert_eq!(view.commands, vec![SharedString::from("open"); 2]);
            assert!(!view.menu.read(cx).is_open(cx));
            assert!(view.menu.read(cx).focus_handle().is_focused(window));
            assert!(view.dialog.read(cx).is_open());
        });
    }

    #[gpui::test]
    fn arrows_edges_and_single_character_typeahead_skip_disabled_commands(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx, items());
        open(cx, &view, "up");
        assert_highlight(cx, &view, 5);
        press(cx, "down");
        assert_highlight(cx, &view, 1);
        press(cx, "down");
        assert_highlight(cx, &view, 3);
        press(cx, "home");
        assert_highlight(cx, &view, 1);
        press(cx, "up");
        assert_highlight(cx, &view, 5);
        press(cx, "home");
        press(cx, "s");
        assert_highlight(cx, &view, 3);
        press(cx, "s");
        assert_highlight(cx, &view, 4);
        press(cx, "S");
        assert_highlight(cx, &view, 3);
        press(cx, "ctrl-s");
        press(cx, "alt-s");
        press(cx, "x");
        assert_highlight(cx, &view, 3);
        // The typed character takes precedence over a keyboard layout's key.
        press(cx, "t->q");
        assert_highlight(cx, &view, 5);
        press(cx, "home");
        press(cx, "end");
        assert_highlight(cx, &view, 5);
        cx.update(|_, cx| assert!(view.read(cx).commands.is_empty()));
    }

    #[gpui::test]
    fn pointer_hover_and_activation_share_the_cursor_and_disabled_contract(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx, items());
        open(cx, &view, "down");
        let disabled = cx.debug_bounds("command-2").unwrap().center();
        cx.simulate_click(disabled, Modifiers::default());
        settle(cx);
        assert_highlight(cx, &view, 1);
        cx.update(|_, cx| assert!(view.read(cx).commands.is_empty()));

        let target = cx.debug_bounds("command-4").unwrap().center();
        cx.simulate_mouse_move(target, None, Modifiers::default());
        settle(cx);
        assert_highlight(cx, &view, 4);
        cx.simulate_click(target, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert_eq!(view.commands, vec![SharedString::from("search")]);
            assert!(!view.menu.read(cx).is_open(cx));
        });

        let trigger = cx.debug_bounds("menu-trigger").unwrap().center();
        cx.simulate_click(trigger, Modifiers::default());
        settle(cx);
        assert_highlight(cx, &view, 1);
        cx.simulate_mouse_down(trigger, MouseButton::Left, Modifiers::default());
        settle(cx);
        cx.simulate_mouse_up(trigger, MouseButton::Left, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).menu.read(cx).is_open(cx)));
    }

    #[gpui::test]
    fn outside_dismissal_and_escape_clear_cursor_before_reopening(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx, items());
        open(cx, &view, "up");
        let before = cx.debug_bounds("before-menu").unwrap().center();
        cx.simulate_click(before, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.menu.read(cx).is_open(cx));
            assert_eq!(view.menu.read(cx).highlighted_index(), None);
            assert!(view.before.is_focused(window));
            assert!(view.commands.is_empty());
        });
        open(cx, &view, "enter");
        assert_highlight(cx, &view, 1);
        press(cx, "end");
        press(cx, "escape");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.menu.read(cx).is_open(cx));
            assert_eq!(view.menu.read(cx).highlighted_index(), None);
            assert!(view.menu.read(cx).focus_handle().is_focused(window));
            assert!(view.dialog.read(cx).is_open());
        });
        open(cx, &view, "space");
        assert_highlight(cx, &view, 1);
    }

    #[gpui::test]
    fn tab_exits_commands_and_respects_the_parent_modal(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx, items());
        open(cx, &view, "down");
        press(cx, "tab");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.menu.read(cx).is_open(cx));
            assert!(view.after.is_focused(window));
        });
        open(cx, &view, "down");
        press(cx, "shift-tab");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.menu.read(cx).is_open(cx));
            assert!(view.menu.read(cx).focus_handle().is_focused(window));
        });
        view.update(cx, |view, cx| {
            view.trailing = false;
            cx.notify();
        });
        settle(cx);
        open(cx, &view, "down");
        press(cx, "tab");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.menu.read(cx).is_open(cx));
            assert!(view.before.is_focused(window));
            assert!(
                view.dialog
                    .read(cx)
                    .focus_handle()
                    .contains_focused(window, cx)
            );
            assert!(view.commands.is_empty());
        });
    }

    #[gpui::test]
    fn nested_menu_pointer_and_escape_preserve_the_parent_popover_and_dialog(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx, items());
        view.update(cx, |view, cx| {
            view.nested = true;
            cx.notify();
        });
        settle(cx);
        cx.update(|window, cx| {
            let outer = view.read(cx).outer.clone();
            outer.update(cx, |state, cx| state.open(window, cx));
        });
        settle(cx);
        open(cx, &view, "down");
        let target = cx.debug_bounds("command-5").unwrap().center();
        assert!(!cx.debug_bounds("outer-panel").unwrap().contains(&target));
        cx.simulate_click(target, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert_eq!(view.commands, vec![SharedString::from("quit")]);
            assert!(!view.menu.read(cx).is_open(cx));
            assert!(view.outer.read(cx).is_open());
            assert!(view.dialog.read(cx).is_open());
        });
        open(cx, &view, "down");
        press(cx, "escape");
        cx.update(|_, cx| {
            assert!(!view.read(cx).menu.read(cx).is_open(cx));
            assert!(view.read(cx).outer.read(cx).is_open());
        });
        press(cx, "escape");
        cx.update(|_, cx| {
            assert!(!view.read(cx).outer.read(cx).is_open());
            assert!(view.read(cx).dialog.read(cx).is_open());
        });
        press(cx, "escape");
        cx.update(|window, cx| {
            assert!(!view.read(cx).dialog.read(cx).is_open());
            assert!(view.read(cx).outside.is_focused(window));
        });
    }

    #[gpui::test]
    fn disabling_an_open_menu_blocks_activation_but_preserves_tab_exit(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx, items());
        open(cx, &view, "down");
        cx.update(|window, cx| {
            let menu = view.read(cx).menu.clone();
            menu.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        settle(cx);
        press(cx, "enter");
        press(cx, "space");
        press(cx, "down");
        press(cx, "up");
        let trigger = cx.debug_bounds("menu-trigger").unwrap().center();
        cx.simulate_click(trigger, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let menu = view.read(cx).menu.clone();
            menu.update(cx, |state, cx| state.open(window, cx));
            assert!(!menu.read(cx).is_open(cx));
            assert!(menu.read(cx).is_disabled(cx));
            assert_eq!(menu.read(cx).highlighted_index(), None);
            assert!(menu.read(cx).focus_handle().is_focused(window));
            assert!(view.read(cx).commands.is_empty());
        });
        press(cx, "tab");
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
    }

    #[gpui::test]
    fn empty_disabled_and_single_command_collections_have_defined_navigation(
        cx: &mut TestAppContext,
    ) {
        for commands in [
            Vec::new(),
            vec![DropdownMenuItem::new("disabled", "Disabled").disabled(true)],
        ] {
            let (cx, view) = harness(cx, commands);
            open(cx, &view, "enter");
            for key in ["down", "up", "home", "end", "d", "enter", "space"] {
                press(cx, key);
            }
            cx.update(|_, cx| {
                assert!(view.read(cx).menu.read(cx).is_open(cx));
                assert_eq!(view.read(cx).menu.read(cx).highlighted_index(), None);
                assert!(view.read(cx).commands.is_empty());
            });
            press(cx, "tab");
            cx.update(|window, cx| {
                assert!(!view.read(cx).menu.read(cx).is_open(cx));
                assert!(view.read(cx).after.is_focused(window));
            });
        }

        let (cx, view) = harness(cx, vec![DropdownMenuItem::new("only", "Only")]);
        open(cx, &view, "up");
        for key in ["down", "up", "home", "end", "o"] {
            press(cx, key);
            assert_highlight(cx, &view, 0);
        }
        press(cx, "enter");
        cx.update(|_, cx| assert_eq!(view.read(cx).commands, vec![SharedString::from("only")]));
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn accessibility_exposes_expanded_button_and_menu_commands_without_selection(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx, items());
        cx.update(|window, _| {
            let trigger = crate::test_support::find(window, &[], &"menu-trigger".into()).unwrap();
            assert_eq!(trigger.role(), Some(Role::Button));
            assert_eq!(trigger.label(), Some("Open commands"));
            assert_eq!(trigger.expanded(), Some(false));
        });
        open(cx, &view, "down");
        cx.update(|window, cx| {
            let menu = view.read(cx).menu.clone();
            let trigger = crate::test_support::find(window, &[], &"menu-trigger".into()).unwrap();
            assert_eq!(trigger.expanded(), Some(true));
            let scope =
                crate::test_support::find(window, &[], &("dropdown-menu", menu.entity_id()).into())
                    .unwrap();
            assert_eq!(scope.role(), Some(Role::Menu));
            assert_eq!(scope.label(), Some("Commands"));
            assert_eq!(scope.focused(), Some(true));
            for (index, item) in menu.read(cx).items().iter().enumerate() {
                let option =
                    crate::test_support::find(window, &[], &("dropdown-menu-option", index).into())
                        .unwrap();
                assert_eq!(option.role(), Some(Role::MenuItem));
                assert_eq!(option.label(), Some(item.label().as_ref()));
                assert_eq!(option.selected(), None);
                assert_eq!(option.disabled(), item.is_disabled().then_some(true));
            }
            let dialogs = crate::test_support::snapshots(window)
                .into_iter()
                .filter(|node| node.role() == Some(Role::Dialog))
                .count();
            assert_eq!(
                dialogs, 1,
                "The menu wrapper must not advertise another Dialog"
            );
        });
    }
}
