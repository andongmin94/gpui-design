//! Editable single or multiple selection over Command's search and the existing Popup.
//! The native input owns the draft query; the combobox retains only the committed
//! option and popup lifecycle. Applications supply all presentation.

use std::{cell::Cell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, AppContext as _, Bounds, Context, ElementId, Entity,
    EntityInputHandler as _, EventEmitter, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window, div, px,
};

use crate::command::CommandGroup;
use crate::{
    CommandEvent, CommandList, CommandOption, CommandState, Input, InputAccessibility, InputBase,
    InputState, InputStyles, Popup, SearchItem, TestSupportExt as _,
    global_state::{DeferredPopover, GlobalState},
    input::{Enter, Escape, InputEvent, MoveDown, MoveUp},
};

/// Committed values after a user selection, removal, or clear, in item order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComboboxEvent {
    Change(Vec<SharedString>),
}

/// An ordered collection with editable search and committed values.
///
/// Opening with the pointer or an arrow clears the draft to show all options.
/// Typing filters suggestions without changing the committed value. Enter or a
/// result click commits; Escape, Tab, blur, and outside presses restore the
/// committed label. Only the native input participates in Tab traversal.
///
/// Single selection is the default. Multiple mode toggles results without closing
/// the popup or clearing the draft; consumers render selected-value chips.
pub struct ComboboxState {
    search: Entity<CommandState>,
    input: Entity<InputState>,
    focus: FocusHandle,
    selected: Vec<usize>,
    multiple: bool,
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
            selected: Vec::new(),
            multiple: false,
            open: false,
            disabled: false,
            deferred_context: None,
            _search_observation: search_observation,
            _search_actions: search_actions,
            _input_events: input_events,
        }
    }

    /// Allows several committed values. Configure before rendering the state.
    /// Disabling multiple mode with more than one selected value is an error.
    pub fn multiple(mut self, multiple: bool) -> Self {
        assert!(
            multiple || self.selected.len() <= 1,
            "ComboboxState single mode cannot have multiple selected values"
        );
        self.multiple = multiple;
        self
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

    pub fn selected_indices(&self) -> &[usize] {
        &self.selected
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected.binary_search(&index).is_ok()
    }

    pub fn selected_values(&self, cx: &App) -> Vec<SharedString> {
        self.selected
            .iter()
            .map(|&index| self.items(cx)[index].value().clone())
            .collect()
    }

    pub fn selected_items<'a>(&'a self, cx: &'a App) -> impl Iterator<Item = &'a SearchItem> {
        self.selected
            .iter()
            .map(move |&index| &self.items(cx)[index])
    }

    pub fn visible_groups(&self, cx: &App) -> Vec<SharedString> {
        self.search.read(cx).visible_groups()
    }

    pub fn visible_group_indices(&self, group: Option<&str>, cx: &App) -> Vec<usize> {
        self.search.read(cx).visible_group_indices(group)
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets committed values and closes the draft without emitting Change.
    /// Unknown/disabled values and multiple values in single mode are rejected.
    pub fn set_selected_values<V: AsRef<str>>(
        &mut self,
        values: impl IntoIterator<Item = V>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut selected = Vec::new();
        for value in values {
            let Some(index) = self
                .items(cx)
                .iter()
                .position(|item| item.value().as_ref() == value.as_ref() && !item.is_disabled())
            else {
                return false;
            };
            if !selected.contains(&index) {
                selected.push(index);
            }
        }
        if !self.multiple && selected.len() > 1 {
            return false;
        }
        selected.sort_unstable();
        self.selected = selected;
        self.close(window, cx);
        cx.notify();
        true
    }

    /// Replaces options without emitting Change. Selection follows stable values;
    /// missing or disabled values are removed. An active native draft and IME
    /// composition remain untouched. Closed single selection refreshes its label.
    pub fn set_items(
        &mut self,
        items: impl IntoIterator<Item = SearchItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let values = self.selected_values(cx);
        self.search
            .update(cx, |search, cx| search.set_items(items, window, cx));
        self.selected = self
            .items(cx)
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_disabled() && values.contains(item.value()))
            .map(|(index, _)| index)
            .collect();
        if !self.open && !self.is_composing(window, cx) {
            self.close(window, cx);
        }
        cx.notify();
        window.refresh();
    }

    /// Removes a committed value as a user action, returning focus to the query.
    /// Disabled controls and active native composition reject the request.
    pub fn remove_value(
        &mut self,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.disabled || self.is_composing(window, cx) {
            return false;
        }
        let Some(position) = self
            .selected
            .iter()
            .position(|&index| self.items(cx)[index].value().as_ref() == value)
        else {
            return false;
        };
        self.focus.focus(window, cx);
        self.selected.remove(position);
        if !self.multiple {
            self.close(window, cx);
        }
        self.emit_change(window, cx);
        true
    }

    /// Clears committed values and the query as a user action, preserving whether
    /// results are open. Application-owned clear buttons call this method.
    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.disabled || self.is_composing(window, cx) {
            return false;
        }
        let changed = !self.selected.is_empty();
        self.focus.focus(window, cx);
        self.selected.clear();
        self.search
            .update(cx, |search, cx| search.set_query("", window, cx));
        if changed {
            self.emit_change(window, cx);
        }
        changed
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
            .first()
            .copied()
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
        let label = if self.multiple {
            SharedString::default()
        } else {
            self.selected
                .first()
                .map(|&index| self.items(cx)[index].label().clone())
                .unwrap_or_default()
        };
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
        if self.items(cx)[index].is_disabled() || self.is_composing(window, cx) {
            return;
        }
        let changed = if self.multiple {
            match self.selected.binary_search(&index) {
                Ok(position) => {
                    self.selected.remove(position);
                }
                Err(position) => self.selected.insert(position, index),
            }
            true
        } else {
            let changed = self.selected != [index];
            self.selected = vec![index];
            self.close(window, cx);
            changed
        };
        if changed {
            self.emit_change(window, cx);
        }
    }

    fn emit_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(ComboboxEvent::Change(self.selected_values(cx)));
        cx.notify();
        window.refresh();
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

/// A named scrolling list. Render ComboboxOption children in result order,
/// optionally inside ComboboxGroup wrappers. Keep empty text outside the list.
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
        let state = self.state.read(cx);
        CommandList::from_base(self.base, &state.search, self.label, state.multiple)
    }
}

/// A named result group and heading over the existing Command list navigation.
/// Items with one group label must be contiguous in the collection. Render only
/// this group's visible indices; empty groups have no layout or native nodes.
#[derive(IntoElement)]
pub struct ComboboxGroup {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ComboboxState>,
    label: SharedString,
    heading: gpui::Div,
    children: Vec<AnyElement>,
}

impl ComboboxGroup {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<ComboboxState>,
        label: impl Into<SharedString>,
    ) -> Self {
        let label = label.into();
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            heading: div().child(label.clone()),
            label,
            children: Vec::new(),
        }
    }

    pub fn heading(mut self, build: impl FnOnce(gpui::Div) -> gpui::Div) -> Self {
        self.heading = build(self.heading);
        self
    }
}

impl Styled for ComboboxGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ComboboxGroup {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for ComboboxGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        CommandGroup::from_base(
            self.base,
            &self.state.read(cx).search,
            self.label,
            self.heading,
        )
        .children(self.children)
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
            state.is_selected(self.index),
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
            let option = |index| {
                ComboboxOption::new(&self.combo, index)
                    .debug_selector(move || format!("choice-{index}").into())
                    .h(px(32.))
                    .child(state.items(cx)[index].label().clone())
            };
            let groups = state.visible_groups(cx);
            let mut list = ComboboxList::new("preset-results", &self.combo)
                .accessibility_label("Presets")
                .w(px(320.))
                .max_h(px(if groups.is_empty() { 160. } else { 64. }));
            if groups.is_empty() {
                list = list.children(state.visible_indices(cx).iter().copied().map(option));
            } else {
                for group in groups {
                    list = list.child(
                        ComboboxGroup::new(
                            (ElementId::from("group"), group.clone()),
                            &self.combo,
                            group.clone(),
                        )
                        .heading(|heading| heading.h(px(24.)))
                        .children(
                            state
                                .visible_group_indices(Some(&group), cx)
                                .into_iter()
                                .map(option),
                        ),
                    );
                }
            }
            div()
                .w(px(320.))
                .flex()
                .flex_col()
                .on_action(move |_: &Escape, _, _| cancels.set(cancels.get() + 1))
                .when(self.shown, |this| {
                    this.child(
                        Combobox::new("preset", &self.combo)
                            .input(|input| input.accessibility_label("Preset").w_full().h(px(32.)))
                            .content(list),
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

    type Changes = Rc<RefCell<Vec<Vec<SharedString>>>>;

    fn harness(cx: &mut TestAppContext) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        harness_with_mode(cx, false)
    }

    fn harness_with_mode(
        cx: &mut TestAppContext,
        multiple: bool,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
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
                    .multiple(multiple)
                });
                combo.update(cx, |state, cx| {
                    assert!(state.set_selected_values(["balanced"], window, cx));
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
    fn multiple_toggles_preserve_drafts_and_user_removal_clear_respect_editability(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness_with_mode(cx, true);
        press(cx, "down");
        press(cx, "down");
        press(cx, "enter");
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(state.is_open());
            assert_eq!(state.selected_indices(), [0, 2]);
        });
        press(cx, "enter");
        type_query(cx, "hq");
        let option = cx.debug_bounds("choice-3").unwrap().center();
        cx.simulate_click(option, Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            combo.update(cx, |state, cx| {
                assert!(state.is_open());
                assert_eq!(state.input().read(cx).value(), "hq");
                assert_eq!(state.selected_indices(), [0, 3]);
                assert!(state.remove_value("balanced", window, cx));
                assert!(!state.remove_value("unavailable", window, cx));
            });
        });
        draw(cx);
        assert_eq!(changes.borrow().len(), 4);
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            combo.update(cx, |state, cx| {
                assert!(state.clear(window, cx));
                assert!(!state.clear(window, cx));
                assert!(state.is_open());
                assert!(state.selected_indices().is_empty());
                assert_eq!(state.input().read(cx).value(), "");
                assert!(state.set_selected_values(["balanced", "quality"], window, cx));
                state.set_query("hq", window, cx);
                let input = state.input().clone();
                input.update(cx, |input, cx| {
                    input.replace_and_mark_text_in_range(None, "q", Some(1..1), window, cx);
                });
                assert!(!state.remove_value("balanced", window, cx));
                assert!(!state.clear(window, cx));
            });
        });
        draw(cx);
        assert_eq!(changes.borrow().len(), 5);
        press(cx, "enter");
        assert_eq!(changes.borrow().len(), 5);
        press(cx, "escape");
        press(cx, "escape");
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            combo.update(cx, |state, cx| {
                assert!(!state.is_open());
                assert_eq!(state.input().read(cx).value(), "");
                assert_eq!(state.selected_indices(), [0, 3]);
                state.set_disabled(true, window, cx);
                assert!(!state.clear(window, cx));
                assert!(!state.remove_value("balanced", window, cx));
            });
        });
        assert_eq!(changes.borrow().len(), 5);
    }

    #[gpui::test]
    fn grouped_results_scroll_and_replacement_preserves_query_composition_and_values(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness_with_mode(cx, true);
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            combo.update(cx, |state, cx| {
                state.set_items(
                    [
                        SearchItem::new("balanced", "Balanced").group("Standard"),
                        SearchItem::new("unavailable", "Unavailable")
                            .group("Standard")
                            .disabled(true),
                        SearchItem::new("presentation", "Presentation").group("Special"),
                        SearchItem::new("quality", "High quality").group("Special"),
                    ],
                    window,
                    cx,
                );
                state.open(window, cx);
            });
        });
        draw(cx);
        press(cx, "down");
        #[cfg(feature = "test-support")]
        let highlighted_bounds = cx.debug_bounds("choice-2").unwrap();
        cx.update(|window, cx| {
            let state = view.read(cx).combo.read(cx);
            assert_eq!(state.highlighted_index(cx), Some(2));
            assert!(state.focus_handle().is_focused(window));
            #[cfg(feature = "test-support")]
            {
                let list =
                    crate::test_support::find(window, &[], &"preset-results".into()).unwrap();
                assert!(highlighted_bounds.top() >= list.bounds().top());
                assert!(highlighted_bounds.bottom() <= list.bounds().bottom());
                let group = crate::test_support::find(
                    window,
                    &[],
                    &(ElementId::from("group"), SharedString::from("Special")).into(),
                )
                .unwrap();
                assert_eq!(group.role(), Some(Role::Group));
                assert_eq!(group.label(), Some("Special"));
                let nodes = crate::test_support::snapshots(window);
                assert!(
                    nodes.iter().any(|node| node.role() == Some(Role::Heading)
                        && node.label() == Some("Special"))
                );
            }
        });
        type_query(cx, "quality");
        cx.update(|window, cx| {
            let _ = window;
            let state = view.read(cx).combo.read(cx);
            assert_eq!(state.visible_groups(cx), [SharedString::from("Special")]);
            #[cfg(feature = "test-support")]
            assert!(
                crate::test_support::find(
                    window,
                    &[],
                    &(ElementId::from("group"), SharedString::from("Standard")).into()
                )
                .is_none()
            );
        });
        press(cx, "enter");
        cx.update(|window, cx| {
            let combo = view.read(cx).combo.clone();
            combo.update(cx, |state, cx| {
                assert!(state.set_selected_values(
                    ["balanced", "presentation", "quality"],
                    window,
                    cx
                ));
                state.set_query("quality", window, cx);
                let input = state.input().clone();
                input.update(cx, |input, cx| {
                    input.replace_and_mark_text_in_range(Some(0..7), "q", Some(1..1), window, cx);
                });
                let before = input.read(cx).selected_range();
                state.set_items(
                    [
                        SearchItem::new("query-preview", "Query preview").group("Special"),
                        SearchItem::new("quality", "Quality renamed").group("Special"),
                        SearchItem::new("balanced", "Balanced")
                            .group("Special")
                            .disabled(true),
                    ],
                    window,
                    cx,
                );
                assert_eq!(state.selected_indices(), [1]);
                assert_eq!(state.visible_indices(cx), [0, 1]);
                assert_eq!(
                    state.highlighted_index(cx),
                    Some(1),
                    "keep the highlighted value when a new matching item is inserted before it"
                );
                assert_eq!(state.selected_values(cx), [SharedString::from("quality")]);
                assert_eq!(input.read(cx).value(), "q");
                assert_eq!(input.read(cx).selected_range(), before);
                input.update(cx, |input, cx| {
                    assert!(input.marked_text_range(window, cx).is_some())
                });
                assert!(state.is_open());
                assert!(state.focus_handle().is_focused(window));
            });
        });
        assert_eq!(
            changes.borrow().len(),
            1,
            "item replacement does not emit selection"
        );
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
            assert_eq!(state.selected_indices(), [0]);
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
        assert_eq!(
            *changes.borrow(),
            [vec![SharedString::from("presentation")]]
        );
        type_query(cx, "no matching preset");
        press(cx, "enter");
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(state.is_open());
            assert!(state.visible_indices(cx).is_empty());
            assert_eq!(
                state
                    .selected_items(cx)
                    .next()
                    .map(|item| item.value().as_ref()),
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
            ["presentation", "quality"].map(|value| vec![SharedString::from(value)])
        );
        cx.update(|window, cx| {
            let state = view.read(cx).combo.read(cx);
            assert_eq!(state.selected_indices(), [3]);
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
                state.set_selected_values(std::iter::empty::<&str>(), window, cx);
            });
        });
        press(cx, "down");
        press(cx, "down");
        press(cx, "escape");
        press(cx, "down");
        cx.update(|_, cx| {
            let state = view.read(cx).combo.read(cx);
            assert!(state.selected_indices().is_empty());
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
            assert_eq!(state.selected_indices(), [0]);
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
        assert_eq!(
            *changes.borrow(),
            [vec![SharedString::from("presentation")]]
        );
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
                assert_eq!(state.selected_indices(), [2]);
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
