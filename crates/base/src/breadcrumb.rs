//! Native breadcrumb structure with application-owned navigation and styling.

use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div,
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
        crate::a11y_current::current_page(self.base.role(Role::ListItem), self.current)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Element as _, TestAppContext, accesskit};

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
