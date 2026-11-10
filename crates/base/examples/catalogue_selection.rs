//! Date ranges, independent dates, and grouped recording targets in one local plan.

use chrono::Datelike as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Role,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_design::{
    Button, Calendar, CalendarDay, CalendarSelection, CalendarState, Combobox, ComboboxGroup,
    ComboboxList, ComboboxOption, ComboboxState, DatePicker, DatePickerState, InputEditorStyle,
    NaiveDate, SearchItem,
};

pub(crate) struct SelectionCatalogue {
    range: Entity<CalendarState>,
    picker: Entity<DatePickerState>,
    availability: Entity<CalendarState>,
    targets: Entity<ComboboxState>,
    extended_targets: bool,
    _subscriptions: Vec<Subscription>,
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
}

impl SelectionCatalogue {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let range = cx.new(|cx| {
            CalendarState::new(date(11), cx)
                .bounds(date(1), NaiveDate::from_ymd_opt(2027, 12, 31).unwrap())
                .default_selection(CalendarSelection::Range {
                    start: None,
                    end: None,
                })
        });
        let picker = cx.new(|cx| DatePickerState::new(&range, window, cx));
        let availability = cx.new(|cx| {
            CalendarState::new(date(11), cx)
                .bounds(date(1), NaiveDate::from_ymd_opt(2027, 12, 31).unwrap())
                .default_selection(CalendarSelection::Multiple(vec![date(11), date(13)]))
        });
        let targets = cx.new(|cx| {
            let mut state =
                ComboboxState::new(Self::target_items(false), window, cx).multiple(true);
            state.set_selected_values(["screen"], window, cx);
            state
        });
        targets.read(cx).input().clone().update(cx, |input, _| {
            input.set_editor_style(InputEditorStyle {
                foreground: rgb(0xf1f5f9).into(),
                muted_foreground: rgb(0x94a3b8).into(),
                selection: rgb(0x284568).into(),
                caret: rgb(0x93c5fd).into(),
                ..Default::default()
            });
        });
        let subscriptions = vec![
            cx.observe(&range, |_, _, cx| cx.notify()),
            cx.observe(&picker, |_, _, cx| cx.notify()),
            cx.observe(&availability, |_, _, cx| cx.notify()),
            cx.observe(&targets, |_, _, cx| cx.notify()),
        ];
        Self {
            range,
            picker,
            availability,
            targets,
            extended_targets: false,
            _subscriptions: subscriptions,
        }
    }

    fn target_items(extended: bool) -> Vec<SearchItem> {
        let mut items = vec![
            SearchItem::new("screen", "Screen capture").group("Local"),
            SearchItem::new("audio", "Microphone audio").group("Local"),
            SearchItem::new("cloud", "Cloud archive").group("Online"),
            SearchItem::new("live", "Live broadcast (unavailable)")
                .group("Online")
                .disabled(true),
        ];
        if extended {
            items.push(SearchItem::new("backup", "Backup archive").group("Online"));
        }
        items
    }

    fn button(button: Button) -> Button {
        button
            .px_3()
            .py_2()
            .rounded_md()
            .bg(rgb(0x1e293b))
            .hover(|style| style.bg(rgb(0x334155)))
            .focus_visible(|style| style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220)))
            .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
    }

    fn calendar_panel(id: &'static str, state: &Entity<CalendarState>, cx: &App) -> AnyElement {
        let model = state.read(cx);
        let month = model.month();
        let previous = state.clone();
        let next = state.clone();
        let navigation = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                Self::button(Button::new((ElementId::from(id), "previous")))
                    .accessibility_label("Previous month")
                    .disabled(!model.can_previous_month())
                    .child("‹")
                    .on_click(move |_, window, cx| {
                        previous.update(cx, |state, cx| state.previous_month(window, cx));
                    }),
            )
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(month.format("%B %Y").to_string()),
            )
            .child(
                Self::button(Button::new((ElementId::from(id), "next")))
                    .accessibility_label("Next month")
                    .disabled(!model.can_next_month())
                    .child("›")
                    .on_click(move |_, window, cx| {
                        next.update(cx, |state, cx| state.next_month(window, cx));
                    }),
            );
        let grid = Calendar::new(id, state)
            .accessibility_label(format!("{id}, {}", month.format("%B %Y")))
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .id((ElementId::from(id), "weekdays"))
                    .role(Role::Row)
                    .aria_row_index(0)
                    .flex()
                    .children(
                        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                            .into_iter()
                            .enumerate()
                            .map(|(column, day)| {
                                div()
                                    .id(("weekday", column))
                                    .role(Role::ColumnHeader)
                                    .aria_column_index(column)
                                    .aria_label(day)
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .text_center()
                                    .text_color(rgb(0x94a3b8))
                                    .child(day)
                            }),
                    ),
            )
            .children(model.days().chunks(7).enumerate().map(|(row, week)| {
                div()
                    .id((id, row))
                    .role(Role::Row)
                    .aria_row_index(row + 1)
                    .flex()
                    .children(week.iter().map(|&day| {
                        CalendarDay::new(state, day)
                            .accessibility_label(day.format("%A, %B %-d, %Y").to_string())
                            .flex_1()
                            .min_w_0()
                            .h(px(36.))
                            .border_1()
                            .rounded_md()
                            .border_color(rgb(0x0f172a))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(if day.month() == month.month() {
                                0xf1f5f9
                            } else {
                                0x94a3b8
                            }))
                            .hover(|style| style.bg(rgb(0x334155)))
                            .focus_visible(|style| style.border_color(rgb(0x93c5fd)))
                            .styles(|styles| styles.disabled(|style| style.opacity(0.35)))
                            .when(model.is_selected(day), |this| {
                                this.bg(rgb(0x1d4ed8)).text_color(rgb(0xffffff))
                            })
                            .child(day.day().to_string())
                    }))
            }));
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(navigation)
            .child(grid)
            .into_any_element()
    }

    fn range_label(selection: &CalendarSelection) -> String {
        match selection {
            CalendarSelection::Range {
                start: Some(start),
                end: Some(end),
            } => format!("{} – {}", start.format("%b %-d"), end.format("%b %-d, %Y")),
            CalendarSelection::Range {
                start: Some(start),
                end: None,
            } => format!("{} → Choose an end date", start.format("%b %-d")),
            _ => "Choose recording dates".into(),
        }
    }

    fn render_targets(&self, cx: &mut Context<Self>) -> AnyElement {
        let model = self.targets.read(cx);
        let groups = model
            .visible_groups(cx)
            .into_iter()
            .map(|label| {
                let indices = model.visible_group_indices(Some(label.as_ref()), cx);
                ComboboxGroup::new(
                    (ElementId::from("target-group"), label.clone()),
                    &self.targets,
                    label,
                )
                .heading(|heading| heading.text_sm().text_color(rgb(0x94a3b8)).px_3().py_2())
                .children(indices.into_iter().map(|index| {
                    let item = &model.items(cx)[index];
                    ComboboxOption::new(&self.targets, index)
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .when(model.highlighted_index(cx) == Some(index), |this| {
                            this.bg(rgb(0x334155))
                        })
                        .when(item.is_disabled(), |this| this.opacity(0.45))
                        .child(format!(
                            "{} {}",
                            if model.is_selected(index) {
                                "✓"
                            } else {
                                "○"
                            },
                            item.label()
                        ))
                }))
            })
            .collect::<Vec<_>>();
        let list = ComboboxList::new("recording-target-options", &self.targets)
            .accessibility_label("Recording targets")
            .max_h(px(240.))
            .p_1()
            .children(groups);
        let chips = model
            .selected_items(cx)
            .map(|item| {
                let value = item.value().clone();
                Self::button(Button::new((
                    ElementId::from("remove-target"),
                    value.clone(),
                )))
                .accessibility_label(format!("Remove {}", item.label()))
                .disabled(model.is_disabled())
                .child(format!("{} ×", item.label()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    let focus = this.targets.read(cx).focus_handle().clone();
                    focus.focus(window, cx);
                    this.targets.update(cx, |state, cx| {
                        state.remove_value(value.as_ref(), window, cx);
                    });
                }))
            })
            .collect::<Vec<_>>();
        div().w_full().flex().flex_col().gap_3()
            .child(div().id("recording-targets-title").role(Role::Heading).aria_level(3)
                .aria_label("Recording targets").font_weight(FontWeight::SEMIBOLD).child("Recording targets"))
            .child("Search and select several targets. Enter toggles a result and keeps the suggestions open; Escape ends the query.")
            .child(Combobox::new("recording-targets", &self.targets)
                .input(|input| input.accessibility_label("Search recording targets")
                    .placeholder("Add a recording target").h(px(40.)).px_3().border_1()
                    .border_color(rgb(0x64748b)).rounded_md().bg(rgb(0x0b1220))
                    .styles(|styles| styles.focused(|style| style.border_color(rgb(0x93c5fd)))))
                .content(div().w(px(320.)).p_2().rounded_md().border_1().border_color(rgb(0x475569))
                    .bg(rgb(0x0f172a)).text_color(rgb(0xf1f5f9)).child(list)
                    .when(model.visible_indices(cx).is_empty(), |this| this.child("No matching targets"))))
            .child(div().id("selected-recording-targets").role(Role::Group)
                .aria_label("Selected recording targets").flex().flex_wrap().gap_2().children(chips))
            .child(div().flex().flex_wrap().gap_2()
                .child(Self::button(Button::new("clear-recording-targets"))
                    .disabled(model.selected_indices().is_empty() || model.is_disabled()).child("Clear targets")
                    .on_click(cx.listener(|this, _, window, cx| {
                        let focus = this.targets.read(cx).focus_handle().clone();
                        focus.focus(window, cx);
                        this.targets.update(cx, |state, cx| { state.clear(window, cx); });
                    })))
                .child(Self::button(Button::new("extend-recording-targets"))
                    .child(if self.extended_targets { "Hide backup target" } else { "Show backup target" })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.extended_targets = !this.extended_targets;
                        let items = Self::target_items(this.extended_targets);
                        this.targets.update(cx, |state, cx| state.set_items(items, window, cx));
                        cx.notify();
                    }))))
            .into_any_element()
    }
}

impl Render for SelectionCatalogue {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let range = Self::range_label(self.picker.read(cx).selection(cx));
        let selected_days = match self.availability.read(cx).selection() {
            CalendarSelection::Multiple(dates) => dates
                .iter()
                .map(|date| date.format("%b %-d").to_string())
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        };
        div().w_full().min_w_0().flex().flex_col().gap_4().p_4().rounded_lg()
            .border_1().border_color(rgb(0x334155)).bg(rgb(0x0f172a))
            .child(div().id("selection-title").role(Role::Heading).aria_level(2)
                .aria_label("Plan a recording").text_lg().font_weight(FontWeight::BOLD).child("Plan a recording"))
            .child("Choose a recording date range, independent available days, and output targets. This plan stays in local application state.")
            .child(DatePicker::new("recording-date-range", &self.picker)
                .accessibility_label("Recording date range")
                .trigger(|button| Self::button(button).accessibility_label(format!("Recording date range: {range}")).child(range))
                .content(div().w(px(328.)).max_w((window.viewport_size().width - px(16.)).max(px(0.)))
                    .p_3().rounded_md().border_1().border_color(rgb(0x475569)).bg(rgb(0x0f172a)).text_color(rgb(0xf1f5f9))
                    .child(Self::calendar_panel("recording-range-calendar", &self.range, cx))))
            .child(Self::button(Button::new("recording-week-preset")).child("Use Nov 11–13")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.picker.update(cx, |state, cx| { state.set_selection(CalendarSelection::Range { start: Some(date(11)), end: Some(date(13)) }, window, cx); });
                })))
            .child(div().id("availability-title").role(Role::Heading).aria_level(3)
                .aria_label("Available recording days").font_weight(FontWeight::SEMIBOLD).child("Available recording days"))
            .child(div().max_w(px(360.)).w_full().child(Self::calendar_panel("availability-calendar", &self.availability, cx)))
            .child(if selected_days.is_empty() { "No available dates selected".to_string() } else { format!("Available: {selected_days}") })
            .child(self.render_targets(cx))
    }
}
