//! Controlled, unstyled switch behavior and accessibility semantics.
// Modified for GPUI Design from GPUI Kit v0.9.0: retain the controlled root,
// remove decorative parts, and preserve traversal when disabled while focused.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, MouseButton, ParentElement, Refineable as _, RenderOnce, Role, SharedString,
    Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Toggled, Window, div,
    prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

use crate::{StateStyle, StyledExt as _, TestSupportExt as _, a11y_disabled::A11yDisabledExt as _};

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// An unstyled binary control that owns switch interaction and semantics.
///
/// The checked value is controlled by the application. Activation reports the
/// next value through [`Switch::on_change`]; the application must render that
/// value back through [`Switch::checked`]. Children and all visual states remain
/// application-owned.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    base: crate::ObservedElement<Stateful<Div>>,
    style: StyleRefinement,
    semantic_styles: SwitchStyles,
    checked: bool,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_change: Option<ChangeHandler>,
    accessibility_label: Option<SharedString>,
    tab_index: isize,
    tab_stop: bool,
    provided_focus_handle: Option<FocusHandle>,
}

/// Semantic root styles supported by [`Switch`].
#[derive(Default)]
pub struct SwitchStyles {
    checked: StyleRefinement,
    disabled: StyleRefinement,
}

impl SwitchStyles {
    /// Refines the root style while the switch is checked.
    pub fn checked(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.checked
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    /// Refines the root style while the switch is disabled.
    pub fn disabled(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.disabled
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            id,
            style: StyleRefinement::default(),
            semantic_styles: SwitchStyles::default(),
            checked: false,
            disabled: false,
            children: SmallVec::new(),
            on_change: None,
            accessibility_label: None,
            tab_index: 0,
            tab_stop: true,
            provided_focus_handle: None,
        }
    }

    /// Sets the application-controlled checked value.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets whether pointer and keyboard activation are ignored.
    ///
    /// Disabled controls are skipped by Tab. A control disabled while focused
    /// keeps its focus binding until focus leaves, so traversal can continue.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Configures application-owned styles for the switch's semantic states.
    pub fn styles(mut self, build: impl FnOnce(SwitchStyles) -> SwitchStyles) -> Self {
        self.semantic_styles = build(self.semantic_styles);
        self
    }

    fn resolved_style(&self) -> StyleRefinement {
        crate::state_style::resolve_style(
            &self.style,
            [
                self.checked.then_some(&self.semantic_styles.checked),
                self.disabled.then_some(&self.semantic_styles.disabled),
            ]
            .into_iter()
            .flatten(),
        )
    }

    /// Handles activation with the next checked value and its input event.
    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Sets the name exposed to accessibility clients.
    ///
    /// A switch without text children has no accessible name, so set this
    /// whenever it is drawn without visible text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets the focus traversal index. Use this within a GPUI tab group.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether the switch participates in keyboard focus traversal.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Uses a caller-owned focus handle instead of creating keyed state.
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.provided_focus_handle = Some(focus_handle.clone());
        self
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        self.provided_focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        })
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Switch {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Switch {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Switch {}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.focus_handle(window, cx);
        let checked = self.checked;
        let disabled = self.disabled;
        let style = self.resolved_style();

        self.base
            .role(Role::Switch)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            // Retain a currently focused control in the dispatch tree after it
            // becomes disabled so Tab can still leave it in either direction.
            .when(!disabled || focus_handle.is_focused(window), |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop && !disabled),
                )
            })
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
            })
            .when_some(
                (!disabled).then_some(self.on_change).flatten(),
                |this, on_change| {
                    this.on_click(move |event, window, cx| {
                        on_change(!checked, event, window, cx);
                    })
                },
            )
            .children(self.children)
            .refine_style(&style)
            .a11y_disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use gpui::{
        AppContext as _, Context, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
        Render, TestAppContext, VisualTestContext, accesskit, canvas, point, px,
    };

    struct SwitchHarness {
        checked: bool,
        disabled: bool,
        toggles: Rc<Cell<usize>>,
        keyboard_events: Rc<Cell<usize>>,
        last_value: Rc<Cell<bool>>,
        parent_clicks: Rc<Cell<usize>>,
    }

    impl Render for SwitchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let toggles = self.toggles.clone();
            let keyboard_events = self.keyboard_events.clone();
            let last_value = self.last_value.clone();
            let parent_clicks = self.parent_clicks.clone();

            div()
                .id("switch-parent")
                .tab_group()
                .size(px(100.))
                .on_click(move |_, _, _| parent_clicks.set(parent_clicks.get() + 1))
                .child(
                    Switch::new("switch")
                        .checked(self.checked)
                        .disabled(self.disabled)
                        .size_full()
                        .on_change(move |value, event, _, _| {
                            toggles.set(toggles.get() + 1);
                            last_value.set(value);
                            if matches!(event, ClickEvent::Keyboard(_)) {
                                keyboard_events.set(keyboard_events.get() + 1);
                            }
                        }),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        checked: bool,
        disabled: bool,
    ) -> (
        &mut VisualTestContext,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
        Rc<Cell<bool>>,
        Rc<Cell<usize>>,
    ) {
        let toggles = Rc::new(Cell::new(0));
        let keyboard_events = Rc::new(Cell::new(0));
        let last_value = Rc::new(Cell::new(checked));
        let parent_clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let toggles = toggles.clone();
            let keyboard_events = keyboard_events.clone();
            let last_value = last_value.clone();
            let parent_clicks = parent_clicks.clone();
            move |_, _| SwitchHarness {
                checked,
                disabled,
                toggles,
                keyboard_events,
                last_value,
                parent_clicks,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, toggles, keyboard_events, last_value, parent_clicks)
    }

    fn activate_key(cx: &mut VisualTestContext, key: &str) {
        let keystroke =
            cx.update(|_, cx| Keystroke::parse_with_os(key, cx.platform_info().os).unwrap());
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
    }

    #[gpui::test]
    fn pointer_enter_and_space_report_one_activation_each(cx: &mut TestAppContext) {
        let (cx, toggles, keyboard_events, last_value, _) = harness(cx, false, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(toggles.get(), 1);
        assert_eq!(keyboard_events.get(), 0);
        assert!(last_value.get());
        toggles.set(0);
        cx.update(|window, cx| {
            assert!(window.focused(cx).is_some());
            window.draw(cx).clear(cx);
        });

        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert_eq!(toggles.get(), 2);
        assert_eq!(keyboard_events.get(), 2);
        assert!(last_value.get());
    }

    #[gpui::test]
    fn disabled_switch_is_inert_and_blocks_parent_activation(cx: &mut TestAppContext) {
        let (cx, toggles, _, _, parent_clicks) = harness(cx, false, true);

        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert_eq!(toggles.get(), 0);
        assert_eq!(parent_clicks.get(), 0);
    }

    struct DisableDuringFocus {
        handles: [FocusHandle; 3],
        value: bool,
        disabled: bool,
        activations: usize,
    }

    impl Render for DisableDuringFocus {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let change = cx.listener(|this, value: &bool, _, cx| {
                this.value = *value;
                this.disabled = true;
                this.activations += 1;
                cx.notify();
            });
            div()
                .size_full()
                .flex()
                .tab_group()
                .child(
                    crate::Button::new("before")
                        .track_focus(&self.handles[0])
                        .child("Before"),
                )
                .child(
                    Switch::new("disable-self")
                        .checked(self.value)
                        .track_focus(&self.handles[1])
                        .disabled(self.disabled)
                        .child("Disable self")
                        .on_change(move |value, _, window, cx| change(&value, window, cx)),
                )
                .child(
                    crate::Button::new("after")
                        .track_focus(&self.handles[2])
                        .child("After"),
                )
        }
    }

    #[gpui::test]
    fn disabling_focused_switch_preserves_bidirectional_traversal(cx: &mut TestAppContext) {
        cx.update(crate::init);
        for (key, expected, back, opposite) in
            [("tab", 2, "shift-tab", 0), ("shift-tab", 0, "tab", 2)]
        {
            let handles = cx.update(|cx| std::array::from_fn(|_| cx.focus_handle()));
            let controls = cx.new(|_| DisableDuringFocus {
                handles: handles.clone(),
                value: false,
                disabled: false,
                activations: 0,
            });
            let (_, cx) =
                cx.add_window_view(|window, cx| crate::Root::new(controls.clone(), window, cx));
            cx.update(|window, cx| {
                window.activate_window();
                window.draw(cx).clear(cx);
                handles[0].focus(window, cx);
                window.draw(cx).clear(cx);
            });
            activate_key(cx, "tab");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(window.focused(cx).as_ref(), Some(&handles[1]));
            });
            activate_key(cx, "space");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(controls.read(cx).disabled);
                assert!(controls.read(cx).value);
                assert_eq!(controls.read(cx).activations, 1);
            });
            activate_key(cx, "enter");
            activate_key(cx, "space");
            cx.update(|_, cx| assert_eq!(controls.read(cx).activations, 1));
            activate_key(cx, key);
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(window.focused(cx).as_ref(), Some(&handles[expected]));
            });
            activate_key(cx, back);
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(window.focused(cx).as_ref(), Some(&handles[opposite]));
            });
            controls.update(cx, |this, cx| {
                this.disabled = false;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            activate_key(cx, key);
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(window.focused(cx).as_ref(), Some(&handles[1]));
            });
        }
    }

    #[test]
    fn semantic_root_styles_follow_switch_priority() {
        let styled = |switch: Switch| {
            switch.styles(|styles| {
                styles
                    .checked(|style| style.opacity(0.8))
                    .disabled(|style| style.opacity(0.5))
            })
        };

        assert_eq!(styled(Switch::new("normal")).resolved_style().opacity, None);
        assert_eq!(
            styled(Switch::new("checked").checked(true))
                .resolved_style()
                .opacity,
            Some(0.8)
        );
        assert_eq!(
            styled(Switch::new("checked-disabled").checked(true).disabled(true))
                .resolved_style()
                .opacity,
            Some(0.5)
        );
        assert_eq!(
            styled(
                Switch::new("state-over-instance")
                    .checked(true)
                    .disabled(true)
                    .opacity(0.9),
            )
            .resolved_style()
            .opacity,
            Some(0.5)
        );
    }

    #[gpui::test]
    fn accessibility_exposes_switch_role_label_and_toggled_state(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct A11yProbe {
            captured: Captured,
        }

        impl Render for A11yProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let mut info = |switch: Switch| {
                            let element = switch.render(window, cx).into_element();
                            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                            element.write_a11y_info(&mut node);
                            node
                        };
                        let enabled = info(
                            Switch::new("enabled")
                                .checked(true)
                                .accessibility_label("Airplane mode")
                                .on_change(|_, _, _, _| {}),
                        );
                        let disabled = info(
                            Switch::new("disabled")
                                .checked(false)
                                .disabled(true)
                                .accessibility_label("Airplane mode")
                                .on_change(|_, _, _, _| {}),
                        );
                        *captured.lock().unwrap() = Some((enabled, disabled));
                    },
                    |_, _, _, _| {},
                )
            }
        }

        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| A11yProbe { captured });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let (enabled, disabled) = result.lock().unwrap().take().unwrap();

        assert_eq!(enabled.role(), Role::Switch);
        assert_eq!(enabled.label(), Some("Airplane mode"));
        assert_eq!(enabled.toggled(), Some(Toggled::True));
        assert!(enabled.supports_action(accesskit::Action::Click));

        assert_eq!(disabled.role(), Role::Switch);
        assert_eq!(disabled.toggled(), Some(Toggled::False));
        assert!(!disabled.supports_action(accesskit::Action::Click));
        assert!(!enabled.is_disabled());
        assert!(disabled.is_disabled());
    }
}
