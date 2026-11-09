//! A two-pane split with shared native layout and one focusable divider.

use std::ops::RangeInclusive;

use gpui::{
    AccessibleAction, AnyElement, App, AppContext as _, Axis, Bounds, Context, DragMoveEvent,
    ElementId, Empty, Entity, EntityId, EventEmitter, FocusHandle, InteractiveElement,
    Interactivity, IntoElement, KeyBinding, MouseButton, MouseDownEvent, Orientation,
    ParentElement, Pixels, Point, Render, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, accesskit, div,
    prelude::FluentBuilder as _, px,
};

use crate::{
    ElementExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
};

const STEP: f32 = 0.02;

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(
            "left",
            SelectLeft,
            Some("ResizableHandle && axis == horizontal"),
        ),
        KeyBinding::new(
            "right",
            SelectRight,
            Some("ResizableHandle && axis == horizontal"),
        ),
        KeyBinding::new("up", SelectUp, Some("ResizableHandle && axis == vertical")),
        KeyBinding::new(
            "down",
            SelectDown,
            Some("ResizableHandle && axis == vertical"),
        ),
        KeyBinding::new("home", SelectFirst, Some("ResizableHandle")),
        KeyBinding::new("end", SelectLast, Some("ResizableHandle")),
    ]);
}

/// Emitted once when the first pane's ratio changes, including programmatic updates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizableEvent {
    Change(f32),
}

/// Retained first-pane ratio and divider focus for one mounted two-pane group.
///
/// The ratio divides the space remaining after the actual divider thickness.
/// Constraints are fractions of that usable space, so they stay valid when the
/// application's window changes size. Both panes remain mounted. Nest groups
/// to compose more panes; this state does not manage collapse or persistence.
pub struct ResizableState {
    axis: Axis,
    ratio: f32,
    minimum: f32,
    maximum: f32,
    disabled: bool,
    focus: FocusHandle,
    first_bounds: Bounds<Pixels>,
    second_bounds: Bounds<Pixels>,
    handle_bounds: Bounds<Pixels>,
    grab: Pixels,
}

impl ResizableState {
    /// Starts with equal panes, with each retaining at least ten percent.
    pub fn new(axis: Axis, cx: &mut Context<Self>) -> Self {
        Self {
            axis,
            ratio: 0.5,
            minimum: 0.1,
            maximum: 0.9,
            disabled: false,
            focus: cx.focus_handle(),
            first_bounds: Bounds::default(),
            second_bounds: Bounds::default(),
            handle_bounds: Bounds::default(),
            grab: px(0.),
        }
    }

    /// Bounds the first pane's share before mounting; the second uses the rest.
    ///
    /// # Panics
    /// Bounds must be finite and satisfy `0 < minimum < maximum < 1`. Zero-size
    /// collapse is deliberately outside this two-visible-pane contract.
    pub fn limits(mut self, limits: RangeInclusive<f32>) -> Self {
        let (minimum, maximum) = limits.into_inner();
        assert!(
            minimum.is_finite()
                && maximum.is_finite()
                && 0. < minimum
                && minimum < maximum
                && maximum < 1.,
            "Resizable limits must satisfy 0 < minimum < maximum < 1"
        );
        self.minimum = minimum;
        self.maximum = maximum;
        self.ratio = self.ratio.clamp(minimum, maximum);
        self
    }

    /// Sets a finite initial share, clamped to the limits without an event.
    pub fn default_ratio(mut self, ratio: f32) -> Self {
        assert!(ratio.is_finite(), "Resizable ratio must be finite");
        self.ratio = ratio.clamp(self.minimum, self.maximum);
        self
    }

    pub fn axis(&self) -> Axis {
        self.axis
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    pub fn min_ratio(&self) -> f32 {
        self.minimum
    }

    pub fn max_ratio(&self) -> f32 {
        self.maximum
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Changes the first pane's share, clamped to its limits. Programmatic
    /// changes remain allowed while user resizing is disabled.
    pub fn set_ratio(&mut self, ratio: f32, window: &mut Window, cx: &mut Context<Self>) {
        assert!(ratio.is_finite(), "Resizable ratio must be finite");
        let ratio = ratio.clamp(self.minimum, self.maximum);
        if self.ratio != ratio {
            self.ratio = ratio;
            cx.emit(ResizableEvent::Change(ratio));
            cx.notify();
            window.refresh();
        }
    }

    /// Disables pointer, keyboard, and accessibility resizing, and normal Tab
    /// entry. An already focused divider retains its binding so Tab can exit.
    pub fn set_disabled(&mut self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            cx.notify();
            window.refresh();
        }
    }

    fn nudge(&mut self, amount: f32, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled {
            self.set_ratio(self.ratio + amount, window, cx);
        }
    }

    fn edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled {
            self.set_ratio(if last { self.maximum } else { self.minimum }, window, cx);
        }
    }

    fn pointer_coordinate(&self, point: Point<Pixels>) -> Pixels {
        match self.axis {
            Axis::Horizontal => point.x,
            Axis::Vertical => point.y,
        }
    }

    fn length(&self, bounds: Bounds<Pixels>) -> Pixels {
        match self.axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
    }

    fn resize_at_pointer(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let usable = self.length(self.first_bounds) + self.length(self.second_bounds);
        if usable > px(0.) {
            let first = self.pointer_coordinate(position)
                - self.pointer_coordinate(self.first_bounds.origin)
                - self.grab;
            self.set_ratio(first / usable, window, cx);
        }
    }
}

impl EventEmitter<ResizableEvent> for ResizableState {}

#[derive(Clone)]
struct DragResizable(EntityId);

impl Render for DragResizable {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// An unstyled two-pane layout. Give its frame bounded dimensions, supply pane
/// contents, and size the divider through [`Self::handle`]. Horizontal groups
/// use the handle's width; vertical groups use its height.
///
/// Ordinary styles apply to the frame, including padding. The inner structure
/// owns pane sizes and has no gap; spacing belongs inside pane content. Panes
/// clip overflowing content, so compose ScrollArea when it should scroll.
#[derive(IntoElement)]
pub struct Resizable {
    id: ElementId,
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ResizableState>,
    first: Option<AnyElement>,
    second: Option<AnyElement>,
    handle: ResizableHandle,
}

impl Resizable {
    pub fn new(id: impl Into<ElementId>, state: &Entity<ResizableState>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            state: state.clone(),
            first: None,
            second: None,
            handle: ResizableHandle::new((id.clone(), "handle"), state),
            id,
        }
    }

    pub fn first(mut self, content: impl IntoElement) -> Self {
        self.first = Some(content.into_any_element());
        self
    }

    pub fn second(mut self, content: impl IntoElement) -> Self {
        self.second = Some(content.into_any_element());
        self
    }

    pub fn handle(mut self, build: impl FnOnce(ResizableHandle) -> ResizableHandle) -> Self {
        self.handle = build(self.handle);
        self
    }
}

impl Styled for Resizable {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Resizable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let axis = model.axis;
        let ratio = model.ratio;
        let first_geometry = self.state.clone();
        let second_geometry = self.state;
        let first = div()
            .id((self.id.clone(), "first"))
            .test_support()
            .min_w_0()
            .min_h_0()
            .flex_basis(px(0.))
            .flex_grow(ratio)
            .overflow_hidden()
            .children(self.first)
            .on_prepaint(move |bounds, _, cx| {
                first_geometry.update(cx, |state, _| state.first_bounds = bounds);
            });
        let second = div()
            .id((self.id.clone(), "second"))
            .test_support()
            .min_w_0()
            .min_h_0()
            .flex_basis(px(0.))
            .flex_grow(1. - ratio)
            .overflow_hidden()
            .children(self.second)
            .on_prepaint(move |bounds, _, cx| {
                second_geometry.update(cx, |state, _| state.second_bounds = bounds);
            });
        self.base.flex().flex_col().child(
            div()
                .id((self.id, "layout"))
                .flex()
                .flex_1()
                .w_full()
                .min_w_0()
                .min_h_0()
                .gap_0()
                .when(axis == Axis::Vertical, |this| this.flex_col())
                .child(first)
                .child(self.handle)
                .child(second),
        )
    }
}

/// The group's focusable divider. Supply a non-interactive visual grip and an
/// accessible label describing the pane it changes. Arrow keys along the
/// group's layout axis adjust the first pane by two percentage points; Home
/// and End reach its minimum and maximum. Other-axis keys propagate normally.
///
/// Native orientation describes the divider line, perpendicular to the layout
/// axis. Numeric accessibility values use percentages of the usable pane space.
/// Main-axis margins are reserved for the group's geometry and are reset.
#[derive(IntoElement)]
pub struct ResizableHandle {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ResizableState>,
    label: Option<SharedString>,
}

impl ResizableHandle {
    fn new(id: impl Into<ElementId>, state: &Entity<ResizableState>) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Styled for ResizableHandle {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ResizableHandle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for ResizableHandle {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ResizableHandle {}

impl RenderOnce for ResizableHandle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let axis = model.axis;
        let disabled = model.disabled;
        let focus = model.focus.clone();
        let entity_id = self.state.entity_id();
        let geometry = self.state.clone();
        let drag_start = self.state.clone();
        let increment = self.state.clone();
        let decrement = self.state.clone();
        let set_value = self.state.clone();

        self.base
            .role(Role::Splitter)
            .flex_shrink_0()
            .aria_numeric_value((model.ratio * 100.) as f64)
            .aria_min_numeric_value((model.minimum * 100.) as f64)
            .aria_max_numeric_value((model.maximum * 100.) as f64)
            .aria_numeric_value_step((STEP * 100.) as f64)
            .when_some(self.label, |this, label| this.aria_label(label))
            .when(axis == Axis::Horizontal, |this| {
                this.mx_0()
                    .cursor_col_resize()
                    .aria_orientation(Orientation::Vertical)
                    .key_context("ResizableHandle axis=horizontal")
            })
            .when(axis == Axis::Vertical, |this| {
                this.my_0()
                    .cursor_row_resize()
                    .aria_orientation(Orientation::Horizontal)
                    .key_context("ResizableHandle axis=vertical")
            })
            .when(!disabled || focus.is_focused(window), |this| {
                this.track_focus(&focus.tab_stop(!disabled))
            })
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectLeft, window, cx| {
                    state.nudge(-STEP, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectRight, window, cx| {
                    state.nudge(STEP, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectUp, window, cx| {
                    state.nudge(-STEP, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectDown, window, cx| {
                    state.nudge(STEP, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectFirst, window, cx| {
                    state.edge(false, window, cx);
                }),
            )
            .on_action(
                window.listener_for(&self.state, |state, _: &SelectLast, window, cx| {
                    state.edge(true, window, cx);
                }),
            )
            .when(disabled, |this| {
                this.cursor_default()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .when(!disabled, |this| {
                this.on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &self.state,
                        |state, event: &MouseDownEvent, window, cx| {
                            if state.disabled {
                                return;
                            }
                            state.grab = state.pointer_coordinate(event.position)
                                - state.pointer_coordinate(state.handle_bounds.origin);
                            state.focus.focus(window, cx);
                        },
                    ),
                )
                .on_drag(DragResizable(entity_id), move |drag, _, window, cx| {
                    let position = window.mouse_position();
                    drag_start.update(cx, |state, cx| {
                        state.resize_at_pointer(position, window, cx)
                    });
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(window.listener_for(
                    &self.state,
                    move |state, event: &DragMoveEvent<DragResizable>, window, cx| {
                        if event.drag(cx).0 == entity_id {
                            state.resize_at_pointer(event.event.position, window, cx);
                        }
                    },
                ))
                .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
                    increment.update(cx, |state, cx| state.nudge(STEP, window, cx));
                })
                .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
                    decrement.update(cx, |state, cx| state.nudge(-STEP, window, cx));
                })
                .on_a11y_action(
                    AccessibleAction::SetValue,
                    move |data, window, cx| {
                        if let Some(accesskit::ActionData::NumericValue(value)) = data
                            && value.is_finite()
                        {
                            set_value.update(cx, |state, cx| {
                                if !state.disabled {
                                    let ratio = (value / 100.)
                                        .clamp(state.minimum as f64, state.maximum as f64);
                                    state.set_ratio(ratio as f32, window, cx);
                                }
                            });
                        }
                    },
                )
            })
            .on_prepaint(move |bounds, _, cx| {
                geometry.update(cx, |state, _| state.handle_bounds = bounds);
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
        state: Entity<ResizableState>,
        before: FocusHandle,
        first_action: FocusHandle,
        second_action: FocusHandle,
        after: FocusHandle,
        extent: Pixels,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let axis = self.state.read(cx).axis();
            div()
                .size(px(600.))
                .flex()
                .flex_col()
                .items_start()
                .child(
                    Button::new("before")
                        .track_focus(&self.before)
                        .h(px(30.))
                        .child("Before"),
                )
                .child(
                    Resizable::new("split", &self.state)
                        .w(if axis == Axis::Horizontal {
                            self.extent
                        } else {
                            px(120.)
                        })
                        .h(if axis == Axis::Vertical {
                            self.extent
                        } else {
                            px(120.)
                        })
                        .p(px(10.))
                        .flex_shrink_0()
                        .first(
                            Button::new("first-action")
                                .track_focus(&self.first_action)
                                .h(px(30.))
                                .child("First"),
                        )
                        .second(
                            Button::new("second-action")
                                .track_focus(&self.second_action)
                                .h(px(30.))
                                .child("Second"),
                        )
                        .handle(|handle| {
                            handle
                                .accessibility_label("First pane size")
                                .when(axis == Axis::Horizontal, |handle| handle.w(px(10.)))
                                .when(axis == Axis::Vertical, |handle| handle.h(px(10.)))
                        }),
                )
                .child(
                    Button::new("after")
                        .track_focus(&self.after)
                        .h(px(30.))
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
                let state = cx.new(|cx| {
                    ResizableState::new(axis, cx)
                        .limits(0.2..=0.8)
                        .default_ratio(0.5)
                });
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    before: cx.focus_handle(),
                    first_action: cx.focus_handle(),
                    second_action: cx.focus_handle(),
                    after: cx.focus_handle(),
                    extent: px(320.),
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
            let state = view.read(cx).state.clone();
            let changes = changes.clone();
            cx.subscribe(&state, move |_, event: &ResizableEvent, _| {
                let ResizableEvent::Change(ratio) = event;
                changes.borrow_mut().push(*ratio);
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

    fn assert_ratio(cx: &mut VisualTestContext, view: &Entity<Harness>, expected: f32) {
        cx.update(|_, cx| {
            let ratio = view.read(cx).state.read(cx).ratio();
            assert!((ratio - expected).abs() < 0.000001, "{ratio} != {expected}");
        });
    }

    #[gpui::test]
    fn divider_keyboard_and_native_semantics_preserve_child_focus(cx: &mut TestAppContext) {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let (cx, view, changes) = harness(cx, axis);
            let increase = if axis == Axis::Horizontal {
                "right"
            } else {
                "down"
            };
            let decrease = if axis == Axis::Horizontal {
                "left"
            } else {
                "up"
            };
            let other_axis = if axis == Axis::Horizontal {
                "down"
            } else {
                "right"
            };
            cx.simulate_keystrokes("tab");
            cx.simulate_keystrokes(increase);
            draw(cx);
            cx.update(|window, cx| assert!(view.read(cx).first_action.is_focused(window)));
            assert_ratio(cx, &view, 0.5);

            cx.simulate_keystrokes("tab");
            cx.simulate_keystrokes(increase);
            draw(cx);
            assert_ratio(cx, &view, 0.52);
            cx.simulate_keystrokes(other_axis);
            draw(cx);
            assert_ratio(cx, &view, 0.52);
            cx.simulate_keystrokes(decrease);
            cx.simulate_keystrokes("home end end");
            draw(cx);
            assert_ratio(cx, &view, 0.8);
            assert_eq!(changes.borrow().len(), 4);

            cx.update(|window, cx| {
                let state = view.read(cx).state.clone();
                assert!(state.read(cx).focus_handle().is_focused(window));
                let element = ResizableHandle::new("probe", &state)
                    .accessibility_label("First pane size")
                    .render(window, cx)
                    .into_element();
                let mut node = accesskit::Node::new(Role::Splitter);
                element.write_a11y_info(&mut node);
                assert_eq!(element.a11y_role(), Some(Role::Splitter));
                assert_eq!(node.label(), Some("First pane size"));
                assert_eq!(node.numeric_value(), Some(80.));
                assert_eq!(node.min_numeric_value(), Some(20.));
                assert_eq!(node.max_numeric_value(), Some(80.));
                assert_eq!(node.numeric_value_step(), Some(2.));
                assert_eq!(
                    node.orientation(),
                    Some(if axis == Axis::Horizontal {
                        Orientation::Vertical
                    } else {
                        Orientation::Horizontal
                    })
                );
                assert!(node.supports_action(AccessibleAction::SetValue));
                state.update(cx, |state, cx| state.set_disabled(true, window, cx));
                let element = ResizableHandle::new("disabled-probe", &state)
                    .render(window, cx)
                    .into_element();
                let mut node = accesskit::Node::new(Role::Splitter);
                element.write_a11y_info(&mut node);
                assert!(node.is_disabled());
                assert!(!node.supports_action(AccessibleAction::SetValue));
            });
            draw(cx);
            cx.simulate_keystrokes("home");
            cx.simulate_keystrokes(decrease);
            draw(cx);
            assert_ratio(cx, &view, 0.8);
            cx.simulate_keystrokes("tab");
            draw(cx);
            cx.update(|window, cx| assert!(view.read(cx).second_action.is_focused(window)));
            cx.simulate_keystrokes("shift-tab");
            draw(cx);
            cx.update(|window, cx| assert!(view.read(cx).first_action.is_focused(window)));
            assert_eq!(changes.borrow().len(), 4);
        }
    }

    #[gpui::test]
    fn pointer_resize_uses_actual_pane_space_and_retains_ratio_on_parent_resize(
        cx: &mut TestAppContext,
    ) {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let (cx, view, _) = harness(cx, axis);
            let (handle, origin, usable) = cx.update(|_, cx| {
                let state = view.read(cx).state.read(cx);
                let usable = state.length(state.first_bounds) + state.length(state.second_bounds);
                assert_eq!(
                    usable,
                    px(290.),
                    "subtract frame padding and actual divider thickness"
                );
                assert_eq!(state.length(state.first_bounds), px(145.));
                (state.handle_bounds, state.first_bounds.origin, usable)
            });
            let grab = px(2.);
            let start = match axis {
                Axis::Horizontal => point(handle.left() + grab, handle.center().y),
                Axis::Vertical => point(handle.center().x, handle.top() + grab),
            };
            let at = |ratio: f32| match axis {
                Axis::Horizontal => point(origin.x + usable * ratio + grab, start.y),
                Axis::Vertical => point(start.x, origin.y + usable * ratio + grab),
            };
            cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
            draw(cx);
            assert_ratio(cx, &view, 0.5);
            cx.simulate_mouse_move(at(0.7), MouseButton::Left, Modifiers::default());
            draw(cx);
            assert_ratio(cx, &view, 0.7);
            cx.simulate_mouse_move(at(1.2), MouseButton::Left, Modifiers::default());
            draw(cx);
            assert_ratio(cx, &view, 0.8);
            cx.simulate_mouse_up(at(1.2), MouseButton::Left, Modifiers::default());
            draw(cx);
            cx.simulate_mouse_move(at(0.3), None, Modifiers::default());
            draw(cx);
            assert_ratio(cx, &view, 0.8);
            cx.update(|_, cx| assert!(!cx.has_active_drag()));

            view.update(cx, |view, cx| {
                view.extent = px(420.);
                cx.notify();
            });
            draw(cx);
            cx.update(|_, cx| {
                let state = view.read(cx).state.read(cx);
                let first = state.length(state.first_bounds);
                let second = state.length(state.second_bounds);
                assert_eq!(first + second, px(390.));
                assert!((first / (first + second) - 0.8).abs() < 0.001);
            });
        }
    }
}
