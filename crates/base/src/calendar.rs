//! Single, multiple, or range selection with independent calendar keyboard focus.

use chrono::{Datelike as _, Days, Months};
pub use chrono::{NaiveDate, Weekday};
use gpui::{
    AnyElement, App, Context, ElementId, Entity, EntityId, EventEmitter, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, KeyBinding, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window,
    actions, canvas, div, prelude::FluentBuilder as _,
};

use crate::{
    Button, ButtonStyles, Direction, TestSupportExt as _,
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

/// A calendar's selection mode and retained value.
///
/// Multiple dates are stored in ascending order without duplicates. A range
/// starts empty, then retains a start without an end until a second activation
/// completes it. Completed endpoints are inclusive and ordered; the same date
/// can be both endpoints. Activating a completed range starts a new one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalendarSelection {
    Single(Option<NaiveDate>),
    Multiple(Vec<NaiveDate>),
    Range {
        start: Option<NaiveDate>,
        end: Option<NaiveDate>,
    },
}

impl CalendarSelection {
    /// The first selected date, or None when selection is empty.
    pub fn first_date(&self) -> Option<NaiveDate> {
        match self {
            Self::Single(date) => *date,
            Self::Multiple(dates) => dates.iter().copied().min(),
            Self::Range { start, .. } => *start,
        }
    }
}

/// Emitted once when user activation changes the selected date or dates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalendarEvent {
    Change(CalendarSelection),
}

// Compositions can finish a single or completed range activation without
// duplicating Change events for an already selected single date.
#[derive(Clone)]
pub(crate) struct CalendarActivation {
    pub(crate) selection: CalendarSelection,
    pub(crate) changed: bool,
}

/// Retained selection and one roving Tab stop for a Gregorian calendar.
///
/// The grid contains six weeks, Monday-first by default, including adjacent-month dates.
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
    selection: CalendarSelection,
    minimum: NaiveDate,
    maximum: NaiveDate,
    first_weekday: Weekday,
    disabled_dates: Option<Box<dyn Fn(NaiveDate) -> bool>>,
    disabled: bool,
    direction: Direction,
    focus: [FocusHandle; DAY_COUNT],
    focus_subscriptions: Vec<Subscription>,
    reveal: Option<NaiveDate>,
}

impl CalendarState {
    /// Starts with `initial_date` focused in its month, in empty single-date mode.
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
            selection: CalendarSelection::Single(None),
            minimum: NaiveDate::from_ymd_opt(1, 1, 1).unwrap(),
            maximum: NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
            first_weekday: Weekday::Mon,
            disabled_dates: None,
            disabled: false,
            direction: Direction::Ltr,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            focus_subscriptions: Vec::new(),
            reveal: None,
        }
    }

    /// Sets inclusive selectable bounds before mounting. Focus is clamped into
    /// the range. Out-of-range single or multiple dates are removed; a range
    /// with an out-of-range endpoint is cleared.
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
        self.selection = match self.selection {
            CalendarSelection::Single(date) => {
                CalendarSelection::Single(date.filter(|date| (minimum..=maximum).contains(date)))
            }
            CalendarSelection::Multiple(dates) => CalendarSelection::Multiple(
                dates
                    .into_iter()
                    .filter(|date| (minimum..=maximum).contains(date))
                    .collect(),
            ),
            CalendarSelection::Range { start, end } => {
                if start.is_some_and(|date| !(minimum..=maximum).contains(&date))
                    || end.is_some_and(|date| !(minimum..=maximum).contains(&date))
                {
                    CalendarSelection::Range {
                        start: None,
                        end: None,
                    }
                } else {
                    CalendarSelection::Range { start, end }
                }
            }
        };
        self
    }

    /// Sets the first weekday before mounting. Day order remains chronological;
    /// use [`Self::weekdays`] for the matching application-owned column labels.
    pub fn week_starts_on(mut self, weekday: Weekday) -> Self {
        self.first_weekday = weekday;
        self
    }

    /// Prevents matching dates from being selected, before mounting. Existing
    /// single/multiple values are filtered; a range with a disabled endpoint is
    /// cleared. Disabled dates may remain inside a range.
    ///
    /// Disabled dates within the bounds retain keyboard focus so users can
    /// discover unavailable days. They cannot be activated. The predicate should
    /// return the same result for a date throughout this state's lifetime.
    pub fn disabled_dates(mut self, predicate: impl Fn(NaiveDate) -> bool + 'static) -> Self {
        self.disabled_dates = Some(Box::new(predicate));
        self.selection = match self.selection.clone() {
            CalendarSelection::Single(date) => {
                CalendarSelection::Single(date.filter(|date| self.date_is_selectable(*date)))
            }
            CalendarSelection::Multiple(dates) => CalendarSelection::Multiple(
                dates
                    .into_iter()
                    .filter(|date| self.date_is_selectable(*date))
                    .collect(),
            ),
            selection @ CalendarSelection::Range { .. } => self
                .normalized_selection(selection)
                .unwrap_or(CalendarSelection::Range {
                    start: None,
                    end: None,
                }),
        };
        self
    }

    /// Sets the selection mode and initial value without emitting an event.
    /// Multiple dates are sorted and deduplicated. The first selected date also
    /// becomes the cursor and visible month; empty selections retain navigation.
    ///
    /// # Panics
    /// Dates must be selectable within the configured bounds. Range endpoints must be
    /// ordered, and an end requires a start.
    pub fn default_selection(mut self, selection: CalendarSelection) -> Self {
        self.selection = self
            .normalized_selection(selection)
            .expect("Calendar default_selection must be ordered and selectable inside its bounds");
        if let Some(date) = self.selection.first_date() {
            self.cursor = date;
            self.month = date.with_day(1).unwrap();
        }
        self
    }

    pub fn focused_date(&self) -> NaiveDate {
        self.cursor
    }

    pub fn selection(&self) -> &CalendarSelection {
        &self.selection
    }

    /// Whether a date belongs to the current selection, including range interiors.
    pub fn is_selected(&self, date: NaiveDate) -> bool {
        match &self.selection {
            CalendarSelection::Single(selected) => *selected == Some(date),
            CalendarSelection::Multiple(dates) => dates.binary_search(&date).is_ok(),
            CalendarSelection::Range {
                start: Some(start),
                end,
            } => (*start..=end.unwrap_or(*start)).contains(&date),
            CalendarSelection::Range { start: None, .. } => false,
        }
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

    pub fn first_weekday(&self) -> Weekday {
        self.first_weekday
    }

    /// Weekday order for each chronological row returned by [`Self::days`].
    pub fn weekdays(&self) -> [Weekday; 7] {
        let mut weekday = self.first_weekday;
        std::array::from_fn(|_| {
            let current = weekday;
            weekday = weekday.succ();
            current
        })
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_date_disabled(&self, date: NaiveDate) -> bool {
        self.disabled || !self.date_is_selectable(date)
    }

    /// All six weeks in chronological order, starting on [`Self::first_weekday`].
    /// Adjacent dates outside configured bounds
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

    /// Replaces the selection mode and value without emitting an event. The first
    /// selected date becomes the cursor and visible month. Empty values preserve
    /// navigation. Invalid, disabled, or out-of-bounds endpoints/dates return false unchanged.
    /// Programmatic changes remain allowed while the calendar is disabled.
    pub fn set_selection(
        &mut self,
        selection: CalendarSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(selection) = self.normalized_selection(selection) else {
            return false;
        };
        let changed = self.selection != selection;
        self.selection = selection;
        if let Some(date) = self.selection.first_date() {
            let focused = self.focused_index(window).is_some();
            self.set_cursor(date, focused, window, cx);
        }
        if changed {
            cx.notify();
            window.refresh();
        }
        true
    }

    pub(crate) fn normalized_selection(
        &self,
        selection: CalendarSelection,
    ) -> Option<CalendarSelection> {
        let selectable = |date: &NaiveDate| self.date_is_selectable(*date);
        match selection {
            CalendarSelection::Single(date) => (!date.is_some_and(|date| !selectable(&date)))
                .then_some(CalendarSelection::Single(date)),
            CalendarSelection::Multiple(mut dates) => {
                if !dates.iter().all(selectable) {
                    return None;
                }
                dates.sort_unstable();
                dates.dedup();
                Some(CalendarSelection::Multiple(dates))
            }
            CalendarSelection::Range { start, end } => {
                if start.is_some_and(|date| !selectable(&date))
                    || end.is_some_and(|date| !selectable(&date))
                    || matches!((start, end), (None, Some(_)))
                    || matches!((start, end), (Some(start), Some(end)) if start > end)
                {
                    return None;
                }
                Some(CalendarSelection::Range { start, end })
            }
        }
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
        first - Days::new(self.weekday_offset(first) as u64)
    }

    fn weekday_offset(&self, date: NaiveDate) -> u32 {
        (date.weekday().num_days_from_monday() + 7 - self.first_weekday.num_days_from_monday()) % 7
    }

    fn date_is_selectable(&self, date: NaiveDate) -> bool {
        (self.minimum..=self.maximum).contains(&date)
            && !self
                .disabled_dates
                .as_ref()
                .is_some_and(|predicate| predicate(date))
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
                            if !state.disabled && (state.minimum..=state.maximum).contains(&date) {
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
        let selection = match &self.selection {
            CalendarSelection::Single(_) => CalendarSelection::Single(Some(date)),
            CalendarSelection::Multiple(dates) => {
                let mut dates = dates.clone();
                match dates.binary_search(&date) {
                    Ok(index) => {
                        dates.remove(index);
                    }
                    Err(index) => dates.insert(index, date),
                }
                CalendarSelection::Multiple(dates)
            }
            CalendarSelection::Range {
                start: Some(start),
                end: None,
            } => CalendarSelection::Range {
                start: Some((*start).min(date)),
                end: Some((*start).max(date)),
            },
            CalendarSelection::Range { .. } => CalendarSelection::Range {
                start: Some(date),
                end: None,
            },
        };
        let changed = self.selection != selection;
        if changed {
            self.selection = selection;
            cx.emit(CalendarEvent::Change(self.selection.clone()));
            cx.notify();
            window.refresh();
        }
        cx.emit(CalendarActivation {
            selection: self.selection.clone(),
            changed,
        });
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
/// Match header order to [`CalendarState::weekdays`]. For RTL, use
/// [`Direction::row`] on every header/date row, with mirrored header column
/// indices. Date cells resolve direction from this grid automatically.
#[derive(IntoElement)]
pub struct Calendar {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CalendarState>,
    label: Option<SharedString>,
    direction: Option<Direction>,
}

impl Calendar {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CalendarState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
            direction: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Overrides the inherited logical direction for horizontal arrows and
    /// date-cell column indices. Application-owned row layout must match.
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
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
        let direction = self.direction.unwrap_or_else(|| Direction::current(cx));
        self.state.update(cx, |state, cx| {
            state.direction = direction;
            state.bind_focus(window, cx);
        });
        let disabled = self.state.read(cx).disabled;
        let multiselectable = !matches!(
            self.state.read(cx).selection(),
            CalendarSelection::Single(_)
        );
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
                        state.move_days(if direction.is_rtl() { 1 } else { -1 }, window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectRight, window, cx| {
                right.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(if direction.is_rtl() { -1 } else { 1 }, window, cx);
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
                        state.move_days(-(state.weekday_offset(state.cursor) as i64), window, cx);
                    }
                });
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| {
                    if state.handles_navigation(window, cx) {
                        state.move_days(6 - state.weekday_offset(state.cursor) as i64, window, cx);
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
            .a11y_selection(disabled, multiselectable)
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
        let disabled_focusable =
            !model.disabled && (model.minimum..=model.maximum).contains(&self.date);
        let selected = model.is_selected(self.date);
        let focus = model.focus[index].clone();
        let tab_stop = model.cursor == self.date;
        let column = if model.direction.is_rtl() {
            6 - index % 7
        } else {
            index % 7
        };
        let date = self.date;
        let reveal_state = self.state.clone();
        let state = self.state;
        self.button
            .role(Role::GridCell)
            .accessibility_label(self.label.unwrap_or_else(|| date.to_string().into()))
            .aria_selected(selected)
            .aria_row_index(index / 7 + 1)
            .aria_column_index(column)
            .selected(selected)
            .disabled(disabled)
            .focus_when_disabled(disabled_focusable)
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
    use crate::{DirectionProvider, Root};

    struct Harness {
        state: Entity<CalendarState>,
        before: FocusHandle,
        after: FocusHandle,
        direction: Direction,
        calendar_direction: Option<Direction>,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let days = self.state.read(cx).days();
            let weekdays = self.state.read(cx).weekdays();
            let direction = self.calendar_direction.unwrap_or(self.direction);
            DirectionProvider::new(self.direction).child(
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
                            .when_some(self.calendar_direction, |calendar, direction| {
                                calendar.direction(direction)
                            })
                            .accessibility_label("Recording date")
                            .flex()
                            .flex_col()
                            .child(
                                direction
                                    .row(div().id("weekdays").role(Role::Row))
                                    .children(weekdays.into_iter().enumerate().map(
                                        |(index, label)| {
                                            div()
                                                .id(("weekday", index))
                                                .role(Role::ColumnHeader)
                                                .aria_column_index(if direction.is_rtl() {
                                                    6 - index
                                                } else {
                                                    index
                                                })
                                                .w(px(40.))
                                                .h(px(24.))
                                                .child(label.to_string())
                                        },
                                    )),
                            )
                            .children(days.chunks(7).enumerate().map(|(index, week)| {
                                direction
                                    .row(div().id(("week", index)).role(Role::Row))
                                    .children(week.iter().map(|&date| {
                                        CalendarDay::new(&self.state, date)
                                            .debug_selector(move || format!("date-{date}"))
                                            .w(px(40.))
                                            .h(px(32.))
                                            .child(date.day().to_string())
                                    }))
                            })),
                    )
                    .child(
                        Button::new("after")
                            .track_focus(&self.after)
                            .h(px(32.))
                            .child("After"),
                    ),
            )
        }
    }

    type Changes = Rc<RefCell<Vec<CalendarSelection>>>;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn harness(
        cx: &mut TestAppContext,
        initial: NaiveDate,
        bounds: Option<(NaiveDate, NaiveDate)>,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        configured_harness(cx, initial, bounds, |state| state)
    }

    fn configured_harness(
        cx: &mut TestAppContext,
        initial: NaiveDate,
        bounds: Option<(NaiveDate, NaiveDate)>,
        configure: impl FnOnce(CalendarState) -> CalendarState + 'static,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let state = cx.new(|cx| {
                    let state = CalendarState::new(initial, cx)
                        .default_selection(CalendarSelection::Single(Some(initial)));
                    let state = if let Some((minimum, maximum)) = bounds {
                        state.bounds(minimum, maximum)
                    } else {
                        state
                    };
                    configure(state)
                });
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    before: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
                    direction: Direction::Ltr,
                    calendar_direction: None,
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
                let CalendarEvent::Change(selection) = event;
                changes.borrow_mut().push(selection.clone());
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
            assert_eq!(state.selection(), &CalendarSelection::Single(selected));
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
        assert_eq!(
            *changes.borrow(),
            [CalendarSelection::Single(Some(date(2024, 2, 1)))]
        );

        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                assert!(state.set_selection(CalendarSelection::Single(Some(initial)), window, cx));
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
        assert_eq!(
            *changes.borrow(),
            [CalendarSelection::Single(Some(date(2024, 2, 1)))]
        );

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
        assert_eq!(
            *changes.borrow(),
            [date(2024, 2, 1), date(2025, 2, 24)].map(|date| CalendarSelection::Single(Some(date)))
        );
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
            assert!(!node.is_multiselectable());
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
                assert!(!state.set_selection(
                    CalendarSelection::Single(Some(date(2026, 11, 21))),
                    window,
                    cx
                ));
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
                assert!(state.set_selection(CalendarSelection::Single(None), window, cx));
                state.set_disabled(false, window, cx);
            });
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        assert_cursor(cx, &view, maximum, None);
        press(cx, "space");
        assert_eq!(
            *changes.borrow(),
            [CalendarSelection::Single(Some(maximum))]
        );
    }

    #[gpui::test]
    fn range_selection_completes_reverses_restarts_and_marks_every_inclusive_day(
        cx: &mut TestAppContext,
    ) {
        let initial = date(2024, 1, 31);
        let (cx, view, changes) =
            harness(cx, initial, Some((date(2024, 1, 20), date(2024, 3, 10))));
        let range = |start, end| CalendarSelection::Range { start, end };
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert!(state.update(cx, |state, cx| state.set_selection(
                range(None, None),
                window,
                cx
            )));
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        press(cx, "enter");
        cx.simulate_keystrokes("right right right");
        draw(cx);
        cx.update(|_, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(state.focused_date(), date(2024, 2, 3));
            assert_eq!(state.selection(), &range(Some(initial), None));
        });
        press(cx, "enter");
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let grid = Calendar::new("range-native-calendar", &state)
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(grid.a11y_role().unwrap());
            grid.write_a11y_info(&mut node);
            assert!(node.is_multiselectable());
            for (date, selected) in [
                (date(2024, 1, 30), false),
                (initial, true),
                (date(2024, 2, 1), true),
                (date(2024, 2, 2), true),
                (date(2024, 2, 3), true),
                (date(2024, 2, 4), false),
            ] {
                let day = CalendarDay::new(&state, date)
                    .render(window, cx)
                    .into_element();
                let mut node = accesskit::Node::new(day.a11y_role().unwrap());
                day.write_a11y_info(&mut node);
                assert_eq!(node.is_selected(), Some(selected));
            }
        });
        press(cx, "left");
        press(cx, "enter");
        cx.simulate_keystrokes("left left");
        press(cx, "space");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).state.read(cx).selection(),
                &range(Some(initial), Some(date(2024, 2, 2)))
            )
        });
        press(cx, "enter");
        press(cx, "space");
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(
                state.read(cx).selection(),
                &range(Some(initial), Some(initial))
            );
            for invalid in [
                range(None, Some(initial)),
                range(Some(initial), Some(date(2024, 1, 30))),
                range(Some(initial), Some(date(2024, 3, 11))),
            ] {
                assert!(!state.update(cx, |state, cx| state.set_selection(invalid, window, cx)));
            }
            assert_eq!(
                state.read(cx).selection(),
                &range(Some(initial), Some(initial))
            );
        });
        assert_eq!(
            *changes.borrow(),
            [
                range(Some(initial), None),
                range(Some(initial), Some(date(2024, 2, 3))),
                range(Some(date(2024, 2, 2)), None),
                range(Some(initial), Some(date(2024, 2, 2))),
                range(Some(initial), None),
                range(Some(initial), Some(initial)),
            ]
        );
    }

    #[gpui::test]
    fn multiple_dates_toggle_in_order_without_changing_selection_during_navigation(
        cx: &mut TestAppContext,
    ) {
        let first = date(2026, 11, 10);
        let second = date(2026, 11, 11);
        let third = date(2026, 11, 12);
        let (cx, view, changes) = harness(cx, first, Some((date(2026, 11, 9), date(2026, 11, 20))));
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                assert!(state.set_selection(
                    CalendarSelection::Multiple(vec![third, first, third]),
                    window,
                    cx
                ));
                assert_eq!(
                    state.selection(),
                    &CalendarSelection::Multiple(vec![first, third])
                );
                assert_eq!(state.selection().first_date(), Some(first));
            });
        });
        draw(cx);
        cx.simulate_keystrokes("tab right");
        draw(cx);
        assert!(changes.borrow().is_empty());
        press(cx, "enter");
        let point = cx.debug_bounds("date-2026-11-10").unwrap().center();
        cx.simulate_click(point, Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(
                state.read(cx).selection(),
                &CalendarSelection::Multiple(vec![second, third])
            );
            let grid = Calendar::new("multiple-native-calendar", &state)
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(grid.a11y_role().unwrap());
            grid.write_a11y_info(&mut node);
            assert!(node.is_multiselectable());
            for (date, selected) in [(first, false), (second, true), (third, true)] {
                let day = CalendarDay::new(&state, date)
                    .render(window, cx)
                    .into_element();
                let mut node = accesskit::Node::new(day.a11y_role().unwrap());
                day.write_a11y_info(&mut node);
                assert_eq!(node.is_selected(), Some(selected));
            }
            state.update(cx, |state, cx| {
                assert!(!state.set_selection(
                    CalendarSelection::Multiple(vec![date(2026, 11, 21)]),
                    window,
                    cx
                ));
                state.set_disabled(true, window, cx);
            });
        });
        draw(cx);
        press(cx, "space");
        assert_eq!(
            *changes.borrow(),
            [
                CalendarSelection::Multiple(vec![first, second, third]),
                CalendarSelection::Multiple(vec![second, third]),
            ]
        );
    }

    #[gpui::test]
    fn sunday_first_disabled_dates_remain_discoverable_without_activation(cx: &mut TestAppContext) {
        let initial = date(2026, 11, 11);
        let (cx, view, changes) = configured_harness(cx, initial, None, |state| {
            state
                .week_starts_on(Weekday::Sun)
                .disabled_dates(|date| matches!(date.weekday(), Weekday::Sat | Weekday::Sun))
        });
        cx.update(|_, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(state.first_weekday(), Weekday::Sun);
            assert_eq!(state.days()[0], date(2026, 11, 1));
            assert_eq!(state.days()[41], date(2026, 12, 12));
            assert_eq!(
                state.weekdays(),
                [
                    Weekday::Sun,
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                    Weekday::Sat
                ]
            );
        });
        cx.simulate_keystrokes("tab home");
        draw(cx);
        assert_cursor(cx, &view, date(2026, 11, 8), Some(initial));
        press(cx, "enter");
        press(cx, "end");
        assert_cursor(cx, &view, date(2026, 11, 14), Some(initial));
        press(cx, "space");
        let disabled = cx.debug_bounds("date-2026-11-08").unwrap().center();
        cx.simulate_click(disabled, Modifiers::default());
        draw(cx);
        assert_cursor(cx, &view, date(2026, 11, 14), Some(initial));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        assert_cursor(cx, &view, date(2026, 11, 14), Some(initial));
        assert!(changes.borrow().is_empty());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let day = CalendarDay::new(&state, date(2026, 11, 14))
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(day.a11y_role().unwrap());
            day.write_a11y_info(&mut node);
            assert!(node.is_disabled());
            assert_eq!(node.column_index(), Some(6));
            assert_eq!(node.row_index(), Some(2));
            state.update(cx, |state, cx| {
                for selection in [
                    CalendarSelection::Single(Some(date(2026, 11, 14))),
                    CalendarSelection::Multiple(vec![initial, date(2026, 11, 14)]),
                    CalendarSelection::Range {
                        start: Some(initial),
                        end: Some(date(2026, 11, 14)),
                    },
                ] {
                    assert!(!state.set_selection(selection, window, cx));
                    assert_eq!(state.selection(), &CalendarSelection::Single(Some(initial)));
                }
                let range = CalendarSelection::Range {
                    start: Some(date(2026, 11, 13)),
                    end: Some(date(2026, 11, 16)),
                };
                assert!(state.set_selection(range, window, cx));
                assert!(state.is_selected(date(2026, 11, 14)));
                assert!(state.is_selected(date(2026, 11, 15)));
            });
        });
        draw(cx);
        let enabled = cx.debug_bounds("date-2026-11-12").unwrap().center();
        cx.simulate_click(enabled, Modifiers::default());
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).state.read(cx).selection(),
                &CalendarSelection::Range {
                    start: Some(date(2026, 11, 12)),
                    end: None
                }
            )
        });
        assert_eq!(changes.borrow().len(), 1);
    }

    #[gpui::test]
    fn disabled_date_configuration_filters_retained_values(cx: &mut TestAppContext) {
        let enabled = date(2026, 11, 13);
        let disabled = date(2026, 11, 14);
        cx.update(|cx| {
            for (selection, expected) in [
                (
                    CalendarSelection::Single(Some(disabled)),
                    CalendarSelection::Single(None),
                ),
                (
                    CalendarSelection::Multiple(vec![disabled, enabled]),
                    CalendarSelection::Multiple(vec![enabled]),
                ),
                (
                    CalendarSelection::Range {
                        start: Some(enabled),
                        end: Some(disabled),
                    },
                    CalendarSelection::Range {
                        start: None,
                        end: None,
                    },
                ),
            ] {
                let state = cx.new(|cx| {
                    CalendarState::new(enabled, cx)
                        .default_selection(selection)
                        .disabled_dates(move |date| date == disabled)
                });
                assert_eq!(state.read(cx).selection(), &expected);
            }
        });
    }

    #[gpui::test]
    fn all_dates_disabled_keeps_one_tab_stop_until_whole_calendar_is_disabled(
        cx: &mut TestAppContext,
    ) {
        let initial = date(2026, 11, 11);
        let (cx, view, changes) =
            configured_harness(cx, initial, None, |state| state.disabled_dates(|_| true));
        cx.simulate_keystrokes("tab right enter");
        draw(cx);
        assert_cursor(cx, &view, date(2026, 11, 12), None);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).after.is_focused(window));
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        draw(cx);
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).before.is_focused(window)));
        assert!(changes.borrow().is_empty());
    }

    #[gpui::test]
    fn inherited_rtl_mirrors_arrows_and_native_columns_and_allows_override(
        cx: &mut TestAppContext,
    ) {
        let initial = date(2026, 11, 11);
        let (cx, view, changes) = harness(cx, initial, None);
        view.update(cx, |view, cx| {
            view.direction = Direction::Rtl;
            cx.notify();
        });
        draw(cx);
        cx.simulate_keystrokes("tab left");
        draw(cx);
        assert_cursor(cx, &view, date(2026, 11, 12), Some(initial));
        press(cx, "right");
        assert_cursor(cx, &view, initial, Some(initial));
        let wednesday = cx.debug_bounds("date-2026-11-11").unwrap();
        let thursday = cx.debug_bounds("date-2026-11-12").unwrap();
        assert!(thursday.left() < wednesday.left());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let day = CalendarDay::new(&state, initial)
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(day.a11y_role().unwrap());
            day.write_a11y_info(&mut node);
            assert_eq!(node.column_index(), Some(4));
        });
        view.update(cx, |view, cx| {
            view.calendar_direction = Some(Direction::Ltr);
            cx.notify();
        });
        draw(cx);
        press(cx, "right");
        assert_cursor(cx, &view, date(2026, 11, 12), Some(initial));
        let wednesday = cx.debug_bounds("date-2026-11-11").unwrap();
        let thursday = cx.debug_bounds("date-2026-11-12").unwrap();
        assert!(thursday.left() > wednesday.left());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            let day = CalendarDay::new(&state, initial)
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(day.a11y_role().unwrap());
            day.write_a11y_info(&mut node);
            assert_eq!(node.column_index(), Some(2));
        });
        assert!(changes.borrow().is_empty());
    }
}
