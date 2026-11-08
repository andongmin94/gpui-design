//! Selection behavior and composable, unstyled trigger/listbox parts.
//! Modified for GPUI Design: retained option navigation and focus lifecycle.

use gpui::{
    AccessibleAction, AnyElement, App, Context, ElementId, Entity, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{
    Popup, StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{Cancel, Confirm, SelectDown, SelectUp},
    global_state::{DeferredPopover, GlobalState},
};

const CONTEXT: &str = "Select";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}

/// One value in a select's fixed collection. Values must be unique.
#[derive(Clone, Debug)]
pub struct SelectItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl SelectItem {
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

/// Emitted once when a user commits a different value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectEvent {
    Change(SharedString),
}

/// Retained selection, keyboard cursor, disabled state and popup focus.
///
/// The first release accepts a fixed, ordered collection. Arrow keys wrap and
/// skip disabled items; opening highlights the committed item or first enabled
/// item. Enter/Space commits, Escape discards the pending cursor and restores
/// trigger focus. The application owns option presentation, not this behavior.
pub struct SelectState {
    items: Vec<SelectItem>,
    selected: Option<usize>,
    highlighted: Option<usize>,
    open: bool,
    disabled: bool,
    focus: FocusHandle,
    content_focus: FocusHandle,
    deferred_context: Option<DeferredPopover>,
    dismiss_subscription: Option<Subscription>,
}

impl SelectState {
    pub fn new(items: impl IntoIterator<Item = SelectItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (ix, item) in items.iter().enumerate() {
            assert!(
                items[..ix].iter().all(|other| other.value != item.value),
                "SelectItem values must be unique"
            );
        }
        Self {
            items,
            selected: None,
            highlighted: None,
            open: false,
            disabled: false,
            focus: cx.focus_handle().tab_stop(true),
            content_focus: cx.focus_handle(),
            deferred_context: None,
            dismiss_subscription: None,
        }
    }

    pub fn default_value(mut self, value: impl AsRef<str>) -> Self {
        self.selected = self
            .items
            .iter()
            .position(|item| item.value.as_ref() == value.as_ref() && !item.disabled);
        self
    }

    pub fn items(&self) -> &[SelectItem] {
        &self.items
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn highlighted_index(&self) -> Option<usize> {
        self.highlighted
    }

    pub fn selected_item(&self) -> Option<&SelectItem> {
        self.selected.map(|ix| &self.items[ix])
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn content_focus_handle(&self) -> &FocusHandle {
        &self.content_focus
    }

    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        if disabled {
            self.close(window, cx);
        }
        self.disabled = disabled;
        cx.notify();
        window.refresh();
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.open {
            return;
        }
        self.highlighted = self
            .selected
            .or_else(|| self.items.iter().position(|item| !item.disabled));
        if self.highlighted.is_none() {
            return;
        }
        self.open = true;
        self.deferred_context = Some(GlobalState::register_deferred_popover(cx));
        self.dismiss_subscription =
            Some(
                cx.on_focus_out(&self.content_focus, window, |_, _, window, cx| {
                    // Focus callbacks run while drawing. Close after that phase so
                    // notify/refresh can schedule the frame that removes the popup.
                    cx.defer_in(window, |state, window, cx| {
                        if !window.is_window_active()
                            || (!state.content_focus.is_focused(window)
                                && !state.content_focus.contains_focused(window, cx))
                        {
                            state.close(window, cx);
                        }
                    });
                }),
            );
        self.content_focus.focus(window, cx);
        cx.notify();
        window.refresh();
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.highlighted = None;
        self.deferred_context = None;
        self.dismiss_subscription = None;
        // A pointer press or an ancestor closing may already have moved focus.
        // Do not move it back into a control the user has just left.
        if self.content_focus.is_focused(window) || self.content_focus.contains_focused(window, cx)
        {
            self.focus.focus(window, cx);
        }
        cx.notify();
        window.refresh();
    }

    fn move_cursor(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if !self.open {
            self.open(window, cx);
            return;
        }
        let Some(current) = self.highlighted else {
            return;
        };
        let len = self.items.len();
        self.highlighted = (1..=len)
            .map(|step| {
                if forward {
                    (current + step) % len
                } else {
                    (current + len - step) % len
                }
            })
            .find(|&ix| !self.items[ix].disabled);
        cx.notify();
        window.refresh();
    }

    fn highlight(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open || self.disabled || self.items[ix].disabled || self.highlighted == Some(ix) {
            return;
        }
        self.highlighted = Some(ix);
        cx.notify();
        window.refresh();
    }

    fn commit(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open || self.disabled || self.items[ix].disabled {
            return;
        }
        let changed = self.selected != Some(ix);
        self.selected = Some(ix);
        self.close(window, cx);
        if changed {
            cx.emit(SelectEvent::Change(self.items[ix].value.clone()));
        }
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if !self.open {
            self.open(window, cx);
        } else if let Some(ix) = self.highlighted {
            self.commit(ix, window, cx);
        }
    }
}

impl EventEmitter<SelectEvent> for SelectState {}

/// A select root with application-owned trigger and option-list presentation.
///
/// The root keeps the positioned listbox inside its keyboard/focus ancestry.
/// Its `trigger` is visual content; the root supplies activation and semantics.
/// Build `content` from [`SelectOption`] elements for the same retained state.
#[derive(IntoElement)]
pub struct Select {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<SelectState>,
    label: Option<SharedString>,
    trigger: Option<AnyElement>,
    content: Option<AnyElement>,
    style: StyleRefinement,
}

impl Select {
    pub fn new(id: impl Into<ElementId>, state: &Entity<SelectState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
            trigger: None,
            content: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
}

impl Styled for Select {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Select {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Select {}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let open = model.open;
        let disabled = model.disabled;
        let focus = model.focus.clone();
        let content_focus = model.content_focus.clone();
        let value = model.selected_item().map(|item| item.label.clone());
        let trigger_state = self.state.clone();
        let accessible_state = self.state.clone();
        let up_state = self.state.clone();
        let down_state = self.state.clone();
        let confirm_state = self.state.clone();
        let cancel_state = self.state.clone();
        let outside_state = self.state.clone();
        let tab_state = self.state.clone();
        let tab_prev_state = self.state.clone();

        let trigger = div()
            .id("select-trigger")
            .when(!disabled, |this| {
                this.on_click(move |_, window, cx| {
                    trigger_state.update(cx, |state, cx| {
                        // Use the rendered state: an outside-down handler may
                        // already have closed the list before this click.
                        if open {
                            state.close(window, cx);
                        } else {
                            state.open(window, cx);
                        }
                    });
                })
            })
            .children(self.trigger);
        let popup =
            Popup::new(("select-popup", self.state.entity_id()), trigger).when(open, |this| {
                this.content(
                    div()
                        .id("select-listbox")
                        .test_support()
                        .role(Role::ListBox)
                        .track_focus(&content_focus)
                        .on_mouse_down_out(move |_, window, cx| {
                            outside_state.update(cx, |state, cx| state.close(window, cx));
                        })
                        .children(self.content),
                )
            });

        self.base
            .role(Role::ComboBox)
            .aria_expanded(open)
            .when_some(self.label, |this, label| this.aria_label(label))
            .when_some(value, |this, value| this.aria_value(value))
            .key_context(CONTEXT)
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(!disabled))
            })
            .when(!disabled, |this| {
                this.on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    accessible_state.update(cx, |state, cx| {
                        if state.is_open() {
                            state.close(window, cx);
                        } else {
                            state.open(window, cx);
                        }
                    });
                })
            })
            .on_action(move |_: &SelectUp, window, cx| {
                up_state.update(cx, |state, cx| state.move_cursor(false, window, cx));
            })
            .on_action(move |_: &SelectDown, window, cx| {
                down_state.update(cx, |state, cx| state.move_cursor(true, window, cx));
            })
            .on_action(move |_: &Confirm, window, cx| {
                confirm_state.update(cx, |state, cx| state.confirm(window, cx));
            })
            .on_action(move |_: &Cancel, window, cx| {
                if !cancel_state.read(cx).is_open() {
                    cx.propagate();
                    return;
                }
                cx.stop_propagation();
                cancel_state.update(cx, |state, cx| state.close(window, cx));
            })
            .on_action(move |_: &crate::root::Tab, window, cx| {
                tab_state.update(cx, |state, cx| state.close(window, cx));
                cx.propagate();
            })
            .on_action(move |_: &crate::root::TabPrev, window, cx| {
                tab_prev_state.update(cx, |state, cx| state.close(window, cx));
                cx.propagate();
            })
            .child(popup)
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}

/// One unstyled option. Selection, hover, disabled and activation come from state.
#[derive(IntoElement)]
pub struct SelectOption {
    state: Entity<SelectState>,
    index: usize,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl SelectOption {
    /// `index` must refer to an item in this state's fixed collection.
    pub fn new(state: &Entity<SelectState>, index: usize) -> Self {
        Self {
            state: state.clone(),
            index,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for SelectOption {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for SelectOption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SelectOption {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let item = model
            .items
            .get(self.index)
            .expect("SelectOption index must identify an item");
        let disabled = model.disabled || item.disabled;
        let selected = model.selected == Some(self.index);
        let highlighted = model.highlighted == Some(self.index);
        let label = item.label.clone();
        let index = self.index;
        let hover_state = self.state.clone();
        let click_state = self.state;

        div()
            .id(("select-option", index))
            .test_support()
            .role(Role::ListBoxOption)
            .aria_label(label)
            .aria_selected(selected)
            .when(highlighted, |this| this.aria_active_descendant())
            .when(!disabled, |this| {
                this.on_hover(move |hovered, window, cx| {
                    if *hovered {
                        hover_state.update(cx, |state, cx| state.highlight(index, window, cx));
                    }
                })
                .on_click(move |_, window, cx| {
                    click_state.update(cx, |state, cx| state.commit(index, window, cx));
                })
            })
            .children(self.children)
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}
