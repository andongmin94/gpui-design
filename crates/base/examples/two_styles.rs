//! The same headless controls with two application-owned presentations.
//! Run with `compact` (the default) or `settings`.

use chrono::Datelike as _;
use gpui::{
    AppContext as _, Axis, Context, Entity, FocusHandle, Focusable as _, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Rems, Render, Role,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    div, prelude::FluentBuilder as _, px, relative, rems, rgb,
};
use gpui_design::{
    Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionState, AlertDialog, Button,
    Calendar, CalendarDay, CalendarState, Checkbox, CheckboxState, Collapsible, CollapsibleContent,
    CollapsibleState, CollapsibleTrigger, Combobox, ComboboxList, ComboboxOption, ComboboxState,
    Command, CommandEvent, CommandInput, CommandList, CommandOption, CommandState, Dialog,
    DialogBackdrop, DialogClose, DialogDescription, DialogPopup, DialogState, DialogTitle,
    DropdownMenu, DropdownMenuEvent, DropdownMenuItem, DropdownMenuOption, DropdownMenuState,
    Field, Input, InputBase, NaiveDate, Popover, PopoverState, Progress, RadioGroup,
    RadioGroupState, RadioItem, RadioOption, ScrollArea, ScrollbarAxis, ScrollbarMode, SearchItem,
    Select, SelectItem, SelectOption, SelectState, Separator, Slider, SliderState, Switch, Tab,
    TabItem, TabList, TabPanel, Table, TableCaption, TableCell, TableColumn, TableRow, TabsState,
    TestTextExt as _, Textarea, TextareaState, Toggle, ToggleGroup, ToggleGroupState, ToggleItem,
    ToggleOption, Tooltip, TooltipState,
    input::{InputAccessibility, InputEditorStyle, InputState},
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Appearance {
    Compact,
    Settings,
}

// These tokens belong to this consumer, not the primitive library.
#[derive(Clone, Copy)]
struct Palette {
    canvas: Hsla,
    surface: Hsla,
    field: Hsla,
    foreground: Hsla,
    muted: Hsla,
    border: Hsla,
    accent: Hsla,
    accent_foreground: Hsla,
    highlight: Hsla,
    error: Hsla,
    padding: Rems,
    gap: Rems,
    control_height: Rems,
    font_size: Rems,
    radius: Pixels,
    dialog_width: Rems,
}

impl Appearance {
    fn palette(self) -> Palette {
        match self {
            Self::Compact => Palette {
                canvas: rgb(0x10141c).into(),
                surface: rgb(0x1a2130).into(),
                field: rgb(0x121a28).into(),
                foreground: rgb(0xf1f5f9).into(),
                muted: rgb(0xa9b6c8).into(),
                border: rgb(0x445269).into(),
                accent: rgb(0x93c5fd).into(),
                accent_foreground: rgb(0x10223d).into(),
                highlight: rgb(0x2b4160).into(),
                error: rgb(0xfca5a5).into(),
                padding: rems(1.),
                gap: rems(0.75),
                control_height: rems(1.875),
                font_size: rems(0.8125),
                radius: px(4.),
                dialog_width: rems(22.),
            },
            Self::Settings => Palette {
                canvas: rgb(0xf3f2ee).into(),
                surface: rgb(0xffffff).into(),
                field: rgb(0xfafaf8).into(),
                foreground: rgb(0x243328).into(),
                muted: rgb(0x606c63).into(),
                border: rgb(0xa6b2a7).into(),
                accent: rgb(0x286447).into(),
                accent_foreground: rgb(0xffffff).into(),
                highlight: rgb(0xe1eee3).into(),
                error: rgb(0xb42318).into(),
                padding: rems(1.75),
                gap: rems(1.25),
                control_height: rems(2.625),
                font_size: rems(0.9375),
                radius: px(10.),
                dialog_width: rems(28.),
            },
        }
    }
}

pub(crate) struct Demo {
    appearance: Appearance,
    pub(crate) input: Entity<InputState>,
    pub(crate) notes: Entity<TextareaState>,
    pub(crate) preset: Entity<ComboboxState>,
    pub(crate) edit_hint: Entity<TooltipState>,
    fields_scroll: ScrollHandle,
    pub(crate) calendar: Entity<CalendarState>,
    pub(crate) schedule_scroll: ScrollHandle,
    pub(crate) commands: Entity<CommandState>,
    pub(crate) command_dialog: Entity<DialogState>,
    pub(crate) dialog: Entity<DialogState>,
    pub(crate) select: Entity<SelectState>,
    pub(crate) stroke: Entity<RadioGroupState>,
    pub(crate) tabs: Entity<TabsState>,
    pub(crate) help: Entity<PopoverState>,
    pub(crate) actions: Entity<DropdownMenuState>,
    pub(crate) accordion: Entity<AccordionState>,
    pub(crate) preview: Entity<SliderState>,
    pub(crate) preview_options: Entity<CollapsibleState>,
    pub(crate) reset_dialog: Entity<DialogState>,
    pub(crate) reset_cancel_focus: FocusHandle,
    pub(crate) formatting: Entity<ToggleGroupState>,
    pub(crate) preview_indeterminate: bool,
    pub(crate) show_cursor: bool,
    pub(crate) keep_toolbar: bool,
    _subscriptions: Vec<Subscription>,
}

impl Demo {
    pub(crate) fn new(appearance: Appearance, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let palette = appearance.palette();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder("Enter a session name")
                .default_value("MiniCast");
            state.set_editor_style(Self::input_style(palette));
            state.set_accessibility(
                InputAccessibility::new(Some(Role::TextInput))
                    .with_placeholder(Some("Enter a session name".into())),
            );
            state
        });
        let input_focus = input.focus_handle(cx);
        let dialog = cx.new(|cx| DialogState::new(cx).with_initial_focus(&input_focus));
        let notes = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .rows(3)
                .placeholder("Add session notes");
            state.set_editor_style(Self::input_style(palette));
            state.set_accessibility(
                InputAccessibility::new(Some(Role::MultilineTextInput))
                    .with_placeholder(Some("Add session notes".into())),
            );
            state
        });
        let commands = cx.new(|cx| {
            CommandState::new(
                [
                    SearchItem::new("edit", "Edit session").keywords(["settings", "설정"]),
                    SearchItem::new("share", "Share (unavailable)").disabled(true),
                    SearchItem::new("cursor", "Toggle cursor").keywords(["pointer"]),
                    SearchItem::new("toolbar", "Toggle toolbar pin").keywords(["pin"]),
                    SearchItem::new("reset", "Reset preview").keywords(["progress"]),
                ],
                window,
                cx,
            )
        });
        commands.read(cx).input().clone().update(cx, |state, _| {
            state.set_editor_style(Self::input_style(palette));
        });
        let command_focus = commands.read(cx).focus_handle().clone();
        let command_dialog = cx.new(|cx| DialogState::new(cx).with_initial_focus(&command_focus));
        let preset = cx.new(|cx| {
            let mut state = ComboboxState::new(
                [
                    SearchItem::new("balanced", "Balanced").keywords(["default", "균형"]),
                    SearchItem::new("broadcast", "Broadcast (unavailable)").disabled(true),
                    SearchItem::new("presentation", "Presentation").keywords(["slides", "발표"]),
                    SearchItem::new("quality", "High quality").keywords(["detail", "고화질"]),
                ],
                window,
                cx,
            );
            state.set_selected_value(Some("balanced"), window, cx);
            state
        });
        preset.read(cx).input().clone().update(cx, |state, _| {
            state.set_editor_style(Self::input_style(palette));
        });
        let trigger_focus = cx.focus_handle().tab_stop(true);
        let edit_hint = cx.new(|cx| TooltipState::new(&trigger_focus, window, cx));
        let select = cx.new(|cx| {
            SelectState::new(
                [
                    SelectItem::new("pen", "Pen"),
                    SelectItem::new("laser", "Laser (unavailable)").disabled(true),
                    SelectItem::new("rectangle", "Rectangle"),
                    SelectItem::new("arrow", "Arrow"),
                ],
                cx,
            )
            .default_value("pen")
        });
        let stroke = cx.new(|cx| {
            RadioGroupState::new(
                [
                    RadioItem::new("fine", "Fine"),
                    RadioItem::new("medium", "Medium (unavailable)").disabled(true),
                    RadioItem::new("bold", "Bold"),
                ],
                cx,
            )
            .default_value("fine")
        });
        let tabs = cx.new(|cx| {
            TabsState::new(
                [
                    TabItem::new("preferences", "Preferences"),
                    TabItem::new("sync", "Sync (unavailable)").disabled(true),
                    TabItem::new("help", "Help"),
                    TabItem::new("schedule", "Schedule"),
                ],
                cx,
            )
            .default_value("preferences")
        });
        let calendar = cx.new(|cx| {
            let initial = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
            CalendarState::new(initial, cx)
                .bounds(
                    NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2027, 12, 31).unwrap(),
                )
                .default_value(initial)
        });
        let help = cx.new(PopoverState::new);
        let actions = cx.new(|cx| {
            DropdownMenuState::new(
                [
                    DropdownMenuItem::new("edit", "Edit session"),
                    DropdownMenuItem::new("share", "Share (unavailable)").disabled(true),
                    DropdownMenuItem::new("cursor", "Toggle cursor"),
                    DropdownMenuItem::new("toolbar", "Toggle toolbar pin"),
                    DropdownMenuItem::new("reset", "Reset preview"),
                    DropdownMenuItem::new("commands", "Search commands"),
                ],
                cx,
            )
        });
        let accordion = cx.new(|cx| {
            AccordionState::new(
                [
                    AccordionItem::new("keyboard", "Keyboard shortcuts"),
                    AccordionItem::new("sync", "Sync help (unavailable)").disabled(true),
                    AccordionItem::new("state", "State ownership"),
                ],
                cx,
            )
            .default_open("keyboard")
        });
        let preview = cx.new(|cx| SliderState::new(0.0..=100.0, 5.0, cx).default_value(40.0));
        let preview_options = cx.new(CollapsibleState::new);
        let reset_cancel_focus = cx.focus_handle();
        let reset_dialog =
            cx.new(|cx| DialogState::new(cx).with_initial_focus(&reset_cancel_focus));
        let formatting = cx.new(|cx| {
            ToggleGroupState::new(
                [
                    ToggleItem::new("bold", "Bold"),
                    ToggleItem::new("underline", "Underline (unavailable)").disabled(true),
                    ToggleItem::new("italic", "Italic"),
                ],
                cx,
            )
            .default_values(["bold"])
        });
        // The form summary and its presentation read these retained states.
        let subscriptions = vec![
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.observe(&notes, |_, _, cx| cx.notify()),
            cx.observe(&preset, |_, _, cx| cx.notify()),
            cx.observe(&edit_hint, |_, _, cx| cx.notify()),
            cx.observe(&commands, |_, _, cx| cx.notify()),
            cx.observe(&command_dialog, |_, _, cx| cx.notify()),
            cx.observe(&select, |_, _, cx| cx.notify()),
            cx.observe(&stroke, |_, _, cx| cx.notify()),
            cx.observe(&tabs, |_, _, cx| cx.notify()),
            cx.observe(&calendar, |_, _, cx| cx.notify()),
            cx.observe(&help, |_, _, cx| cx.notify()),
            cx.observe(&actions, |_, _, cx| cx.notify()),
            cx.observe(&accordion, |_, _, cx| cx.notify()),
            cx.observe(&preview, |_, _, cx| cx.notify()),
            cx.observe(&preview_options, |_, _, cx| cx.notify()),
            cx.observe(&reset_dialog, |_, _, cx| cx.notify()),
            cx.observe(&formatting, |_, _, cx| cx.notify()),
            cx.observe(&dialog, |_, _, cx| cx.notify()),
            cx.subscribe_in(&actions, window, |this, _, event, window, cx| {
                let DropdownMenuEvent::Action(value) = event;
                this.perform_action(value, window, cx);
            }),
            cx.subscribe_in(&commands, window, |this, _, event, window, cx| {
                let CommandEvent::Action(value) = event;
                this.command_dialog
                    .update(cx, |state, cx| state.close(window, cx));
                this.perform_action(value, window, cx);
            }),
        ];
        Self {
            appearance,
            input,
            notes,
            preset,
            edit_hint,
            fields_scroll: ScrollHandle::new(),
            calendar,
            schedule_scroll: ScrollHandle::new(),
            commands,
            command_dialog,
            dialog,
            select,
            stroke,
            tabs,
            help,
            actions,
            accordion,
            preview,
            preview_options,
            reset_dialog,
            reset_cancel_focus,
            formatting,
            preview_indeterminate: false,
            show_cursor: true,
            keep_toolbar: false,
            _subscriptions: subscriptions,
        }
    }

    fn input_style(palette: Palette) -> InputEditorStyle {
        let mut style = InputEditorStyle::default();
        style.foreground = palette.foreground;
        style.muted_foreground = palette.muted;
        style.caret = palette.accent;
        style.selection = palette.highlight;
        style
    }

    fn perform_action(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        match value {
            "edit" => self.dialog.update(cx, |state, cx| state.open(window, cx)),
            "cursor" => self.show_cursor = !self.show_cursor,
            "toolbar" => self.keep_toolbar = !self.keep_toolbar,
            "reset" => self
                .reset_dialog
                .update(cx, |state, cx| state.open(window, cx)),
            "commands" => {
                self.commands
                    .update(cx, |state, cx| state.set_query("", window, cx));
                self.command_dialog
                    .update(cx, |state, cx| state.open(window, cx));
            }
            _ => return,
        }
        cx.notify();
    }

    fn button(palette: Palette, button: Button) -> Button {
        button
            .h(palette.control_height)
            .px_3()
            .rounded(palette.radius)
            .border_1()
            .border_color(palette.accent)
            .bg(palette.accent)
            .text_color(palette.accent_foreground)
            .hover(|style| style.opacity(0.85))
            .focus_visible(|style| style.border_color(palette.foreground))
    }

    fn schedule(&self, palette: Palette, cx: &Context<Self>) -> impl IntoElement + use<> {
        let state = self.calendar.read(cx);
        let month = state.month();
        let selected = state.selected_date();
        let date_label = selected
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Not scheduled".to_string());
        let calendar = Calendar::new("recording-calendar", &self.calendar)
            .accessibility_label(format!("Recording date, {}", month.format("%B %Y")))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .id("weekdays")
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
                                    .flex_1()
                                    .min_w_0()
                                    .text_center()
                                    .text_sm()
                                    .text_color(palette.muted)
                                    .child(day)
                            }),
                    ),
            )
            .children(state.days().chunks(7).enumerate().map(|(row, week)| {
                div()
                    .id(("week", row))
                    .role(Role::Row)
                    .aria_row_index(row + 1)
                    .flex()
                    .children(week.iter().map(|&date| {
                        CalendarDay::new(&self.calendar, date)
                            .accessibility_label(date.format("%A, %B %-d, %Y").to_string())
                            .flex_1()
                            .min_w_0()
                            .h(palette.control_height)
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(palette.surface)
                            .rounded(palette.radius)
                            .text_color(if date.month() == month.month() {
                                palette.foreground
                            } else {
                                palette.muted
                            })
                            .hover(|style| style.bg(palette.highlight))
                            .focus_visible(|style| style.border_color(palette.accent))
                            .styles(|styles| styles.disabled(|style| style.opacity(0.35)))
                            .when(selected == Some(date), |day| {
                                day.bg(palette.accent).text_color(palette.accent_foreground)
                            })
                            .child(date.day().to_string())
                    }))
            }));
        let previous = self.calendar.clone();
        let next = self.calendar.clone();
        let month_controls = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(Self::button(
                palette,
                Button::new("previous-month")
                    .accessibility_label("Previous month")
                    .disabled(!state.can_previous_month())
                    .on_click(move |_, window, cx| {
                        previous.update(cx, |state, cx| state.previous_month(window, cx));
                    })
                    .child("‹"),
            ))
            .child(div().child(month.format("%B %Y").to_string()))
            .child(Self::button(
                palette,
                Button::new("next-month")
                    .accessibility_label("Next month")
                    .disabled(!state.can_next_month())
                    .on_click(move |_, window, cx| {
                        next.update(cx, |state, cx| state.next_month(window, cx));
                    })
                    .child("›"),
            ));
        let preset = self
            .preset
            .read(cx)
            .selected_item(cx)
            .map(|item| item.label().clone())
            .unwrap_or_else(|| "Choose a preset".into());
        let plan = Table::new(
            "recording-plan",
            [
                TableColumn::fixed(px(180.)),
                TableColumn::fixed(px(130.)),
                TableColumn::fixed(px(180.)),
            ],
        )
        .accessibility_label("Recording plan")
        .w(px(490.))
        .caption(
            TableCaption::new("recording-plan-caption")
                .py_2()
                .text_sm()
                .text_color(palette.muted)
                .child("The plan follows the retained form values."),
        )
        .header(
            TableRow::new("recording-plan-header")
                .bg(palette.highlight)
                .cell(
                    TableCell::new("plan-session-heading")
                        .p_2()
                        .child("Session"),
                )
                .cell(TableCell::new("plan-date-heading").p_2().child("Date"))
                .cell(TableCell::new("plan-preset-heading").p_2().child("Preset")),
        )
        .row(
            TableRow::new("recording-plan-session")
                .border_b_1()
                .border_color(palette.border)
                .cell(
                    TableCell::new("plan-session")
                        .row_header(true)
                        .p_2()
                        .child(self.input.read(cx).value()),
                )
                .cell(
                    TableCell::new("plan-date")
                        .p_2()
                        .child(date_label.clone().test_text(date_label)),
                )
                .cell(TableCell::new("plan-preset").p_2().child(preset)),
        );
        div()
            .flex()
            .flex_col()
            .gap(palette.gap)
            .child(month_controls)
            .child(calendar)
            .child(
                div().text_sm().text_color(palette.muted).child(
                    "Arrows move by day or week. Page Up/Down changes month. Enter selects.",
                ),
            )
            .child(
                ScrollArea::new("recording-plan-scroll", &self.schedule_scroll)
                    .axis(ScrollbarAxis::Horizontal)
                    .accessibility_label("Recording plan columns")
                    .scrollbar_mode(ScrollbarMode::Always)
                    .scrollbar_styles(|styles| {
                        styles.thumb(|thumb| thumb.bg(palette.muted).radius(px(3.)))
                    })
                    .w_full()
                    .min_w_0()
                    .content_styles(|style| style.pb(px(16.)))
                    .child(plan),
            )
    }
}

impl Render for Demo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.appearance.palette();
        let dialog = self.dialog.clone();
        let name_invalid = self.input.read(cx).value().trim().is_empty();
        let select = self.select.read(cx);
        let selected_label = select
            .selected_item()
            .map(|item| item.label().clone())
            .unwrap_or_else(|| SharedString::from("Choose a tool"));
        let selected_index = select.selected_index();
        let highlighted = select.highlighted_index();
        let options = select.items().iter().enumerate().map(|(ix, item)| {
            SelectOption::new(&self.select, ix)
                .h(palette.control_height)
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .rounded(palette.radius)
                .when(highlighted == Some(ix), |option| {
                    option.bg(palette.highlight)
                })
                .when(item.is_disabled(), |option| option.opacity(0.45))
                .child(item.label().clone())
                .when(selected_index == Some(ix), |option| option.child("✓"))
        });
        let field = InputBase::new("session-name")
            .role(None::<Role>)
            .semantic_owner::<Input>(self.input.entity_id())
            .focused(self.input.focus_handle(cx).is_focused(window))
            .h(palette.control_height)
            .px_3()
            .w_full()
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .bg(palette.field)
            .styles(|styles| styles.focused(|style| style.border_color(palette.accent)))
            .when(name_invalid, |field| field.border_color(palette.error))
            .child(Input::new(&self.input));
        let name_field = Field::new("session-name-field", &self.input, "Session name")
            .description("Choose a name to recognize this session.")
            .required(true)
            .error(name_invalid.then(|| "A session name is required.".into()))
            .styles(|styles| {
                styles
                    .description(|style| style.text_sm().text_color(palette.muted))
                    .error(|style| style.text_sm().text_color(palette.error))
            })
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .child(field);
        let notes = InputBase::new("session-notes")
            .role(None::<Role>)
            .semantic_owner::<Textarea>(self.notes.entity_id())
            .focused(self.notes.focus_handle(cx).is_focused(window))
            .h(px(84.))
            .w_full()
            .px_3()
            .py_2()
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .bg(palette.field)
            .styles(|styles| styles.focused(|style| style.border_color(palette.accent)))
            .child(Textarea::new(&self.notes));
        let notes_field = Field::new("session-notes-field", &self.notes, "Session notes")
            .description("Optional notes stay with this session.")
            .styles(|styles| styles.description(|style| style.text_sm().text_color(palette.muted)))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .child(notes);
        let preset_state = self.preset.read(cx);
        let preset_input = preset_state.input().clone();
        let preset_options = preset_state.visible_indices(cx).iter().map(|&index| {
            let item = &preset_state.items(cx)[index];
            ComboboxOption::new(&self.preset, index)
                .h(palette.control_height)
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .rounded(palette.radius)
                .when(
                    preset_state.highlighted_index(cx) == Some(index),
                    |option| option.bg(palette.highlight),
                )
                .when(item.is_disabled(), |option| option.opacity(0.45))
                .child(item.label().clone())
                .when(preset_state.selected_index() == Some(index), |option| {
                    option.child("✓")
                })
        });
        let preset = Combobox::new("recording-preset", &self.preset)
            .input(|input| {
                input
                    .accessibility_label("Recording preset")
                    .placeholder("Search recording presets")
                    .h(palette.control_height)
                    .px_3()
                    .w_full()
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .bg(palette.field)
                    .styles(|styles| styles.focused(|style| style.border_color(palette.accent)))
            })
            .content(
                div()
                    .w(palette.dialog_width - palette.padding * 2.)
                    .p_1()
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .bg(palette.surface)
                    .text_color(palette.foreground)
                    .child(
                        ComboboxList::new("recording-preset-options", &self.preset)
                            .accessibility_label("Recording presets")
                            .max_h(px(180.))
                            .children(preset_options),
                    )
                    .when(preset_state.visible_indices(cx).is_empty(), |list| {
                        list.child(
                            div()
                                .p_3()
                                .text_color(palette.muted)
                                .child("No presets found"),
                        )
                    }),
            );
        let preset_label = preset_state
            .selected_item(cx)
            .map(|item| item.label().clone())
            .unwrap_or_else(|| "None".into());
        let preset_field = Field::new("recording-preset-field", &preset_input, "Recording preset")
            .description("Search a preset and press Enter to select it.")
            .styles(|styles| styles.description(|style| style.text_sm().text_color(palette.muted)))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .child(preset);
        let tool = Select::new("drawing-tool", &self.select)
            .accessibility_label("Drawing tool")
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .bg(palette.field)
            .focus_visible(|style| style.border_color(palette.accent))
            .trigger(
                div()
                    .h(palette.control_height)
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(selected_label.clone())
                    .child("⌄"),
            )
            .content(
                div()
                    .w(palette.dialog_width - palette.padding * 2.)
                    .p_1()
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .bg(palette.surface)
                    .children(options),
            );
        let owner = cx.entity();
        let show_cursor = Checkbox::new("show-cursor")
            .checked(self.show_cursor)
            .accessibility_label("Show cursor")
            .flex()
            .items_center()
            .gap_2()
            .py_1()
            .rounded(palette.radius)
            .focus_visible(|style| style.bg(palette.highlight))
            .on_change(move |state, _, _, cx| {
                owner.update(cx, |this, cx| {
                    this.show_cursor = state == CheckboxState::Checked;
                    cx.notify();
                });
            })
            .child(
                div()
                    .size_4()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(palette.accent)
                    .rounded(px(3.))
                    .when(self.show_cursor, |indicator| {
                        indicator
                            .bg(palette.accent)
                            .text_color(palette.accent_foreground)
                            .child("✓")
                    }),
            )
            .child("Show cursor");
        let owner = cx.entity();
        let keep_toolbar = Switch::new("keep-toolbar")
            .checked(self.keep_toolbar)
            .accessibility_label("Keep toolbar visible")
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .py_1()
            .rounded(palette.radius)
            .focus_visible(|style| style.bg(palette.highlight))
            .on_change(move |checked, _, _, cx| {
                owner.update(cx, |this, cx| {
                    this.keep_toolbar = checked;
                    cx.notify();
                });
            })
            .child("Keep toolbar visible")
            .child(
                div()
                    .w_8()
                    .h_5()
                    .p(px(2.))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .bg(palette.border)
                    .when(self.keep_toolbar, |track| {
                        track.justify_end().bg(palette.accent)
                    })
                    .child(div().size_4().rounded_full().bg(palette.surface)),
            );
        let stroke_state = self.stroke.read(cx);
        let stroke_label = stroke_state.selected_item().unwrap().label().clone();
        let stroke = RadioGroup::new("stroke-width", &self.stroke)
            .axis(Axis::Horizontal)
            .accessibility_label("Stroke width")
            .flex()
            .flex_wrap()
            .gap_2()
            .children(
                stroke_state
                    .items()
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        RadioOption::new(&self.stroke, index)
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .border_1()
                            .border_color(palette.border)
                            .rounded(palette.radius)
                            .focus_visible(|style| style.border_color(palette.accent))
                            .when(stroke_state.selected_index() == Some(index), |option| {
                                option.bg(palette.highlight)
                            })
                            .when(item.is_disabled(), |option| option.opacity(0.45))
                            .child(item.label().clone())
                    }),
            );
        let tab_state = self.tabs.read(cx);
        let tab_list = TabList::new("session-sections", &self.tabs)
            .axis(Axis::Horizontal)
            .accessibility_label("Session sections")
            .flex()
            .flex_wrap()
            .gap_1()
            .children(tab_state.items().iter().enumerate().map(|(index, item)| {
                Tab::new(&self.tabs, index)
                    .px_2()
                    .py_1()
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .focus_visible(|style| style.border_color(palette.accent))
                    .when(tab_state.selected_index() == Some(index), |tab| {
                        tab.bg(palette.accent).text_color(palette.accent_foreground)
                    })
                    .when(item.is_disabled(), |tab| tab.opacity(0.45))
                    .child(item.label().clone())
            }));
        let help_state = self.help.clone();
        let help = Popover::new("keyboard-help", &self.help)
            .accessibility_label("Keyboard help")
            .offset(px(6.))
            .trigger(|button| {
                Self::button(
                    palette,
                    button
                        .accessibility_label("Show keyboard help")
                        .child("Show keyboard help"),
                )
            })
            .content(
                div()
                    .w(palette.dialog_width - palette.padding * 2.)
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .bg(palette.surface)
                    .text_color(palette.foreground)
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .child("Tab moves between controls. Arrow keys choose options and tabs. Escape closes this help before the settings dialog.")
                    .child(Self::button(
                        palette,
                        Button::new("close-help")
                            .accessibility_label("Close help")
                            .on_click(move |_, window, cx| {
                                help_state.update(cx, |state, cx| state.close(window, cx));
                            })
                            .child("Close help"),
                    )),
            );
        let accordion_state = self.accordion.read(cx);
        let header = |index: usize| {
            let item = &accordion_state.items()[index];
            AccordionHeader::new(&self.accordion, index)
                .heading_level(3)
                .w_full()
                .px_2()
                .py_1()
                .flex()
                .items_center()
                .justify_between()
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .focus_visible(|style| style.border_color(palette.accent))
                .when(item.is_disabled(), |header| header.opacity(0.45))
                .child(item.label().clone())
                .child(if accordion_state.open_index() == Some(index) {
                    "−"
                } else {
                    "+"
                })
        };
        let help_sections = Accordion::new("help-sections", &self.accordion)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().child(header(0)).child(
                    AccordionPanel::new(&self.accordion, 0)
                        .p_2()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child("Use the keyboard guide to explore these controls.")
                        .child(help),
                ),
            )
            .child(div().child(header(1)))
            .child(
                div().child(header(2)).child(
                    AccordionPanel::new(&self.accordion, 2)
                        .tab_stop(true)
                        .p_2()
                        .rounded(palette.radius)
                        .focus_visible(|style| style.bg(palette.highlight))
                        .child("The application retains form values and supplies every style. Closing a panel or dialog preserves those values."),
                ),
            );
        let panels = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(palette.gap)
            .child(tab_list)
            .child(
                TabPanel::new(&self.tabs, 0)
                    .flex()
                    .flex_col()
                    .gap(palette.gap)
                    .child(show_cursor)
                    .child(keep_toolbar)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child("Stroke width")
                            .child(stroke),
                    )
                    .child(
                        Separator::new("notes-separator")
                            .h(px(1.))
                            .w_full()
                            .bg(palette.border),
                    )
                    .child(notes_field)
                    .child(preset_field),
            )
            .child(
                TabPanel::new(&self.tabs, 2)
                    .flex()
                    .flex_col()
                    .gap(palette.gap)
                    .child(help_sections),
            )
            .child(
                TabPanel::new(&self.tabs, 3)
                    .flex()
                    .flex_col()
                    .gap(palette.gap)
                    .child(self.schedule(palette, cx)),
            );
        let popup = DialogPopup::new()
            .w(palette.dialog_width)
            .max_h(window.viewport_size().height - px(32.))
            .p(palette.padding)
            .flex()
            .flex_col()
            .gap(palette.gap)
            .bg(palette.surface)
            .text_color(palette.foreground)
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .child(DialogTitle::new().text_lg().child("Session settings"))
            .child(
                DialogDescription::new()
                    .text_color(palette.muted)
                    .child("Choose the session's drawing and toolbar settings."),
            )
            .child(
                ScrollArea::new("settings-fields", &self.fields_scroll)
                    .accessibility_label("Session settings fields")
                    .scrollbar_styles(|styles| {
                        styles.thumb(|thumb| thumb.bg(palette.muted).radius(px(3.)))
                    })
                    .min_h_0()
                    .content_styles(|style| style.flex().flex_col().gap(palette.gap))
                    .child(name_field)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_shrink_0()
                            .gap_1()
                            .child("Drawing tool")
                            .child(tool),
                    )
                    .child(panels),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(DialogClose::new(&self.dialog).trigger(|button| {
                        Self::button(
                            palette,
                            button
                                .accessibility_label("Done")
                                .disabled(name_invalid)
                                .child("Done"),
                        )
                    })),
            );

        let action_state = self.actions.read(cx);
        let actions = DropdownMenu::new("session-actions", &self.actions)
            .accessibility_label("Session actions")
            .offset(px(6.))
            .trigger(|button| {
                Self::button(
                    palette,
                    button
                        .accessibility_label("Session actions")
                        .child("Session actions ▾"),
                )
            })
            .content(
                div()
                    .min_w(rems(14.))
                    .p_1()
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .bg(palette.surface)
                    .text_color(palette.foreground)
                    .children(
                        action_state
                            .items()
                            .iter()
                            .enumerate()
                            .map(|(index, item)| {
                                DropdownMenuOption::new(&self.actions, index)
                                    .h(palette.control_height)
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(palette.radius)
                                    .when(
                                        action_state.highlighted_index() == Some(index),
                                        |option| option.bg(palette.highlight),
                                    )
                                    .when(item.is_disabled(), |option| option.opacity(0.45))
                                    .child(item.label().clone())
                            }),
                    ),
            );

        let owner = cx.entity();
        let unknown_progress = Toggle::new("unknown-progress")
            .pressed(self.preview_indeterminate)
            .accessibility_label("Unknown progress")
            .h(palette.control_height)
            .px_3()
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .focus_visible(|style| style.border_color(palette.accent))
            .when(self.preview_indeterminate, |toggle| {
                toggle
                    .bg(palette.accent)
                    .text_color(palette.accent_foreground)
            })
            .on_change(move |pressed, _, _, cx| {
                owner.update(cx, |this, cx| {
                    this.preview_indeterminate = pressed;
                    cx.notify();
                });
            })
            .child("Unknown progress");
        let preview = self.preview.read(cx);
        let progress_preview = div()
            .flex()
            .flex_col()
            .gap_2()
            .px_2()
            .child(if self.preview_indeterminate {
                "Progress preview · Unknown".to_string()
            } else {
                format!("Progress preview · {:.0}%", preview.value())
            })
            .child(
                Slider::new("preview-percentage", &self.preview)
                    .accessibility_label("Preview percentage")
                    .relative()
                    .h(palette.control_height)
                    .rounded(palette.radius)
                    .focus_visible(|style| style.bg(palette.highlight))
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(relative(0.5))
                            .mt(px(-2.))
                            .h(px(4.))
                            .rounded_full()
                            .bg(palette.border),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top(relative(0.5))
                            .mt(px(-2.))
                            .h(px(4.))
                            .w(relative(preview.percentage()))
                            .rounded_full()
                            .bg(palette.accent),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(relative(preview.percentage()))
                            .top(relative(0.5))
                            .ml(px(-7.))
                            .mt(px(-7.))
                            .size(px(14.))
                            .rounded_full()
                            .bg(palette.accent),
                    ),
            )
            .child(
                Progress::new("preview-progress")
                    .accessibility_label("Preview progress")
                    .value(preview.value())
                    .indeterminate(self.preview_indeterminate)
                    .h_2()
                    .rounded_full()
                    .overflow_hidden()
                    .bg(palette.border)
                    .child(
                        div()
                            .debug_selector(|| "preview-progress-fill".into())
                            .h_full()
                            .w(relative(if self.preview_indeterminate {
                                1. / 3.
                            } else {
                                preview.percentage()
                            }))
                            .rounded_full()
                            .when(self.preview_indeterminate, |fill| fill.opacity(0.5))
                            .bg(palette.accent),
                    ),
            )
            .child(unknown_progress);
        let formatting = self.formatting.read(cx);
        let emphasis = div()
            .flex()
            .flex_col()
            .gap_2()
            .child("Summary text emphasis")
            .child(
                ToggleGroup::new("text-emphasis", &self.formatting)
                    .accessibility_label("Summary text emphasis")
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(formatting.items().iter().enumerate().map(|(index, item)| {
                        ToggleOption::new(&self.formatting, index)
                            .h(palette.control_height)
                            .px_2()
                            .border_1()
                            .border_color(palette.border)
                            .rounded(palette.radius)
                            .focus_visible(|style| style.border_color(palette.accent))
                            .when(formatting.is_pressed(index), |option| {
                                option
                                    .bg(palette.accent)
                                    .text_color(palette.accent_foreground)
                            })
                            .when(item.is_disabled(), |option| option.opacity(0.45))
                            .child(item.label().clone())
                    })),
            );

        let preview_options = Collapsible::new("preview-options", &self.preview_options)
            .flex()
            .flex_col()
            .gap_3()
            .child(
                CollapsibleTrigger::new(&self.preview_options)
                    .accessibility_label("Preview options")
                    .w_full()
                    .h(palette.control_height)
                    .px_3()
                    .justify_between()
                    .border_1()
                    .border_color(palette.border)
                    .rounded(palette.radius)
                    .focus_visible(|style| style.border_color(palette.accent))
                    .child("Preview options")
                    .child(if self.preview_options.read(cx).is_open() {
                        "−"
                    } else {
                        "+"
                    }),
            )
            .child(
                CollapsibleContent::new(&self.preview_options)
                    .accessibility_label("Preview options")
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(progress_preview)
                    .child(emphasis),
            );
        let owner = cx.entity();
        let reset_popup = DialogPopup::new()
            .w(palette.dialog_width)
            .p(palette.padding)
            .flex()
            .flex_col()
            .gap(palette.gap)
            .bg(palette.surface)
            .text_color(palette.foreground)
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .child(DialogTitle::new().text_lg().child("Reset preview?"))
            .child(
                DialogDescription::new()
                    .text_color(palette.muted)
                    .child("Set progress back to 40% and show its known value."),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(DialogClose::new(&self.reset_dialog).trigger(|button| {
                        Self::button(
                            palette,
                            button
                                .track_focus(&self.reset_cancel_focus)
                                .accessibility_label("Cancel reset")
                                .child("Cancel"),
                        )
                    }))
                    .child(Self::button(
                        palette,
                        Button::new("confirm-preview-reset")
                            .accessibility_label("Reset preview")
                            .on_click(move |_, window, cx| {
                                owner.update(cx, |this, cx| {
                                    this.preview.update(cx, |state, cx| {
                                        state.set_value(40.0, window, cx);
                                    });
                                    this.preview_indeterminate = false;
                                    this.reset_dialog
                                        .update(cx, |state, cx| state.close(window, cx));
                                    cx.notify();
                                });
                            })
                            .child("Reset preview"),
                    )),
            );

        let command_state = self.commands.read(cx);
        let command_popup = DialogPopup::new()
            .w(palette.dialog_width)
            .p(palette.padding)
            .flex()
            .flex_col()
            .gap(palette.gap)
            .bg(palette.surface)
            .text_color(palette.foreground)
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .child(DialogTitle::new().text_lg().child("Search commands"))
            .child(
                Command::new("session-commands", &self.commands)
                    .accessibility_label("Session commands")
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        CommandInput::new("command-query", &self.commands)
                            .accessibility_label("Search commands")
                            .placeholder("Search actions or type 설정")
                            .h(palette.control_height)
                            .px_3()
                            .w_full()
                            .border_1()
                            .border_color(palette.border)
                            .rounded(palette.radius)
                            .bg(palette.field)
                            .styles(|styles| {
                                styles.focused(|style| style.border_color(palette.accent))
                            }),
                    )
                    .child(
                        Separator::new("command-separator")
                            .h(px(1.))
                            .w_full()
                            .bg(palette.border),
                    )
                    .child(
                        CommandList::new("command-results", &self.commands)
                            .accessibility_label("Matching commands")
                            .max_h(px(220.))
                            .children(command_state.visible_indices().iter().map(|&index| {
                                let item = &command_state.items()[index];
                                CommandOption::new(&self.commands, index)
                                    .h(palette.control_height)
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(palette.radius)
                                    .when(
                                        command_state.highlighted_index() == Some(index),
                                        |option| option.bg(palette.highlight),
                                    )
                                    .when(item.is_disabled(), |option| option.opacity(0.45))
                                    .child(item.label().clone())
                            })),
                    )
                    .when(command_state.visible_indices().is_empty(), |command| {
                        command.child(
                            div()
                                .p_3()
                                .text_color(palette.muted)
                                .child("No commands found"),
                        )
                    }),
            )
            .child(
                div()
                    .text_color(palette.muted)
                    .child("↑ ↓ to choose · Enter to run · Escape to close"),
            );

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(palette.canvas)
            .text_color(palette.foreground)
            .text_size(palette.font_size)
            .child(
                div()
                    .w(palette.dialog_width)
                    .p(palette.padding)
                    .flex()
                    .flex_col()
                    .gap(palette.gap)
                    .bg(palette.surface)
                    .rounded(palette.radius)
                    .child(div().text_xl().child(match self.appearance {
                        Appearance::Compact => "MiniCast controls",
                        Appearance::Settings => "Workspace settings",
                    }))
                    .child(
                        div()
                            .text_color(palette.muted)
                            .when(formatting.is_pressed(0), |text| {
                                text.font_weight(FontWeight::BOLD)
                            })
                            .when(formatting.is_pressed(2), |text| text.italic())
                            .child(format!(
                                "{} · {} · {} stroke",
                                self.input.read(cx).value(),
                                selected_label,
                                stroke_label
                            )),
                    )
                    .child(div().text_color(palette.muted).child(format!(
                        "Cursor {} · Toolbar {}",
                        if self.show_cursor {
                            "visible"
                        } else {
                            "hidden"
                        },
                        if self.keep_toolbar {
                            "pinned"
                        } else {
                            "automatic"
                        }
                    )))
                    .child(
                        div()
                            .text_color(palette.muted)
                            .child(format!("Preset: {preset_label}")),
                    )
                    .child(div().text_color(palette.muted).child(format!(
                        "Recording date: {}",
                        self.calendar
                            .read(cx)
                            .selected_date()
                            .map(|date| date.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "Not scheduled".to_string())
                    )))
                    .child(
                        Tooltip::new(
                            "edit-session",
                            &self.edit_hint,
                            "Change the session name, recording preset, and notes.",
                        )
                        .max_w(rems(20.))
                        .px_3()
                        .py_2()
                        .border_1()
                        .border_color(palette.border)
                        .rounded(palette.radius)
                        .bg(palette.surface)
                        .text_color(palette.foreground)
                        .trigger(|button| {
                            Self::button(
                                palette,
                                button
                                    .accessibility_label("Edit session")
                                    .on_click(move |_, window, cx| {
                                        dialog.update(cx, |state, cx| state.open(window, cx));
                                    })
                                    .child("Edit session"),
                            )
                        }),
                    )
                    .child(actions)
                    .child(preview_options)
                    .child(
                        div()
                            .text_color(palette.muted)
                            .child("Keyboard: Tab, arrows, Space, Enter and Escape"),
                    ),
            )
            .child(
                Dialog::new(&self.dialog)
                    .accessibility_label("Session settings")
                    .accessibility_description("Choose the session's drawing and toolbar settings.")
                    .backdrop(
                        DialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(gpui::black())
                            .opacity(0.45),
                    )
                    .popup(popup),
            )
            .child(
                AlertDialog::new(&self.reset_dialog)
                    .accessibility_label("Reset preview?")
                    .accessibility_description("Set progress back to 40% and show its known value.")
                    .backdrop(
                        DialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(gpui::black())
                            .opacity(0.45),
                    )
                    .popup(reset_popup),
            )
            .child(
                Dialog::new(&self.command_dialog)
                    .accessibility_label("Search commands")
                    .accessibility_description("Search and run a session action.")
                    .backdrop(
                        DialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(gpui::black())
                            .opacity(0.45),
                    )
                    .popup(command_popup),
            )
    }
}

#[cfg(not(test))]
fn main() {
    use gpui::{WindowBounds, WindowOptions, size};
    use gpui_design::Root;

    let appearance = match std::env::args().nth(1).as_deref() {
        None | Some("compact") => Appearance::Compact,
        Some("settings") => Appearance::Settings,
        Some(_) => {
            eprintln!("Usage: two_styles [compact|settings]");
            std::process::exit(2);
        }
    };
    gpui_platform::application().run(move |cx| {
        gpui_design::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(760.), px(780.)), cx)),
                ..WindowOptions::default()
            },
            move |window, cx| {
                let demo = cx.new(|cx| Demo::new(appearance, window, cx));
                cx.new(|cx| Root::new(demo, window, cx))
            },
        )
        .expect("failed to open two-styles example");
        cx.activate(true);
    });
}
