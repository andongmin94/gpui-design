//! A single-thumb linear slider with retained value and application-owned visuals.
//! Derived from GPUI Kit v0.9.0's slider pointer/geometry primitives; GPUI Design
//! narrows the value model and adds keyboard, focus, and numeric accessibility.

use std::ops::RangeInclusive;

use gpui::{
    AccessibleAction, AnyElement, App, AppContext as _, Axis, Bounds, Context, DragMoveEvent,
    ElementId, Empty, Entity, EntityId, EventEmitter, FocusHandle, InteractiveElement,
    Interactivity, IntoElement, KeyBinding, MouseButton, MouseDownEvent, Orientation,
    ParentElement, Pixels, Point, Render, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, accesskit, div, prelude::FluentBuilder as _, px,
};

use crate::{
    ElementExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{
        SelectDown, SelectFirst, SelectLast, SelectLeft, SelectPageDown, SelectPageUp, SelectRight,
        SelectUp,
    },
};

const CONTEXT: &str = "Slider";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
    ]);
}

/// Emitted after a value changes, including through [`SliderState::set_value`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderEvent {
    Change(f32),
}

/// Retained value, focus, and pointer geometry for one slider in one window.
///
/// Values snap to steps measured from the minimum. Both endpoints are reachable,
/// including a maximum that is not on the step grid. Configuration is fixed for
/// the entity's lifetime; applications observe this state to redraw their visuals.
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    disabled: bool,
    focus: FocusHandle,
    bounds: Bounds<Pixels>,
}

impl SliderState {
    /// Creates a slider at its minimum value.
    ///
    /// # Panics
    /// Endpoints must be finite with `min < max`; `step` must be finite and positive.
    pub fn new(range: RangeInclusive<f32>, step: f32, cx: &mut Context<Self>) -> Self {
        let (min, max) = range.into_inner();
        assert!(
            min.is_finite() && max.is_finite() && min < max,
            "Slider range must have finite endpoints with min < max"
        );
        assert!(
            step.is_finite() && step > 0.,
            "Slider step must be finite and positive"
        );
        Self {
            min,
            max,
            step,
            value: min,
            disabled: false,
            focus: cx.focus_handle(),
            bounds: Bounds::default(),
        }
    }

    /// Sets a finite initial value, clamped and snapped without emitting an event.
    pub fn default_value(mut self, value: f32) -> Self {
        self.value = self.normalized(value);
        self
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn min_value(&self) -> f32 {
        self.min
    }

    pub fn max_value(&self) -> f32 {
        self.max
    }

    pub fn step_value(&self) -> f32 {
        self.step
    }

    /// The thumb center's fraction along the travel rail, in `0..=1`.
    pub fn percentage(&self) -> f32 {
        ((self.value as f64 - self.min as f64) / (self.max as f64 - self.min as f64)) as f32
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets a finite value and emits one event only if the normalized value changes.
    /// Programmatic changes remain allowed while user interaction is disabled.
    pub fn set_value(&mut self, value: f32, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.normalized(value);
        if self.value == value {
            return;
        }
        self.value = value;
        cx.emit(SliderEvent::Change(value));
        cx.notify();
        window.refresh();
    }

    /// Disables user interaction and removes the slider from normal Tab order.
    /// An already-focused slider retains its focus binding so Tab can leave it.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
        window.refresh();
    }

    fn normalized(&self, value: f32) -> f32 {
        assert!(value.is_finite(), "Slider value must be finite");
        if value <= self.min {
            return self.min;
        }
        if value >= self.max {
            return self.max;
        }
        let min = self.min as f64;
        let step = self.step as f64;
        let snapped = min + ((value as f64 - min) / step).round() * step;
        snapped.clamp(min, self.max as f64) as f32
    }

    fn step_by(&mut self, steps: f64, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let min = self.min as f64;
        let step = self.step as f64;
        let index = (self.value as f64 - min) / step;
        let index = if steps < 0. && self.value == self.max {
            // An off-grid maximum is the final stop after the last grid value.
            // Subtracting a raw step and rounding can skip that adjacent value.
            let last = index.floor();
            if (min + last * step) as f32 == self.max {
                last
            } else {
                last + 1.
            }
        } else {
            index.round()
        };
        let value = (min + (index + steps) * step).clamp(min, self.max as f64) as f32;
        self.set_value(value, window, cx);
    }

    fn to_edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled {
            self.set_value(if last { self.max } else { self.min }, window, cx);
        }
    }

    fn value_at_pointer(
        &mut self,
        axis: Axis,
        direction: crate::Direction,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let (offset, length) = match axis {
            Axis::Horizontal => (position.x - self.bounds.left(), self.bounds.size.width),
            Axis::Vertical => (self.bounds.bottom() - position.y, self.bounds.size.height),
        };
        if length <= px(0.) {
            return;
        }
        let fraction = (offset / length).clamp(0., 1.);
        let fraction = if axis == Axis::Horizontal {
            direction.physical_fraction(fraction)
        } else {
            fraction
        } as f64;
        let value = self.min as f64 + (self.max as f64 - self.min as f64) * fraction;
        self.set_value(value as f32, window, cx);
    }
}

impl EventEmitter<SliderEvent> for SliderState {}

#[derive(Clone)]
struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// An unstyled, focusable single-thumb slider.
///
/// The root's bounds define the thumb-center travel rail. Applications supply
/// non-interactive track, fill, and thumb children and position the thumb using
/// [`SliderState::percentage`]. Horizontal values grow left to right; vertical
/// values grow bottom to top. Reserve any desired space around the thumb outside
/// this root; the library imposes no track thickness, thumb size, or colors.
#[derive(IntoElement)]
pub struct Slider {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<SliderState>,
    axis: Axis,
    direction: Option<crate::Direction>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>, state: &Entity<SliderState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            axis: Axis::Horizontal,
            direction: None,
            label: None,
            children: Vec::new(),
        }
    }

    /// Mirrors horizontal keys and pointer geometry in RTL. Application-owned
    /// fill/thumb layout can use `direction.physical_fraction(percentage)`.
    pub fn direction(mut self, direction: crate::Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Slider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Slider {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Slider {}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let focus = model.focus.clone();
        let disabled = model.disabled;
        let axis = self.axis;
        let direction = self
            .direction
            .unwrap_or_else(|| crate::Direction::current(cx));
        let entity_id = self.state.entity_id();
        let geometry = self.state.clone();
        let drag_start = self.state.clone();
        let increment = self.state.clone();
        let decrement = self.state.clone();
        let set_value = self.state.clone();

        self.base
            .role(Role::Slider)
            .when_some(self.label, |this, label| this.aria_label(label))
            .aria_numeric_value(model.value as f64)
            .aria_min_numeric_value(model.min as f64)
            .aria_max_numeric_value(model.max as f64)
            .aria_numeric_value_step(model.step as f64)
            .aria_orientation(match axis {
                Axis::Horizontal => Orientation::Horizontal,
                Axis::Vertical => Orientation::Vertical,
            })
            .key_context(CONTEXT)
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(!disabled))
            })
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectLeft, window, cx| {
                    state.step_by(
                        if axis == Axis::Horizontal && direction.is_rtl() {
                            1.
                        } else {
                            -1.
                        },
                        window,
                        cx,
                    );
                },
            ))
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectRight, window, cx| {
                    state.step_by(
                        if axis == Axis::Horizontal && direction.is_rtl() {
                            -1.
                        } else {
                            1.
                        },
                        window,
                        cx,
                    );
                },
            ))
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectDown, window, cx| {
                    state.step_by(-1., window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectUp, window, cx| {
                    state.step_by(1., window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectFirst, window, cx| {
                    state.to_edge(false, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectLast, window, cx| {
                    state.to_edge(true, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectPageUp, window, cx| {
                    state.step_by(10., window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectPageDown, window, cx| {
                    state.step_by(-10., window, cx);
                }),
            )
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .when(!disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &self.state,
                        move |state, event: &MouseDownEvent, window, cx| {
                            if !state.disabled {
                                state.focus.focus(window, cx);
                                state.value_at_pointer(axis, direction, event.position, window, cx);
                            }
                        },
                    ),
                )
                .on_drag(DragSlider(entity_id), move |drag, _, window, cx| {
                    let position = window.mouse_position();
                    drag_start.update(cx, |state, cx| {
                        state.value_at_pointer(axis, direction, position, window, cx)
                    });
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(window.listener_for(
                    &self.state,
                    move |state, event: &DragMoveEvent<DragSlider>, window, cx| {
                        if event.drag(cx).0 == entity_id {
                            state.value_at_pointer(
                                axis,
                                direction,
                                event.event.position,
                                window,
                                cx,
                            );
                        }
                    },
                ))
                .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
                    increment.update(cx, |state, cx| state.step_by(1., window, cx));
                })
                .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
                    decrement.update(cx, |state, cx| state.step_by(-1., window, cx));
                })
                .on_a11y_action(
                    AccessibleAction::SetValue,
                    move |data, window, cx| {
                        if let Some(accesskit::ActionData::NumericValue(value)) = data {
                            set_value.update(cx, |state, cx| {
                                if !state.disabled && value.is_finite() {
                                    let value =
                                        value.clamp(state.min as f64, state.max as f64) as f32;
                                    state.set_value(value, window, cx);
                                }
                            });
                        }
                    },
                )
            })
            .children(self.children)
            .on_prepaint(move |bounds, _, cx| {
                if geometry.read(cx).bounds != bounds {
                    geometry.update(cx, |state, _| state.bounds = bounds);
                }
            })
            .a11y_disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{Element as _, Modifiers, Subscription, TestAppContext, VisualTestContext, point};

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        state: Entity<SliderState>,
        axis: Axis,
        before: FocusHandle,
        after: FocusHandle,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(300.))
                .flex()
                .flex_col()
                .child(
                    Button::new("before")
                        .h(px(30.))
                        .track_focus(&self.before)
                        .child("Before"),
                )
                .child(
                    Slider::new("slider", &self.state)
                        .debug_selector(|| "slider".into())
                        .axis(self.axis)
                        .accessibility_label("Position")
                        .when(self.axis == Axis::Horizontal, |this| {
                            this.w(px(200.)).h(px(24.))
                        })
                        .when(self.axis == Axis::Vertical, |this| {
                            this.w(px(24.)).h(px(200.))
                        }),
                )
                .child(
                    Button::new("after")
                        .h(px(30.))
                        .track_focus(&self.after)
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<f32>>>;

    fn harness(
        cx: &mut TestAppContext,
        axis: Axis,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                let state = cx.new(|cx| SliderState::new(3.0..=100.0, 10., cx).default_value(34.));
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    axis,
                    before: cx.focus_handle().tab_stop(true),
                    after: cx.focus_handle().tab_stop(true),
                    _subscription: subscription,
                }
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        let changes = Rc::new(RefCell::new(Vec::new()));
        cx.update(|window, cx| {
            let changes = changes.clone();
            let state = view.read(cx).state.clone();
            cx.subscribe(&state, move |_, event: &SliderEvent, _| {
                let SliderEvent::Change(value) = *event;
                changes.borrow_mut().push(value);
            })
            .detach();
            window.activate_window();
            view.read(cx).before.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        (cx, view, changes)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[gpui::test]
    fn value_snaps_from_minimum_and_native_metadata_tracks_the_value(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(state.read(cx).value(), 33.);
            assert_eq!(state.read(cx).percentage(), 30. / 97.);
            state.update(cx, |state, cx| {
                for value in [34., -100., 1000., 94., 96.] {
                    state.set_value(value, window, cx);
                }
            });
            for (max, step, previous) in [(10., 3., 9.), (0.3, 0.1, 0.2)] {
                let edge = cx.new(|cx| SliderState::new(0.0..=max, step, cx).default_value(max));
                edge.update(cx, |state, cx| {
                    state.step_by(-1., window, cx);
                    assert_eq!(state.value(), previous, "move to the preceding grid value");
                    state.step_by(1., window, cx);
                    assert_eq!(state.value(), max);
                });
            }
            let element = Slider::new("numeric-probe", &state)
                .axis(Axis::Vertical)
                .accessibility_label("Position")
                .render(window, cx)
                .into_element();
            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut node);
            assert_eq!(node.role(), Role::Slider);
            assert_eq!(node.label(), Some("Position"));
            assert_eq!(node.numeric_value(), Some(93.));
            assert_eq!(node.min_numeric_value(), Some(3.));
            assert_eq!(node.max_numeric_value(), Some(100.));
            assert_eq!(node.numeric_value_step(), Some(10.));
            assert_eq!(node.orientation(), Some(Orientation::Vertical));
            for action in [
                AccessibleAction::Increment,
                AccessibleAction::Decrement,
                AccessibleAction::SetValue,
            ] {
                assert!(node.supports_action(action));
            }
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
            let element = Slider::new("disabled-probe", &state)
                .render(window, cx)
                .into_element();
            let mut disabled = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut disabled);
            assert!(disabled.is_disabled());
            assert!(!disabled.supports_action(AccessibleAction::SetValue));
        });
        assert_eq!(*changes.borrow(), [3., 100., 93.]);
    }

    #[gpui::test]
    fn keyboard_changes_are_bounded_and_disabling_keeps_a_tab_exit(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal);
        cx.simulate_keystrokes("tab");
        for (key, expected) in [
            ("right", 43.),
            ("up", 53.),
            ("left", 43.),
            ("down", 33.),
            ("home", 3.),
            ("end", 100.),
            ("down", 93.),
            ("pagedown", 3.),
            ("pageup", 100.),
            ("right", 100.),
        ] {
            cx.simulate_keystrokes(key);
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.read(cx);
                assert!(state.focus_handle().is_focused(window));
                assert_eq!(state.value(), expected);
            });
        }
        assert_eq!(
            changes.borrow().len(),
            9,
            "clamped keys emit no duplicate value"
        );
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        draw(cx);
        cx.simulate_keystrokes("home left");
        draw(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(), 100.));
        let center = cx.debug_bounds("slider").unwrap().center();
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(), 100.));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).before.is_focused(window)));
        assert_eq!(changes.borrow().len(), 9);
    }

    #[gpui::test]
    fn track_press_and_drag_follow_both_axes_and_stop_on_release(cx: &mut TestAppContext) {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let (cx, view, changes) = harness(cx, axis);
            let bounds = cx.debug_bounds("slider").unwrap();
            let at = |fraction: f32| match axis {
                Axis::Horizontal => point(
                    bounds.left() + bounds.size.width * fraction,
                    bounds.center().y,
                ),
                Axis::Vertical => point(
                    bounds.center().x,
                    bounds.bottom() - bounds.size.height * fraction,
                ),
            };
            cx.simulate_mouse_down(at(0.5), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.read(cx);
                assert_eq!(state.value(), 53.);
                assert!(state.focus_handle().is_focused(window));
            });
            cx.simulate_mouse_move(at(1.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(), 100.));
            cx.simulate_mouse_move(at(-0.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(), 3.));
            cx.simulate_mouse_up(at(-0.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.simulate_mouse_move(at(0.5), None, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| {
                assert_eq!(view.read(cx).state.read(cx).value(), 3.);
                assert!(!cx.has_active_drag());
            });
            assert_eq!(*changes.borrow(), [53., 100., 3.]);
        }
    }
}
