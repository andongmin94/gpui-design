//! Navigation links and coordinated disclosure panels, without command-menu roles.

use gpui::{
    Anchor, AnyElement, App, AppContext as _, ClickEvent, Context, ElementId, Entity, EventEmitter,
    FocusHandle, InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    Button, Direction, DirectionProvider, Popover, PopoverState, StyledExt as _,
    TestSupportExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight},
};

const CONTEXT: &str = "NavigationMenuTrigger";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
    ]);
}

/// One top-level destination or disclosure trigger in a fixed collection.
#[derive(Clone)]
pub struct NavigationMenuItem {
    value: SharedString,
    label: SharedString,
    popup: bool,
    disabled: bool,
    initial_focus: Option<FocusHandle>,
}

impl NavigationMenuItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            popup: false,
            disabled: false,
            initial_focus: None,
        }
    }

    /// Makes this item a disclosure trigger instead of a destination link.
    pub fn with_popup(mut self) -> Self {
        self.popup = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Gives a panel a known first focus target. Otherwise its content scope
    /// receives focus and Tab enters the first interactive child.
    pub fn initial_focus(mut self, focus: &FocusHandle) -> Self {
        self.initial_focus = Some(focus.clone());
        self
    }

    pub fn value(&self) -> &SharedString {
        &self.value
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn has_popup(&self) -> bool {
        self.popup
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Destination activation after closing any open panel and restoring focus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationMenuEvent {
    Navigate(SharedString),
}

/// Retained navigation for one window. Applications own routing and current-page state.
///
/// Top-level triggers share one Tab stop and support wrapping horizontal arrows,
/// Home and End. Enter/Space activates a link or toggles a disclosure; ArrowDown
/// opens a disclosure. Panel descendants keep their ordinary keyboard behavior
/// and Tab order. Escape, outside input and focus departure use [`PopoverState`].
pub struct NavigationMenuState {
    items: Vec<NavigationMenuItem>,
    popovers: Vec<Entity<PopoverState>>,
    active: Option<usize>,
    direction: Option<Direction>,
    resolved_direction: Direction,
    pressed_open: bool,
    _popover_subscriptions: Vec<Subscription>,
    focus_subscriptions: Vec<Subscription>,
}

impl NavigationMenuState {
    pub fn new(
        items: impl IntoIterator<Item = NavigationMenuItem>,
        cx: &mut Context<Self>,
    ) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        for (index, item) in items.iter().enumerate() {
            assert!(
                items[..index].iter().all(|other| other.value != item.value),
                "NavigationMenuItem values must be unique"
            );
        }
        let active = items.iter().position(|item| !item.disabled);
        let popovers: Vec<_> = items
            .iter()
            .map(|item| {
                cx.new(|cx| {
                    let popover = PopoverState::new(cx);
                    if let Some(focus) = &item.initial_focus {
                        popover.with_initial_focus(focus)
                    } else {
                        popover
                    }
                })
            })
            .collect();
        let subscriptions = popovers
            .iter()
            .enumerate()
            .map(|(index, popover)| {
                cx.observe(popover, move |state, popover, cx| {
                    if popover.read(cx).is_open() {
                        state.active = Some(index);
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            items,
            popovers,
            active,
            direction: None,
            resolved_direction: Direction::Ltr,
            pressed_open: false,
            _popover_subscriptions: subscriptions,
            focus_subscriptions: Vec::new(),
        }
    }

    /// Overrides the nearest [`crate::DirectionProvider`].
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }
    pub fn items(&self) -> &[NavigationMenuItem] {
        &self.items
    }
    pub fn active_index(&self) -> Option<usize> {
        self.active
    }
    pub fn open_index(&self, cx: &App) -> Option<usize> {
        self.popovers
            .iter()
            .position(|popover| popover.read(cx).is_open())
    }
    pub fn focus_handle(&self, index: usize, cx: &App) -> FocusHandle {
        self.popovers[index].read(cx).focus_handle().clone()
    }
    pub fn content_focus_handle(&self, index: usize, cx: &App) -> FocusHandle {
        self.popovers[index].read(cx).content_focus_handle().clone()
    }

    pub fn set_disabled(
        &mut self,
        index: usize,
        disabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.items.get_mut(index) else {
            return;
        };
        if item.disabled == disabled {
            return;
        }
        item.disabled = disabled;
        self.popovers[index].update(cx, |popover, cx| popover.set_disabled(disabled, window, cx));
        if self.active.is_none_or(|index| self.items[index].disabled) {
            self.active = self.items.iter().position(|item| !item.disabled);
        }
        cx.notify();
    }

    /// Opens a disclosure, closing its peers. Link-only and disabled items do nothing.
    pub fn open(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .items
            .get(index)
            .is_none_or(|item| item.disabled || !item.popup)
        {
            return;
        }
        self.close_others(Some(index), window, cx);
        self.active = Some(index);
        self.popovers[index].update(cx, |popover, cx| popover.open(window, cx));
        cx.notify();
        window.refresh();
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_others(None, window, cx);
        cx.notify();
    }

    /// Reports a destination. Use this for custom link presentations in a panel.
    pub fn navigate(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close(window, cx);
        cx.emit(NavigationMenuEvent::Navigate(value.into()));
    }

    fn close_others(&self, keep: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        for (index, popover) in self.popovers.iter().enumerate() {
            if keep != Some(index) {
                popover.update(cx, |popover, cx| popover.close(window, cx));
            }
        }
    }

    fn bind_focus(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        self.resolved_direction = self.direction.unwrap_or(direction);
        if !self.focus_subscriptions.is_empty() {
            return;
        }
        self.focus_subscriptions = self
            .popovers
            .iter()
            .enumerate()
            .map(|(index, popover)| {
                let focus = popover.read(cx).focus_handle().clone();
                cx.on_focus(&focus, window, move |_, window, cx| {
                    cx.defer_in(window, move |state, window, cx| {
                        if state.focus_handle(index, cx).is_focused(window)
                            && !state.items[index].disabled
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

    fn focus_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.items[index].disabled {
            return;
        }
        let switch_open = self.open_index(cx).is_some() && self.items[index].popup;
        self.close_others(Some(index), window, cx);
        self.active = Some(index);
        self.focus_handle(index, cx).focus(window, cx);
        if switch_open {
            self.open(index, window, cx);
        }
        cx.notify();
        window.refresh();
    }

    fn move_trigger(
        &mut self,
        start: usize,
        right: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let count = self.items.len();
        let forward = self.resolved_direction.horizontal_forward(right);
        for distance in 1..=count {
            let index = if forward {
                (start + distance) % count
            } else {
                (start + count - distance) % count
            };
            if !self.items[index].disabled {
                self.focus_index(index, window, cx);
                return;
            }
        }
    }

    fn edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        let index = if last {
            self.items.iter().rposition(|item| !item.disabled)
        } else {
            self.items.iter().position(|item| !item.disabled)
        };
        if let Some(index) = index {
            self.focus_index(index, window, cx);
        }
    }

    fn activate(
        &mut self,
        index: usize,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.items[index].disabled {
            return;
        }
        self.active = Some(index);
        if self.items[index].popup {
            let was_open = match event {
                ClickEvent::Mouse(_) => self.pressed_open,
                _ => self.open_index(cx) == Some(index),
            };
            if was_open {
                self.close(window, cx);
            } else {
                self.open(index, window, cx);
            }
        } else {
            let value = self.items[index].value.clone();
            self.navigate(value, window, cx);
        }
    }
}

impl EventEmitter<NavigationMenuEvent> for NavigationMenuState {}

/// A named navigation landmark. Supply [`NavigationMenuEntry`] children in order.
#[derive(IntoElement)]
pub struct NavigationMenu {
    state: Entity<NavigationMenuState>,
    label: SharedString,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl NavigationMenu {
    pub fn new(state: &Entity<NavigationMenuState>) -> Self {
        Self {
            state: state.clone(),
            label: "Navigation".into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

impl ParentElement for NavigationMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl Styled for NavigationMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for NavigationMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let direction = Direction::current(cx);
        self.state
            .update(cx, |state, cx| state.bind_focus(direction, window, cx));
        div()
            .id(("navigation-menu", self.state.entity_id()))
            .test_support()
            .role(Role::Navigation)
            .aria_label(self.label)
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// A top-level link or disclosure panel. Presentation belongs to the caller.
///
/// Disclosure content can contain ordinary links and controls; it is not exposed
/// as a command menu. Pointer hover switches disclosures only while one is open.
#[derive(IntoElement)]
pub struct NavigationMenuEntry {
    state: Entity<NavigationMenuState>,
    index: usize,
    trigger: Button,
    content: Option<AnyElement>,
    current: bool,
    anchor: Anchor,
    offset: Pixels,
    style: StyleRefinement,
}

impl NavigationMenuEntry {
    pub fn new(state: &Entity<NavigationMenuState>, index: usize) -> Self {
        Self {
            state: state.clone(),
            index,
            trigger: Button::new(("navigation-menu-trigger", index)),
            content: None,
            current: false,
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
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
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
impl Styled for NavigationMenuEntry {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for NavigationMenuEntry {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let item = state
            .items
            .get(self.index)
            .expect("NavigationMenuEntry index is out of bounds");
        let popup = item.popup;
        let label = item.label.clone();
        let disabled = item.disabled;
        let popover = state.popovers[self.index].clone();
        let focus = state.focus_handle(self.index, cx);
        let open = popover.read(cx).is_open();
        let tab_stop = state.active == Some(self.index);
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
        let trigger = self
            .trigger
            .role(if popup { Role::Button } else { Role::Link })
            .accessibility_label(label.clone())
            .track_focus(&focus)
            .focusable(true)
            .tab_stop(tab_stop)
            .disabled(disabled)
            .current(!popup && self.current)
            .when(popup, |this| this.aria_expanded(open))
            .key_context(CONTEXT)
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                press.update(cx, |state, cx| {
                    state.pressed_open = state.open_index(cx) == Some(index)
                });
            })
            .on_click(move |event, window, cx| {
                activate.update(cx, |state, cx| state.activate(index, event, window, cx))
            })
            .on_hover(move |hovered, window, cx| {
                if *hovered && popup && !disabled {
                    hover.update(cx, |state, cx| {
                        if state.open_index(cx).is_some() && state.open_index(cx) != Some(index) {
                            state.open(index, window, cx);
                        }
                    });
                }
            })
            .on_action(move |_: &SelectLeft, window, cx| {
                left.update(cx, |state, cx| state.move_trigger(index, false, window, cx))
            })
            .on_action(move |_: &SelectRight, window, cx| {
                right.update(cx, |state, cx| state.move_trigger(index, true, window, cx))
            })
            .on_action(move |_: &SelectFirst, window, cx| {
                first.update(cx, |state, cx| state.edge(false, window, cx))
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| state.edge(true, window, cx))
            })
            .on_action(move |_: &SelectDown, window, cx| {
                down.update(cx, |state, cx| state.open(index, window, cx))
            });
        if popup {
            Popover::new(ElementId::from(("navigation-menu-entry", index)), &popover)
                .supplied_trigger(trigger)
                .content_role(Some(Role::Group))
                .accessibility_label(label)
                .anchor(self.anchor)
                .offset(self.offset)
                .content(DirectionProvider::new(direction).children(self.content))
                .refine_style(&self.style)
                .into_any_element()
        } else {
            div()
                .refine_style(&self.style)
                .child(trigger)
                .into_any_element()
        }
    }
}

/// A panel link that closes navigation before reporting its destination.
#[derive(IntoElement)]
pub struct NavigationMenuLink {
    state: Entity<NavigationMenuState>,
    value: SharedString,
    button: Button,
}

impl NavigationMenuLink {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<NavigationMenuState>,
        value: impl Into<SharedString>,
    ) -> Self {
        Self {
            state: state.clone(),
            value: value.into(),
            button: Button::new(id),
        }
    }
    pub fn button(mut self, build: impl FnOnce(Button) -> Button) -> Self {
        self.button = build(self.button);
        self
    }
    pub fn current(mut self, current: bool) -> Self {
        self.button = self.button.current(current);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.button = self.button.disabled(disabled);
        self
    }
}
impl Styled for NavigationMenuLink {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}
impl ParentElement for NavigationMenuLink {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(elements);
    }
}
impl RenderOnce for NavigationMenuLink {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.button.role(Role::Link).on_click(move |_, window, cx| {
            self.state.update(cx, |state, cx| {
                state.navigate(self.value.clone(), window, cx)
            });
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Root;
    use gpui::{
        KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext, VisualTestContext,
    };

    struct Harness {
        navigation: Entity<NavigationMenuState>,
        first_link: FocusHandle,
        after: FocusHandle,
        destinations: Vec<SharedString>,
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
                    NavigationMenu::new(&self.navigation)
                        .accessibility_label("Documentation")
                        .flex()
                        .gap(px(4.))
                        .children(self.navigation.read(cx).items().iter().enumerate().map(
                            |(index, item)| {
                                NavigationMenuEntry::new(&self.navigation, index)
                                    .current(index == 0)
                                    .trigger(|button| {
                                        button
                                            .debug_selector(move || {
                                                format!("navigation-{index}").into()
                                            })
                                            .w(px(90.))
                                            .h(px(30.))
                                            .child(item.label().clone())
                                    })
                                    .content(
                                        div().flex().flex_col().w(px(180.)).child(
                                            NavigationMenuLink::new(
                                                ("navigation-link", index),
                                                &self.navigation,
                                                format!("page-{index}"),
                                            )
                                            .button(|button| {
                                                button
                                                    .track_focus(&self.first_link)
                                                    .debug_selector(|| "panel-link".into())
                                            })
                                            .h(px(30.))
                                            .child("Read this guide"),
                                        ),
                                    )
                            },
                        )),
                )
                .child(
                    Button::new("after-navigation")
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
    fn navigation_disclosures_links_and_disabled_roving_entries_keep_distinct_semantics(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let first_link = cx.focus_handle();
                let navigation = cx.new(|cx| {
                    NavigationMenuState::new(
                        [
                            NavigationMenuItem::new("home", "Home"),
                            NavigationMenuItem::new("disabled", "Unavailable").disabled(true),
                            NavigationMenuItem::new("guides", "Guides")
                                .with_popup()
                                .initial_focus(&first_link),
                            NavigationMenuItem::new("reference", "Reference")
                                .with_popup()
                                .initial_focus(&first_link),
                        ],
                        cx,
                    )
                });
                let subscriptions = vec![
                    cx.observe(&navigation, |_, _, cx| cx.notify()),
                    cx.subscribe_in(
                        &navigation,
                        window,
                        |this: &mut Harness, navigation, event, _, cx| {
                            assert_eq!(navigation.read(cx).open_index(cx), None);
                            let NavigationMenuEvent::Navigate(value) = event;
                            this.destinations.push(value.clone());
                            cx.notify();
                        },
                    ),
                ];
                Harness {
                    navigation,
                    first_link,
                    after: cx.focus_handle(),
                    destinations: Vec::new(),
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
            let focus = view.read(cx).navigation.read(cx).focus_handle(0, cx);
            focus.focus(window, cx);
        });
        settle(cx);
        #[cfg(feature = "test-support")]
        cx.update(|window, cx| {
            let scope = crate::test_support::find(
                window,
                &[],
                &("navigation-menu", view.read(cx).navigation.entity_id()).into(),
            )
            .unwrap();
            assert_eq!(scope.role(), Some(Role::Navigation));
            assert_eq!(scope.label(), Some("Documentation"));
            let link =
                crate::test_support::find(window, &[], &("navigation-menu-trigger", 0usize).into())
                    .unwrap();
            assert_eq!(link.role(), Some(Role::Link));
            assert_eq!(link.expanded(), None);
            let disclosure =
                crate::test_support::find(window, &[], &("navigation-menu-trigger", 2usize).into())
                    .unwrap();
            assert_eq!(disclosure.role(), Some(Role::Button));
            assert_eq!(disclosure.expanded(), Some(false));
        });
        press(cx, "enter");
        cx.update(|_, cx| assert_eq!(view.read(cx).destinations, vec![SharedString::from("home")]));
        press(cx, "right");
        cx.update(|window, cx| {
            assert!(
                view.read(cx)
                    .navigation
                    .read(cx)
                    .focus_handle(2, cx)
                    .is_focused(window)
            )
        });
        press(cx, "down");
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).navigation.read(cx).open_index(cx), Some(2));
            assert!(view.read(cx).first_link.is_focused(window));
        });
        press(cx, "enter");
        cx.update(|window, cx| {
            let harness = view.read(cx);
            assert_eq!(
                harness.destinations,
                vec![SharedString::from("home"), SharedString::from("page-2")]
            );
            assert!(
                harness
                    .navigation
                    .read(cx)
                    .focus_handle(2, cx)
                    .is_focused(window)
            );
        });
        press(cx, "down");
        let reference = cx.debug_bounds("navigation-3").unwrap().center();
        cx.simulate_mouse_move(reference, None, Modifiers::default());
        settle(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).navigation.read(cx).open_index(cx), Some(3)));
        press(cx, "escape");
        cx.update(|window, cx| {
            let state = view.read(cx).navigation.read(cx);
            assert_eq!(state.open_index(cx), None);
            assert!(state.focus_handle(3, cx).is_focused(window));
        });
        press(cx, "tab");
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        press(cx, "shift-tab");
        cx.update(|window, cx| {
            let navigation = view.read(cx).navigation.clone();
            navigation.update(cx, |state, cx| state.set_disabled(3, true, window, cx));
        });
        settle(cx);
        // Disabling the focused trigger changes the roving Tab entry, but
        // arrows still start from the control that actually owns focus.
        for (key, index) in [("right", 0), ("left", 2), ("home", 0)] {
            press(cx, key);
            cx.update(|window, cx| {
                assert!(
                    view.read(cx)
                        .navigation
                        .read(cx)
                        .focus_handle(index, cx)
                        .is_focused(window)
                )
            });
        }
    }
}
