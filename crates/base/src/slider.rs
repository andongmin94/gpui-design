//! A linear slider with independently focusable thumbs and application-owned visuals.
//! Derived from GPUI Kit v0.9.0's slider pointer/geometry primitives; GPUI Design
//! adds retained ordered values, keyboard, focus, and numeric accessibility.

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

/// The ordered values after one change, including programmatic changes.
#[derive(Clone, Debug, PartialEq)]
pub enum SliderEvent {
    Change(Vec<f32>),
}

/// Retained ordered values, thumb focus, and pointer geometry for one slider in one window.
///
/// Values snap to steps measured from the minimum. Both endpoints are reachable,
/// including a maximum that is not on the step grid. Range, step, and thumb count
/// are fixed for the entity's lifetime. Applications observe this state to redraw.
pub struct SliderState {
    min: f32,
    max: f32,
    step: f32,
    values: Vec<f32>,
    disabled: bool,
    focus: Vec<FocusHandle>,
    bounds: Bounds<Pixels>,
    axis: Axis,
    direction: crate::Direction,
    pointer_thumb: usize,
}

impl SliderState {
    /// Creates one thumb per initial value. Thumb identities and count remain fixed.
    /// Values are clamped and snapped, and equal values are allowed.
    ///
    /// # Panics
    /// Endpoints must be finite with `min < max`; `step` must be finite and positive.
    /// Values must be finite, nonempty, and ordered after normalization.
    pub fn new(
        range: RangeInclusive<f32>,
        step: f32,
        values: impl IntoIterator<Item = f32>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (min, max) = range.into_inner();
        assert!(
            min.is_finite() && max.is_finite() && min < max,
            "Slider range must have finite endpoints with min < max"
        );
        assert!(
            step.is_finite() && step > 0.,
            "Slider step must be finite and positive"
        );
        let mut state = Self {
            min,
            max,
            step,
            values: Vec::new(),
            disabled: false,
            focus: Vec::new(),
            bounds: Bounds::default(),
            axis: Axis::Horizontal,
            direction: crate::Direction::Ltr,
            pointer_thumb: 0,
        };
        state.values = state.normalized_values(values);
        assert!(!state.values.is_empty(), "Slider needs at least one thumb");
        state.focus = state.values.iter().map(|_| cx.focus_handle()).collect();
        state
    }

    pub fn values(&self) -> &[f32] {
        &self.values
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
    pub fn percentage(&self, index: usize) -> f32 {
        ((self.values[index] as f64 - self.min as f64) / (self.max as f64 - self.min as f64)) as f32
    }

    pub fn thumb_focus_handle(&self, index: usize) -> &FocusHandle {
        &self.focus[index]
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Replaces all values, preserving thumb count and order. Emits one event only
    /// when the normalized values change. The initial validation rules still apply.
    /// Programmatic changes remain allowed while user interaction is disabled.
    pub fn set_values(
        &mut self,
        values: impl IntoIterator<Item = f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let values = self.normalized_values(values);
        assert_eq!(
            values.len(),
            self.values.len(),
            "Slider thumb count is fixed"
        );
        if self.values == values {
            return;
        }
        self.values = values;
        cx.emit(SliderEvent::Change(self.values.clone()));
        cx.notify();
        window.refresh();
    }

    /// Sets one thumb, snapping from the global minimum and clamping to its
    /// neighbors so thumbs never cross. Equal adjacent values are allowed.
    /// Programmatic changes remain allowed while user interaction is disabled.
    pub fn set_thumb_value(
        &mut self,
        index: usize,
        value: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (min, max) = self.thumb_range(index);
        let value = self.normalized(value).clamp(min, max);
        if self.values[index] == value {
            return;
        }
        self.values[index] = value;
        cx.emit(SliderEvent::Change(self.values.clone()));
        cx.notify();
        window.refresh();
    }

    /// Disables user interaction and removes all thumbs from normal Tab order.
    /// An already-focused thumb retains its focus binding so Tab can leave it.
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

    fn normalized_values(&self, values: impl IntoIterator<Item = f32>) -> Vec<f32> {
        let values: Vec<_> = values
            .into_iter()
            .map(|value| self.normalized(value))
            .collect();
        assert!(
            values.windows(2).all(|pair| pair[0] <= pair[1]),
            "Slider values must be ordered"
        );
        values
    }

    fn thumb_range(&self, index: usize) -> (f32, f32) {
        assert!(
            index < self.values.len(),
            "Slider thumb index must identify a value"
        );
        let min = index
            .checked_sub(1)
            .map_or(self.min, |index| self.values[index]);
        let max = self.values.get(index + 1).copied().unwrap_or(self.max);
        (min, max)
    }

    fn step_by(&mut self, thumb: usize, steps: f64, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let min = self.min as f64;
        let step = self.step as f64;
        let current = self.values[thumb];
        let index = (current as f64 - min) / step;
        let index = if steps < 0. && current == self.max {
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
        self.set_thumb_value(thumb, value, window, cx);
    }

    fn to_edge(&mut self, thumb: usize, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled {
            self.set_thumb_value(thumb, if last { self.max } else { self.min }, window, cx);
        }
    }

    fn pointer_value(&self, position: Point<Pixels>) -> Option<f32> {
        let (offset, length) = match self.axis {
            Axis::Horizontal => (position.x - self.bounds.left(), self.bounds.size.width),
            Axis::Vertical => (self.bounds.bottom() - position.y, self.bounds.size.height),
        };
        if length <= px(0.) {
            return None;
        }
        let fraction = (offset / length).clamp(0., 1.);
        let fraction = if self.axis == Axis::Horizontal {
            self.direction.physical_fraction(fraction)
        } else {
            fraction
        } as f64;
        let value = self.min as f64 + (self.max as f64 - self.min as f64) * fraction;
        Some(value as f32)
    }

    fn press_track(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        if let Some(value) = self.pointer_value(position) {
            let thumb = self
                .values
                .iter()
                .enumerate()
                .min_by(|(left_index, left), (right_index, right)| {
                    ((**left as f64) - value as f64)
                        .abs()
                        .total_cmp(&((**right as f64) - value as f64).abs())
                        .then_with(|| {
                            // A collapsed range must open toward the pointer:
                            // its last thumb can increase, and its first can decrease.
                            if left == right && value > **left {
                                right_index.cmp(left_index)
                            } else {
                                left_index.cmp(right_index)
                            }
                        })
                })
                .map(|(index, _)| index)
                .unwrap();
            self.pointer_thumb = thumb;
            self.focus[thumb].focus(window, cx);
            self.set_thumb_value(thumb, value, window, cx);
        }
    }

    fn drag_at_pointer(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.disabled
            && let Some(value) = self.pointer_value(position)
        {
            self.set_thumb_value(self.pointer_thumb, value, window, cx);
        }
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

/// An unstyled slider group and thumb-center travel rail.
///
/// Render one [`SliderThumb`] per value, in index order, alongside non-interactive
/// track and fill children. Applications position each thumb using
/// [`SliderState::percentage`]. Horizontal values grow in the logical direction;
/// vertical values grow bottom to top. Reserve desired space around the thumbs
/// outside this rail. The library imposes no thumb size, track thickness, or colors.
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let direction = self
            .direction
            .unwrap_or_else(|| crate::Direction::current(cx));
        self.state.update(cx, |state, _| {
            state.axis = axis;
            state.direction = direction;
        });
        let disabled = self.state.read(cx).disabled;
        let entity_id = self.state.entity_id();
        let press = self.state.clone();
        let drag_start = self.state.clone();
        let drag_move = self.state.clone();
        let geometry = self.state;
        self.base
            .role(Role::Group)
            .when_some(self.label, |this, label| this.aria_label(label))
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .when(!disabled, |this| {
                this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    press.update(cx, |state, cx| {
                        state.press_track(event.position, window, cx)
                    });
                })
                .on_drag(DragSlider(entity_id), move |drag, _, window, cx| {
                    let position = window.mouse_position();
                    drag_start.update(cx, |state, cx| state.drag_at_pointer(position, window, cx));
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(
                    move |event: &DragMoveEvent<DragSlider>, window, cx| {
                        if event.drag(cx).0 == entity_id {
                            drag_move.update(cx, |state, cx| {
                                state.drag_at_pointer(event.event.position, window, cx);
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

fn thumb_id(state: EntityId, index: usize) -> ElementId {
    (ElementId::from(("slider-thumb", state)), index.to_string()).into()
}

/// One independently focusable slider value. Render inside its matching [`Slider`]
/// and give each thumb an accessible name distinguishing its purpose.
///
/// The root supplies axis, direction, and pointer geometry. This part owns native
/// numeric metadata, keyboard and assistive-technology adjustment, and pointer
/// selection of this exact thumb. Its range is bounded by its neighboring values;
/// thumbs can meet but never exchange identities or cross. Applications own all
/// positioning, dimensions, and focus/disabled styling.
#[derive(IntoElement)]
pub struct SliderThumb {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<SliderState>,
    index: usize,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl SliderThumb {
    /// `index` must identify a value in the matching state.
    pub fn new(state: &Entity<SliderState>, index: usize) -> Self {
        Self {
            base: div().id(thumb_id(state.entity_id(), index)).test_support(),
            state: state.clone(),
            index,
            label: None,
            children: Vec::new(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Styled for SliderThumb {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for SliderThumb {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for SliderThumb {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for SliderThumb {}

impl RenderOnce for SliderThumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let index = self.index;
        let focus = model.focus[index].clone();
        let disabled = model.disabled;
        let axis = model.axis;
        let direction = model.direction;
        let (min, max) = model.thumb_range(index);
        let entity_id = self.state.entity_id();
        let drag_start = self.state.clone();
        let increment = self.state.clone();
        let decrement = self.state.clone();
        let set_value = self.state.clone();

        self.base
            .role(Role::Slider)
            .when_some(self.label, |this, label| this.aria_label(label))
            .aria_numeric_value(model.values[index] as f64)
            .aria_min_numeric_value(min as f64)
            .aria_max_numeric_value(max as f64)
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
                        index,
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
                        index,
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
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectDown, window, cx| {
                    state.step_by(index, -1., window, cx);
                },
            ))
            .on_action(
                window.listener_for(&self.state, move |state, _: &SelectUp, window, cx| {
                    state.step_by(index, 1., window, cx);
                }),
            )
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectFirst, window, cx| {
                    state.to_edge(index, false, window, cx);
                },
            ))
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectLast, window, cx| {
                    state.to_edge(index, true, window, cx);
                },
            ))
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectPageUp, window, cx| {
                    state.step_by(index, 10., window, cx);
                },
            ))
            .on_action(window.listener_for(
                &self.state,
                move |state, _: &SelectPageDown, window, cx| {
                    state.step_by(index, -10., window, cx);
                },
            ))
            .when(disabled, |this| {
                this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .when(!disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &self.state,
                        move |state, _: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            if !state.disabled {
                                state.pointer_thumb = index;
                                state.focus[index].focus(window, cx);
                            }
                        },
                    ),
                )
                .on_drag(DragSlider(entity_id), move |drag, _, window, cx| {
                    let position = window.mouse_position();
                    drag_start.update(cx, |state, cx| state.drag_at_pointer(position, window, cx));
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
                    increment.update(cx, |state, cx| state.step_by(index, 1., window, cx));
                })
                .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
                    decrement.update(cx, |state, cx| state.step_by(index, -1., window, cx));
                })
                .on_a11y_action(
                    AccessibleAction::SetValue,
                    move |data, window, cx| {
                        if let Some(accesskit::ActionData::NumericValue(value)) = data {
                            set_value.update(cx, |state, cx| {
                                if !state.disabled && value.is_finite() {
                                    let value =
                                        value.clamp(state.min as f64, state.max as f64) as f32;
                                    state.set_thumb_value(index, value, window, cx);
                                }
                            });
                        }
                    },
                )
            })
            .children(self.children)
            .a11y_disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        Element as _, Modifiers, Subscription, TestAppContext, VisualTestContext, point, relative,
    };

    use super::*;
    use crate::{Button, Root};

    struct Harness {
        state: Entity<SliderState>,
        axis: Axis,
        direction: crate::Direction,
        before: FocusHandle,
        after: FocusHandle,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = self.state.read(cx);
            let thumbs = state
                .values()
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let percentage = state.percentage(index);
                    SliderThumb::new(&self.state, index)
                        .accessibility_label(format!("Position {}", index + 1))
                        .debug_selector(move || format!("thumb-{index}").into())
                        .absolute()
                        .when(self.axis == Axis::Horizontal, |thumb| {
                            thumb
                                .left(relative(self.direction.physical_fraction(percentage)))
                                .top_0()
                                .ml(px(-6.))
                                .w(px(12.))
                                .h(px(24.))
                        })
                        .when(self.axis == Axis::Vertical, |thumb| {
                            thumb
                                .bottom(relative(percentage))
                                .left_0()
                                .mb(px(-6.))
                                .w(px(24.))
                                .h(px(12.))
                        })
                })
                .collect::<Vec<_>>();
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
                        .direction(self.direction)
                        .relative()
                        .accessibility_label("Position")
                        .when(self.axis == Axis::Horizontal, |this| {
                            this.w(px(200.)).h(px(24.))
                        })
                        .when(self.axis == Axis::Vertical, |this| {
                            this.w(px(24.)).h(px(200.))
                        })
                        .children(thumbs),
                )
                .child(
                    Button::new("after")
                        .h(px(30.))
                        .track_focus(&self.after)
                        .child("After"),
                )
        }
    }

    type Changes = Rc<RefCell<Vec<Vec<f32>>>>;

    fn harness(
        cx: &mut TestAppContext,
        axis: Axis,
        values: Vec<f32>,
    ) -> (&mut VisualTestContext, Entity<Harness>, Changes) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                let state = cx.new(|cx| SliderState::new(3.0..=100.0, 10., values, cx));
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    axis,
                    direction: crate::Direction::Ltr,
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
                let SliderEvent::Change(values) = event;
                changes.borrow_mut().push(values.clone());
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
        let (cx, view, changes) = harness(cx, Axis::Horizontal, vec![34.]);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(state.read(cx).values()[0], 33.);
            assert_eq!(state.read(cx).percentage(0), 30. / 97.);
            state.update(cx, |state, cx| {
                for value in [34., -100., 1000., 94., 96.] {
                    state.set_values([value], window, cx);
                }
            });
            for (max, step, previous) in [(10., 3., 9.), (0.3, 0.1, 0.2)] {
                let edge = cx.new(|cx| SliderState::new(0.0..=max, step, [max], cx));
                edge.update(cx, |state, cx| {
                    state.step_by(0, -1., window, cx);
                    assert_eq!(
                        state.values()[0],
                        previous,
                        "move to the preceding grid value"
                    );
                    state.step_by(0, 1., window, cx);
                    assert_eq!(state.values()[0], max);
                });
            }
            let _ = Slider::new("numeric-probe", &state)
                .axis(Axis::Vertical)
                .accessibility_label("Position")
                .render(window, cx);
            let element = SliderThumb::new(&state, 0)
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
            let element = SliderThumb::new(&state, 0)
                .render(window, cx)
                .into_element();
            let mut disabled = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut disabled);
            assert!(disabled.is_disabled());
            assert!(!disabled.supports_action(AccessibleAction::SetValue));
        });
        assert_eq!(*changes.borrow(), [vec![3.], vec![100.], vec![93.]]);
    }

    #[gpui::test]
    fn keyboard_changes_are_bounded_and_disabling_keeps_a_tab_exit(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal, vec![34.]);
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
                assert!(state.thumb_focus_handle(0).is_focused(window));
                assert_eq!(state.values()[0], expected);
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
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).values()[0], 100.));
        let center = cx.debug_bounds("slider").unwrap().center();
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).values()[0], 100.));
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
            let (cx, view, changes) = harness(cx, axis, vec![34.]);
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
                assert_eq!(state.values()[0], 53.);
                assert!(state.thumb_focus_handle(0).is_focused(window));
            });
            cx.simulate_mouse_move(at(1.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).values()[0], 100.));
            cx.simulate_mouse_move(at(-0.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).values()[0], 3.));
            cx.simulate_mouse_up(at(-0.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.simulate_mouse_move(at(0.5), None, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| {
                assert_eq!(view.read(cx).state.read(cx).values()[0], 3.);
                assert!(!cx.has_active_drag());
            });
            assert_eq!(*changes.borrow(), [vec![53.], vec![100.], vec![3.]]);
        }
    }

    #[gpui::test]
    fn each_thumb_has_stable_tab_focus_and_neighbor_bounded_native_values(cx: &mut TestAppContext) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal, vec![23., 53., 83.]);
        cx.simulate_keystrokes("tab end right tab end tab home end shift-tab home right");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(state.read(cx).values(), [53., 63., 100.]);
            assert!(state.read(cx).thumb_focus_handle(1).is_focused(window));
            for (index, min, value, max) in
                [(0, 3., 53., 63.), (1, 53., 63., 100.), (2, 63., 100., 100.)]
            {
                let element = SliderThumb::new(&state, index)
                    .accessibility_label(format!("Position {}", index + 1))
                    .render(window, cx)
                    .into_element();
                let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                element.write_a11y_info(&mut node);
                assert_eq!(node.role(), Role::Slider);
                assert_eq!(
                    node.label(),
                    Some(format!("Position {}", index + 1).as_str())
                );
                assert_eq!(node.numeric_value(), Some(value));
                assert_eq!(node.min_numeric_value(), Some(min));
                assert_eq!(node.max_numeric_value(), Some(max));
            }
        });
        assert_eq!(
            changes.borrow().len(),
            5,
            "neighbor clamps emit no duplicate changes"
        );
        cx.simulate_keystrokes("tab tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
        cx.simulate_keystrokes("shift-tab shift-tab");
        draw(cx);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert!(state.read(cx).thumb_focus_handle(1).is_focused(window));
            state.update(cx, |state, cx| state.set_disabled(true, window, cx));
        });
        draw(cx);
        cx.simulate_keystrokes("home left tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(view.read(cx).after.is_focused(window));
            assert_eq!(view.read(cx).state.read(cx).values(), [53., 63., 100.]);
        });
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).before.is_focused(window)));
        assert_eq!(changes.borrow().len(), 5);
    }

    #[gpui::test]
    fn range_updates_snap_preserve_order_and_remain_programmatic_while_disabled(
        cx: &mut TestAppContext,
    ) {
        let (cx, view, changes) = harness(cx, Axis::Horizontal, vec![23., 83.]);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                state.set_values([24., 84.], window, cx);
                state.set_thumb_value(0, 200., window, cx);
                state.set_thumb_value(0, 100., window, cx);
                state.set_values([3., 100.], window, cx);
                state.set_disabled(true, window, cx);
                state.set_values([13., 93.], window, cx);
                state.set_thumb_value(1, -100., window, cx);
                assert_eq!(state.values(), [13., 13.]);
            });
        });
        assert_eq!(
            *changes.borrow(),
            [
                vec![83., 83.],
                vec![3., 100.],
                vec![13., 93.],
                vec![13., 13.]
            ]
        );
    }

    #[gpui::test]
    fn track_picks_nearest_and_thumb_drag_keeps_identity_in_rtl_and_vertical_layouts(
        cx: &mut TestAppContext,
    ) {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let (cx, view, changes) = harness(cx, axis, vec![23., 53., 83.]);
            view.update(cx, |view, cx| {
                view.direction = crate::Direction::Rtl;
                cx.notify();
            });
            draw(cx);
            let bounds = cx.debug_bounds("slider").unwrap();
            let at = |fraction: f32| match axis {
                Axis::Horizontal => point(
                    bounds.right() - bounds.size.width * fraction,
                    bounds.center().y,
                ),
                Axis::Vertical => point(
                    bounds.center().x,
                    bounds.bottom() - bounds.size.height * fraction,
                ),
            };
            cx.simulate_click(at(0.95), Modifiers::default());
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.read(cx);
                assert_eq!(state.values(), [23., 53., 93.]);
                assert!(state.thumb_focus_handle(2).is_focused(window));
            });
            let middle = cx.debug_bounds("thumb-1").unwrap().center();
            cx.simulate_mouse_down(middle, MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.simulate_mouse_move(at(1.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.read(cx);
                assert_eq!(state.values(), [23., 93., 93.]);
                assert!(state.thumb_focus_handle(1).is_focused(window));
            });
            cx.simulate_mouse_move(at(-0.25), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.clone();
                assert_eq!(state.read(cx).values(), [23., 23., 93.]);
                state.update(cx, |state, cx| state.set_disabled(true, window, cx));
            });
            draw(cx);
            cx.simulate_mouse_move(at(1.), MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(at(1.), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.update(|window, cx| {
                let state = view.read(cx).state.clone();
                state.update(cx, |state, cx| state.set_disabled(false, window, cx));
                assert_eq!(state.read(cx).values(), [23., 23., 93.]);
                assert!(!cx.has_active_drag());
            });
            draw(cx);
            cx.simulate_mouse_move(at(0.5), None, Modifiers::default());
            draw(cx);
            cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).values(), [23., 23., 93.]));
            assert_eq!(
                *changes.borrow(),
                [
                    vec![23., 53., 93.],
                    vec![23., 93., 93.],
                    vec![23., 23., 93.]
                ]
            );
            for (fraction, expected, selected) in
                [(0.95, [53., 53., 93.], 2), (0.1, [13., 53., 53.], 0)]
            {
                cx.update(|window, cx| {
                    let state = view.read(cx).state.clone();
                    state.update(cx, |state, cx| {
                        state.set_values([53., 53., 53.], window, cx)
                    });
                });
                draw(cx);
                cx.simulate_click(at(fraction), Modifiers::default());
                draw(cx);
                cx.update(|window, cx| {
                    let state = view.read(cx).state.read(cx);
                    assert_eq!(state.values(), expected);
                    assert!(state.thumb_focus_handle(selected).is_focused(window));
                });
            }
            assert_eq!(changes.borrow().len(), 7);
        }
    }

    #[gpui::test]
    fn thumb_count_and_order_are_fixed_configuration_contracts(cx: &mut TestAppContext) {
        cx.new(|cx| {
            for values in [vec![], vec![83., 23.]] {
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        SliderState::new(3.0..=100.0, 10., values, cx)
                    }))
                    .is_err()
                );
            }
            SliderState::new(3.0..=100.0, 10., [23., 83.], cx)
        });
        let (cx, view, changes) = harness(cx, Axis::Horizontal, vec![23., 83.]);
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| {
                for values in [vec![23.], vec![83., 23.]] {
                    assert!(
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            state.set_values(values, window, cx);
                        }))
                        .is_err()
                    );
                }
                assert_eq!(state.values(), [23., 83.]);
            });
        });
        assert!(changes.borrow().is_empty());
    }
}
