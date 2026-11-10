//! Scoped logical direction for layout and horizontal interactions.

use gpui::{
    AnyElement, App, Bounds, Div, Element, ElementId, Global, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ParentElement, Pixels, StyleRefinement, Styled, Window, div,
};

/// The logical order of horizontal content. Vertical order is unchanged.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Direction {
    #[default]
    Ltr,
    Rtl,
}

impl Direction {
    /// The nearest provider during element rendering; LTR outside a provider.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<DirectionScope>()
            .map_or(Self::Ltr, |scope| scope.0)
    }

    pub fn is_rtl(self) -> bool {
        self == Self::Rtl
    }

    /// Whether a physical Left/Right action advances the logical item order.
    pub fn horizontal_forward(self, physical_right: bool) -> bool {
        physical_right != self.is_rtl()
    }

    /// Maps a logical `0..=1` position to its physical horizontal position.
    pub fn physical_fraction(self, logical: f32) -> f32 {
        if self.is_rtl() { 1. - logical } else { logical }
    }

    /// Applies logical row layout to an application-owned container.
    /// Column layout, spacing, and all decorative styles remain caller-owned.
    pub fn row<E: Styled>(self, element: E) -> E {
        if self.is_rtl() {
            element.flex().flex_row_reverse()
        } else {
            element.flex().flex_row()
        }
    }

    fn scope<T>(self, cx: &mut App, run: impl FnOnce(&mut App) -> T) -> T {
        let previous = cx.default_global::<DirectionScope>().0;
        cx.default_global::<DirectionScope>().0 = self;
        let result = run(cx);
        cx.default_global::<DirectionScope>().0 = previous;
        result
    }
}

#[derive(Default)]
struct DirectionScope(Direction);
impl Global for DirectionScope {}

/// Supplies inherited direction to descendant controls, including nested views.
///
/// Calendars, tabs, radio/toggle groups, sliders, carousels, and coordinated menus resolve
/// this scope when rendered. A control's explicit `.direction(...)` wins.
/// Nested providers restore the outer direction for following siblings.
///
/// The provider aligns text to its logical start. Use [`Direction::row`] for
/// application-owned horizontal layout; arbitrary GPUI flex rows are not
/// rewritten. Text shaping and bidirectional editing remain owned by GPUI.
pub struct DirectionProvider {
    direction: Direction,
    base: Div,
}

impl DirectionProvider {
    pub fn new(direction: Direction) -> Self {
        let base = if direction.is_rtl() {
            div().text_right()
        } else {
            div().text_left()
        };
        Self { direction, base }
    }
}

impl Styled for DirectionProvider {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for DirectionProvider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl IntoElement for DirectionProvider {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for DirectionProvider {
    type RequestLayoutState = <Div as Element>::RequestLayoutState;
    type PrepaintState = <Div as Element>::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Element::id(&self.base)
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.base.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.direction
            .scope(cx, |cx| self.base.request_layout(id, inspector, window, cx))
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.direction.scope(cx, |cx| {
            self.base
                .prepaint(id, inspector, bounds, layout, window, cx)
        })
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.direction.scope(cx, |cx| {
            self.base
                .paint(id, inspector, bounds, layout, prepaint, window, cx)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Root, Slider, SliderState, SliderThumb, Tab, TabItem, TabList, TabsState};
    use gpui::{AppContext as _, Axis, Context, Entity, Render, TestAppContext, px, relative};
    use std::{cell::RefCell, rc::Rc};

    #[derive(IntoElement)]
    struct Probe(Rc<RefCell<Vec<Direction>>>);
    impl gpui::RenderOnce for Probe {
        fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
            self.0.borrow_mut().push(Direction::current(cx));
            div()
        }
    }

    struct Harness {
        tabs: Entity<TabsState>,
        slider: Entity<SliderState>,
        vertical_slider: Entity<SliderState>,
        directions: Rc<RefCell<Vec<Direction>>>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(Probe(self.directions.clone()))
                .child(
                    DirectionProvider::new(Direction::Rtl)
                        .w(px(400.))
                        .child(Probe(self.directions.clone()))
                        .child(
                            DirectionProvider::new(Direction::Ltr)
                                .child(Probe(self.directions.clone())),
                        )
                        .child(Probe(self.directions.clone()))
                        .child(
                            Direction::Rtl
                                .row(TabList::new("rtl-tabs", &self.tabs))
                                .child(Tab::new(&self.tabs, 0).child("First"))
                                .child(Tab::new(&self.tabs, 1).child("Second")),
                        )
                        .child(
                            Slider::new("rtl-slider", &self.slider)
                                .relative()
                                .w(px(200.))
                                .h(px(30.))
                                .child(
                                    SliderThumb::new(&self.slider, 0)
                                        .absolute()
                                        .left(relative(
                                            Direction::Rtl.physical_fraction(
                                                self.slider.read(cx).percentage(0),
                                            ),
                                        ))
                                        .ml(px(-10.))
                                        .w(px(20.))
                                        .h(px(30.)),
                                ),
                        )
                        .child(
                            Slider::new("rtl-vertical-slider", &self.vertical_slider)
                                .axis(Axis::Vertical)
                                .relative()
                                .w(px(30.))
                                .h(px(120.))
                                .child(
                                    SliderThumb::new(&self.vertical_slider, 0)
                                        .absolute()
                                        .bottom(relative(
                                            self.vertical_slider.read(cx).percentage(0),
                                        ))
                                        .mb(px(-10.))
                                        .w(px(30.))
                                        .h(px(20.)),
                                ),
                        ),
                )
                .child(Probe(self.directions.clone()))
        }
    }

    #[gpui::test]
    fn nested_scope_restores_siblings_and_mirrors_native_keys(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let directions = Rc::new(RefCell::new(Vec::new()));
        let observed = directions.clone();
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Harness {
                tabs: cx.new(|cx| {
                    TabsState::new(
                        [TabItem::new("a", "First"), TabItem::new("b", "Second")],
                        cx,
                    )
                }),
                slider: cx.new(|cx| SliderState::new(0.0..=100.0, 10., [50.], cx)),
                vertical_slider: cx.new(|cx| SliderState::new(0.0..=100.0, 10., [50.], cx)),
                directions,
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            let focus = view.read(cx).tabs.read(cx).tab_focus_handle(0).clone();
            focus.focus(window, cx);
            observed.borrow_mut().clear();
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            &*observed.borrow(),
            &[
                Direction::Ltr,
                Direction::Rtl,
                Direction::Ltr,
                Direction::Rtl,
                Direction::Ltr
            ]
        );
        cx.simulate_keystrokes("left");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).tabs.read(cx).selected_index(), Some(1));
            let focus = view.read(cx).slider.read(cx).thumb_focus_handle(0).clone();
            focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("left");
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).slider.read(cx).values()[0], 60.);
            assert_eq!(Direction::current(cx), Direction::Ltr);
            view.read(cx)
                .vertical_slider
                .read(cx)
                .thumb_focus_handle(0)
                .clone()
                .focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("left");
        cx.update(|_, cx| assert_eq!(view.read(cx).vertical_slider.read(cx).values()[0], 40.));
        cx.simulate_keystrokes("right up");
        cx.update(|_, cx| assert_eq!(view.read(cx).vertical_slider.read(cx).values()[0], 60.));
        #[cfg(feature = "test-support")]
        {
            let bounds = cx.update(|window, _| {
                crate::test_support::find(window, &[], &"rtl-slider".into())
                    .unwrap()
                    .bounds()
            });
            cx.simulate_mouse_down(
                gpui::point(bounds.left(), bounds.center().y),
                gpui::MouseButton::Left,
                gpui::Modifiers::default(),
            );
            cx.simulate_mouse_up(
                gpui::point(bounds.left(), bounds.center().y),
                gpui::MouseButton::Left,
                gpui::Modifiers::default(),
            );
            cx.update(|_, cx| assert_eq!(view.read(cx).slider.read(cx).values()[0], 100.));
        }
    }
}
