//! Searchable actions composed from the existing text input and GPUI scrolling.
//! GPUI Design owns matching and the highlighted action; applications own the
//! surrounding surface, item presentation, and the action's effects.

use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, EntityId,
    EntityInputHandler as _, EventEmitter, FocusHandle, Focusable as _, InteractiveElement,
    Interactivity, IntoElement, MouseButton, ParentElement, RenderOnce, Role, ScrollHandle,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{
    Input, InputAccessibility, InputBase, InputState, InputStyles, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    input::{InputEvent, MoveDown, MoveUp},
};

/// One action in a fixed, ordered command collection. Values must be unique.
#[derive(Clone, Debug)]
pub struct CommandItem {
    value: SharedString,
    label: SharedString,
    search_text: String,
    disabled: bool,
}

impl CommandItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let value = value.into();
        let label = label.into();
        Self {
            search_text: format!("{value} {label}").to_lowercase(),
            value,
            label,
            disabled: false,
        }
    }

    /// Adds searchable aliases without changing the visible or accessible label.
    pub fn keywords<S: AsRef<str>>(mut self, keywords: impl IntoIterator<Item = S>) -> Self {
        for keyword in keywords {
            self.search_text.push(' ');
            self.search_text.push_str(&keyword.as_ref().to_lowercase());
        }
        self
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

/// An explicit command activation. The application decides what it does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandEvent {
    Action(SharedString),
}

/// A query input and its fixed collection of searchable actions in one window.
///
/// InputState is the query's source of truth. Each whitespace-separated query
/// term must be a case-insensitive substring of the value, label, or keywords.
/// Results retain item order; matching does not rank or fuzzy-match them.
/// Query changes highlight the first enabled result. Arrows move that highlight
/// without committing a value, and Enter or a pointer click emits an action.
///
/// Keep this entity in the application and observe it to render its results.
/// The command has no open/close lifecycle; compose it inside Dialog when needed.
pub struct CommandState {
    items: Vec<CommandItem>,
    input: Entity<InputState>,
    focus: FocusHandle,
    // Only the last text processed by the derived filter, never a second value.
    filtered_query: SharedString,
    visible: Vec<usize>,
    highlighted: Option<usize>,
    scroll: ScrollHandle,
    _input_observation: Subscription,
    _input_events: Subscription,
}

impl CommandState {
    pub fn new(
        items: impl IntoIterator<Item = CommandItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.value != item.value),
                "CommandItem values must be unique"
            );
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search commands..."));
        let focus = input.focus_handle(cx);
        let input_observation = cx.observe(&input, |state, _, cx| {
            state.sync_query(cx);
            // Focus and disabled changes also affect the application-owned frame.
            cx.notify();
        });
        let input_events = cx.subscribe_in(&input, window, |state, _, event, window, cx| {
            if matches!(
                event,
                InputEvent::PressEnter {
                    secondary: false,
                    shift: false
                }
            ) && state.focus.is_focused(window)
            {
                state.sync_query(cx);
                if let Some(index) = state.highlighted {
                    state.activate(index, window, cx);
                }
            }
        });
        Self {
            visible: (0..items.len()).collect(),
            highlighted: items.iter().position(|item| !item.disabled),
            items,
            input,
            focus,
            filtered_query: SharedString::default(),
            scroll: ScrollHandle::new(),
            _input_observation: input_observation,
            _input_events: input_events,
        }
    }

    /// The native query input, also available for application text styling.
    pub fn input(&self) -> &Entity<InputState> {
        &self.input
    }

    /// The only ordinary Tab stop supplied by this component.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn items(&self) -> &[CommandItem] {
        &self.items
    }

    /// Original item indices in display order, including matching disabled items.
    pub fn visible_indices(&self) -> &[usize] {
        &self.visible
    }

    pub fn highlighted_index(&self) -> Option<usize> {
        self.highlighted
    }

    /// Changes the native input and immediately refreshes its derived results.
    /// Like InputState::set_value, this resets the query's editing history.
    pub fn set_query(
        &mut self,
        query: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let query: SharedString = query.into();
        self.input.update(cx, |input, cx| {
            input.set_value(query.to_string(), window, cx)
        });
        self.sync_query(cx);
        cx.notify();
    }

    fn sync_query(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value();
        if query == self.filtered_query {
            return;
        }
        let terms: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
        self.visible = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                terms
                    .iter()
                    .all(|term| item.search_text.contains(term.as_str()))
            })
            .map(|(index, _)| index)
            .collect();
        self.highlighted = self
            .visible
            .iter()
            .copied()
            .find(|&index| !self.items[index].disabled);
        self.filtered_query = query;
        self.scroll.set_offset(Default::default());
        self.scroll_to_highlight();
    }

    fn is_composing(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.input.update(cx, |input, cx| {
            input.marked_text_range(window, cx).is_some()
        })
    }

    fn scroll_to_highlight(&self) {
        if let Some(position) = self
            .highlighted
            .and_then(|index| self.visible.binary_search(&index).ok())
        {
            self.scroll.scroll_to_item(position);
        }
    }

    fn move_cursor(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window)
            || self.input.read(cx).presentation().is_disabled()
            || self.is_composing(window, cx)
        {
            cx.propagate();
            return;
        }
        self.sync_query(cx);
        let count = self.visible.len();
        if count == 0 {
            return;
        }
        let start = self
            .highlighted
            .and_then(|index| self.visible.binary_search(&index).ok())
            .unwrap_or(if forward { count - 1 } else { 0 });
        for distance in 1..=count {
            let position = if forward {
                (start + distance) % count
            } else {
                (start + count - distance) % count
            };
            let index = self.visible[position];
            if !self.items[index].disabled {
                self.highlight(index, window, cx);
                break;
            }
        }
    }

    fn highlight(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.highlighted == Some(index)
            || self.visible.binary_search(&index).is_err()
            || self.items[index].disabled
            || self.input.read(cx).presentation().is_disabled()
            || self.is_composing(window, cx)
        {
            return;
        }
        self.highlighted = Some(index);
        self.scroll_to_highlight();
        cx.notify();
        window.refresh();
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.visible.binary_search(&index).is_err()
            || self.items[index].disabled
            || self.input.read(cx).presentation().is_disabled()
            || self.is_composing(window, cx)
        {
            return;
        }
        self.highlight(index, window, cx);
        self.focus.focus(window, cx);
        cx.emit(CommandEvent::Action(self.items[index].value.clone()));
    }
}

impl EventEmitter<CommandEvent> for CommandState {}

/// An unstyled command root containing a query input and a result list.
///
/// Only Input's unmodified Up/Down actions move the command cursor. Home/End,
/// Space, selection, clipboard, and IME input remain owned by the text engine.
#[derive(IntoElement)]
pub struct Command {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CommandState>,
    label: Option<SharedString>,
}

impl Command {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CommandState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Styled for Command {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Command {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for Command {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Command {}

impl RenderOnce for Command {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let up = self.state.clone();
        let down = self.state;
        self.base
            .role(Role::Group)
            .when_some(self.label, |this, label| this.aria_label(label))
            .on_action(move |_: &MoveUp, window, cx| {
                up.update(cx, |state, cx| state.move_cursor(false, window, cx));
            })
            .on_action(move |_: &MoveDown, window, cx| {
                down.update(cx, |state, cx| state.move_cursor(true, window, cx));
            })
    }
}

/// The query's application-styled InputBase frame and existing native Input.
#[derive(IntoElement)]
pub struct CommandInput {
    base: InputBase,
    state: Entity<CommandState>,
    label: SharedString,
    placeholder: SharedString,
}

impl CommandInput {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CommandState>) -> Self {
        Self {
            base: InputBase::new(id).role(None::<Role>),
            state: state.clone(),
            label: "Search commands".into(),
            placeholder: "Search commands...".into(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn styles(mut self, build: impl FnOnce(InputStyles) -> InputStyles) -> Self {
        self.base = self.base.styles(build);
        self
    }
}

impl Styled for CommandInput {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for CommandInput {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for CommandInput {}

impl RenderOnce for CommandInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.state.read(cx).input.clone();
        input.update(cx, |input, cx| {
            if input.presentation().placeholder() != &self.placeholder {
                input.set_placeholder(self.placeholder.clone(), window, cx);
            }
            input.set_accessibility(
                InputAccessibility::new(Some(Role::TextInput))
                    .with_label(Some(self.label))
                    .with_placeholder(Some(self.placeholder)),
            );
        });
        let presentation = input.read(cx).presentation();
        self.base
            .semantic_owner::<Input>(input.entity_id())
            .focused(presentation.focus_handle().is_focused(window))
            .disabled(presentation.is_disabled())
            .child(Input::new(&input))
    }
}

/// A named, vertically scrolling result list with no independent Tab stop.
///
/// Render CommandOption children for `visible_indices()` in that exact order.
/// Keep extra controls, group headings, and empty-result text outside this list.
/// Applications supply its height limit, spacing, and item presentation.
#[derive(IntoElement)]
pub struct CommandList {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CommandState>,
    label: Option<SharedString>,
}

impl CommandList {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CommandState>) -> Self {
        Self {
            base: div().id(id).test_support().flex().flex_col().min_h_0(),
            state: state.clone(),
            label: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Styled for CommandList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for CommandList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for CommandList {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for CommandList {}

impl RenderOnce for CommandList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .role(Role::ListBox)
            .when_some(self.label, |this, label| this.aria_label(label))
            .overflow_y_scroll()
            .track_scroll(&self.state.read(cx).scroll)
    }
}

fn option_id(state: EntityId, index: usize) -> ElementId {
    (
        ElementId::from(("command-option", state)),
        index.to_string(),
    )
        .into()
}

/// One action row. The native text input keeps keyboard focus during navigation.
///
/// The highlighted row exposes selected metadata but is not a committed value.
/// The pinned GPUI API cannot link an input to a sibling list's active descendant;
/// this component does not claim that unsupported relationship.
#[derive(IntoElement)]
pub struct CommandOption {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CommandState>,
    index: usize,
}

impl CommandOption {
    pub fn new(state: &Entity<CommandState>, index: usize) -> Self {
        Self {
            base: div()
                .id(option_id(state.entity_id(), index))
                .test_support()
                .flex_shrink_0(),
            state: state.clone(),
            index,
        }
    }
}

impl Styled for CommandOption {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for CommandOption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for CommandOption {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for CommandOption {}

impl RenderOnce for CommandOption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let item = state
            .items
            .get(self.index)
            .expect("CommandOption index must identify an item");
        if state.visible.binary_search(&self.index).is_err() {
            return gpui::Empty.into_any_element();
        }
        let disabled = item.disabled || state.input.read(cx).presentation().is_disabled();
        let selected = state.highlighted == Some(self.index);
        let label = item.label.clone();
        let index = self.index;
        let focus = state.focus.clone();
        let hover = self.state.clone();
        let activate = self.state;
        self.base
            .role(Role::ListBoxOption)
            .aria_label(label)
            .aria_selected(selected)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                if !disabled {
                    focus.focus(window, cx);
                }
            })
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
            .a11y_disabled(disabled)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext, VisualTestContext,
        px,
    };

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        command: Entity<CommandState>,
        after: FocusHandle,
        _observation: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = self.command.read(cx);
            div()
                .w(px(320.))
                .flex()
                .flex_col()
                .child(
                    Command::new("commands", &self.command)
                        .accessibility_label("Session commands")
                        .flex()
                        .flex_col()
                        .child(CommandInput::new("query", &self.command).h(px(32.)))
                        .child(
                            CommandList::new("results", &self.command)
                                .accessibility_label("Results")
                                .h(px(64.))
                                .children(state.visible_indices().iter().map(|&index| {
                                    CommandOption::new(&self.command, index)
                                        .debug_selector(move || format!("option-{index}").into())
                                        .h(px(32.))
                                        .child(state.items()[index].label().clone())
                                })),
                        ),
                )
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Actions = Rc<RefCell<Vec<SharedString>>>;

    fn items() -> Vec<CommandItem> {
        vec![
            CommandItem::new("edit", "Edit session").keywords(["preferences", "settings"]),
            CommandItem::new("share", "Share").disabled(true),
            CommandItem::new("cursor", "Toggle cursor").keywords(["pointer"]),
            CommandItem::new("toolbar", "Toggle toolbar pin"),
            CommandItem::new("notes", "메모").keywords(["memo"]),
        ]
    }

    fn harness(
        cx: &mut TestAppContext,
        items: Vec<CommandItem>,
    ) -> (&mut VisualTestContext, Entity<Harness>, Actions) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                let command = cx.new(|cx| CommandState::new(items, window, cx));
                let observation = cx.observe(&command, |_, _, cx| cx.notify());
                Harness {
                    command,
                    after: cx.focus_handle(),
                    _observation: observation,
                }
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        let actions = Rc::new(RefCell::new(Vec::new()));
        cx.update(|window, cx| {
            let command = view.read(cx).command.clone();
            let actions = actions.clone();
            cx.subscribe(&command, move |_, event: &CommandEvent, _| {
                let CommandEvent::Action(value) = event;
                actions.borrow_mut().push(value.clone());
            })
            .detach();
            window.activate_window();
            let focus = command.read(cx).focus_handle().clone();
            focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        (cx, view, actions)
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

    fn set_query(cx: &mut VisualTestContext, view: &Entity<Harness>, query: &str) {
        cx.update(|window, cx| {
            let command = view.read(cx).command.clone();
            command.update(cx, |state, cx| {
                state.set_query(query.to_owned(), window, cx)
            });
        });
        draw(cx);
    }

    #[gpui::test]
    fn filtering_uses_native_query_and_keeps_plain_text_editing(cx: &mut TestAppContext) {
        let (cx, view, actions) = harness(cx, items());
        set_query(cx, &view, "  PREF session ");
        cx.update(|_, cx| {
            let state = view.read(cx).command.read(cx);
            assert_eq!(state.input().read(cx).value(), "  PREF session ");
            assert_eq!(state.visible_indices(), [0]);
            assert_eq!(state.highlighted_index(), Some(0));
        });
        // The filter also follows programmatic input changes, which emit no Change event.
        cx.update(|window, cx| {
            let input = view.read(cx).command.read(cx).input().clone();
            input.update(cx, |input, cx| input.set_value("pOIntER", window, cx));
        });
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).command.read(cx).visible_indices(), [2]);
        });
        set_query(cx, &view, "");
        cx.simulate_input("Toggle");
        cx.simulate_input(" ");
        draw(cx);
        cx.update(|_, cx| {
            let state = view.read(cx).command.read(cx);
            assert_eq!(state.input().read(cx).value(), "Toggle ");
            assert_eq!(state.visible_indices(), [2, 3]);
            assert_eq!(state.highlighted_index(), Some(2));
        });
        cx.simulate_keystrokes("home");
        draw(cx);
        cx.update(|_, cx| {
            let state = view.read(cx).command.read(cx);
            assert_eq!(state.input().read(cx).selected_range(), 0..0);
            assert_eq!(state.highlighted_index(), Some(2));
        });
        cx.simulate_keystrokes("end");
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .command
                    .read(cx)
                    .input()
                    .read(cx)
                    .selected_range(),
                7..7,
            );
        });
        set_query(cx, &view, "no results");
        press(cx, "enter");
        cx.update(|_, cx| {
            let state = view.read(cx).command.read(cx);
            assert!(state.visible_indices().is_empty());
            assert_eq!(state.highlighted_index(), None);
        });
        set_query(cx, &view, "Share");
        press(cx, "enter");
        let first_option = cx.debug_bounds("option-1").unwrap().center();
        cx.simulate_click(first_option, Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).command.read(cx);
            assert_eq!(state.visible_indices(), [1]);
            assert_eq!(state.highlighted_index(), None);
            assert!(state.focus_handle().is_focused(window));
            #[cfg(feature = "test-support")]
            {
                let input = crate::test_support::find(window, &[], &"query".into()).unwrap();
                assert_eq!(input.role(), Some(Role::TextInput));
                assert_eq!(input.label(), Some("Search commands"));
                assert_eq!(input.value(), Some("Share"));
                let option = crate::test_support::snapshots(window)
                    .into_iter()
                    .find(|item| item.role() == Some(Role::ListBoxOption))
                    .unwrap();
                assert_eq!(option.label(), Some("Share"));
                assert_eq!(option.disabled(), Some(true));
                assert_eq!(option.selected(), Some(false));
            }
        });
        assert!(actions.borrow().is_empty());
    }

    #[gpui::test]
    fn arrows_and_pointer_share_one_cursor_and_leave_the_input_focused(cx: &mut TestAppContext) {
        let (cx, view, actions) = harness(cx, items());
        cx.simulate_keystrokes("down");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).command.read(cx);
            assert_eq!(state.highlighted_index(), Some(2), "skip the disabled row");
            assert!(state.focus_handle().is_focused(window));
            assert!(
                state.scroll.offset().y < px(0.),
                "reveal an offscreen result"
            );
        });
        assert!(actions.borrow().is_empty());
        cx.simulate_keystrokes("up up");
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).command.read(cx).highlighted_index(), Some(4));
        });
        let last_option = cx.debug_bounds("option-4").unwrap().center();
        cx.simulate_click(last_option, Modifiers::default());
        draw(cx);
        assert_eq!(*actions.borrow(), [SharedString::from("notes")]);
        press(cx, "enter");
        assert_eq!(
            *actions.borrow(),
            ["notes", "notes"].map(SharedString::from)
        );
        set_query(cx, &view, "Toggle");
        cx.simulate_keystrokes("down");
        draw(cx);
        press(cx, "enter");
        assert_eq!(
            *actions.borrow(),
            ["notes", "notes", "toolbar"].map(SharedString::from),
        );
        cx.update(|window, cx| {
            let state = view.read(cx).command.read(cx);
            assert!(state.focus_handle().is_focused(window));
            assert_eq!(state.input().read(cx).value(), "Toggle");
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
    }

    #[gpui::test]
    fn marked_text_never_navigates_or_executes_a_command(cx: &mut TestAppContext) {
        let mut items = items();
        items.push(CommandItem::new("note-settings", "메모 설정"));
        let (cx, view, actions) = harness(cx, items);
        cx.update(|window, cx| {
            let input = view.read(cx).command.read(cx).input().clone();
            input.update(cx, |input, cx| {
                input.replace_and_mark_text_in_range(None, "메", Some(1..1), window, cx);
            });
        });
        draw(cx);
        cx.simulate_keystrokes("down");
        press(cx, "enter");
        cx.update(|window, cx| {
            let command = view.read(cx).command.clone();
            assert_eq!(command.read(cx).visible_indices(), [4, 5]);
            assert_eq!(command.read(cx).highlighted_index(), Some(4));
            let input = command.read(cx).input().clone();
            input.update(cx, |input, cx| {
                assert!(input.marked_text_range(window, cx).is_some());
                input.replace_text_in_range(None, "메모", window, cx);
            });
        });
        assert!(actions.borrow().is_empty());
        draw(cx);
        cx.simulate_keystrokes("down");
        press(cx, "enter");
        assert_eq!(*actions.borrow(), [SharedString::from("note-settings")]);
        cx.update(|window, cx| {
            let input = view.read(cx).command.read(cx).input().clone();
            input.update(cx, |input, cx| {
                assert!(input.marked_text_range(window, cx).is_none());
                assert_eq!(input.value(), "메모");
            });
        });
    }
}
