//! Controlled, unstyled checkbox behavior and accessibility semantics.
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

type ChangeHandler = Rc<dyn Fn(CheckboxState, &ClickEvent, &mut Window, &mut App)>;

/// The semantic value exposed by an unstyled [`Checkbox`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckboxState {
    /// No items are checked.
    #[default]
    Unchecked,
    /// The checkbox is checked.
    Checked,
    /// A mixed selection, exposed as `Toggled::Mixed` to accessibility clients.
    Indeterminate,
}

impl CheckboxState {
    fn toggled(self) -> Toggled {
        match self {
            Self::Unchecked => Toggled::False,
            Self::Checked => Toggled::True,
            Self::Indeterminate => Toggled::Mixed,
        }
    }

    fn activated(self) -> Self {
        match self {
            Self::Unchecked | Self::Indeterminate => Self::Checked,
            Self::Checked => Self::Unchecked,
        }
    }
}

/// An unstyled checkbox that owns toggle, focus, keyboard, and accessibility behavior.
///
/// The value is controlled by the application. Activation reports the next value
/// through [`Self::on_change`]; render it back through [`Self::state`] to accept the
/// change. Children, layout, and visual states are supplied by the application.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: crate::ObservedElement<Stateful<Div>>,
    style: StyleRefinement,
    semantic_styles: CheckboxStyles,
    state: CheckboxState,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_change: Option<ChangeHandler>,
    accessibility_label: Option<SharedString>,
    tab_index: isize,
    tab_stop: bool,
    provided_focus_handle: Option<FocusHandle>,
}

impl Checkbox {
    /// Creates an unchecked checkbox with a stable element identifier.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            id,
            style: StyleRefinement::default(),
            semantic_styles: CheckboxStyles::default(),
            state: CheckboxState::Unchecked,
            disabled: false,
            children: SmallVec::new(),
            on_change: None,
            accessibility_label: None,
            tab_index: 0,
            tab_stop: true,
            provided_focus_handle: None,
        }
    }

    /// Sets the controlled semantic state.
    pub fn state(mut self, state: CheckboxState) -> Self {
        self.state = state;
        self
    }

    /// Sets the checked state, clearing any indeterminate state.
    pub fn checked(self, checked: bool) -> Self {
        self.state(if checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        })
    }

    /// Sets whether pointer and keyboard activation are ignored.
    ///
    /// Disabled controls are skipped by Tab. A control disabled while focused
    /// keeps its focus binding until focus leaves, so traversal can continue.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Defines application-owned styles for the checkbox's semantic states.
    pub fn styles(mut self, build: impl FnOnce(CheckboxStyles) -> CheckboxStyles) -> Self {
        self.semantic_styles = build(self.semantic_styles);
        self
    }

    /// Sets the label exposed to accessibility clients.
    ///
    /// A checkbox without text children has no accessible name, so set this
    /// whenever it is drawn without visible text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Handles an activation with the next controlled state.
    ///
    /// An indeterminate checkbox becomes checked when activated. The activating
    /// [`ClickEvent`] is reported so callers can read its modifiers, for example
    /// to extend a selection.
    pub fn on_change(
        mut self,
        handler: impl Fn(CheckboxState, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Sets the focus traversal index. The default is `0`.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether this checkbox participates in keyboard focus traversal.
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

    fn resolved_style(&self) -> StyleRefinement {
        crate::state_style::resolve_style(
            &self.style,
            [
                match self.state {
                    CheckboxState::Unchecked => None,
                    CheckboxState::Checked => Some(&self.semantic_styles.checked),
                    CheckboxState::Indeterminate => Some(&self.semantic_styles.indeterminate),
                },
                self.disabled.then_some(&self.semantic_styles.disabled),
            ]
            .into_iter()
            .flatten(),
        )
    }
}

/// Semantic styles supported by [`Checkbox`].
#[derive(Default)]
pub struct CheckboxStyles {
    checked: StyleRefinement,
    indeterminate: StyleRefinement,
    disabled: StyleRefinement,
}

impl CheckboxStyles {
    /// Refines the root style when the checkbox is checked.
    pub fn checked(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.checked
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    /// Refines the root style when the checkbox is indeterminate.
    pub fn indeterminate(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.indeterminate
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    /// Refines the root style when the checkbox is disabled.
    pub fn disabled(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.disabled
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Checkbox {}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.focus_handle(window, cx);
        let disabled = self.disabled;
        let next_state = self.state.activated();
        let style = self.resolved_style();
        let on_change = self.on_change;

        self.base
            .role(Role::CheckBox)
            .aria_toggled(self.state.toggled())
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
                (!disabled).then_some(on_change).flatten(),
                |this, on_change| {
                    this.on_click(move |event, window, cx| {
                        on_change(next_state, event, window, cx);
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
        cell::{Cell, RefCell},
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use gpui::{
        AppContext as _, Context, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
        Render, TestAppContext, VisualTestContext, accesskit, canvas, point, px,
    };

    struct CheckboxHarness {
        state: CheckboxState,
        disabled: bool,
        changes: Rc<RefCell<Vec<CheckboxState>>>,
        parent_clicks: Rc<Cell<usize>>,
    }

    impl Render for CheckboxHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            let parent_clicks = self.parent_clicks.clone();
            div()
                .id("checkbox-parent")
                .tab_group()
                .size(px(100.))
                .on_click(move |_, _, _| parent_clicks.set(parent_clicks.get() + 1))
                .child(
                    Checkbox::new("checkbox")
                        .state(self.state)
                        .disabled(self.disabled)
                        .size_full()
                        .on_change(move |state, _, _, _| changes.borrow_mut().push(state)),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        state: CheckboxState,
        disabled: bool,
    ) -> (
        &mut VisualTestContext,
        Rc<RefCell<Vec<CheckboxState>>>,
        Rc<Cell<usize>>,
    ) {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let parent_clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let changes = changes.clone();
            let parent_clicks = parent_clicks.clone();
            move |_, _| CheckboxHarness {
                state,
                disabled,
                changes,
                parent_clicks,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, changes, parent_clicks)
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
    fn pointer_activation_emits_the_next_state_once(cx: &mut TestAppContext) {
        let (cx, changes, _) = harness(cx, CheckboxState::Unchecked, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(&*changes.borrow(), &[CheckboxState::Checked]);
    }

    #[gpui::test]
    fn indeterminate_activation_becomes_checked(cx: &mut TestAppContext) {
        let (cx, changes, _) = harness(cx, CheckboxState::Indeterminate, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(&*changes.borrow(), &[CheckboxState::Checked]);
    }

    #[gpui::test]
    fn enter_and_space_each_emit_once(cx: &mut TestAppContext) {
        let (cx, changes, _) = harness(cx, CheckboxState::Checked, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        changes.borrow_mut().clear();
        cx.update(|window, cx| {
            assert!(window.focused(cx).is_some());
            window.draw(cx).clear(cx);
        });

        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert_eq!(
            &*changes.borrow(),
            &[CheckboxState::Unchecked, CheckboxState::Unchecked]
        );
    }

    #[gpui::test]
    fn disabled_checkbox_is_inert_and_blocks_parent_activation(cx: &mut TestAppContext) {
        let (cx, changes, parent_clicks) = harness(cx, CheckboxState::Unchecked, true);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.update(|window, cx| window.focus_next(cx));
        activate_key(cx, "enter");
        activate_key(cx, "space");
        assert!(changes.borrow().is_empty());
        assert_eq!(parent_clicks.get(), 0);
    }

    struct DisableDuringFocus {
        handles: [FocusHandle; 3],
        value: CheckboxState,
        disabled: bool,
        activations: usize,
    }

    impl Render for DisableDuringFocus {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let change = cx.listener(|this, value: &CheckboxState, _, cx| {
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
                    Checkbox::new("disable-self")
                        .state(self.value)
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
    fn disabling_focused_checkbox_preserves_bidirectional_traversal(cx: &mut TestAppContext) {
        cx.update(crate::init);
        for (key, expected, back, opposite) in
            [("tab", 2, "shift-tab", 0), ("shift-tab", 0, "tab", 2)]
        {
            let handles = cx.update(|cx| std::array::from_fn(|_| cx.focus_handle()));
            let controls = cx.new(|_| DisableDuringFocus {
                handles: handles.clone(),
                value: CheckboxState::Unchecked,
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
                assert_eq!(controls.read(cx).value, CheckboxState::Checked);
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
    fn semantic_root_styles_follow_checkbox_priority() {
        let styles = |checkbox: Checkbox| {
            checkbox.styles(|styles| {
                styles
                    .checked(|style| style.opacity(0.8))
                    .indeterminate(|style| style.opacity(0.7))
                    .disabled(|style| style.opacity(0.5))
            })
        };

        assert_eq!(
            styles(Checkbox::new("normal")).resolved_style().opacity,
            None
        );
        assert_eq!(
            styles(Checkbox::new("checked").checked(true))
                .resolved_style()
                .opacity,
            Some(0.8)
        );
        assert_eq!(
            styles(Checkbox::new("indeterminate").state(CheckboxState::Indeterminate))
                .resolved_style()
                .opacity,
            Some(0.7)
        );
        assert_eq!(
            styles(Checkbox::new("disabled").disabled(true))
                .resolved_style()
                .opacity,
            Some(0.5)
        );
        assert_eq!(
            styles(
                Checkbox::new("checked-disabled")
                    .checked(true)
                    .disabled(true),
            )
            .resolved_style()
            .opacity,
            Some(0.5)
        );

        let checked_color = gpui::hsla(0.6, 0.7, 0.5, 1.0);
        let combined = Checkbox::new("combined")
            .checked(true)
            .disabled(true)
            .styles(|styles| {
                styles
                    .checked(|style| style.border_color(checked_color))
                    .disabled(|style| style.opacity(0.5))
            })
            .resolved_style();
        assert_eq!(combined.border_color, Some(checked_color));
        assert_eq!(combined.opacity, Some(0.5));

        let state_over_instance = styles(
            Checkbox::new("state-over-instance")
                .checked(true)
                .disabled(true)
                .opacity(0.9),
        );
        assert_eq!(state_over_instance.resolved_style().opacity, Some(0.5));
    }

    #[gpui::test]
    fn accessibility_exposes_role_label_and_all_toggle_states(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<[accesskit::Node; 4]>>>;

        struct A11yProbe {
            captured: Captured,
        }

        impl Render for A11yProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let mut info = |checkbox: Checkbox| {
                            let element = checkbox.render(window, cx).into_element();
                            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                            element.write_a11y_info(&mut node);
                            node
                        };
                        *captured.lock().unwrap() = Some([
                            info(
                                Checkbox::new("unchecked")
                                    .accessibility_label("Remember me")
                                    .on_change(|_, _, _, _| {}),
                            ),
                            info(
                                Checkbox::new("checked")
                                    .checked(true)
                                    .on_change(|_, _, _, _| {}),
                            ),
                            info(
                                Checkbox::new("mixed")
                                    .state(CheckboxState::Indeterminate)
                                    .on_change(|_, _, _, _| {}),
                            ),
                            info(
                                Checkbox::new("disabled")
                                    .disabled(true)
                                    .on_change(|_, _, _, _| {}),
                            ),
                        ]);
                    },
                    |_, _, _, _| {},
                )
            }
        }

        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| A11yProbe { captured });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let [unchecked, checked, mixed, disabled] = result.lock().unwrap().take().unwrap();

        assert_eq!(unchecked.role(), Role::CheckBox);
        assert_eq!(unchecked.label(), Some("Remember me"));
        assert_eq!(unchecked.toggled(), Some(Toggled::False));
        assert_eq!(checked.toggled(), Some(Toggled::True));
        assert_eq!(mixed.toggled(), Some(Toggled::Mixed));
        assert!(unchecked.supports_action(accesskit::Action::Click));
        assert!(!disabled.supports_action(accesskit::Action::Click));
        assert!(!unchecked.is_disabled());
        assert!(disabled.is_disabled());
    }
}
