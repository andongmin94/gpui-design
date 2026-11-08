//! Static separator semantics with application-owned geometry and appearance.

use gpui::{
    App, Axis, Div, ElementId, InteractiveElement as _, IntoElement, RenderOnce, Role,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, accesskit, div,
    prelude::FluentBuilder as _,
};

use crate::TestSupportExt as _;

/// An unstyled, non-interactive separator between regions of content.
///
/// Supply its thickness, length, and color with ordinary GPUI styles. Axis sets
/// accessibility orientation; it does not choose layout. Semantic separators
/// expose the native Splitter role without focus or actions. Decorative
/// separators have no accessibility node. Neither form accepts children.
#[derive(IntoElement)]
pub struct Separator {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    axis: Axis,
    decorative: bool,
}

impl Separator {
    /// Creates a semantic horizontal separator.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
            axis: Axis::Horizontal,
            decorative: false,
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Omits the separator's native role and orientation when purely decorative.
    pub fn decorative(mut self, decorative: bool) -> Self {
        self.decorative = decorative;
        self
    }
}

impl Styled for Separator {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let orientation = match self.axis {
            Axis::Horizontal => accesskit::Orientation::Horizontal,
            Axis::Vertical => accesskit::Orientation::Vertical,
        };
        self.base.when(!self.decorative, |base| {
            base.role(Role::Splitter).aria_orientation(orientation)
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Element as _, TestAppContext};

    use super::*;

    #[gpui::test]
    fn semantic_orientation_and_decorative_omission_are_noninteractive(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            for (axis, orientation) in [
                (Axis::Horizontal, accesskit::Orientation::Horizontal),
                (Axis::Vertical, accesskit::Orientation::Vertical),
            ] {
                for decorative in [false, true] {
                    let element = Separator::new("separator")
                        .axis(axis)
                        .decorative(decorative)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(element.a11y_role(), (!decorative).then_some(Role::Splitter));
                    let mut node = accesskit::Node::new(Role::Splitter);
                    element.write_a11y_info(&mut node);
                    assert_eq!(node.orientation(), (!decorative).then_some(orientation));
                    assert!(!node.supports_action(accesskit::Action::Focus));
                    assert!(!node.supports_action(accesskit::Action::Click));
                }
            }
        });
    }
}
