//! The same headless controls with two application-owned presentations.
//! Run with `compact` (the default) or `settings`.

use gpui::{
    AppContext as _, Axis, Context, Entity, FocusHandle, Focusable as _, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Rems, Render, Role,
    SharedString, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px, rems,
    rgb,
};
use gpui_design::{
    Button, Checkbox, CheckboxState, Dialog, DialogBackdrop, DialogClose, DialogDescription,
    DialogPopup, DialogState, DialogTitle, Input, InputBase, Popover, PopoverState, RadioGroup,
    RadioGroupState, RadioItem, RadioOption, Select, SelectItem, SelectOption, SelectState, Switch,
    Tab, TabItem, TabList, TabPanel, TabsState,
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
    pub(crate) dialog: Entity<DialogState>,
    pub(crate) select: Entity<SelectState>,
    pub(crate) stroke: Entity<RadioGroupState>,
    pub(crate) tabs: Entity<TabsState>,
    pub(crate) help: Entity<PopoverState>,
    pub(crate) show_cursor: bool,
    pub(crate) keep_toolbar: bool,
    pub(crate) trigger_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Demo {
    pub(crate) fn new(appearance: Appearance, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let palette = appearance.palette();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder("Enter a session name")
                .default_value("MiniCast");
            let mut editor_style = InputEditorStyle::default();
            editor_style.foreground = palette.foreground;
            editor_style.muted_foreground = palette.muted;
            editor_style.caret = palette.accent;
            editor_style.selection = palette.highlight;
            state.set_editor_style(editor_style);
            state.set_accessibility(
                InputAccessibility::new(Some(Role::TextInput))
                    .with_label(Some("Session name".into()))
                    .with_placeholder(Some("Enter a session name".into())),
            );
            state
        });
        let input_focus = input.focus_handle(cx);
        let dialog = cx.new(|cx| DialogState::new(cx).with_initial_focus(&input_focus));
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
                ],
                cx,
            )
            .default_value("preferences")
        });
        let help = cx.new(PopoverState::new);
        // The form summary and its presentation read these retained states.
        let subscriptions = vec![
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.observe(&select, |_, _, cx| cx.notify()),
            cx.observe(&stroke, |_, _, cx| cx.notify()),
            cx.observe(&tabs, |_, _, cx| cx.notify()),
            cx.observe(&help, |_, _, cx| cx.notify()),
            cx.observe(&dialog, |_, _, cx| cx.notify()),
        ];
        Self {
            appearance,
            input,
            dialog,
            select,
            stroke,
            tabs,
            help,
            show_cursor: true,
            keep_toolbar: false,
            trigger_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: subscriptions,
        }
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
}

impl Render for Demo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.appearance.palette();
        let dialog = self.dialog.clone();
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
            .semantic_owner(self.input.entity_id())
            .focused(self.input.focus_handle(cx).is_focused(window))
            .h(palette.control_height)
            .px_3()
            .w_full()
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .bg(palette.field)
            .styles(|styles| styles.focused(|style| style.border_color(palette.accent)))
            .child(Input::new(&self.input));
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
        let panels = div()
            .flex()
            .flex_col()
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
                    ),
            )
            .child(
                TabPanel::new(&self.tabs, 2)
                    .flex()
                    .flex_col()
                    .gap(palette.gap)
                    .child("These controls share behavior in both appearances. The application owns their styles and saved values.")
                    .child(help),
            );
        let popup = DialogPopup::new()
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
            .child(DialogTitle::new().text_lg().child("Session settings"))
            .child(
                DialogDescription::new()
                    .text_color(palette.muted)
                    .child("Choose the session's drawing and toolbar settings."),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child("Session name")
                    .child(field),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child("Drawing tool")
                    .child(tool),
            )
            .child(panels)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(DialogClose::new(&self.dialog).trigger(|button| {
                        Self::button(palette, button.accessibility_label("Done").child("Done"))
                    })),
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
                    .child(div().text_color(palette.muted).child(format!(
                        "{} · {} · {} stroke",
                        self.input.read(cx).value(),
                        selected_label,
                        stroke_label
                    )))
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
                    .child(Self::button(
                        palette,
                        Button::new("edit-session")
                            .track_focus(&self.trigger_focus)
                            .accessibility_label("Edit session")
                            .on_click(move |_, window, cx| {
                                dialog.update(cx, |state, cx| state.open(window, cx));
                            })
                            .child("Edit session"),
                    ))
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
