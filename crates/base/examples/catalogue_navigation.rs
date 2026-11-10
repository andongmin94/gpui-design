//! Command and document navigation with application-owned layout and state.

use gpui::{
    AppContext as _, Context, Entity, FocusHandle, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div, px, rgb,
};
use gpui_design::{
    Button, CheckboxState, Direction, DirectionProvider, MenuEvent, MenuItem, MenuOption,
    MenuState, MenuSubmenu, Menubar, MenubarItem, MenubarMenu, MenubarState, NavigationMenu,
    NavigationMenuEntry, NavigationMenuEvent, NavigationMenuItem, NavigationMenuLink,
    NavigationMenuState,
};

pub(crate) struct NavigationCatalogue {
    bar: Entity<MenubarState>,
    navigation: Entity<NavigationMenuState>,
    file: Entity<MenuState>,
    view: Entity<MenuState>,
    export: Entity<MenuState>,
    guide_focus: FocusHandle,
    reference_focus: FocusHandle,
    direction: Direction,
    page: SharedString,
    last_command: SharedString,
    show_grid: bool,
    selection: CheckboxState,
    theme: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl NavigationCatalogue {
    pub(crate) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let export = cx.new(|cx| {
            MenuState::new(
                [
                    MenuItem::new("export-png", "PNG image"),
                    MenuItem::new("export-svg", "SVG image"),
                ],
                cx,
            )
        });
        let file = cx.new(|cx| {
            MenuState::new(
                [
                    MenuItem::new("new", "New document"),
                    MenuItem::new("save", "Save document").disabled(true),
                    MenuItem::new("export", "Export").submenu(&export),
                ],
                cx,
            )
        });
        let view = cx.new(|cx| {
            MenuState::new(
                [
                    MenuItem::new("grid", "Show grid"),
                    MenuItem::new("selection", "Selected layers"),
                    MenuItem::new("system", "System theme"),
                    MenuItem::new("light", "Light theme"),
                    MenuItem::new("dark", "Dark theme"),
                ],
                cx,
            )
        });
        let bar = cx.new(|cx| {
            MenubarState::new(
                [
                    MenubarItem::new("File", &file),
                    MenubarItem::new("View", &view),
                ],
                cx,
            )
        });
        let guide_focus = cx.focus_handle();
        let reference_focus = cx.focus_handle();
        let navigation = cx.new(|cx| {
            NavigationMenuState::new(
                [
                    NavigationMenuItem::new("overview", "Overview"),
                    NavigationMenuItem::new("learn", "Learn")
                        .with_popup()
                        .initial_focus(&guide_focus),
                    NavigationMenuItem::new("reference", "Reference")
                        .with_popup()
                        .initial_focus(&reference_focus),
                    NavigationMenuItem::new("community", "Community"),
                ],
                cx,
            )
        });
        let mut subscriptions = vec![
            cx.observe(&bar, |_, _, cx| cx.notify()),
            cx.observe(&navigation, |_, _, cx| cx.notify()),
            cx.subscribe(&navigation, |this: &mut Self, _, event, cx| {
                let NavigationMenuEvent::Navigate(value) = event;
                this.page = value.clone();
                cx.notify();
            }),
        ];
        for menu in [&file, &view] {
            subscriptions.push(cx.subscribe(menu, |this: &mut Self, _, event, cx| {
                let MenuEvent::Action(value) = event;
                this.last_command = value.clone();
                match value.as_ref() {
                    "grid" => this.show_grid = !this.show_grid,
                    "selection" => {
                        this.selection = if this.selection == CheckboxState::Checked {
                            CheckboxState::Unchecked
                        } else {
                            CheckboxState::Checked
                        }
                    }
                    "system" | "light" | "dark" => this.theme = value.clone(),
                    _ => {}
                }
                cx.notify();
            }));
        }
        Self {
            bar,
            navigation,
            file,
            view,
            export,
            guide_focus,
            reference_focus,
            direction: Direction::Ltr,
            page: "overview".into(),
            last_command: "No command selected".into(),
            show_grid: true,
            selection: CheckboxState::Indeterminate,
            theme: "system".into(),
            _subscriptions: subscriptions,
        }
    }

    fn menu_option(
        &self,
        menu: &Entity<MenuState>,
        index: usize,
        cx: &Context<Self>,
    ) -> MenuOption {
        let state = menu.read(cx);
        let item = &state.items()[index];
        let marker = if menu == &self.view {
            match index {
                0 if self.show_grid => "✓ ",
                1 if self.selection == CheckboxState::Indeterminate => "− ",
                1 if self.selection == CheckboxState::Checked => "✓ ",
                2..=4 if item.value() == &self.theme => "● ",
                _ => "   ",
            }
        } else {
            ""
        };
        MenuOption::new(menu, index)
            .px_3()
            .py_2()
            .rounded_sm()
            .bg(rgb(if state.highlighted_index() == Some(index) {
                0x334155
            } else {
                0x172033
            }))
            .text_color(rgb(if item.is_disabled() {
                0x64748b
            } else {
                0xe2e8f0
            }))
            .child(format!("{marker}{}", item.label()))
    }

    fn link(
        &self,
        id: &'static str,
        value: &'static str,
        label: &'static str,
    ) -> NavigationMenuLink {
        NavigationMenuLink::new(id, &self.navigation, value)
            .current(self.page.as_ref() == value)
            .px_3()
            .py_2()
            .rounded_sm()
            .bg(rgb(if self.page.as_ref() == value {
                0x1d4ed8
            } else {
                0x172033
            }))
            .child(label)
    }
}

fn panel() -> gpui::Div {
    div()
        .w(px(230.))
        .p_2()
        .flex()
        .flex_col()
        .gap_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x475569))
        .bg(rgb(0x172033))
}

fn trigger(button: Button, label: &'static str, open: bool) -> Button {
    button
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(if open { 0x334155 } else { 0x172033 }))
        .child(label)
}

impl Render for NavigationCatalogue {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open_menu = self.bar.read(cx).open_index(cx);
        let open_navigation = self.navigation.read(cx).open_index(cx);
        let export_highlighted = self.file.read(cx).highlighted_index() == Some(2);
        let menu =
            self.direction
                .row(Menubar::new(&self.bar).accessibility_label("Document commands"))
                .gap_2()
                .child(
                    MenubarMenu::new(&self.bar, 0)
                        .offset(px(5.))
                        .trigger(|button| trigger(button, "File", open_menu == Some(0)))
                        .content(
                            panel()
                                .child(self.menu_option(&self.file, 0, cx))
                                .child(self.menu_option(&self.file, 1, cx))
                                .child(
                                    MenuSubmenu::new(&self.file, 2)
                                        .trigger(|option| {
                                            option
                                                .px_3()
                                                .py_2()
                                                .rounded_sm()
                                                .bg(rgb(if export_highlighted {
                                                    0x334155
                                                } else {
                                                    0x172033
                                                }))
                                                .child(if self.direction.is_rtl() {
                                                    "‹ Export"
                                                } else {
                                                    "Export ›"
                                                })
                                        })
                                        .content(panel().children((0..2).map(|index| {
                                            self.menu_option(&self.export, index, cx)
                                        }))),
                                ),
                        ),
                )
                .child(
                    MenubarMenu::new(&self.bar, 1)
                        .offset(px(5.))
                        .trigger(|button| trigger(button, "View", open_menu == Some(1)))
                        .content(
                            panel()
                                .child(self.menu_option(&self.view, 0, cx).checked(self.show_grid))
                                .child(
                                    self.menu_option(&self.view, 1, cx)
                                        .check_state(self.selection),
                                )
                                .child(
                                    div()
                                        .id("catalogue-appearance")
                                        .role(Role::Group)
                                        .aria_label("Appearance")
                                        .flex()
                                        .flex_col()
                                        .border_t_1()
                                        .border_color(rgb(0x475569))
                                        .pt_1()
                                        .children((2..5).map(|index| {
                                            self.menu_option(&self.view, index, cx).radio_checked(
                                                self.view.read(cx).items()[index].value()
                                                    == &self.theme,
                                            )
                                        })),
                                ),
                        ),
                );

        let navigation = self
            .direction
            .row(NavigationMenu::new(&self.navigation).accessibility_label("Documentation pages"))
            .gap_2()
            .flex_wrap()
            .child(
                NavigationMenuEntry::new(&self.navigation, 0)
                    .current(self.page.as_ref() == "overview")
                    .trigger(|button| {
                        trigger(button, "Overview", self.page.as_ref() == "overview")
                    }),
            )
            .child(
                NavigationMenuEntry::new(&self.navigation, 1)
                    .offset(px(5.))
                    .trigger(|button| trigger(button, "Learn", open_navigation == Some(1)))
                    .content(
                        panel()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Build with composition"),
                            )
                            .child(
                                self.link("guide-start", "getting-started", "Getting started")
                                    .button(|button| button.track_focus(&self.guide_focus)),
                            )
                            .child(self.link(
                                "guide-composition",
                                "composition",
                                "Composing controls",
                            )),
                    ),
            )
            .child(
                NavigationMenuEntry::new(&self.navigation, 2)
                    .offset(px(5.))
                    .trigger(|button| trigger(button, "Reference", open_navigation == Some(2)))
                    .content(
                        panel()
                            .child(
                                self.link("reference-api", "api", "Component APIs")
                                    .button(|button| button.track_focus(&self.reference_focus)),
                            )
                            .child(self.link(
                                "reference-keyboard",
                                "keyboard",
                                "Keyboard navigation",
                            )),
                    ),
            )
            .child(
                NavigationMenuEntry::new(&self.navigation, 3)
                    .current(self.page.as_ref() == "community")
                    .trigger(|button| {
                        trigger(button, "Community", self.page.as_ref() == "community")
                    }),
            );

        let description = match self.page.as_ref() {
            "getting-started" => {
                "Create retained state, render a headless control, and supply the application's presentation."
            }
            "composition" => {
                "Menu, Popover and Button behaviors combine into these navigation surfaces. One owner handles each state transition."
            }
            "api" => {
                "Menus report commands; navigation reports destinations. The application decides what each value means."
            }
            "keyboard" => {
                "Use Left and Right between top-level entries. Down opens a disclosure; Escape returns to its trigger."
            }
            "community" => {
                "Use the project's issue tracker to discuss reproducible behavior and focused component improvements."
            }
            _ => {
                "This local page changes when a navigation link is activated. Command menus above control independent document preferences."
            }
        };
        DirectionProvider::new(self.direction).w_full().child(
            div().w_full().flex().flex_col().gap_4().p_4().rounded_lg()
                .border_1().border_color(rgb(0x334155)).bg(rgb(0x0f172a))
                .child(div().flex().justify_between().items_center().gap_3()
                    .child(div().text_lg().font_weight(FontWeight::BOLD).child("Commands and navigation"))
                    .child(Button::new("navigation-direction").px_3().py_2().rounded_md().bg(rgb(0x334155))
                        .child(if self.direction.is_rtl() { "Direction: RTL" } else { "Direction: LTR" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.direction = if this.direction.is_rtl() { Direction::Ltr } else { Direction::Rtl }; cx.notify();
                        }))))
                .child(menu)
                .child(div().text_sm().text_color(rgb(0x94a3b8))
                    .child("Open File or View with ↓. Arrow keys switch menus; Export opens a submenu. Escape returns one level."))
                .child(div().text_sm().child(format!("Last command: {} · Grid: {} · Selection: {:?} · Theme: {}",
                    self.last_command, if self.show_grid { "shown" } else { "hidden" }, self.selection, self.theme)))
                .child(div().h(px(1.)).bg(rgb(0x334155)))
                .child(navigation)
                .child(div().p_4().rounded_md().bg(rgb(0x172033)).flex().flex_col().gap_2()
                    .child(div().font_weight(FontWeight::BOLD).child(format!("Page: {}", self.page)))
                    .child(description)),
        )
    }
}
