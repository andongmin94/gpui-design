//! A single, multiple, or range date picker composed from Calendar and Popover.

use gpui::{
    Anchor, AnyElement, App, AppContext as _, Context, ElementId, Entity, EventEmitter,
    FocusHandle, IntoElement, Pixels, RenderOnce, SharedString, StyleRefinement, Styled,
    Subscription, Window, prelude::FluentBuilder as _, px,
};

use crate::{
    Button, CalendarState, Popover, PopoverState, StyledExt as _,
    calendar::{CalendarActivation, CalendarSelection},
};

/// Emitted after user activation changes the selection. Single and complete-range
/// selections close the popup before publishing; partial ranges and multiple
/// selections keep the popup open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatePickerEvent {
    Change(CalendarSelection),
}

/// Coordinates one Calendar with one non-modal Popover in the same window.
///
/// Calendar exclusively owns the selected value, date arithmetic, and grid
/// navigation. Popover exclusively owns opening, dismissal, and return focus.
/// This composition resolves the current day focus immediately before opening
/// and closes on single-date or completed-range activation. Multiple selection
/// remains open until an explicit close or normal Popover dismissal.
///
/// Keep the matching DatePicker mounted while closed. Render the supplied
/// calendar only inside this picker's content, not in a second simultaneous
/// Calendar. Opening focuses the first selected date, or the retained cursor
/// when empty. Multiple selection retains its cursor if that date is selected.
pub struct DatePickerState {
    calendar: Entity<CalendarState>,
    popover: Entity<PopoverState>,
    focus: FocusHandle,
    _calendar_observation: Subscription,
    _popover_observation: Subscription,
    _activation_subscription: Subscription,
}

impl DatePickerState {
    /// Uses an existing Calendar state, including its week start, disabled-date
    /// predicate, bounds, and initial value.
    pub fn new(
        calendar: &Entity<CalendarState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let disabled = calendar.read(cx).is_disabled();
        let prepare_calendar = calendar.clone();
        let popover = cx.new(|cx| {
            let mut popover =
                PopoverState::new(cx).with_initial_focus_builder(move |window, cx| {
                    prepare_calendar.update(cx, |calendar, cx| {
                        if !matches!(calendar.selection(), CalendarSelection::Multiple(_))
                            || !calendar.is_selected(calendar.focused_date())
                        {
                            calendar.set_selection(calendar.selection().clone(), window, cx);
                        }
                        calendar.focus_handle().clone()
                    })
                });
            popover.set_disabled(disabled, window, cx);
            popover
        });
        let focus = popover.read(cx).focus_handle().clone();
        let calendar_observation = cx.observe(calendar, |_, _, cx| cx.notify());
        let popover_observation = cx.observe(&popover, |_, _, cx| cx.notify());
        let activation_subscription = cx.subscribe_in(
            calendar,
            window,
            |state, _, event: &CalendarActivation, window, cx| {
                if !state.is_open(cx) || state.is_disabled(cx) {
                    return;
                }
                // Subscribers may open another surface. Finish return focus
                // before notifying them so this popup cannot steal it later.
                if matches!(
                    event.selection,
                    CalendarSelection::Single(_) | CalendarSelection::Range { end: Some(_), .. }
                ) {
                    state.close(window, cx);
                }
                if event.changed {
                    cx.emit(DatePickerEvent::Change(event.selection.clone()));
                }
            },
        );
        Self {
            calendar: calendar.clone(),
            popover,
            focus,
            _calendar_observation: calendar_observation,
            _popover_observation: popover_observation,
            _activation_subscription: activation_subscription,
        }
    }

    /// The shared date state used to render Calendar and its day/month controls.
    pub fn calendar(&self) -> &Entity<CalendarState> {
        &self.calendar
    }

    pub fn selection<'a>(&self, cx: &'a App) -> &'a CalendarSelection {
        self.calendar.read(cx).selection()
    }

    pub fn is_open(&self, cx: &App) -> bool {
        self.popover.read(cx).is_open()
    }

    pub fn is_disabled(&self, cx: &App) -> bool {
        self.popover.read(cx).is_disabled()
    }

    /// The stable native Button trigger focus handle.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// Opens at the selection or retained cursor, using the Calendar focus policy.
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.popover.update(cx, |state, cx| state.open(window, cx));
    }

    /// Closes without changing the value. Restores the trigger while the popup
    /// owns focus; outside/Tab dismissal retains the new focus target.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.popover.update(cx, |state, cx| state.close(window, cx));
    }

    /// Disables the trigger and calendar, closing an open popup first.
    /// Use this method to coordinate disabled state rather than changing the
    /// nested Calendar independently.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.popover
            .update(cx, |state, cx| state.set_disabled(disabled, window, cx));
        self.calendar
            .update(cx, |state, cx| state.set_disabled(disabled, window, cx));
    }

    /// Replaces the selection mode and value without emitting Change, closing an
    /// open popup first. Invalid, date-disabled, or out-of-bounds selections leave both states
    /// unchanged. An empty selection preserves the Calendar's navigation date.
    pub fn set_selection(
        &mut self,
        selection: CalendarSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(selection) = self.calendar.read(cx).normalized_selection(selection) else {
            return false;
        };
        // Close while Popover still knows its original initial focus. Before
        // first paint, a different month's grid slot is not yet in its tree.
        self.close(window, cx);
        self.calendar
            .update(cx, |state, cx| state.set_selection(selection, window, cx))
    }
}

impl EventEmitter<DatePickerEvent> for DatePickerState {}

/// An unstyled Button trigger and a date-selection popup.
///
/// The application supplies formatted trigger text, the popup surface, month
/// buttons, and the Calendar rendered from [`DatePickerState::calendar`]. Keep
/// using ordinary CalendarDay children; single and complete-range activation
/// close this picker. Partial range and multiple selections remain open.
/// `accessibility_label` names the popup. Name the trigger separately through
/// its builder. Expansion, disabled state, activation, and focus belong to the
/// underlying Popover. Text parsing and trigger formatting belong to the application.
#[derive(IntoElement)]
pub struct DatePicker {
    id: ElementId,
    state: Entity<DatePickerState>,
    trigger: Button,
    content: Option<AnyElement>,
    label: Option<SharedString>,
    anchor: Anchor,
    offset: Pixels,
    style: StyleRefinement,
}

impl DatePicker {
    pub fn new(id: impl Into<ElementId>, state: &Entity<DatePickerState>) -> Self {
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

impl Styled for DatePicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DatePicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        Popover::new(self.id, &self.state.read(cx).popover)
            .trigger(|_| self.trigger)
            .when_some(self.content, |popover, content| popover.content(content))
            .when_some(self.label, |popover, label| {
                popover.accessibility_label(label)
            })
            .anchor(self.anchor)
            .offset(self.offset)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use chrono::Datelike as _;
    use gpui::{
        InteractiveElement as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
        ParentElement as _, Render, Role, StatefulInteractiveElement as _, TestAppContext,
        VisualTestContext, div,
    };

    use super::*;
    use crate::{Calendar, CalendarDay, NaiveDate, Root};

    struct Harness {
        calendar: Entity<CalendarState>,
        picker: Entity<DatePickerState>,
        before: FocusHandle,
        after: FocusHandle,
        show_picker: bool,
        redirect_change: bool,
        changes: Rc<RefCell<Vec<CalendarSelection>>>,
        _subscriptions: Vec<Subscription>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let days = self.calendar.read(cx).days();
            let calendar = Calendar::new("calendar", &self.calendar)
                .accessibility_label("Date")
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
                    div()
                        .id(("week", index))
                        .role(Role::Row)
                        .flex()
                        .children(week.iter().map(|&date| {
                            CalendarDay::new(&self.calendar, date)
                                .debug_selector(move || format!("date-{date}"))
                                .w(px(40.))
                                .h(px(28.))
                                .child(date.day().to_string())
                        }))
                }));
            div()
                .w(px(680.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    Button::new("before")
                        .debug_selector(|| "before".into())
                        .track_focus(&self.before)
                        .w(px(120.))
                        .h(px(32.))
                        .child("Before"),
                )
                .when(self.show_picker, |base| {
                    base.child(
                        DatePicker::new("picker", &self.picker)
                            .accessibility_label("Choose a date")
                            .w(px(180.))
                            .offset(px(4.))
                            .trigger(|button| {
                                button
                                    .debug_selector(|| "picker".into())
                                    .accessibility_label("Recording date")
                                    .h(px(32.))
                                    .child("Pick a date")
                            })
                            .content(div().w(px(280.)).child(calendar)),
                    )
                })
                .child(
                    Button::new("after")
                        .debug_selector(|| "after".into())
                        .track_focus(&self.after)
                        .ml(px(400.))
                        .w(px(120.))
                        .h(px(32.))
                        .child("After"),
                )
        }
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn harness(cx: &mut TestAppContext) -> (&mut VisualTestContext, Entity<Harness>) {
        configured_harness(cx, |calendar| calendar)
    }

    fn configured_harness(
        cx: &mut TestAppContext,
        configure: impl FnOnce(CalendarState) -> CalendarState + 'static,
    ) -> (&mut VisualTestContext, Entity<Harness>) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let content = cx.new(|cx| {
                let calendar = cx.new(|cx| {
                    configure(
                        CalendarState::new(date(2024, 1, 31), cx)
                            .bounds(date(2024, 1, 1), date(2026, 12, 31))
                            .default_selection(CalendarSelection::Single(Some(date(2024, 1, 31)))),
                    )
                });
                let picker = cx.new(|cx| DatePickerState::new(&calendar, window, cx));
                let subscriptions = vec![
                    cx.observe(&picker, |_, _, cx| cx.notify()),
                    cx.subscribe_in(
                        &picker,
                        window,
                        |this: &mut Harness, picker, event, window, cx| {
                            let DatePickerEvent::Change(selection) = event;
                            let finished = matches!(
                                selection,
                                CalendarSelection::Single(_)
                                    | CalendarSelection::Range { end: Some(_), .. }
                            );
                            assert_eq!(picker.read(cx).is_open(cx), !finished);
                            this.changes.borrow_mut().push(selection.clone());
                            if this.redirect_change {
                                this.after.focus(window, cx);
                            }
                        },
                    ),
                ];
                Harness {
                    calendar,
                    picker,
                    before: cx.focus_handle(),
                    after: cx.focus_handle(),
                    show_picker: true,
                    redirect_change: false,
                    changes: Rc::new(RefCell::new(Vec::new())),
                    _subscriptions: subscriptions,
                }
            });
            Root::new(content, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            view.read(cx).before.clone().focus(window, cx);
        });
        settle(cx);
        (cx, view)
    }

    fn settle(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
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
        settle(cx);
    }

    fn assert_open_at(cx: &mut VisualTestContext, view: &Entity<Harness>, date: NaiveDate) {
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.picker.read(cx).is_open(cx));
            let calendar = view.calendar.read(cx);
            assert_eq!(calendar.focused_date(), date);
            assert_eq!(calendar.month(), date.with_day(1).unwrap());
            assert!(calendar.focus_handle().is_focused(window));
        });
    }

    #[gpui::test]
    fn opening_resolves_the_selected_day_and_every_activation_finishes_the_popup(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx);
        press(cx, "tab");
        press(cx, "enter");
        assert_open_at(cx, &view, date(2024, 1, 31));
        press(cx, "pagedown");
        assert_open_at(cx, &view, date(2024, 2, 29));
        press(cx, "escape");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert_eq!(
                view.picker.read(cx).selection(cx),
                &CalendarSelection::Single(Some(date(2024, 1, 31)))
            );
            assert!(view.picker.read(cx).focus_handle().is_focused(window));
        });
        press(cx, "space");
        assert_open_at(cx, &view, date(2024, 1, 31));
        press(cx, "right");
        press(cx, "enter");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert_eq!(
                *view.changes.borrow(),
                [CalendarSelection::Single(Some(date(2024, 2, 1)))]
            );
            assert!(view.picker.read(cx).focus_handle().is_focused(window));
        });

        press(cx, "enter");
        assert_open_at(cx, &view, date(2024, 2, 1));
        press(cx, "space");
        cx.update(|_, cx| assert!(!view.read(cx).picker.read(cx).is_open(cx)));
        press(cx, "enter");
        let point = cx.debug_bounds("date-2024-02-01").unwrap().center();
        cx.simulate_click(point, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert_eq!(
                *view.changes.borrow(),
                [CalendarSelection::Single(Some(date(2024, 2, 1)))]
            );
        });

        view.update(cx, |view, _| view.redirect_change = true);
        press(cx, "enter");
        press(cx, "right");
        press(cx, "enter");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert_eq!(
                *view.changes.borrow(),
                [date(2024, 2, 1), date(2024, 2, 2)]
                    .map(|date| CalendarSelection::Single(Some(date)))
            );
            assert!(
                view.after.is_focused(window),
                "Change subscribers retain their new focus"
            );
        });
    }

    #[gpui::test]
    fn dismissal_disabled_and_unselected_reopening_preserve_their_owners(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx);
        // Resolve and return focus even before the Calendar has ever mounted.
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| {
                picker.open(window, cx);
                picker.close(window, cx);
                assert!(picker.focus_handle().is_focused(window));
                picker.open(window, cx);
                assert!(picker.set_selection(
                    CalendarSelection::Single(Some(date(2024, 2, 20))),
                    window,
                    cx
                ));
                assert!(!picker.is_open(cx));
                assert!(picker.focus_handle().is_focused(window));
            });
        });
        settle(cx);
        press(cx, "enter");
        press(cx, "tab");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert!(view.after.is_focused(window));
        });
        press(cx, "shift-tab");
        press(cx, "space");
        let before = cx.debug_bounds("before").unwrap().center();
        cx.simulate_click(before, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert!(view.before.is_focused(window));
            assert!(view.changes.borrow().is_empty());
        });

        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| {
                assert!(picker.set_selection(
                    CalendarSelection::Single(Some(date(2024, 2, 20))),
                    window,
                    cx
                ));
                assert!(!picker.set_selection(
                    CalendarSelection::Single(Some(date(2023, 12, 31))),
                    window,
                    cx
                ));
                assert_eq!(
                    picker.selection(cx),
                    &CalendarSelection::Single(Some(date(2024, 2, 20)))
                );
                assert!(picker.set_selection(CalendarSelection::Single(None), window, cx));
                picker.open(window, cx);
            });
        });
        settle(cx);
        assert_open_at(cx, &view, date(2024, 2, 20));
        press(cx, "pagedown");
        press(cx, "escape");
        press(cx, "enter");
        assert_open_at(cx, &view, date(2024, 3, 20));
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| picker.set_disabled(true, window, cx));
            assert!(picker.read(cx).is_disabled(cx));
            assert!(!picker.read(cx).is_open(cx));
            assert!(picker.read(cx).focus_handle().is_focused(window));
            assert!(view.read(cx).calendar.read(cx).is_disabled());
        });
        settle(cx);
        press(cx, "enter");
        press(cx, "tab");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert!(view.after.is_focused(window));
            assert!(view.changes.borrow().is_empty());
        });

        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| {
                picker.set_disabled(false, window, cx);
                picker.open(window, cx);
            });
        });
        settle(cx);
        view.update(cx, |view, cx| {
            view.show_picker = false;
            cx.notify();
        });
        settle(cx);
        cx.update(|_, cx| assert!(!view.read(cx).picker.read(cx).is_open(cx)));
    }

    #[gpui::test]
    fn range_picker_retains_partial_selection_and_closes_only_when_complete(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx);
        let start = date(2024, 1, 31);
        let end = date(2024, 2, 2);
        let range = |start, end| CalendarSelection::Range { start, end };
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| {
                assert!(picker.set_selection(range(None, None), window, cx));
                picker.open(window, cx);
            });
        });
        settle(cx);
        press(cx, "enter");
        assert_open_at(cx, &view, start);
        press(cx, "right");
        press(cx, "escape");
        press(cx, "enter");
        assert_open_at(cx, &view, start);
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            assert!(!picker.update(cx, |picker, cx| {
                picker.set_selection(range(None, Some(end)), window, cx)
            }));
            assert!(picker.read(cx).is_open(cx));
            assert_eq!(picker.read(cx).selection(cx), &range(Some(start), None));
        });
        press(cx, "right");
        press(cx, "right");
        press(cx, "space");
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.read(cx);
            assert!(!picker.is_open(cx));
            assert!(picker.focus_handle().is_focused(window));
            assert_eq!(picker.selection(cx), &range(Some(start), Some(end)));
        });
        press(cx, "enter");
        assert_open_at(cx, &view, start);
        press(cx, "enter");
        assert_open_at(cx, &view, start);
        press(cx, "space");
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            assert!(!picker.read(cx).is_open(cx));
            assert_eq!(
                picker.read(cx).selection(cx),
                &range(Some(start), Some(start))
            );
            assert_eq!(
                *view.read(cx).changes.borrow(),
                [
                    range(Some(start), None),
                    range(Some(start), Some(end)),
                    range(Some(start), None),
                    range(Some(start), Some(start)),
                ]
            );
            picker.update(cx, |picker, cx| {
                picker.open(window, cx);
                assert!(picker.set_selection(range(Some(start), Some(end)), window, cx));
                assert!(!picker.is_open(cx));
                assert!(picker.focus_handle().is_focused(window));
            });
        });
    }

    #[gpui::test]
    fn multiple_picker_keeps_toggling_until_dismissal_and_reopens_at_selected_focus(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx);
        let first = date(2024, 1, 31);
        let second = date(2024, 2, 1);
        let third = date(2024, 2, 2);
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| {
                assert!(picker.set_selection(CalendarSelection::Multiple(vec![first]), window, cx));
                picker.open(window, cx);
            });
        });
        settle(cx);
        press(cx, "right");
        press(cx, "enter");
        assert_open_at(cx, &view, second);
        press(cx, "left");
        press(cx, "space");
        assert_open_at(cx, &view, first);
        press(cx, "escape");
        press(cx, "enter");
        assert_open_at(cx, &view, second);
        press(cx, "right");
        press(cx, "enter");
        press(cx, "escape");
        press(cx, "enter");
        assert_open_at(cx, &view, third);
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            picker.update(cx, |picker, cx| picker.set_disabled(true, window, cx));
            assert!(!picker.read(cx).is_open(cx));
            assert_eq!(
                picker.read(cx).selection(cx),
                &CalendarSelection::Multiple(vec![second, third])
            );
            assert_eq!(
                *view.read(cx).changes.borrow(),
                [
                    CalendarSelection::Multiple(vec![first, second]),
                    CalendarSelection::Multiple(vec![second]),
                    CalendarSelection::Multiple(vec![second, third]),
                ]
            );
        });
        settle(cx);
        press(cx, "space");
        cx.update(|_, cx| assert!(!view.read(cx).picker.read(cx).is_open(cx)));
    }

    #[gpui::test]
    fn disabled_date_activation_and_programmatic_rejection_keep_the_popup_open(
        cx: &mut TestAppContext,
    ) {
        let disabled = date(2024, 2, 1);
        let (cx, view) = configured_harness(cx, move |calendar| {
            calendar.disabled_dates(move |date| date == disabled)
        });
        press(cx, "tab");
        press(cx, "enter");
        press(cx, "right");
        assert_open_at(cx, &view, disabled);
        press(cx, "enter");
        assert_open_at(cx, &view, disabled);
        cx.update(|window, cx| {
            let picker = view.read(cx).picker.clone();
            assert!(!picker.update(cx, |picker, cx| picker.set_selection(
                CalendarSelection::Single(Some(disabled)),
                window,
                cx
            )));
            assert!(view.read(cx).changes.borrow().is_empty());
        });
        settle(cx);
        assert_open_at(cx, &view, disabled);
        press(cx, "right");
        press(cx, "enter");
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.picker.read(cx).is_open(cx));
            assert!(view.picker.read(cx).focus_handle().is_focused(window));
            assert_eq!(
                *view.changes.borrow(),
                [CalendarSelection::Single(Some(date(2024, 2, 2)))]
            );
        });
    }
}
