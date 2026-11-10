//! Editable single selection composed from Command's search and the existing Popup.
//! The native input owns the draft query; the combobox retains only the committed
//! option and popup lifecycle. Applications supply all presentation.

use std::{cell::Cell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, AppContext as _, Bounds, Context, ElementId, Entity,
    EntityInputHandler as _, EventEmitter, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window, div, px,
};

use crate::{
    CommandEvent, CommandList, CommandOption, CommandState, Input, InputAccessibility, InputBase,
    InputState, InputStyles, Popup, SearchItem, TestSupportExt as _,
    global_state::{DeferredPopover, GlobalState},
    input::{Enter, Escape, InputEvent, MoveDown, MoveUp},
};

/// Emitted once when a user commits a different option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComboboxEvent {
    Change(SharedString),
}

/// A fixed ordered collection with editable search and one committed value.
///
/// Opening with the pointer or an arrow clears the draft to show all options.
/// Typing filters suggestions without changing the committed value. Enter or a
/// result click commits; Escape, Tab, blur, and outside presses restore the
/// committed label. Only the native input participates in Tab traversal.
///
/// This first contract covers fixed single selection, not free-form values,
/// asynchronous sources, multiple selection, or grouped results.
pub struct ComboboxState {
    search: Entity<CommandState>,
    input: Entity<InputState>,
    focus: FocusHandle,
    selected: Option<usize>,
    open: bool,
    disabled: bool,
    deferred_context: Option<DeferredPopover>,
    _search_observation: Subscription,
    _search_actions: Subscription,
    _input_events: Subscription,
}

impl ComboboxState {
    pub fn new(
        items: impl IntoIterator<Item = SearchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| CommandState::new(items, window, cx));
        let input = search.read(cx).input().clone();
        let focus = search.read(cx).focus_handle().clone();
        let search_observation = cx.observe(&search, |_, _, cx| cx.notify());
        let search_actions = cx.subscribe_in(&search, window, |state, _, event, window, cx| {
            let CommandEvent::Action(value) = event;
            state.commit(value, window, cx);
        });
        let input_events = cx.subscribe_in(&input, window, |state, _, event, window, cx| {
            match event {
                InputEvent::Change if !state.disabled && state.focus.is_focused(window) => {
                    state.search.update(cx, |search, cx| search.sync_query(cx));
                    state.show_results(window, cx);
                }
                InputEvent::Blur => {
                    // A detached input can report blur while its retained handle
                    // still owns focus. Discard that draft after drawing too.
                    let detached = state.focus.is_focused(window);
                    cx.defer_in(window, move |state, window, cx| {
                        if detached || !window.is_window_active() || !state.focus.is_focused(window)
                        {
                            state.close(window, cx);
                        }
                    });
                }
                _ => {}
            }
        });
        Self {
            search,
            input,
            focus,
            selected: None,
            open: false,
            disabled: false,
            deferred_context: None,
            _search_observation: search_observation,
            _search_actions: search_actions,
            _input_events: input_events,
        }
    }

    /// The native query state, also available for application text styling.
    pub fn input(&self) -> &Entity<InputState> {
        &self.input
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn items<'a>(&'a self, cx: &'a App) -> &'a [SearchItem] {
        self.search.read(cx).items()
    }

    /// Original item indices in search order, including matching disabled items.
    pub fn visible_indices<'a>(&'a self, cx: &'a App) -> &'a [usize] {
        self.search.read(cx).visible_indices()
    }

    pub fn highlighted_index(&self, cx: &App) -> Option<usize> {
        self.search.read(cx).highlighted_index()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_item<'a>(&'a self, cx: &'a App) -> Option<&'a SearchItem> {
        self.selected.map(|index| &self.items(cx)[index])
    }

    pub fn selected_value<'a>(&'a self, cx: &'a App) -> Option<&'a SharedString> {
        self.selected_item(cx).map(SearchItem::value)
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets a committed value and closes the draft without emitting Change.
    /// None clears selection. An unknown or disabled value is rejected.
    pub fn set_selected_value(
        &mut self,
        value: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let selected = match value {
            Some(value) => {
                let Some(index) = self
                    .items(cx)
                    .iter()
                    .position(|item| item.value().as_ref() == value && !item.is_disabled())
                else {
                    return false;
                };
                Some(index)
            }
            None => None,
        };
        self.selected = selected;
        self.close(window, cx);
        cx.notify();
        true
    }

    /// Starts a focused draft query without changing the committed selection.
    pub fn set_query(
        &mut self,
        query: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.is_composing(window, cx) {
            return;
        }
        let query = query.into();
        self.search
            .update(cx, |search, cx| search.set_query(query, window, cx));
        self.focus.focus(window, cx);
        self.show_results(window, cx);
    }

    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        if disabled {
            self.close(window, cx);
        }
        self.disabled = disabled;
        self.input
            .update(cx, |input, cx| input.set_disabled(disabled, cx));
        cx.notify();
        window.refresh();
    }

    /// Opens all options and highlights the committed option or first enabled item.
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open || self.disabled || self.is_composing(window, cx) {
            return;
        }
        let highlighted = self
            .selected
            .or_else(|| self.items(cx).iter().position(|item| !item.is_disabled()));
        self.search.update(cx, |search, cx| {
            search.set_query("", window, cx);
            if let Some(index) = highlighted {
                search.highlight(index, window, cx);
            }
        });
        self.focus.focus(window, cx);
        self.show_results(window, cx);
    }

    /// Discards the draft, restores the committed label, and preserves current focus.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label = self
            .selected_item(cx)
            .map(|item| item.label().clone())
            .unwrap_or_default();
        let changed = self.input.read(cx).value() != label;
        let was_open = self.open;
        self.open = false;
        self.deferred_context = None;
        if changed {
            self.search
                .update(cx, |search, cx| search.set_query(label, window, cx));
        }
        if was_open || changed {
            cx.notify();
            window.refresh();
        }
    }

    fn show_results(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open || self.disabled {
            return;
        }
        self.open = true;
        self.deferred_context = Some(GlobalState::register_deferred_popover(cx));
        cx.notify();
        window.refresh();
    }

    fn is_composing(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.search
            .update(cx, |search, cx| search.is_composing(window, cx))
    }

    fn move_cursor(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.is_composing(window, cx) {
            return;
        }
        if self.open {
            self.search
                .update(cx, |search, cx| search.move_cursor(forward, window, cx));
        } else {
            self.open(window, cx);
        }
    }

    fn commit(&mut self, value: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open || self.disabled {
            return;
        }
        let Some(index) = self.items(cx).iter().position(|item| item.value() == value) else {
            return;
        };
        let changed = self.selected != Some(index);
        self.selected = Some(index);
        self.close(window, cx);
        if changed {
            cx.emit(ComboboxEvent::Change(value.clone()));
        }
    }
}

impl EventEmitter<ComboboxEvent> for ComboboxState {}

/// An editable input and anchored suggestions with application-owned appearance.
///
/// Keep this mounted while closed. The input builder styles the existing native
/// input's frame. Content should contain a ComboboxList and optional static empty
/// text; result rows do not take keyboard focus.
#[derive(IntoElement)]
pub struct Combobox {
    base: gpui::Stateful<gpui::Div>,
    state: Entity<ComboboxState>,
    input: ComboboxInput,
    content: Option<AnyElement>,
    anchor: Anchor,
    offset: Pixels,
}

impl Combobox {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ComboboxState>) -> Self {
        Self {
            base: div().id(("combobox", state.entity_id())),
            state: state.clone(),
            input: ComboboxInput::new(id, state),
            content: None,
            anchor: Anchor::TopLeft,
            offset: px(0.),
        }
    }

    pub fn input(mut self, build: impl FnOnce(ComboboxInput) -> ComboboxInput) -> Self {
        self.input = build(self.input);
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
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

impl Styled for Combobox {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Combobox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = self.state.read(cx).open;
        let trigger_bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
        let measured_bounds = trigger_bounds.clone();
        let outside = self.state.clone();
        let mut popup = Popup::new(("combobox-popup", self.state.entity_id()), self.input)
            .anchor(self.anchor)
            .offset(self.offset)
            .on_position(move |_, bounds| measured_bounds.set(bounds));
        if open {
            popup = popup.content(
                div()
                    .id("combobox-surface")
                    .on_mouse_down_out(move |event, window, cx| {
                        if !trigger_bounds.get().contains(&event.position) {
                            outside.update(cx, |state, cx| state.close(window, cx));
                        }
                    })
                    .children(self.content),
            );
        }
        let up = self.state.clone();
        let down = self.state.clone();
        let confirm = self.state.clone();
        let cancel = self.state.clone();
        let tab = self.state.clone();
        let tab_prev = self.state;
        self.base
            .on_action(move |_: &MoveUp, window, cx| {
                up.update(cx, |state, cx| state.move_cursor(false, window, cx));
            })
            .on_action(move |_: &MoveDown, window, cx| {
                down.update(cx, |state, cx| state.move_cursor(true, window, cx));
            })
            .capture_action(move |action: &Enter, window, cx| {
                confirm.update(cx, |state, cx| {
                    if state.open && !action.secondary && !action.shift {
                        // The capture path prevents Input's Enter event from
                        // activating the same result a second time.
                        cx.stop_propagation();
                        state
                            .search
                            .update(cx, |search, cx| search.activate_highlighted(window, cx));
                    } else {
                        cx.propagate();
                    }
                });
            })
            .capture_action(move |_: &Escape, window, cx| {
                cancel.update(cx, |state, cx| {
                    // Cancel the native composition first; this Escape must not
                    // also discard the draft or close an enclosing dialog.
                    if state.is_composing(window, cx) {
                        state
                            .input
                            .update(cx, |input, cx| input.unmark_text(window, cx));
                        cx.stop_propagation();
                    } else if state.open {
                        state.close(window, cx);
                        cx.stop_propagation();
                    } else {
                        cx.propagate();
                    }
                });
            })
            .on_action(move |_: &crate::root::Tab, window, cx| {
                tab.update(cx, |state, cx| state.close(window, cx));
                cx.propagate();
            })
            .on_action(move |_: &crate::root::TabPrev, window, cx| {
                tab_prev.update(cx, |state, cx| state.close(window, cx));
                cx.propagate();
            })
            .child(popup)
    }
}

/// The combobox's InputBase frame around its existing single-line editor.
/// Field-provided name, description, error, and required metadata are preserved.
#[derive(IntoElement)]
pub struct ComboboxInput {
    base: InputBase,
    state: Entity<ComboboxState>,
    label: Option<SharedString>,
    placeholder: SharedString,
}

impl ComboboxInput {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ComboboxState>) -> Self {
        Self {
            base: InputBase::new(id).role(None::<Role>),
            state: state.clone(),
            label: None,
            placeholder: "Choose an option...".into(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
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

impl Styled for ComboboxInput {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for ComboboxInput {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ComboboxInput {}

impl RenderOnce for ComboboxInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let open = state.open;
        let input = state.input.clone();
        input.update(cx, |input, cx| {
            if input.presentation().placeholder() != &self.placeholder {
                input.set_placeholder(self.placeholder.clone(), window, cx);
            }
            let accessibility = input
                .accessibility()
                .cloned()
                .unwrap_or_else(|| InputAccessibility::new(Some(Role::ComboBox)));
            let label = self
                .label
                .or_else(|| accessibility.label().cloned())
                .unwrap_or_else(|| "Choose an option".into());
            input.set_accessibility(
                accessibility
                    .with_role(Some(Role::ComboBox))
                    .with_label(Some(label))
                    .with_placeholder(Some(self.placeholder))
                    .with_expanded(Some(open)),
            );
        });
        let presentation = input.read(cx).presentation();
        let trigger = self.state;
        self.base
            .semantic_owner::<Input>(input.entity_id())
            .focused(presentation.focus_handle().is_focused(window))
            .disabled(presentation.is_disabled())
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                trigger.update(cx, |state, cx| state.open(window, cx));
            })
            .child(Input::new(&input))
    }
}

/// A named scrolling list. Render direct ComboboxOption children in the exact
/// order returned by visible_indices; keep empty text outside the list.
#[derive(IntoElement)]
pub struct ComboboxList {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ComboboxState>,
    label: Option<SharedString>,
}

impl ComboboxList {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ComboboxState>) -> Self {
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

impl Styled for ComboboxList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ComboboxList {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}

impl RenderOnce for ComboboxList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        CommandList::from_base(self.base, &self.state.read(cx).search, self.label)
    }
}

/// One searchable option, sharing Command's navigation and pointer activation.
/// Selected metadata describes the committed value, not the pending highlight.
///
/// The pinned GPUI API cannot link a focused input to a sibling list's active
/// descendant. This component does not claim that unsupported relationship.
#[derive(IntoElement)]
pub struct ComboboxOption {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ComboboxState>,
    index: usize,
}

impl ComboboxOption {
    pub fn new(state: &Entity<ComboboxState>, index: usize) -> Self {
        Self {
            base: div()
                .id((
                    ElementId::from(("combobox-option", state.entity_id())),
                    index.to_string(),
                ))
                .test_support()
                .flex_shrink_0(),
            state: state.clone(),
            index,
        }
    }
}

impl Styled for ComboboxOption {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ComboboxOption {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}

impl InteractiveElement for ComboboxOption {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ComboboxOption {}

impl RenderOnce for ComboboxOption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        CommandOption::from_base(
            self.base,
            &state.search,
            self.index,
            state.selected == Some(self.index),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext, VisualTestContext,
        prelude::FluentBuilder as _,
    };

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        combo: Entity<ComboboxState>,
        shown: bool,
        after: FocusHandle,
        outer_cancels: Rc<Cell<usize>>,
        _observation: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = self.combo.read(cx);
            let cancels = self.outer_cancels.clone();
            div()
                .w(px(320.))
                .flex()
                .flex_col()
                .on_action(move |_: &Escape, _, _| cancels.set(cancels.get() + 1))
                .when(self.shown, |this| {
                    this.child(
                        Combobox::new("preset", &self.combo)
                            .input(|input| input.accessibility_label("Preset").w_full().h(px(32.)))
                            .content(
                                ComboboxList::new("preset-results", &self.combo)
                                    .accessibility_label("Presets")
                                    .w(px(320.))
                                    .max_h(px(160.))
                                    .children(state.visible_indices(cx).iter().map(|&index| {
                                        ComboboxOption::new(&self.combo, index)
                                            .debug_selector(move || {
                                                format!("choice-{index}").into()
                                            })
                                            .h(px(32.))
                                            .child(state.items(cx)[index].label().clone())
                                    })),
                            ),
                    )
                })
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .debug_selector(|| "after".into())
                        .mt(px(200.))
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<SharedString>>>;

    fn harness(cx: &mut TestAppContext) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let combo = cx.new(|cx| {
                    ComboboxState::new(
                        [
                            SearchItem::new("balanced", "Balanced"),
                            SearchItem::new("unavailable", "Unavailable").disabled(true),
                            SearchItem::new("presentation", "Presentation").keywords(["slides"]),
                            SearchItem::new("quality", "High quality").keywords(["hq"]),
                        ],
                        window,
                        cx,
                    )
                });
                combo.update(cx, |state, cx| {
                    assert!(state.set_selected_value(Some("balanced"), window, cx));
                });
                let observation = cx.observe(&combo, |_, _, cx| cx.notify());
                Harness {
                    combo,
                    shown: true,
                    after: cx.focus_handle(),
                    outer_cancels: Rc::new(Cell::new(0)),
                    _observation: observation,
                }
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        let changes = Rc::new(RefCell::new(Vec::new()));
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            let changes = changes.clone();
            cx.subscribe(&combo, move |_, event: &ComboboxEvent, _| {
                let ComboboxEvent::Change(value) = event;
                changes.borrow_mut().push(value.clone());
            })
            .detach();
            window.activate_window();
            combo.read(cx).focus_handle().clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        (cx, view, changes)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
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

    fn type_query(cx: &mut VisualTestContext, query: &str) {
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        });
        cx.simulate_input(query);
        draw(cx);
    }

    #[gpui::test]
    fn draft_search_commits_once_and_cancellation_restores_selection(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx);
        cx.simulate_keystrokes("down down");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(state.is_open());
            assert_eq!(
                state.highlighted_index(cx),
                Some(2),
                "disabled option is skipped"
            );
            assert_eq!(state.selected_index(), Some(0));
            assert!(state.focus_handle().is_focused(window));
        });
        press(cx, "escape");
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(!state.is_open());
            assert_eq!(state.input().read(cx).value(), "Balanced");
            assert_eq!(view.read(cx).outer_cancels.get(), 0);
        });
        cx.simulate_keystrokes("down down");
        press(cx, "enter");
        press(cx, "enter");
        assert_eq!(*changes.borrow(), [SharedString::from("presentation")]);
        type_query(cx, "no matching preset");
        press(cx, "enter");
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(state.is_open());
            assert!(state.visible_indices(cx).is_empty());
            assert_eq!(
                state.selected_value(cx).map(|value| value.as_ref()),
                Some("presentation")
            );
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(!state.is_open());
            assert_eq!(state.input().read(cx).value(), "Presentation");
            assert!(view.read(cx).after.is_focused(window));
        });
        cx.simulate_keystrokes("shift-tab");
        type_query(cx, "hq");
        let option = cx.debug_bounds("choice-3").unwrap().center();
        cx.simulate_click(option, Modifiers::default());
        draw(cx);
        assert_eq!(
            *changes.borrow(),
            ["presentation", "quality"].map(SharedString::from)
        );
        cx.update(|window, cx| {
            let state = view.read(cx).combo.read(cx);
            assert_eq!(state.selected_index(), Some(3));
            assert_eq!(state.input().read(cx).value(), "High quality");
            assert!(state.focus_handle().is_focused(window));
            #[cfg(feature = "test-support")]
            {
                let input = crate::test_support::find(window, &[], &"preset".into()).unwrap();
                assert_eq!(input.role(), Some(Role::ComboBox));
                assert_eq!(input.label(), Some("Preset"));
                assert_eq!(input.expanded(), Some(false));
                assert_eq!(input.value(), Some("High quality"));
            }
        });
        cx.update(|window, cx| {
            view.read(cx).combo.clone().update(cx, |state, cx| {
                state.set_selected_value(None, window, cx);
            });
        });
        press(cx, "down");
        press(cx, "down");
        press(cx, "escape");
        press(cx, "down");
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert_eq!(state.selected_index(), None);
            assert_eq!(state.highlighted_index(cx), Some(0));
        });
    }

    #[gpui::test]
    fn composition_and_disabled_input_do_not_commit_and_outside_focus_is_preserved(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness(cx);
        cx.simulate_keystrokes("down");
        draw(cx);
        cx.update(|window, cx| {
            let input = view.read(cx).combo.read(cx).input().clone();
            input.update(cx, |input, cx| {
                input.replace_and_mark_text_in_range(None, "pre", Some(3..3), window, cx);
            });
        });
        draw(cx);
        cx.simulate_keystrokes("down");
        press(cx, "enter");
        press(cx, "escape");
        cx.update(|window, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(state.is_open());
            assert_eq!(state.selected_index(), Some(0));
            assert_eq!(view.read(cx).outer_cancels.get(), 0);
            let input = state.input().clone();
            input.update(cx, |input, cx| {
                assert!(input.marked_text_range(window, cx).is_none());
                input.replace_text_in_range(Some(0..3), "presentation", window, cx);
            });
        });
        assert!(changes.borrow().is_empty());
        draw(cx);
        press(cx, "enter");
        assert_eq!(*changes.borrow(), [SharedString::from("presentation")]);
        cx.simulate_keystrokes("down");
        draw(cx);
        let after = cx.debug_bounds("after").unwrap().center();
        cx.simulate_click(after, Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            assert!(!combo.read(cx).is_open());
            assert_eq!(combo.read(cx).input().read(cx).value(), "Presentation");
            assert!(view.read(cx).after.is_focused(window));
            combo.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        draw(cx);
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            assert!(
                view.read(cx).after.is_focused(window),
                "disabled field is skipped"
            );
            combo.update(cx, |state, cx| {
                state.open(window, cx);
                state.set_query("hq", window, cx);
                assert!(!state.is_open());
                assert_eq!(state.selected_index(), Some(2));
            });
        });
        assert_eq!(changes.borrow().len(), 1);
        cx.update(|window, cx| {
            view.read(cx).combo.clone().update(cx, |state, cx| {
                state.set_disabled(false, window, cx);
                state.set_query("hq", window, cx);
            });
        });
        draw(cx);
        view.update(cx, |view, cx| {
            view.shown = false;
            cx.notify();
        });
        draw(cx);
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(!state.is_open());
            assert_eq!(state.input().read(cx).value(), "Presentation");
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }
}
