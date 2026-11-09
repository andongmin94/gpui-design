//! Single-date selection with a month grid and independent keyboard focus.

pub use chrono::NaiveDate;
use chrono::{Datelike as _, Days, Months};
use gpui::{
    AnyElement, App, Context, ElementId, Entity, EntityId, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window,
    actions, canvas, div, prelude::FluentBuilder as _,
};

use crate::{
    Button, ButtonStyles, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{
        SelectDown, SelectFirst, SelectLast, SelectLeft, SelectPageDown, SelectPageUp, SelectRight,
        SelectUp,
    },
};

actions!(calendar, [PreviousYear, NextYear]);

const CONTEXT: &str = "Calendar";
const DAY_COUNT: usize = 42;

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
        KeyBinding::new("shift-pageup", PreviousYear, Some(CONTEXT)),
        KeyBinding::new("shift-pagedown", NextYear, Some(CONTEXT)),
    ]);
}

/// Emitted once when user activation selects a different enabled date.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalendarEvent {
    Change(NaiveDate),
}

// Compositions such as DatePicker close after every successful activation,
// including the already selected date, without duplicating Change events.
#[derive(Clone, Copy)]
pub(crate) struct CalendarActivation {
    pub(crate) date: NaiveDate,
    pub(crate) changed: bool,
}

/// Retained single selection and one roving Tab stop for a Gregorian calendar.
///
/// The grid contains six Monday-first weeks, including adjacent-month dates.
/// Keyboard navigation changes the visible month as necessary, without selecting
/// a date. Pointer/native focus preserves the current grid until activation.
/// Enter, Space, and pointer activation commit selection. Date arithmetic uses
/// Chrono's date-only type, with no clock or timezone dependency.
///
/// One state belongs to one mounted calendar in one window. Render all dates
/// returned by [`Self::days`] once, in order. The supported date range is years
/// 1 through 9999, narrowed by [`Self::bounds`] before mounting if needed.
pub struct CalendarState {
    month: NaiveDate,
    cursor: NaiveDate,
    selected: Option<NaiveDate>,
    minimum: NaiveDate,
    maximum: NaiveDate,
    disabled: bool,
    focus: [FocusHandle; DAY_COUNT],
    focus_subscriptions: Vec<Subscription>,
    reveal: Option<NaiveDate>,
}

impl CalendarState {
    /// Starts with `initial_date` focused in its month, and no selected value.
    ///
    /// # Panics
    /// `initial_date` must have a year in 1..=9999.
    pub fn new(initial_date: NaiveDate, cx: &mut Context<Self>) -> Self {
        assert!(
            (1..=9999).contains(&initial_date.year()),
            "Calendar dates must be in years 1..=9999"
        );
        Self {
            month: initial_date.with_day(1).unwrap(),
            cursor: initial_date,
            selected: None,
            minimum: NaiveDate::from_ymd_opt(1, 1, 1).unwrap(),
            maximum: NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
            disabled: false,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            focus_subscriptions: Vec::new(),
            reveal: None,
        }
    }

    /// Sets inclusive selectable bounds before mounting. Focus is clamped into
    /// the range, and an out-of-range initial selection is cleared.
    ///
    /// # Panics
    /// Bounds must be ordered and lie within years 1..=9999.
    pub fn bounds(mut self, minimum: NaiveDate, maximum: NaiveDate) -> Self {
        assert!(
            minimum <= maximum && minimum.year() >= 1 && maximum.year() <= 9999,
            "Calendar bounds must be ordered and within years 1..=9999"
        );
        self.minimum = minimum;
        self.maximum = maximum;
        self.cursor = self.cursor.clamp(minimum, maximum);
        self.month = self.cursor.with_day(1).unwrap();
        self.selected = self
            .selected
            .filter(|date| (minimum..=maximum).contains(date));
        self
    }

    /// Sets an initial selection and visible month without emitting an event.
    ///
    /// # Panics
    /// The date must lie inside the configured bounds.
    pub fn default_value(mut self, date: NaiveDate) -> Self {
        assert!(
            (self.minimum..=self.maximum).contains(&date),
            "Calendar default_value must lie inside its bounds"
        );
        self.selected = Some(date);
        self.cursor = date;
        self.month = date.with_day(1).unwrap();
        self
    }

    pub fn focused_date(&self) -> NaiveDate {
        self.cursor
    }

    pub fn selected_date(&self) -> Option<NaiveDate> {
        self.selected
    }

    /// The first day of the currently visible month.
    pub fn month(&self) -> NaiveDate {
        self.month
    }

    pub fn minimum(&self) -> NaiveDate {
        self.minimum
    }

    pub fn maximum(&self) -> NaiveDate {
        self.maximum
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_date_disabled(&self, date: NaiveDate) -> bool {
        self.disabled || !(self.minimum..=self.maximum).contains(&date)
    }

    /// All six Monday-first weeks. Adjacent dates outside configured bounds
    /// remain visible and disabled, including year 0/10000 at the outer edges.
    pub fn days(&self) -> [NaiveDate; DAY_COUNT] {
        let first = self.grid_start();
        std::array::from_fn(|index| first + Days::new(index as u64))
    }

    /// Focus handle of the current cursor. Read it when moving focus into the
    /// calendar; month navigation may change which grid slot owns that date.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus[self.index_of(self.cursor)]
    }

    pub fn can_previous_month(&self) -> bool {
        !self.disabled && self.month() > self.minimum.with_day(1).unwrap()
    }

    pub fn can_next_month(&self) -> bool {
        !self.disabled && self.month() < self.maximum.with_day(1).unwrap()
    }

    /// Displays the preceding month while retaining focus on an external
    /// navigation button. If a day owns focus, it follows the new cursor.
    pub fn previous_month(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_previous_month() {
            self.display_month(self.month - Months::new(1), window, cx);
        }
    }

    /// Displays the following month with the same focus policy as
    /// [`Self::previous_month`]. End-of-month dates clamp to the new month.
    pub fn next_month(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_next_month() {
            self.display_month(self.month + Months::new(1), window, cx);
        }
    }

    /// Replaces selection without emitting an event. A selected date also
    /// becomes the cursor and visible month. Returns false for an out-of-range
    /// date. Clearing selection preserves the cursor and visible month.
    pub fn set_selected_date(
        &mut self,
        date: Option<NaiveDate>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if date.is_some_and(|date| !(self.minimum..=self.maximum).contains(&date)) {
            return false;
        }
        let changed = self.selected != date;
        self.selected = date;
        if let Some(date) = date {
            let focused = self.focused_index(window).is_some();
            self.set_cursor(date, focused, window, cx);
        }
        if changed {
            cx.notify();
            window.refresh();
        }
        true
    }

    /// Disables activation, navigation, and normal Tab entry. An already focused
    /// day retains its focus binding until Tab moves away.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            cx.notify();
            window.refresh();
        }
    }

    fn grid_start(&self) -> NaiveDate {
        let first = self.month();
        first - Days::new(first.weekday().num_days_from_monday() as u64)
    }

    fn index_of(&self, date: NaiveDate) -> usize {
        let index = (date - self.grid_start()).num_days();
        assert!(
            (0..DAY_COUNT as i64).contains(&index),
            "CalendarDay date must belong to CalendarState::days()"
        );
        index as usize
    }

    fn focused_index(&self, window: &Window) -> Option<usize> {
        self.focus.iter().position(|focus| focus.is_focused(window))
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
                    cx.defer_in(window, move |state, window, cx| {
                        if state.focus[index].is_focused(window) {
                            let date = state.days()[index];
                            if !state.is_date_disabled(date) {
                                // Pointer focus precedes release. Keep an
                                // adjacent-month cell in place until activation
                                // so that press and release hit the same cell.
                                state.cursor = date;
                                state.reveal = Some(date);
                                cx.notify();
                                window.refresh();
                            }
                        }
                    });
                })
            })
            .collect();
    }

    fn set_cursor(
        &mut self,
        date: NaiveDate,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let date = date.clamp(self.minimum, self.maximum);
        let month = date.with_day(1).unwrap();
        let changed = self.cursor != date || self.month != month;
        self.cursor = date;
        self.month = month;
        if focus {
            if changed || !self.focus_handle().is_focused(window) {
                self.reveal = Some(date);
            }
            self.focus_handle().focus(window, cx);
        }
        if changed {
            cx.notify();
            window.refresh();
        }
    }

    fn select(&mut self, date: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_date_disabled(date) {
            return;
        }
        self.set_cursor(date, true, window, cx);
        let changed = self.selected != Some(date);
        if changed {
            self.selected = Some(date);
            cx.emit(CalendarEvent::Change(date));
            cx.notify();
            window.refresh();
        }
        cx.emit(CalendarActivation { date, changed });
    }

    fn move_days(&mut self, days: i64, window: &mut Window, cx: &mut Context<Self>) {
        let date = if days < 0 {
            self.cursor - Days::new(days.unsigned_abs())
        } else {
            self.cursor + Days::new(days as u64)
        };
        self.set_cursor(date, true, window, cx);
    }

    fn move_months(&mut self, months: i32, window: &mut Window, cx: &mut Context<Self>) {
        let date = if months < 0 {
            self.cursor
                .checked_sub_months(Months::new(months.unsigned_abs()))
        } else {
            self.cursor.checked_add_months(Months::new(months as u32))
        }
        .expect("Calendar month movement stays within Chrono's supported range");
        self.set_cursor(date, self.focused_index(window).is_some(), window, cx);
    }

    fn display_month(&mut self, month: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        let date = month
            .with_day(self.cursor.day())
            .unwrap_or_else(|| month + Months::new(1) - Days::new(1));
        self.set_cursor(date, self.focused_index(window).is_some(), window, cx);
    }

    fn handles_navigation(&self, window: &Window, cx: &mut Context<Self>) -> bool {
        if self.disabled || self.focused_index(window).is_none() {
            cx.propagate();
            false
        } else {
            true
        }
    }
}

impl EventEmitter<CalendarEvent> for CalendarState {}
impl EventEmitter<CalendarActivation> for CalendarState {}

/// An unstyled keyboard and accessibility grid. Render a header row containing
/// seven `Role::ColumnHeader` cells, followed by six `Role::Row` containers with
/// their [`CalendarDay`] children in [`CalendarState::days`] order. Header and
/// row containers use ordinary GPUI layout, for example
/// `div().id("calendar-weekdays").role(Role::Row)`. Native row/column indices are zero
/// based; the header is row 0 and date rows are 1..=6.
///
/// Place month navigation buttons outside this grid and call the state's
/// previous/next month methods. Supply an accessible name, normally the visible
/// month and year, and application-owned weekday/date labels and styles.
#[derive(IntoElement)]
pub struct Calendar {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CalendarState>,
    label: Option<SharedString>,
}

impl Calendar {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CalendarState>) -> Self {
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

impl Styled for Calendar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Calendar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for Calendar {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Calendar {}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state
            .update(cx, |state, cx| state.bind_focus(window, cx));
        let disabled = self.state.read(cx).disabled;
        let left = self.state.clone();
        let right = self.state.clone();
        let up = self.state.clone();
        let down = self.state.clone();
        let first = self.state.clone();
        let last = self.state.clone();
        let previous = self.state.clone();
        let next = self.state.clone();
        let previous_year = self.state.clone();
        let next_year = self.state;
        self.base
            .role(Role::Grid)
            .aria_row_count(7)
            .aria_column_count(7)
            .key_context(CONTEXT)
            .when_some(self.label, |this, label| this.aria_label(label))
            .on_action(move |_: &SelectLeft, window, cx| {
                left.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(-1, window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectRight, window, cx| {
                right.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(1, window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectUp, window, cx| {
                up.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(-7, window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectDown, window, cx| {
                down.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(7, window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectFirst, window, cx| {
                first.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(
                            -(state.cursor.weekday().num_days_from_monday() as i64),
                            window,
                            cx,
                        );
                    }
                });
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(
                            6 - state.cursor.weekday().num_days_from_monday() as i64,
                            window,
                            cx,
                        );
                    }
                });
            })
            .on_action(move |_: &SelectPageUp, window, cx| {
                previous.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_months(-1, window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectPageDown, window, cx| {
                next.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_months(1, window, cx);
                    }
                });
            })
            .on_action(move |_: &PreviousYear, window, cx| {
                previous_year.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_months(-12, window, cx);
                    }
                });
            })
            .on_action(move |_: &NextYear, window, cx| {
                next_year.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_months(12, window, cx);
                    }
                });
            })
            .a11y_disabled(disabled)
    }
}

fn day_id(state: EntityId, date: NaiveDate) -> ElementId {
    (ElementId::from(("calendar-day", state)), date.to_string()).into()
}

/// A date cell composed from Button. It owns selection, focus, disabled state,
/// and native grid-cell metadata; applications provide non-interactive content
/// and styles. The default accessible name is its ISO date, which callers may
/// override with a localized full date.
#[derive(IntoElement)]
pub struct CalendarDay {
    button: Button,
    state: Entity<CalendarState>,
    date: NaiveDate,
    label: Option<SharedString>,
}

impl CalendarDay {
    /// `date` must belong to the enclosing calendar state's visible `days()`.
    pub fn new(state: &Entity<CalendarState>, date: NaiveDate) -> Self {
        Self {
            button: Button::new(day_id(state.entity_id(), date)),
            state: state.clone(),
            date,
            label: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn styles(mut self, build: impl FnOnce(ButtonStyles) -> ButtonStyles) -> Self {
        self.button = self.button.styles(build);
        self
    }
}

impl Styled for CalendarDay {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}

impl ParentElement for CalendarDay {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(elements);
    }
}

impl InteractiveElement for CalendarDay {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.button.interactivity()
    }
}

impl StatefulInteractiveElement for CalendarDay {}

impl RenderOnce for CalendarDay {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let index = model.index_of(self.date);
        let disabled = model.is_date_disabled(self.date);
        let selected = model.selected == Some(self.date);
        let focus = model.focus[index].clone();
        let tab_stop = model.cursor == self.date;
        let date = self.date;
        let reveal_state = self.state.clone();
        let state = self.state;
        self.button
            .role(Role::GridCell)
            .accessibility_label(self.label.unwrap_or_else(|| date.to_string().into()))
            .aria_selected(selected)
            .aria_row_index(index / 7 + 1)
            .aria_column_index(index % 7)
            .selected(selected)
            .disabled(disabled)
            .track_focus(&focus)
            .tab_stop(tab_stop)
            .when(tab_stop, |button| {
                button.child(
                    // An unpainted absolute canvas observes this existing
                    // button's bounds without adding another focus/a11y node.
                    canvas(
                        move |bounds, window, cx| {
                            reveal_state.update(cx, |state, _| {
                                if state.reveal == Some(date) {
                                    state.reveal = None;
                                    if state.cursor == date
                                        && state.focus_handle().is_focused(window)
                                    {
                                        window.request_autoscroll(bounds);
                                    }
                                }
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            })
            .on_click(move |_, window, cx| {
                state.update(cx, |state, cx| state.select(date, window, cx));
            })
            .render(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton,
        MouseDownEvent, MouseUpEvent, Render, TestAppContext, VisualTestContext, accesskit, px,
    };

    use super::*;
    use crate::Root;

    struct Harness {
        state: Entity<CalendarState>,
        before: FocusHandle,
        after: FocusHandle,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let days = self.state.read(cx).days();
            div()
                .w(px(280.))
                .flex()
                .flex_col()
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .h(px(32.))
                        .child("Before"),
                )
                .child(
                    Calendar::new("calendar", &self.state)
                        .accessibility_label("Recording date")
                        .flex()
                        .flex_col()
                        .child(
                            div().id("weekdays").role(Role::Row).flex().children(
                                ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, label)| {
                                        div()
                                            .id(("weekday", index))
                                            .role(Role::ColumnHeader)
                                            .aria_column_index(index)
                                            .w(px(40.))
                                            .h(px(24.))
                                            .child(label)
                                    }),
                            ),
                        )
                        .children(days.chunks(7).enumerate().map(|(index, week)| {
                            div().id(("week", index)).role(Role::Row).flex().children(
                                week.iter().map(|&date| {
                                    CalendarDay::new(&self.state, date)
                                        .debug_selector(move || format!("date-{date}"))
                                        .w(px(40.))
                                        .h(px(32.))
                                        .child(date.day().to_string())
                                }),
                            )
                        })),
                )
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<NaiveDate>>>;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn harness(
        cx: &mut TestAppContext,
        initial: NaiveDate,
        bounds: Option<(NaiveDate, NaiveDate)>,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| {
                    let state = CalendarState::new(initial, cx).default_value(initial);
                    if let Some((minimum, maximum)) = bounds {
                        state.bounds(minimum, maximum)
                    } else {
                        state
                    }
                });
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    before: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
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
            cx.subscribe(&state, move |_, event: &CalendarEvent, _| {
                let CalendarEvent::Change(date) = event;
                changes.borrow_mut().push(*date);
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

    fn assert_cursor(
        cx: &mut VisualTestContext,
        view: &Entity<Harness>,
        expected: NaiveDate,
        selected: Option<NaiveDate>,
    ) {
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(state.focused_date(), expected);
            assert_eq!(state.month(), expected.with_day(1).unwrap());
            assert_eq!(state.selected_date(), selected);
            assert!(state.focus_handle().is_focused(window));
        });
    }

    #[gpui::test]
    fn navigation_crosses_months_and_leap_years_without_committing(cx: &mut TestAppContext) {
        let initial = date(2024, 1, 31);
        let (cx, view, changes) = harness(cx, initial, None);
        cx.simulate_keystrokes("tab right");
        draw(cx);
        assert_cursor(cx, &view, date(2024, 2, 1), Some(initial));
        assert!(changes.borrow().is_empty());
        press(cx, "enter");
        press(cx, "space");
        assert_eq!(*changes.borrow(), [date(2024, 2, 1)]);

        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                assert!(state.set_selected_date(Some(initial), window, cx));
            });
        });
        draw(cx);
        for (key, expected) in [
            ("pagedown", date(2024, 2, 29)),
            ("shift-pagedown", date(2025, 2, 28)),
            ("home", date(2025, 2, 24)),
            ("end", date(2025, 3, 2)),
            ("up", date(2025, 2, 23)),
            ("down", date(2025, 3, 2)),
        ] {
            press(cx, key);
            assert_cursor(cx, &view, expected, Some(initial));
        }
        assert_eq!(*changes.borrow(), [date(2024, 2, 1)]);

        let adjacent = cx.debug_bounds("date-2025-02-24").unwrap().center();
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: adjacent,
            click_count: 1,
            ..Default::default()
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).month(), date(2025, 3, 1)));
        assert_eq!(
            cx.debug_bounds("date-2025-02-24").unwrap().center(),
            adjacent
        );
        cx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: adjacent,
            click_count: 1,
            ..Default::default()
        });
        draw(cx);
        assert_cursor(cx, &view, date(2025, 2, 24), Some(date(2025, 2, 24)));
        assert_eq!(*changes.borrow(), [date(2024, 2, 1), date(2025, 2, 24)]);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        assert_cursor(cx, &view, date(2025, 2, 24), Some(date(2025, 2, 24)));

        // Month buttons keep their own focus and do not commit selection.
        cx.update(|window, cx| {
            let before = view.read(cx).before.clone();
            before.focus(window, cx);
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.next_month(window, cx));
            assert!(before.is_focused(window));
            assert_eq!(state.read(cx).month(), date(2025, 3, 1));
        });
    }

    #[gpui::test]
    fn bounds_disabled_state_and_native_grid_metadata_follow_the_same_dates(
        cx: &mut TestAppContext,
    ) {
        let minimum = date(2026, 11, 9);
        let maximum = date(2026, 11, 20);
        let (cx, view, changes) = harness(cx, minimum, Some((minimum, maximum)));
        cx.simulate_keystrokes("tab left up home");
        draw(cx);
        assert_cursor(cx, &view, minimum, Some(minimum));
        let disabled = cx.debug_bounds("date-2026-11-08").unwrap().center();
        cx.simulate_click(disabled, Modifiers::default());
        draw(cx);
        assert_cursor(cx, &view, minimum, Some(minimum));
        cx.simulate_keystrokes("end down");
        draw(cx);
        assert_cursor(cx, &view, maximum, Some(minimum));
        press(cx, "pagedown");
        assert_cursor(cx, &view, maximum, Some(minimum));
        assert!(changes.borrow().is_empty());

        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert!(!state.read(cx).can_previous_month());
            assert!(!state.read(cx).can_next_month());
            let grid = Calendar::new("native-calendar", &state)
                .accessibility_label("November 2026")
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(grid.a11y_role().unwrap());
            grid.write_a11y_info(&mut node);
            assert_eq!(node.role(), Role::Grid);
            assert_eq!(node.label(), Some("November 2026"));
            assert_eq!(node.row_count(), Some(7));
            assert_eq!(node.column_count(), Some(7));
            for (date, disabled, selected, row, column) in [
                (date(2026, 11, 8), true, false, 2, 6),
                (minimum, false, true, 3, 0),
            ] {
                let day = CalendarDay::new(&state, date)
                    .render(window, cx)
                    .into_element();
                let mut node = accesskit::Node::new(day.a11y_role().unwrap());
                day.write_a11y_info(&mut node);
                assert_eq!(node.role(), Role::GridCell);
                assert_eq!(node.label(), Some(date.to_string().as_str()));
                assert_eq!(node.is_disabled(), disabled);
                assert_eq!(node.is_selected(), Some(selected));
                assert_eq!(node.row_index(), Some(row));
                assert_eq!(node.column_index(), Some(column));
            }
            state.update(cx, |state, cx| {
                assert!(!state.set_selected_date(Some(date(2026, 11, 21)), window, cx));
                state.set_disabled(true, window, cx);
            });
        });
        draw(cx);
        press(cx, "enter");
        cx.simulate_keystrokes("left tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).before.is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                assert!(state.set_selected_date(None, window, cx));
                state.set_disabled(false, window, cx);
            });
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_cursor(cx, &view, maximum, None);
        press(cx, "space");
        assert_eq!(*changes.borrow(), [maximum]);
    }
}
