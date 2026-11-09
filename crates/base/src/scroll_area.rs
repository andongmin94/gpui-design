//! A native GPUI viewport composed with the existing overlay scrollbar.

use gpui::{
    AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement as _, IntoElement, LayoutId, ParentElement, Pixels, Refineable as _,
    RenderOnce, Role, ScrollHandle, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, px,
};

use crate::{
    Scrollbar, ScrollbarAxis, ScrollbarMode, ScrollbarStyles, StateStyle, TestSupportExt as _,
};

/// A scrollable viewport with application-owned content and presentation.
///
/// Retain the supplied [`ScrollHandle`] in the application. Ordinary GPUI styles
/// size and decorate the frame; [`Self::content_styles`] sets its viewport's
/// child layout and padding. Give each scrolling axis a bounded size and keep
/// children from shrinking when necessary. The existing [`Scrollbar`] overlays
/// that viewport without contributing to its content size or reserving a
/// gutter. Its styles and visibility mode can be configured independently.
///
/// The viewport keeps GPUI's wheel/trackpad handling and the scrollbar's pointer
/// interaction. It adds no focus stop or key bindings, so descendant controls
/// keep their own keyboard behavior. Native autoscroll requests from those
/// descendants, including focused Input and Textarea, reveal the requested
/// region on enabled axes. Nested areas pass the adjusted request to their
/// ancestors; requests from siblings do not scroll this viewport.
///
/// This component does not virtualize children or pin streaming content to an
/// edge. Its native role is [`Role::ScrollView`]; provide a descriptive label for
/// a distinct application region with [`Self::accessibility_label`].
#[derive(IntoElement)]
pub struct ScrollArea {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    viewport: gpui::Stateful<Div>,
    handle: ScrollHandle,
    scrollbar: Scrollbar,
    axis: ScrollbarAxis,
}

impl ScrollArea {
    /// Creates a vertical scroll area using the application's retained handle.
    pub fn new(id: impl Into<ElementId>, handle: &ScrollHandle) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .role(Role::ScrollView)
                .relative()
                .flex()
                .flex_col()
                .min_w_0()
                .min_h_0(),
            viewport: div()
                .id((id.clone(), "viewport"))
                .track_scroll(handle)
                .flex_1()
                .min_w_0()
                .min_h_0(),
            handle: handle.clone(),
            scrollbar: Scrollbar::vertical(handle).id((id, "scrollbar")),
            axis: ScrollbarAxis::Vertical,
        }
    }

    /// Enables the corresponding native overflow and overlay scrollbar axes.
    pub fn axis(mut self, axis: impl Into<ScrollbarAxis>) -> Self {
        self.axis = axis.into();
        self.scrollbar = self.scrollbar.axis(self.axis);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.base = self.base.aria_label(label);
        self
    }

    /// Styles the viewport's layout and padding; children are added here.
    ///
    /// For example, use `|style| style.flex().flex_col().gap_2()` for a vertical
    /// form. Set the area's bounded size on the ScrollArea itself.
    pub fn content_styles(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.viewport
            .style()
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    /// Overrides the application's default scrollbar visibility mode.
    pub fn scrollbar_mode(mut self, mode: ScrollbarMode) -> Self {
        self.scrollbar = self.scrollbar.mode(mode);
        self
    }

    pub fn scrollbar_styles(
        mut self,
        build: impl FnOnce(ScrollbarStyles) -> ScrollbarStyles,
    ) -> Self {
        self.scrollbar = self.scrollbar.styles(build);
        self
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.viewport.extend(elements);
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let viewport = self.viewport.overflow_hidden();
        let viewport = match self.axis {
            ScrollbarAxis::Vertical => viewport.overflow_y_scroll(),
            ScrollbarAxis::Horizontal => viewport.overflow_x_scroll(),
            ScrollbarAxis::Both => viewport.overflow_scroll(),
        };
        self.base
            .child(RevealViewport {
                viewport,
                handle: self.handle,
                axis: self.axis,
            })
            .child(self.scrollbar)
    }
}

// A transparent prepaint scope keeps reveal requests attached to the viewport
// whose children produced them. Layout, clipping, drawing, and pointer scrolling
// remain owned by the ordinary GPUI div.
struct RevealViewport {
    viewport: gpui::Stateful<Div>,
    handle: ScrollHandle,
    axis: ScrollbarAxis,
}

impl RevealViewport {
    fn reveal(&self, mut request: Bounds<Pixels>, window: &mut Window) {
        let viewport = self.handle.bounds();
        let old_offset = self.handle.offset();
        let mut offset = old_offset;
        let maximum = self.handle.max_offset();
        if self.axis.has_horizontal() {
            offset.x = (offset.x
                + reveal_delta(
                    request.left(),
                    request.right(),
                    viewport.left(),
                    viewport.right(),
                ))
            .clamp(-maximum.x, px(0.));
        }
        if self.axis.has_vertical() {
            offset.y = (offset.y
                + reveal_delta(
                    request.top(),
                    request.bottom(),
                    viewport.top(),
                    viewport.bottom(),
                ))
            .clamp(-maximum.y, px(0.));
        }
        if offset != old_offset {
            self.handle.set_offset(offset);
            // The viewport's children have already prepainted. Apply the new
            // offset on the next frame and forward their resulting position
            // now so an enclosing ScrollArea can reveal the same region.
            request.origin += offset - old_offset;
            window.request_animation_frame();
        }
        window.request_autoscroll(request);
    }
}

fn reveal_delta(
    start: Pixels,
    end: Pixels,
    viewport_start: Pixels,
    viewport_end: Pixels,
) -> Pixels {
    if start < viewport_start {
        viewport_start - start
    } else if end > viewport_end {
        // A request larger than the viewport aligns its leading edge instead
        // of hiding that edge in an attempt to show the impossible full extent.
        (viewport_end - end).max(viewport_start - start)
    } else {
        px(0.)
    }
}

impl IntoElement for RevealViewport {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for RevealViewport {
    type RequestLayoutState = <gpui::Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = <gpui::Stateful<Div> as Element>::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Element::id(&self.viewport)
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.viewport.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.viewport.request_layout(id, inspector, window, cx)
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
        let mut offset = self.handle.offset();
        if !self.axis.has_horizontal() {
            offset.x = px(0.);
        }
        if !self.axis.has_vertical() {
            offset.y = px(0.);
        }
        self.handle.set_offset(offset);

        // Scope requests to the children being prepainted, while preserving a
        // prior sibling's request for the actual enclosing scroll container.
        let pending = window.take_autoscroll();
        let viewport = self
            .viewport
            .prepaint(id, inspector, bounds, layout, window, cx);
        if let Some(request) = window.take_autoscroll() {
            self.reveal(request, window);
        } else if let Some(pending) = pending {
            window.request_autoscroll(pending);
        }

        viewport
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
        self.viewport
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Context, Entity, Focusable as _, Modifiers, MouseButton, Point, Render,
        ScrollDelta, ScrollEvent, TestAppContext, accesskit, point,
    };

    use super::*;
    use crate::{ElementExt as _, Input, InputBase, InputState};

    struct SizingHarness {
        vertical: ScrollHandle,
        horizontal: ScrollHandle,
        both: ScrollHandle,
        extent: Pixels,
    }

    impl Render for SizingHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(200.))
                .flex()
                .flex_col()
                .items_start()
                .child(
                    div()
                        .w_full()
                        .max_h(px(140.))
                        .flex()
                        .flex_col()
                        .flex_shrink_0()
                        .child(div().h(px(20.)).flex_shrink_0())
                        .child(
                            ScrollArea::new("form", &self.vertical)
                                .accessibility_label("Form fields")
                                .scrollbar_mode(ScrollbarMode::Always)
                                .content_styles(|style| style.flex().flex_col())
                                .child(div().h(self.extent).w_full().flex_shrink_0()),
                        ),
                )
                .child(
                    ScrollArea::new("table", &self.horizontal)
                        .axis(ScrollbarAxis::Horizontal)
                        .w_full()
                        .flex_shrink_0()
                        .scrollbar_mode(ScrollbarMode::Always)
                        .child(div().w(self.extent).h(px(72.)).flex_shrink_0()),
                )
                .child(
                    ScrollArea::new("canvas", &self.both)
                        .axis(ScrollbarAxis::Both)
                        .size(px(100.))
                        .flex_shrink_0()
                        .scrollbar_mode(ScrollbarMode::Always)
                        .child(div().size(self.extent).flex_shrink_0()),
                )
        }
    }

    #[gpui::test]
    fn natural_sizing_wheel_and_overlay_drag_share_the_viewport_handles(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let vertical = ScrollHandle::new();
        let horizontal = ScrollHandle::new();
        let both = ScrollHandle::new();
        let (view, cx) = cx.add_window_view({
            let vertical = vertical.clone();
            let horizontal = horizontal.clone();
            let both = both.clone();
            move |_, _| SizingHarness {
                vertical,
                horizontal,
                both,
                extent: px(400.),
            }
        });
        cx.update(|window, cx| {
            let area = ScrollArea::new("semantics", &vertical)
                .accessibility_label("Form fields")
                .render(window, cx)
                .into_element();
            assert_eq!(area.a11y_role(), Some(Role::ScrollView));
            let mut node = accesskit::Node::new(Role::ScrollView);
            area.write_a11y_info(&mut node);
            assert_eq!(node.label(), Some("Form fields"));
            assert!(!node.supports_action(accesskit::Action::Focus));
            window.draw(cx).clear(cx);
        });

        // A form body shrinks under its parent's max-height without a fixed
        // height. A horizontally scrolling table retains its content height.
        assert_eq!(vertical.bounds().size.height, px(120.));
        assert_eq!(horizontal.bounds().size.height, px(72.));
        assert_eq!(vertical.max_offset().y, px(280.));
        assert_eq!(horizontal.max_offset().x, px(200.));
        for (handle, delta) in [
            (&vertical, point(px(0.), px(-40.))),
            (&horizontal, point(px(-30.), px(0.))),
            (&both, point(px(-30.), px(0.))),
            (&both, point(px(0.), px(-40.))),
        ] {
            cx.simulate_event(ScrollEvent {
                position: handle.bounds().center(),
                delta: ScrollDelta::Pixels(delta),
                ..Default::default()
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        assert_eq!(vertical.offset(), point(px(0.), px(-40.)));
        assert_eq!(horizontal.offset(), point(px(-30.), px(0.)));
        assert_eq!(both.offset(), point(px(-30.), px(-40.)));

        // The overlay stays on the viewport while its content moves, and its
        // existing drag behavior reaches the same native handle's exact end.
        vertical.set_offset(Point::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = vertical.bounds();
        let start = point(bounds.right() - px(6.), bounds.top() + px(12.));
        let end = point(start.x, bounds.bottom() - px(1.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(vertical.offset().y, -vertical.max_offset().y);
        cx.update(|window, cx| assert!(window.focused(cx).is_none()));

        view.update(cx, |view, cx| {
            view.extent = px(40.);
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(vertical.bounds().size.height, px(40.));
        assert_eq!(horizontal.bounds().size.height, px(72.));
        for handle in [&vertical, &horizontal, &both] {
            assert_eq!(handle.offset(), Point::default());
        }
    }

    struct RevealHarness {
        outer: ScrollHandle,
        inner: ScrollHandle,
        sibling: ScrollHandle,
        input: Entity<InputState>,
        input_bounds: Rc<Cell<Bounds<Pixels>>>,
    }

    impl Render for RevealHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let input_bounds = self.input_bounds.clone();
            ScrollArea::new("outer", &self.outer)
                .w(px(160.))
                .h(px(120.))
                .content_styles(|style| style.flex().flex_col())
                .child(div().h(px(240.)).flex_shrink_0())
                .child(
                    ScrollArea::new("inner", &self.inner)
                        .h(px(80.))
                        .flex_shrink_0()
                        .content_styles(|style| style.flex().flex_col())
                        .child(div().h(px(140.)).flex_shrink_0())
                        .child(
                            div()
                                .relative()
                                .h(px(32.))
                                .flex_shrink_0()
                                .child(
                                    InputBase::new("reveal-input")
                                        .w_full()
                                        .h(px(32.))
                                        .child(Input::new(&self.input)),
                                )
                                .on_prepaint(move |bounds, _, _| input_bounds.set(bounds)),
                        )
                        .child(div().h(px(40.)).flex_shrink_0()),
                )
                .child(
                    ScrollArea::new("sibling", &self.sibling)
                        .h(px(80.))
                        .flex_shrink_0()
                        .child(div().h(px(400.))),
                )
        }
    }

    #[gpui::test]
    fn focused_input_reveals_through_nested_areas_without_scrolling_its_sibling(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let outer = ScrollHandle::new();
        let inner = ScrollHandle::new();
        let sibling = ScrollHandle::new();
        let input_bounds = Rc::new(Cell::new(Bounds::default()));
        let (view, cx) = cx.add_window_view({
            let outer = outer.clone();
            let inner = inner.clone();
            let sibling = sibling.clone();
            let input_bounds = input_bounds.clone();
            move |window, cx| RevealHarness {
                outer,
                inner,
                sibling,
                input: cx.new(|cx| InputState::new(window, cx)),
                input_bounds,
            }
        });
        let input = view.read_with(cx, |view, _| view.input.clone());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            input.update(cx, |state, cx| state.focus(window, cx));
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
            window.draw(cx).clear(cx);
        });

        assert!(inner.offset().y < px(0.));
        assert!(outer.offset().y < px(0.));
        assert_eq!(sibling.offset(), Point::default());
        let field = input_bounds.get();
        for viewport in [outer.bounds(), inner.bounds()] {
            assert!(
                field.top() >= viewport.top(),
                "{field:?} above {viewport:?}"
            );
            assert!(
                field.bottom() <= viewport.bottom(),
                "{field:?} below {viewport:?}"
            );
        }
        cx.update(|window, cx| assert!(input.focus_handle(cx).is_focused(window)));

        // Once revealed, manual movement remains under application/user control
        // until a new native reveal request, rather than snapping to focus.
        outer.set_offset(point(px(0.), px(-10.)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(outer.offset().y, px(-10.));
    }
}
