//! A single-slide carousel with native focus, discrete swiping, and logical navigation.

use gpui::{
    AnyElement, App, AppContext as _, Axis, Context, DragMoveEvent, ElementId, Empty, Entity,
    EntityId, EventEmitter, FocusHandle, InteractiveElement, IntoElement, KeyBinding, MouseButton,
    ParentElement, Pixels, Point, Render, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};

use crate::{
    Button, Direction, TestSupportExt as _,
    actions::{SelectDown, SelectFirst, SelectLast, SelectLeft, SelectRight, SelectUp},
};

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some("Carousel && axis == horizontal")),
        KeyBinding::new("right", SelectRight, Some("Carousel && axis == horizontal")),
        KeyBinding::new("up", SelectUp, Some("Carousel && axis == vertical")),
        KeyBinding::new("down", SelectDown, Some("Carousel && axis == vertical")),
        KeyBinding::new("home", SelectFirst, Some("Carousel")),
        KeyBinding::new("end", SelectLast, Some("Carousel")),
    ]);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CarouselEvent {
    Change(usize),
}

/// Retained state for a fixed ordered set of slides. Labels name each slide.
/// Only the selected slide is mounted; application entities retain slide data.
/// There is no autoplay, timer, or focus movement merely because data changes.
pub struct CarouselState {
    labels: Vec<SharedString>,
    selected: Option<usize>,
    looping: bool,
    focus: FocusHandle,
    slide_focus: Vec<FocusHandle>,
    drag_origin: Point<Pixels>,
    drag_committed: bool,
}

impl CarouselState {
    pub fn new<L: Into<SharedString>>(
        labels: impl IntoIterator<Item = L>,
        cx: &mut Context<Self>,
    ) -> Self {
        let labels: Vec<_> = labels.into_iter().map(Into::into).collect();
        Self {
            selected: (!labels.is_empty()).then_some(0),
            slide_focus: labels.iter().map(|_| cx.focus_handle()).collect(),
            labels,
            looping: false,
            focus: cx.focus_handle(),
            drag_origin: Point::default(),
            drag_committed: false,
        }
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }
    pub fn labels(&self) -> &[SharedString] {
        &self.labels
    }
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }
    pub fn can_previous(&self) -> bool {
        self.labels.len() > 1 && (self.looping || self.selected.is_some_and(|index| index > 0))
    }
    pub fn can_next(&self) -> bool {
        self.labels.len() > 1
            && (self.looping
                || self
                    .selected
                    .is_some_and(|index| index + 1 < self.labels.len()))
    }

    /// Selects an existing slide. Focus inside the outgoing slide returns to the
    /// carousel scope before that content is unmounted; external focus stays put.
    pub fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if index >= self.labels.len() || self.selected == Some(index) {
            return false;
        }
        if let Some(previous) = self.selected
            && self.slide_focus[previous].contains_focused(window, cx)
        {
            self.focus.focus(window, cx);
        }
        self.selected = Some(index);
        cx.emit(CarouselEvent::Change(index));
        cx.notify();
        window.refresh();
        true
    }

    pub fn previous(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.advance(false, window, cx)
    }
    pub fn next(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.advance(true, window, cx)
    }
    fn advance(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        if if forward {
            !self.can_next()
        } else {
            !self.can_previous()
        } {
            return false;
        }
        let next = if forward {
            (index + 1) % self.labels.len()
        } else {
            (index + self.labels.len() - 1) % self.labels.len()
        };
        self.select(next, window, cx)
    }
    fn key_advance(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.is_focused(window) {
            self.advance(forward, window, cx);
        } else {
            cx.propagate();
        }
    }
    fn key_edge(&mut self, last: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            cx.propagate();
            return;
        }
        if !self.labels.is_empty() {
            self.select(if last { self.labels.len() - 1 } else { 0 }, window, cx);
        }
    }
}
impl EventEmitter<CarouselEvent> for CarouselState {}

/// Named carousel region. Arrow keys act only while the region itself has focus,
/// leaving interactive slide descendants their own key bindings. Use native
/// Buttons or [`CarouselControl`] for pointer and keyboard previous/next actions.
#[derive(IntoElement)]
pub struct Carousel {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CarouselState>,
    label: SharedString,
    axis: Axis,
    direction: Option<Direction>,
}
impl Carousel {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<CarouselState>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: label.into(),
            axis: Axis::Horizontal,
            direction: None,
        }
    }
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }
}
impl Styled for Carousel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
impl ParentElement for Carousel {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}
impl RenderOnce for Carousel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let direction = self.direction.unwrap_or_else(|| Direction::current(cx));
        let focus = self.state.read(cx).focus.clone();
        let first = self.state.clone();
        let last = self.state.clone();
        let previous = self.state.clone();
        let next = self.state;
        let base = self
            .base
            .role(Role::Region)
            .aria_label(self.label)
            .track_focus(&focus)
            .on_action(move |_: &SelectFirst, window, cx| {
                first.update(cx, |state, cx| state.key_edge(false, window, cx));
            })
            .on_action(move |_: &SelectLast, window, cx| {
                last.update(cx, |state, cx| state.key_edge(true, window, cx));
            });
        match self.axis {
            Axis::Horizontal => base
                .key_context("Carousel axis=horizontal")
                .on_action(move |_: &SelectLeft, window, cx| {
                    previous.update(cx, |state, cx| {
                        state.key_advance(direction.horizontal_forward(false), window, cx)
                    });
                })
                .on_action(move |_: &SelectRight, window, cx| {
                    next.update(cx, |state, cx| {
                        state.key_advance(direction.horizontal_forward(true), window, cx)
                    });
                }),
            Axis::Vertical => base
                .key_context("Carousel axis=vertical")
                .on_action(move |_: &SelectUp, window, cx| {
                    previous.update(cx, |state, cx| state.key_advance(false, window, cx));
                })
                .on_action(move |_: &SelectDown, window, cx| {
                    next.update(cx, |state, cx| state.key_advance(true, window, cx));
                }),
        }
    }
}

#[derive(Clone)]
struct DragCarousel(EntityId);
impl Render for DragCarousel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// One active-only slide with position/count metadata and a stable focus scope.
/// Dragging its content by 40 logical pixels snaps once per gesture. GPUI owns
/// drag cancellation and suppresses click activation after a drag begins. The
/// slide and region must use the same axis/direction. There is no inertia or
/// autoplay; use application animation only when the product calls for it.
#[derive(IntoElement)]
pub struct CarouselSlide {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<CarouselState>,
    index: usize,
    axis: Axis,
    direction: Option<Direction>,
}
impl CarouselSlide {
    pub fn new(state: &Entity<CarouselState>, index: usize) -> Self {
        Self {
            base: div()
                .id((
                    ElementId::from(("carousel-slide", state.entity_id())),
                    index.to_string(),
                ))
                .test_support(),
            state: state.clone(),
            index,
            axis: Axis::Horizontal,
            direction: None,
        }
    }
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }
}
impl Styled for CarouselSlide {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
impl ParentElement for CarouselSlide {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}
impl RenderOnce for CarouselSlide {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        assert!(
            self.index < model.labels.len(),
            "CarouselSlide index must identify a slide"
        );
        if model.selected != Some(self.index) {
            return Empty.into_any_element();
        }
        let direction = self.direction.unwrap_or_else(|| Direction::current(cx));
        let axis = self.axis;
        let id = self.state.entity_id();
        let drag = self.state.clone();
        self.base
            .role(Role::Group)
            .aria_label(model.labels[self.index].clone())
            .aria_position_in_set(self.index + 1)
            .aria_size_of_set(model.labels.len())
            .track_focus(&model.slide_focus[self.index].clone().tab_stop(false))
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.state, |state, event: &gpui::MouseDownEvent, _, _| {
                    state.drag_origin = event.position;
                    state.drag_committed = false;
                }),
            )
            .on_drag(DragCarousel(id), move |value, _, _, cx| {
                drag.update(cx, |state, _| state.drag_committed = false);
                cx.stop_propagation();
                cx.new(|_| value.clone())
            })
            .on_drag_move(window.listener_for(
                &self.state,
                move |state, event: &DragMoveEvent<DragCarousel>, window, cx| {
                    if event.drag(cx).0 != id || state.drag_committed {
                        return;
                    }
                    let delta = match axis {
                        Axis::Horizontal => event.event.position.x - state.drag_origin.x,
                        Axis::Vertical => event.event.position.y - state.drag_origin.y,
                    };
                    if delta.abs() >= px(40.) {
                        state.drag_committed = true;
                        let forward = if axis == Axis::Horizontal {
                            direction.horizontal_forward(delta < px(0.))
                        } else {
                            delta < px(0.)
                        };
                        state.advance(forward, window, cx);
                    }
                },
            ))
            .into_any_element()
    }
}

/// Previous/next Button composition, including boundary disabled behavior.
#[derive(IntoElement)]
pub struct CarouselControl {
    button: Button,
    state: Entity<CarouselState>,
    forward: bool,
}
impl CarouselControl {
    pub fn previous(id: impl Into<ElementId>, state: &Entity<CarouselState>) -> Self {
        Self {
            button: Button::new(id).accessibility_label("Previous slide"),
            state: state.clone(),
            forward: false,
        }
    }
    pub fn next(id: impl Into<ElementId>, state: &Entity<CarouselState>) -> Self {
        Self {
            button: Button::new(id).accessibility_label("Next slide"),
            state: state.clone(),
            forward: true,
        }
    }
}
impl Styled for CarouselControl {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}
impl ParentElement for CarouselControl {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(children);
    }
}
impl RenderOnce for CarouselControl {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let enabled = if self.forward {
            self.state.read(cx).can_next()
        } else {
            self.state.read(cx).can_previous()
        };
        self.button
            .disabled(!enabled)
            .on_click(move |_, window, cx| {
                self.state.update(cx, |state, cx| {
                    state.advance(self.forward, window, cx);
                });
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Root;
    use gpui::{Modifiers, Subscription, TestAppContext, point};

    struct Harness {
        state: Entity<CarouselState>,
        button: FocusHandle,
        _subscription: Subscription,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Carousel::new("carousel", &self.state, "Screenshots")
                .w(px(300.))
                .child(
                    CarouselSlide::new(&self.state, 0)
                        .w(px(300.))
                        .h(px(120.))
                        .child("First image"),
                )
                .child(
                    CarouselSlide::new(&self.state, 1)
                        .w(px(300.))
                        .h(px(120.))
                        .child(
                            Button::new("slide-action")
                                .track_focus(&self.button)
                                .child("Inspect image"),
                        ),
                )
                .child(CarouselControl::previous("previous", &self.state).child("Previous"))
                .child(CarouselControl::next("next", &self.state).child("Next"))
        }
    }
    #[gpui::test]
    fn keys_swipe_and_outgoing_slide_keep_focus_reachable(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let state = cx.new(|cx| CarouselState::new(["First", "Second"], cx));
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
                    button: cx.focus_handle(),
                    _subscription: subscription,
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
            let focus = view.read(cx).state.read(cx).focus.clone();
            focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("right");
        cx.run_until_parked();
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(state.read(cx).selected_index(), Some(1));
            assert!(!state.read(cx).can_next());
            let button = view.read(cx).button.clone();
            button.focus(window, cx);
            window.draw(cx).clear(cx);
            state.update(cx, |state, cx| {
                assert!(state.previous(window, cx));
                assert!(state.focus.is_focused(window));
            });
            window.draw(cx).clear(cx);
        });
        // A single native drag snaps once, even if the pointer continues moving.
        cx.simulate_mouse_down(
            point(px(180.), px(50.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(130.), px(50.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(90.), px(50.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(90.), px(50.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(state.read(cx).selected_index(), Some(1));
            state.update(cx, |state, cx| {
                assert!(!state.next(window, cx));
                state.looping = true;
                assert!(state.next(window, cx));
                assert_eq!(state.selected, Some(0));
            });
        });
    }
}
