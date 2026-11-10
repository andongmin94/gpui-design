//! Current-page metadata shared by breadcrumb items and navigation buttons.

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, Hitbox, InspectorElementId,
    InteractiveElement, Interactivity, IntoElement, LayoutId, Pixels, Role, Window, accesskit,
};

// GPUI has no aria_current builder. Keep observation outside this transparent
// adapter, without inserting another layout, focus, or accessibility node.
pub(crate) fn current_page<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement>(
    element: crate::ObservedElement<E>,
    current: bool,
) -> crate::ObservedElement<CurrentPage<E>> {
    #[cfg(feature = "test-support")]
    {
        element.map_inner(|inner| CurrentPage { inner, current })
    }
    #[cfg(not(feature = "test-support"))]
    {
        CurrentPage {
            inner: element,
            current,
        }
    }
}

pub(crate) struct CurrentPage<E> {
    inner: E,
    current: bool,
}

impl<E: Element> IntoElement for CurrentPage<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: InteractiveElement> InteractiveElement for CurrentPage<E> {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}

impl<E: Element> Element for CurrentPage<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Element::id(&self.inner)
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
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
        self.inner
            .prepaint(id, inspector, bounds, layout, window, cx)
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
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<Role> {
        self.inner.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if self.current {
            node.set_aria_current(accesskit::AriaCurrent::Page);
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        Element::a11y_synthetic_children(&mut self.inner, prepaint, builder);
    }
}
