//! Headless categorical chart data, visibility, and point inspection.
//!
//! Plot paths, axes, legend, tooltip, and table presentation belong to the
//! application. See the copyable native chart recipe in the catalogue example.

use crate::{
    Direction, ElementExt as _, TestSupportExt as _,
    actions::{SelectFirst, SelectLast, SelectLeft, SelectRight},
};
use gpui::{
    AnyElement, App, Bounds, Context, ElementId, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels, Point,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div,
};

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", SelectLeft, Some("Chart")),
        KeyBinding::new("right", SelectRight, Some("Chart")),
        KeyBinding::new("home", SelectFirst, Some("Chart")),
        KeyBinding::new("end", SelectLast, Some("Chart")),
    ]);
}

/// A named numeric series. Every series must have one finite value per category.
#[derive(Clone, Debug)]
pub struct ChartSeries {
    label: SharedString,
    values: Vec<f64>,
}
impl ChartSeries {
    pub fn new(label: impl Into<SharedString>, values: impl IntoIterator<Item = f64>) -> Self {
        let values: Vec<_> = values.into_iter().collect();
        assert!(
            values.iter().all(|value| value.is_finite()),
            "Chart values must be finite"
        );
        Self {
            label: label.into(),
            values,
        }
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChartEvent {
    Inspect(usize),
    Visibility(usize, bool),
}

/// A fixed categorical dataset and its interactive inspection/legend state.
/// The numeric range includes zero and only visible series. The empty or all-zero
/// dataset uses `0..1`; negative values are supported without a false baseline.
pub struct ChartState {
    categories: Vec<SharedString>,
    series: Vec<ChartSeries>,
    visible: Vec<bool>,
    inspected: Option<usize>,
    focus: FocusHandle,
    bounds: Bounds<Pixels>,
}
impl ChartState {
    pub fn new<L: Into<SharedString>>(
        categories: impl IntoIterator<Item = L>,
        series: impl IntoIterator<Item = ChartSeries>,
        cx: &mut Context<Self>,
    ) -> Self {
        let categories: Vec<_> = categories.into_iter().map(Into::into).collect();
        let series: Vec<_> = series.into_iter().collect();
        assert!(
            series
                .iter()
                .all(|series| series.values.len() == categories.len()),
            "Each chart series needs one value per category"
        );
        Self {
            inspected: (!categories.is_empty()).then_some(0),
            visible: vec![true; series.len()],
            categories,
            series,
            focus: cx.focus_handle(),
            bounds: Bounds::default(),
        }
    }
    pub fn categories(&self) -> &[SharedString] {
        &self.categories
    }
    pub fn series(&self) -> &[ChartSeries] {
        &self.series
    }
    pub fn is_visible(&self, index: usize) -> bool {
        self.visible[index]
    }
    pub fn inspected_index(&self) -> Option<usize> {
        self.inspected
    }
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn set_visible(
        &mut self,
        index: usize,
        visible: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.visible[index] == visible {
            return false;
        }
        self.visible[index] = visible;
        cx.emit(ChartEvent::Visibility(index, visible));
        cx.notify();
        window.refresh();
        true
    }
    pub fn inspect(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if index >= self.categories.len() || self.inspected == Some(index) {
            return false;
        }
        self.inspected = Some(index);
        cx.emit(ChartEvent::Inspect(index));
        cx.notify();
        window.refresh();
        true
    }
    pub fn range(&self) -> (f64, f64) {
        let (min, max) = self
            .series
            .iter()
            .zip(&self.visible)
            .filter(|(_, visible)| **visible)
            .flat_map(|(series, _)| series.values.iter())
            .fold((0_f64, 0_f64), |(min, max), &value| {
                (min.min(value), max.max(value))
            });
        if min == max { (0., 1.) } else { (min, max) }
    }
    /// A unit height above the visible data range's lower bound.
    pub fn normalized_value(&self, value: f64) -> f32 {
        let (min, max) = self.range();
        let scale = min.abs().max(max.abs());
        ((value / scale - min / scale) / (max / scale - min / scale)).clamp(0., 1.) as f32
    }
    pub fn inspected_description(&self) -> SharedString {
        let Some(index) = self.inspected else {
            return "No chart data".into();
        };
        let values = self
            .series
            .iter()
            .zip(&self.visible)
            .filter(|(_, visible)| **visible)
            .map(|(series, _)| format!("{}: {}", series.label, series.values[index]))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{}: {}",
            self.categories[index],
            if values.is_empty() {
                "No visible series"
            } else {
                &values
            }
        )
        .into()
    }
    fn step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            cx.propagate();
            return;
        }
        if let Some(index) = self.inspected {
            let next = if forward {
                (index + 1).min(self.categories.len() - 1)
            } else {
                index.saturating_sub(1)
            };
            self.inspect(next, window, cx);
        }
    }

    fn inspect_at_pointer(
        &mut self,
        position: Point<Pixels>,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.categories.is_empty() || self.bounds.size.width <= gpui::px(0.) {
            return;
        }
        let fraction = direction.physical_fraction(
            ((position.x - self.bounds.left()) / self.bounds.size.width).clamp(0., 1.),
        );
        let index =
            ((fraction * self.categories.len() as f32) as usize).min(self.categories.len() - 1);
        self.inspect(index, window, cx);
    }
}
impl EventEmitter<ChartEvent> for ChartState {}

/// Unstyled plot interaction region. Category centers are evenly spaced across
/// its width. Hover/click inspect a category; Left/Right and Home/End inspect
/// while the plot holds focus. The current exact values form its native value.
///
/// Supply a sized native plot as its child and keep axes/legend outside this
/// hit region. Include an ordinary Table of the same dataset for full nonvisual
/// access. The component does not choose geometry, colors, or numeric formatting.
#[derive(IntoElement)]
pub struct Chart {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: Entity<ChartState>,
    label: SharedString,
    direction: Option<Direction>,
}
impl Chart {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<ChartState>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            base: div().id(id).test_support(),
            state: state.clone(),
            label: label.into(),
            direction: None,
        }
    }
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }
}
impl Styled for Chart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
impl ParentElement for Chart {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(children);
    }
}
impl RenderOnce for Chart {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let direction = self.direction.unwrap_or_else(|| Direction::current(cx));
        let model = self.state.read(cx);
        let focus = model.focus.clone();
        let description = model.inspected_description();
        let previous = self.state.clone();
        let next = self.state.clone();
        let first = self.state.clone();
        let last = self.state.clone();
        let geometry = self.state.clone();
        self.base.role(Role::Group).aria_label(self.label).aria_value(description)
            .aria_description("Use Left and Right to inspect categories. The accompanying data table contains all values.")
            .track_focus(&focus).key_context("Chart")
            .on_action(move |_:&SelectLeft,window,cx|{previous.update(cx,|state,cx|state.step(direction.horizontal_forward(false),window,cx));})
            .on_action(move |_:&SelectRight,window,cx|{next.update(cx,|state,cx|state.step(direction.horizontal_forward(true),window,cx));})
            .on_action(move |_:&SelectFirst,window,cx|{first.update(cx,|state,cx|{if state.focus.is_focused(window){state.inspect(0,window,cx);}else{cx.propagate();}});})
            .on_action(move |_:&SelectLast,window,cx|{last.update(cx,|state,cx|{if state.focus.is_focused(window){if !state.categories.is_empty(){state.inspect(state.categories.len()-1,window,cx);}}else{cx.propagate();}});})
            .on_mouse_down(MouseButton::Left,window.listener_for(&self.state,move |state,event: &gpui::MouseDownEvent,window,cx|{
                state.focus.focus(window,cx);
                state.inspect_at_pointer(event.position,direction,window,cx);
            }))
            .on_mouse_move(window.listener_for(&self.state,move |state,event: &gpui::MouseMoveEvent,window,cx|{
                state.inspect_at_pointer(event.position,direction,window,cx);
            }))
            .on_prepaint(move |bounds,_,cx|geometry.update(cx,|state,_|state.bounds=bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Root;
    use gpui::{AppContext as _, Render, Subscription, TestAppContext, point, px};
    struct Harness {
        state: Entity<ChartState>,
        _subscription: Subscription,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Chart::new("chart", &self.state, "Session delta")
                .w(px(300.))
                .h(px(120.))
        }
    }
    #[gpui::test]
    fn inspection_and_legend_keep_exact_values_and_signed_domain(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let state = cx.new(|cx| {
                    ChartState::new(
                        ["Mon", "Tue", "Wed"],
                        [
                            ChartSeries::new("Desktop", [-10., 0., 20.]),
                            ChartSeries::new("Mobile", [5., 10., 15.]),
                        ],
                        cx,
                    )
                });
                let subscription = cx.observe(&state, |_, _, cx| cx.notify());
                Harness {
                    state,
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
            let state = view.read(cx).state.clone();
            state.read(cx).focus.clone().focus(window, cx);
            window.draw(cx).clear(cx);
            assert_eq!(state.read(cx).range(), (-10., 20.));
            assert_eq!(state.read(cx).normalized_value(-10.), 0.);
            assert_eq!(state.read(cx).normalized_value(20.), 1.);
        });
        cx.simulate_keystrokes("right");
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx)
                    .state
                    .read(cx)
                    .inspected_description()
                    .as_ref(),
                "Tue: Desktop: 0, Mobile: 10"
            )
        });
        cx.simulate_mouse_move(point(px(290.), px(50.)), None, gpui::Modifiers::default());
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            assert_eq!(state.read(cx).inspected_index(), Some(2));
            state.update(cx, |state, cx| {
                state.set_visible(0, false, window, cx);
                assert_eq!(state.range(), (0., 15.));
                assert_eq!(state.inspected_description().as_ref(), "Wed: Mobile: 15");
                state.set_visible(1, false, window, cx);
                assert_eq!(state.range(), (0., 1.));
            });
        });
        cx.simulate_mouse_down(
            point(px(10.), px(50.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(50.)),
            MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert_eq!(
                state.inspected_index(),
                Some(0),
                "a press inspects without an earlier move"
            );
            assert!(state.focus_handle().is_focused(window));
        });
    }
}
