//! A coordinated strip of command menus with one keyboard entry point.

use gpui::{
    Anchor, AnyElement, App, ClickEvent, Context, ElementId, Entity, FocusHandle,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    Button, Direction, MenuState, Popover, StyledExt as _, TestSupportExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
};

const TRIGGER_CONTEXT: &str = "MenubarTrigger";
const CONTENT_CONTEXT: &str = "MenubarContent";

pub(crate) fn init(cx: &mut App) {
    for context in [TRIGGER_CONTEXT, CONTENT_CONTEXT] {
        cx.bind_keys([
            KeyBinding::new("left", SelectLeft, Some(context)),
            KeyBinding::new("right", SelectRight, Some(context)),
        ]);
    }
    cx.bind_keys([
        KeyBinding::new("home", SelectFirst, Some(TRIGGER_CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(TRIGGER_CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(TRIGGER_CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(TRIGGER_CONTEXT)),
    ]);
}

/// One top-level command menu. Command values and events belong to `menu`.
#[derive(Clone)]
pub struct MenubarItem {
    label: SharedString,
    menu: Entity<MenuState>,
}

impl MenubarItem {
    pub fn new(label: impl Into<SharedString>, menu: &Entity<MenuState>) -> Self {
        Self {
            label: label.into(),
            menu: menu.clone(),
        }
    }

    pub fn label(&self) -> &SharedString {
        &self.label
    }

    pub fn menu(&self) -> &Entity<MenuState> {
        &self.menu
    }
}

/// Fixed top-level menus for a single menubar in one window.
///
/// Each menu keeps its own commands and active descendant. This state owns the
/// roving trigger and switches the open menu on horizontal keys or pointer hover.
/// Use [`Self::open`] for programmatic opening so only one top-level menu opens.
pub struct MenubarState {
    items: Vec<MenubarItem>,
    active: Option<usize>,
    direction: Option<Direction>,
    resolved_direction: Direction,
    pressed_open: bool,
    _menu_subscriptions: Vec<Subscription>,
    focus_subscriptions: Vec<Subscription>,
}

impl MenubarState {
    pub fn new(items: impl IntoIterator<Item = MenubarItem>, cx: &mut Context<Self>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.menu != item.menu),
                "A MenuState can appear only once in a MenubarState"
            );
        }
        let active = items
            .iter()
            .position(|item| !item.menu.read(cx).is_disabled(cx));
        let subscriptions = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                cx.observe(&item.menu, move |state, menu, cx| {
                    if menu.read(cx).is_open(cx) {
                        state.active = Some(index);
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            items,
            active,
            direction: None,
            resolved_direction: Direction::Ltr,
            pressed_open: false,
            _menu_subscriptions: subscriptions,
            focus_subscriptions: Vec::new(),
        }
    }

    /// Overrides the nearest [`crate::DirectionProvider`].
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn items(&self) -> &[MenubarItem] {
        &self.items
    }

    pub fn active_index(&self, cx: &App) -> Option<usize> {
        self.active
            .filter(|&index| !self.items[index].menu.read(cx).is_disabled(cx))
            .or_else(|| {
                self.items
                    .iter()
                    .position(|item| !item.menu.read(cx).is_disabled(cx))
            })
    }

    pub fn open_index(&self, cx: &App) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.menu.read(cx).is_open(cx))
    }

    pub fn focus_handle(&self, index: usize, cx: &App) -> FocusHandle {
        self.items[index].menu.read(cx).focus_handle().clone()
    }

    /// Opens an enabled menu with its first command highlighted.
    pub fn open(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.select(index, true, false, window, cx);
    }

    /// Closes all menus. The focused menu restores its own trigger.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for item in &self.items {
            item.menu.update(cx, |menu, cx| menu.close(window, cx));
        }
        cx.notify();
    }

    fn bind_focus(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        self.resolved_direction = self.direction.unwrap_or(direction);
        if !self.focus_subscriptions.is_empty() {
            return;
        }
        self.focus_subscriptions = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let focus = item.menu.read(cx).focus_handle().clone();
                cx.on_focus(&focus, window, move |_, window, cx| {
                    cx.defer_in(window, move |state, window, cx| {
                        if state.focus_handle(index, cx).is_focused(window)
                            && state.active != Some(index)
                        {
                            state.active = Some(index);
                            cx.notify();
                        }
                    });
                })
            })
            .collect();
    }

    fn select(
        &mut self,
        index: usize,
        open: bool,
        last: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        if item.menu.read(cx).is_disabled(cx) {
            return;
        }
        let menu = item.menu.clone();
        for (other_index, item) in self.items.iter().enumerate() {
            if other_index != index {
                item.menu.update(cx, |menu, cx| menu.close(window, cx));
            }
        }
        self.active = Some(index);
        if open {
            menu.update(cx, |menu, cx| menu.open_at(last, window, cx));
        } else {
            let focus = menu.read(cx).focus_handle().clone();
            focus.focus(window, cx);
        }
        cx.notify();
        window.refresh();
    }

    fn move_trigger(&mut self, right: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let forward = self.resolved_direction.horizontal_forward(right);
        let start = self
            .active_index(cx)
            .unwrap_or(if forward { count - 1 } else { 0 });
        for distance in 1..=count {
            let index = if forward {
                (start + distance) % count
            } else {
                (start + count - distance) % count
            };
            if !self.items[index].menu.read(cx).is_disabled(cx) {
                let open = self.open_index(cx).is_some();
                self.select(index, open, false, window, cx);
                return;
            }
        }
    }

    fn edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        let index = if last {
            self.items
                .iter()
                .rposition(|item| !item.menu.read(cx).is_disabled(cx))
        } else {
            self.items
                .iter()
                .position(|item| !item.menu.read(cx).is_disabled(cx))
        };
        if let Some(index) = index {
            self.select(index, false, false, window, cx);
        }
    }

    fn activate(
        &mut self,
        index: usize,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_open = match event {
            ClickEvent::Mouse(_) => self.pressed_open,
            _ => self.open_index(cx) == Some(index),
        };
        if was_open {
            self.close(window, cx);
        } else {
            self.open(index, window, cx);
        }
    }
}

/// Semantic menubar scope. Supply [`MenubarMenu`] children in collection order.
#[derive(IntoElement)]
pub struct Menubar {
    state: Entity<MenubarState>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Menubar {
    pub fn new(state: &Entity<MenubarState>) -> Self {
        Self {
            state: state.clone(),
            label: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl ParentElement for Menubar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Menubar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Menubar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let direction = Direction::current(cx);
        self.state
            .update(cx, |state, cx| state.bind_focus(direction, window, cx));
        div()
            .id(("menubar", self.state.entity_id()))
            .test_support()
            .role(Role::MenuBar)
            .when_some(self.label, |this, label| this.aria_label(label))
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// One top-level trigger and its command surface.
///
/// The trigger's role, focus, disabled state, expansion, and activation are owned
/// here. `content` supplies the associated menu's [`crate::MenuOption`] children.
#[derive(IntoElement)]
pub struct MenubarMenu {
    state: Entity<MenubarState>,
    index: usize,
    trigger: Button,
    content: Option<AnyElement>,
    anchor: Anchor,
    offset: Pixels,
    style: StyleRefinement,
}

impl MenubarMenu {
    pub fn new(state: &Entity<MenubarState>, index: usize) -> Self {
        Self {
            state: state.clone(),
            index,
            trigger: Button::new(("menubar-trigger", index)),
            content: None,
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
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }
    pub fn offset(mut self, offset: Pixels) -> Self {
        self.offset = offset;
        self
    }
}

impl Styled for MenubarMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MenubarMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let item = state
            .items
            .get(self.index)
            .expect("MenubarMenu index is out of bounds");
        let menu = item.menu.clone();
        let label = item.label.clone();
        let focus = menu.read(cx).focus_handle().clone();
        let popover = menu.read(cx).popover.clone();
        let disabled = menu.read(cx).is_disabled(cx);
        let open = menu.read(cx).is_open(cx);
        let tab_stop = state.active_index(cx) == Some(self.index);
        let direction = state.resolved_direction;
        let index = self.index;
        let press = self.state.clone();
        let activate = self.state.clone();
        let hover = self.state.clone();
        let left = self.state.clone();
        let right = self.state.clone();
        let first = self.state.clone();
        let last = self.state.clone();
        let down = self.state.clone();
        let up = self.state.clone();
        let content_left = self.state.clone();
        let content_right = self.state.clone();
        let trigger = self
            .trigger
            .role(Role::MenuItem)
            .accessibility_label(label.clone())
            .track_focus(&focus)
            .focusable(true)
            .tab_stop(tab_stop)
            .disabled(disabled)
            .aria_expanded(open)
            .key_context(TRIGGER_CONTEXT)
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                press.update(cx, |state, cx| {
                    state.pressed_open = state.open_index(cx) == Some(index)
                });
            })
            .on_click(move |event, window, cx| {
                activate.update(cx, |state, cx| state.activate(index, event, window, cx));
            })
            .on_hover(move |hovered, window, cx| {
                if *hovered {
                    hover.update(cx, |state, cx| {
                        if state.open_index(cx).is_some() && state.open_index(cx) != Some(index) {
                            state.open(index, window, cx);
                        }
                    });
                }
            })
            .on_action(move |_: &SelectLeft, window, cx| {
                left.update(cx, |state, cx| state.move_trigger(false, window, cx))
            })
            .on_action(move |_: &SelectRight, window, cx| {
                right.update(cx, |state, cx| state.move_trigger(true, window, cx))
            })
            .on_action(move |_: &SelectFirst, window, cx| {
                first.update(cx, |state, cx| state.edge(false, window, cx))
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| state.edge(true, window, cx))
            })
            .on_action(move |_: &SelectDown, window, cx| {
                down.update(cx, |state, cx| state.select(index, true, false, window, cx))
            })
            .on_action(move |_: &SelectUp, window, cx| {
                up.update(cx, |state, cx| state.select(index, true, true, window, cx))
            });
        let content = div()
            .key_context(CONTENT_CONTEXT)
            .on_action(move |_: &SelectLeft, window, cx| {
                content_left.update(cx, |state, cx| state.move_trigger(false, window, cx))
            })
            .on_action(move |_: &SelectRight, window, cx| {
                content_right.update(cx, |state, cx| state.move_trigger(true, window, cx))
            })
            .child(crate::menu::menu_content_with_direction(
                &menu,
                Some(label),
                self.content,
                direction,
                cx,
            ));
        Popover::new(ElementId::from(("menubar-menu", index)), &popover)
            .supplied_trigger(trigger)
            .content_role(None)
            .anchor(self.anchor)
            .offset(self.offset)
            .content(content)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MenuEvent, MenuItem, MenuOption, MenuSubmenu, Root};
    use gpui::{
        AppContext as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext,
        VisualTestContext,
    };

    struct Harness {
        bar: Entity<MenubarState>,
        nested: Entity<MenuState>,
        before: FocusHandle,
        after: FocusHandle,
        commands: Vec<SharedString>,
        _subscriptions: Vec<Subscription>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .size_full()
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .h(px(30.))
                        .child("Before"),
                )
                .child(
                    Menubar::new(&self.bar)
                        .accessibility_label("Application")
                        .flex()
                        .gap(px(4.))
                        .children(self.bar.read(cx).items().iter().enumerate().map(
                            |(index, item)| {
                                MenubarMenu::new(&self.bar, index)
                                    .trigger(|button| {
                                        button
                                            .debug_selector(move || format!("bar-{index}").into())
                                            .w(px(90.))
                                            .h(px(30.))
                                            .child(item.label().clone())
                                    })
                                    .content(div().flex().flex_col().w(px(160.)).children(
                                        item.menu().read(cx).items().iter().enumerate().map(
                                            |(index, command)| {
                                                if let Some(child) = command.submenu_state() {
                                                    MenuSubmenu::new(item.menu(), index)
                                                        .trigger(|option| option.h(px(30.)).child(
                                                            div().debug_selector(|| "submenu-row".into())
                                                                .size_full().child(command.label().clone())))
                                                        .content(div().flex().flex_col().w(px(150.)).children(
                                                            child.read(cx).items().iter().enumerate().map(|(index, item)|
                                                                MenuOption::new(child, index).h(px(30.)).child(
                                                                    div().debug_selector(move || format!("submenu-command-{index}").into())
                                                                        .size_full().child(item.label().clone())))))
                                                        .into_any_element()
                                                } else {
                                                    MenuOption::new(item.menu(), index)
                                                        .h(px(30.))
                                                        .child(command.label().clone()).into_any_element()
                                                }
                                            },
                                        ),
                                    ))
                            },
                        )),
                )
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(30.))
                        .child("After"),
                )
        }
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

    #[gpui::test]
    fn coordinates_roving_focus_open_menus_pointer_switching_and_commands(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let nested = cx.new(|cx| {
                    MenuState::new(
                        [
                            MenuItem::new("nested-first", "First nested command"),
                            MenuItem::new("nested-disabled", "Disabled nested command")
                                .disabled(true),
                            MenuItem::new("nested-last", "Last nested command"),
                        ],
                        cx,
                    )
                });
                let menus: Vec<_> = ["File", "Edit", "View"]
                    .into_iter()
                    .map(|label| {
                        cx.new(|cx| {
                            MenuState::new(
                                [
                                    MenuItem::new(format!("{label}-first"), "First"),
                                    MenuItem::new("disabled", "Disabled").disabled(true),
                                    if label == "File" {
                                        MenuItem::new("more", "More").submenu(&nested)
                                    } else {
                                        MenuItem::new(format!("{label}-last"), "Last")
                                    },
                                ],
                                cx,
                            )
                        })
                    })
                    .collect();
                menus[1].update(cx, |menu, cx| menu.set_disabled(true, window, cx));
                let bar = cx.new(|cx| {
                    MenubarState::new(
                        ["File", "Edit", "View"]
                            .into_iter()
                            .zip(&menus)
                            .map(|(label, menu)| MenubarItem::new(label, menu)),
                        cx,
                    )
                });
                let mut subscriptions = vec![cx.observe(&bar, |_, _, cx| cx.notify())];
                for menu in &menus {
                    subscriptions.push(cx.subscribe(menu, |this: &mut Harness, _, event, cx| {
                        let MenuEvent::Action(value) = event;
                        this.commands.push(value.clone());
                        cx.notify();
                    }));
                }
                Harness {
                    bar,
                    nested,
                    before: cx.focus_handle(),
                    after: cx.focus_handle(),
                    commands: Vec::new(),
                    _subscriptions: subscriptions,
                }
            });
            Root::new(view, window, cx)
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
        press(cx, "tab");
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .bar
                    .read(cx)
                    .focus_handle(0, cx)
                    .is_focused(window)
            )
        });
        press(cx, "right");
        cx.update(|window, cx| {
            let bar = view.read(cx).bar.read(cx);
            assert_eq!(bar.active_index(cx), Some(2));
            assert_eq!(bar.open_index(cx), None);
            assert!(bar.focus_handle(2, cx).is_focused(window));
        });
        press(cx, "tab");
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        press(cx, "shift-tab");
        press(cx, "up");
        cx.update(|_, cx| {
            let bar = view.read(cx).bar.read(cx);
            assert_eq!(bar.open_index(cx), Some(2));
            assert_eq!(bar.items[2].menu.read(cx).highlighted_index(), Some(2));
        });
        press(cx, "left");
        cx.update(|window, cx| {
            let bar = view.read(cx).bar.read(cx);
            assert_eq!(bar.open_index(cx), Some(0));
            assert!(!bar.items[2].menu.read(cx).is_open(cx));
            assert!(
                bar.items[0]
                    .menu
                    .read(cx)
                    .content_focus_handle()
                    .is_focused(window)
            );
            #[cfg(feature = "test-support")]
            {
                let scope = crate::test_support::find(
                    window,
                    &[],
                    &("menubar", view.read(cx).bar.entity_id()).into(),
                )
                .unwrap();
                assert_eq!(scope.role(), Some(Role::MenuBar));
                let trigger =
                    crate::test_support::find(window, &[], &("menubar-trigger", 0usize).into())
                        .unwrap();
                assert_eq!(trigger.role(), Some(Role::MenuItem));
                assert_eq!(trigger.expanded(), Some(true));
            }
        });
        press(cx, "enter");
        cx.update(|window, cx| {
            let harness = view.read(cx);
            let bar = harness.bar.read(cx);
            assert_eq!(harness.commands, vec![SharedString::from("File-first")]);
            assert_eq!(bar.open_index(cx), None);
            assert!(bar.focus_handle(0, cx).is_focused(window));
        });
        // RTL reverses physical arrows without reversing command Up/Down order.
        cx.update(|_, cx| {
            view.read(cx).bar.clone().update(cx, |bar, cx| {
                bar.direction = Some(Direction::Rtl);
                cx.notify();
            })
        });
        settle(cx);
        press(cx, "right");
        cx.update(|_, cx| assert_eq!(view.read(cx).bar.read(cx).active_index(cx), Some(2)));
        press(cx, "down");
        let file = cx.debug_bounds("bar-0").unwrap().center();
        cx.simulate_mouse_move(file, None, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).bar.read(cx).open_index(cx), Some(0)));
        cx.simulate_click(file, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).bar.read(cx).open_index(cx), None));
        cx.simulate_click(file, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).bar.read(cx).open_index(cx), Some(0)));
        press(cx, "escape");
        cx.update(|window, cx| {
            let bar = view.read(cx).bar.read(cx);
            assert_eq!(bar.open_index(cx), None);
            assert!(bar.focus_handle(0, cx).is_focused(window));
        });
        // Submenus inherit the bar's explicit RTL direction, and closing a
        // child leaves the root menu and its active descendant intact.
        press(cx, "down");
        press(cx, "end");
        press(cx, "left");
        cx.update(|window, cx| {
            let harness = view.read(cx);
            assert!(harness.nested.read(cx).is_open(cx));
            assert!(
                harness
                    .nested
                    .read(cx)
                    .content_focus_handle()
                    .is_focused(window)
            );
            assert_eq!(harness.bar.read(cx).open_index(cx), Some(0));
        });
        press(cx, "right");
        cx.update(|window, cx| {
            let harness = view.read(cx);
            let bar = harness.bar.read(cx);
            assert!(!harness.nested.read(cx).is_open(cx));
            assert!(
                bar.items[0]
                    .menu
                    .read(cx)
                    .content_focus_handle()
                    .is_focused(window)
            );
            assert_eq!(bar.items[0].menu.read(cx).highlighted_index(), Some(2));
        });
        // Native accessibility clients can focus the submenu row itself.
        press(cx, "home");
        cx.update(|window, cx| {
            let focus = view.read(cx).nested.read(cx).focus_handle().clone();
            focus.focus(window, cx);
        });
        settle(cx);
        press(cx, "left");
        cx.update(|_, cx| assert!(view.read(cx).nested.read(cx).is_open(cx)));
        press(cx, "escape");
        cx.update(|_, cx| {
            let harness = view.read(cx);
            assert!(!harness.nested.read(cx).is_open(cx));
            assert_eq!(harness.bar.read(cx).open_index(cx), Some(0));
        });
        press(cx, "home");
        cx.update(|window, cx| {
            let focus = view.read(cx).nested.read(cx).focus_handle().clone();
            focus.focus(window, cx);
        });
        settle(cx);
        press(cx, "enter");
        cx.update(|_, cx| {
            let harness = view.read(cx);
            assert!(harness.nested.read(cx).is_open(cx));
            assert_eq!(harness.commands, ["File-first"]);
        });
        press(cx, "down");
        press(cx, "enter");
        cx.update(|window, cx| {
            let harness = view.read(cx);
            let bar = harness.bar.read(cx);
            assert_eq!(harness.commands, ["File-first", "nested-last"]);
            assert!(!harness.nested.read(cx).is_open(cx));
            assert_eq!(bar.open_index(cx), None);
            assert!(bar.focus_handle(0, cx).is_focused(window));
        });
        press(cx, "down");
        press(cx, "end");
        press(cx, "left");
        press(cx, "tab");
        cx.update(|window, cx| {
            let harness = view.read(cx);
            assert!(!harness.nested.read(cx).is_open(cx));
            assert_eq!(harness.bar.read(cx).open_index(cx), None);
            assert!(harness.after.is_focused(window));
        });
        press(cx, "shift-tab");
        press(cx, "down");
        let row = cx.debug_bounds("submenu-row").unwrap().center();
        cx.simulate_mouse_move(row, None, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert!(view.read(cx).nested.read(cx).is_open(cx)));
        let command = cx.debug_bounds("submenu-command-0").unwrap().center();
        cx.simulate_click(command, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let harness = view.read(cx);
            assert_eq!(
                harness.commands,
                ["File-first", "nested-last", "nested-first"]
            );
            assert_eq!(harness.bar.read(cx).open_index(cx), None);
            assert!(harness.bar.read(cx).focus_handle(0, cx).is_focused(window));
        });
    }
}
