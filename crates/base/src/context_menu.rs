//! A focusable context target for the shared command menu.

use gpui::{
    AccessibleAction, AnyElement, App, ElementId, Entity, InteractiveElement, Interactivity,
    IntoElement, KeyBinding, MouseButton, ParentElement as _, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, actions, div,
    prelude::FluentBuilder as _,
};

use crate::{MenuState, Popover, TestSupportExt as _, a11y_disabled::A11yDisabledExt as _};

const CONTEXT: &str = "ContextMenuTarget";

actions!(ui, [OpenContextMenu]);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-f10", OpenContextMenu, Some(CONTEXT)),
        KeyBinding::new("menu", OpenContextMenu, Some(CONTEXT)),
    ]);
}

/// A focusable region that opens a command menu on a native secondary click.
///
/// Right-click positions the popup at that window coordinate. Shift-F10, the
/// Menu key, the native ShowContextMenu accessibility action, and imperative
/// [`MenuState::open`] position it below the target. Left-click only focuses the
/// target; it never opens the menu. Commands, active-descendant navigation,
/// disabled state, dismissal, and return focus use the same MenuState and
/// Popover lifecycle as DropdownMenu.
///
/// Supply non-interactive target content and [`crate::MenuOption`]
/// children in collection order. `target_label` names the focusable region;
/// `accessibility_label` names its menu. Styles and interactive style builders
/// apply to the target. Keep this component mounted while closed and use a
/// distinct MenuState for each mounted target. Options can expose an
/// application-controlled boolean through [`crate::MenuOption::checked`].
/// Touch long-press, submenus, mixed checked values, and radio groups are not
/// provided.
#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<MenuState>,
    target_label: Option<SharedString>,
    label: Option<SharedString>,
    content: Option<AnyElement>,
}

impl ContextMenu {
    pub fn new(id: impl Into<ElementId>, state: &Entity<MenuState>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            id,
            state: state.clone(),
            target_label: None,
            label: None,
            content: None,
        }
    }

    pub fn target(mut self, content: impl IntoElement) -> Self {
        self.base = self.base.child(content);
        self
    }

    /// Names the focusable region whose commands are being shown.
    pub fn target_label(mut self, label: impl Into<SharedString>) -> Self {
        self.target_label = Some(label.into());
        self
    }

    /// Names the menu itself.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for ContextMenu {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ContextMenu {}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let open = state.is_open(cx);
        let disabled = state.is_disabled(cx);
        let focus = state.focus_handle().clone();
        let popover = state.popover.clone();
        let position = state.position();
        let left = self.state.clone();
        let right = self.state.clone();
        let keyboard = self.state.clone();
        let accessible = self.state.clone();
        let content = crate::menu::menu_content(&self.state, self.label, self.content, cx);
        let target = self
            .base
            .role(Role::Region)
            .aria_expanded(open)
            .aria_keyshortcuts("Shift+F10")
            .when_some(self.target_label, |this, label| this.aria_label(label))
            .key_context(CONTEXT)
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(!disabled))
            })
            .when(!disabled, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    let state = left.read(cx);
                    if !state.is_disabled(cx) {
                        let focus = state.focus_handle().clone();
                        focus.focus(window, cx);
                    }
                })
                .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    right.update(cx, |state, cx| {
                        if !state.is_disabled(cx) {
                            state.open_at_position(event.position, window, cx);
                            cx.stop_propagation();
                        }
                    });
                })
                .on_action(move |_: &OpenContextMenu, window, cx| {
                    keyboard.update(cx, |state, cx| state.open(window, cx));
                })
                .on_a11y_action(
                    AccessibleAction::ShowContextMenu,
                    move |_, window, cx| {
                        accessible.update(cx, |state, cx| state.open(window, cx));
                    },
                )
            })
            .a11y_disabled(disabled);

        Popover::new((self.id, "context-menu-host"), &popover)
            .supplied_trigger(target)
            .content_role(None)
            .when_some(position, |popover, position| popover.at_position(position))
            .content(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
        Render, Subscription, TestAppContext, VisualTestContext, point, px, size,
    };

    use crate::{Button, MenuEvent, MenuItem, MenuOption, Root};

    struct Harness {
        menu: Entity<MenuState>,
        before: FocusHandle,
        after: FocusHandle,
        shown: bool,
        commands: Vec<SharedString>,
        _subscriptions: Vec<Subscription>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let options = self
                .menu
                .read(cx)
                .items()
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    MenuOption::new(&self.menu, index).h(px(28.)).child(
                        div()
                            .debug_selector(move || format!("context-command-{index}"))
                            .size_full()
                            .child(item.label().clone()),
                    )
                });
            div()
                .size_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    Button::new("before")
                        .debug_selector(|| "before".into())
                        .track_focus(&self.before)
                        .h(px(28.))
                        .child("Before"),
                )
                .when(self.shown, |this| {
                    this.child(
                        ContextMenu::new("context-target", &self.menu)
                            .debug_selector(|| "context-target".into())
                            .target_label("Recording summary")
                            .accessibility_label("Recording actions")
                            .h(px(220.))
                            .w_full()
                            .target("Recording summary")
                            .content(
                                div()
                                    .debug_selector(|| "context-surface".into())
                                    .w(px(180.))
                                    .flex()
                                    .flex_col()
                                    .children(options),
                            ),
                    )
                })
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(28.))
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

    fn harness(cx: &mut TestAppContext) -> (&mut VisualTestContext, Entity<Harness>) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|cx| {
                let menu = cx.new(|cx| {
                    MenuState::new(
                        [
                            MenuItem::new("disabled", "Disabled").disabled(true),
                            MenuItem::new("open", "Open"),
                            MenuItem::new("share", "Share"),
                        ],
                        cx,
                    )
                });
                let subscriptions = vec![
                    cx.observe(&menu, |_, _, cx| cx.notify()),
                    cx.subscribe_in(
                        &menu,
                        window,
                        |this: &mut Harness, menu, event, window, cx| {
                            let MenuEvent::Action(value) = event;
                            assert!(!menu.read(cx).is_open(cx));
                            assert!(menu.read(cx).focus_handle().is_focused(window));
                            this.commands.push(value.clone());
                        },
                    ),
                ];
                Harness {
                    menu,
                    before: cx.focus_handle(),
                    after: cx.focus_handle(),
                    shown: true,
                    commands: Vec::new(),
                    _subscriptions: subscriptions,
                }
            });
            Root::new(content, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.simulate_resize(size(px(460.), px(360.)));
        cx.update(|window, cx| {
            window.activate_window();
            let focus = view.read(cx).before.clone();
            focus.focus(window, cx);
        });
        settle(cx);
        (cx, view)
    }

    fn secondary_click(cx: &mut VisualTestContext, position: gpui::Point<gpui::Pixels>) {
        cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
        settle(cx);
    }

    #[gpui::test]
    fn secondary_click_places_repositions_and_clamps_the_shared_menu(cx: &mut TestAppContext) {
        let (cx, view) = harness(cx);
        let target = cx.debug_bounds("context-target").unwrap();
        let first = target.origin + point(px(20.), px(24.));
        cx.simulate_click(first, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).menu.read(cx);
            assert!(!state.is_open(cx), "left-click focuses without opening");
            assert!(state.focus_handle().is_focused(window));
        });
        secondary_click(cx, first);
        assert_eq!(cx.debug_bounds("context-surface").unwrap().origin, first);
        cx.update(|window, cx| {
            let state = view.read(cx).menu.read(cx);
            assert!(state.is_open(cx));
            assert_eq!(state.highlighted_index(), Some(1));
            assert!(state.content_focus_handle().is_focused(window));
            assert!(view.read(cx).commands.is_empty());
        });

        // A second request on the target moves the same menu; near the window
        // edge the existing Positioner keeps the whole command surface inside.
        secondary_click(cx, target.bottom_right() - point(px(2.), px(2.)));
        let popup = cx.debug_bounds("context-surface").unwrap();
        assert!(popup.origin.x > first.x);
        cx.update(|window, _| {
            assert!(popup.right() <= window.viewport_size().width - px(8.));
            assert!(popup.bottom() <= window.viewport_size().height - px(8.));
        });
        let command = cx.debug_bounds("context-command-1").unwrap().center();
        cx.simulate_click(command, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.menu.read(cx).is_open(cx));
            assert_eq!(view.commands, [SharedString::from("open")]);
            assert!(view.menu.read(cx).focus_handle().is_focused(window));
        });
    }

    #[gpui::test]
    fn keyboard_target_dismissal_disabled_and_unmount_keep_the_shared_lifecycle(
        cx: &mut TestAppContext,
    ) {
        let (cx, view) = harness(cx);
        press(cx, "tab");
        press(cx, "shift-f10");
        let target = cx.debug_bounds("context-target").unwrap();
        let popup = cx.debug_bounds("context-surface").unwrap();
        assert_eq!(popup.origin.x, target.origin.x);
        cx.update(|window, cx| {
            let state = view.read(cx).menu.read(cx);
            assert!(state.content_focus_handle().is_focused(window));
            assert_eq!(
                popup.top(),
                target
                    .bottom()
                    .min(window.viewport_size().height - px(8.) - popup.size.height)
            );
        });
        press(cx, "escape");
        cx.update(|window, cx| {
            let state = view.read(cx).menu.read(cx);
            assert!(!state.is_open(cx));
            assert!(state.focus_handle().is_focused(window));
        });
        press(cx, "menu");
        press(cx, "tab");
        cx.update(|window, cx| {
            assert!(!view.read(cx).menu.read(cx).is_open(cx));
            assert!(view.read(cx).after.is_focused(window));
        });
        press(cx, "shift-tab");
        press(cx, "shift-f10");
        let before = cx.debug_bounds("before").unwrap().center();
        cx.simulate_click(before, Modifiers::default());
        settle(cx);
        cx.update(|window, cx| {
            assert!(!view.read(cx).menu.read(cx).is_open(cx));
            assert!(view.read(cx).before.is_focused(window));
        });
        press(cx, "tab");
        press(cx, "shift-f10");
        cx.update(|window, cx| {
            let menu = view.read(cx).menu.clone();
            menu.update(cx, |state, cx| state.set_disabled(true, window, cx));
            assert!(menu.read(cx).focus_handle().is_focused(window));
        });
        settle(cx);
        press(cx, "shift-f10");
        secondary_click(cx, target.center());
        press(cx, "tab");
        cx.update(|window, cx| {
            assert!(!view.read(cx).menu.read(cx).is_open(cx));
            assert!(view.read(cx).after.is_focused(window));
            let menu = view.read(cx).menu.clone();
            menu.update(cx, |state, cx| {
                state.set_disabled(false, window, cx);
                state.open(window, cx);
            });
        });
        settle(cx);
        view.update(cx, |view, cx| {
            view.shown = false;
            cx.notify();
        });
        settle(cx);
        cx.update(|_, cx| {
            assert!(!view.read(cx).menu.read(cx).is_open(cx));
            assert!(view.read(cx).commands.is_empty());
        });
    }
}
