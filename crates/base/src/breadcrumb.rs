//! Native breadcrumb structure with application-owned navigation and styling.

use gpui::{
    AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, Interactivity, IntoElement, LayoutId, ParentElement, Pixels, RenderOnce,
    Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Window,
    accesskit, div,
};

use crate::TestSupportExt as _;

/// An unstyled navigation landmark containing a [`BreadcrumbList`].
///
/// The accessible name defaults to "Breadcrumb". Applications supply the path,
/// layout, separators, and navigation handlers. These structural elements add
/// no focus stops; compose [`crate::Button`] inside ancestor items for ordinary
/// pointer and keyboard activation.
#[derive(IntoElement)]
pub struct Breadcrumb {
    base: crate::ObservedElement<Stateful<Div>>,
    label: SharedString,
}

impl Breadcrumb {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
            label: "Breadcrumb".into(),
        }
    }

    /// Names this navigation landmark, including a localized name when needed.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

impl Styled for Breadcrumb {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Breadcrumb {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.role(Role::Navigation).aria_label(self.label)
    }
}

/// A native list containing [`BreadcrumbItem`] elements in path order.
///
/// List direction, spacing, wrapping, and separators belong to the application.
#[derive(IntoElement)]
pub struct BreadcrumbList {
    base: crate::ObservedElement<Stateful<Div>>,
}

impl BreadcrumbList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
        }
    }
}

impl Styled for BreadcrumbList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for BreadcrumbList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for BreadcrumbList {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.role(Role::List)
    }
}

/// One location in a breadcrumb path, exposed as a native list item.
///
/// Compose a [`crate::Button`] for an ancestor and plain text for the current
/// location. The application marks its current item and handles navigation;
/// this wrapper does not change the interaction of its children.
#[derive(IntoElement)]
pub struct BreadcrumbItem {
    base: crate::ObservedElement<Stateful<Div>>,
    current: bool,
}

impl BreadcrumbItem {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
            current: false,
        }
    }

    /// Marks this item as the current page for assistive technology.
    ///
    /// The default is false, which omits current-page metadata. Mark one item
    /// in the path current; this does not select, disable, or focus its content.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }
}

impl Styled for BreadcrumbItem {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for BreadcrumbItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for BreadcrumbItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let base = self.base.role(Role::ListItem);
        #[cfg(feature = "test-support")]
        {
            base.map_inner(|inner| CurrentPage {
                inner,
                current: self.current,
            })
        }
        #[cfg(not(feature = "test-support"))]
        {
            CurrentPage {
                inner: base,
                current: self.current,
            }
        }
    }
}

// GPUI has no aria_current builder. Add the native property without inserting
// another layout or accessibility node, keeping observation outside the adapter.
struct CurrentPage {
    inner: Stateful<Div>,
    current: bool,
}

impl IntoElement for CurrentPage {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl InteractiveElement for CurrentPage {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}

impl Element for CurrentPage {
    type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = <Stateful<Div> as Element>::PrepaintState;

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

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    fn node_of(element: impl IntoElement, role: Role) -> accesskit::Node {
        let element = element.into_element();
        assert_eq!(element.a11y_role(), Some(role));
        let mut node = accesskit::Node::new(role);
        element.write_a11y_info(&mut node);
        assert!(!node.supports_action(accesskit::Action::Focus));
        assert!(!node.supports_action(accesskit::Action::Click));
        node
    }

    #[gpui::test]
    fn native_path_names_current_page_without_structural_focus(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let root = node_of(
                Breadcrumb::new("path")
                    .accessibility_label("Session path")
                    .render(window, cx),
                Role::Navigation,
            );
            assert_eq!(root.label(), Some("Session path"));
            assert_eq!(root.aria_current(), None);

            let list = node_of(
                BreadcrumbList::new("path-list").render(window, cx),
                Role::List,
            );
            assert_eq!(list.aria_current(), None);

            for current in [false, true] {
                let item = node_of(
                    BreadcrumbItem::new("path-item")
                        .current(current)
                        .child("Keyboard guide")
                        .render(window, cx),
                    Role::ListItem,
                );
                assert_eq!(
                    item.aria_current(),
                    current.then_some(accesskit::AriaCurrent::Page)
                );
            }
        });
    }
}
