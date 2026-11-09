//! Application-owned shared framing for existing inputs and their addons.

use gpui::{
    AnyElement, App, Axis, Div, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use super::super::Palette;

/// A frame around one existing control and optional leading/trailing elements.
///
/// Keep the control's native input renderer and semantics intact. The frame
/// adds no focus stop, editing state, click handler, or disabled policy. Supplied
/// buttons keep their own focus and actions. `focused` and `invalid` affect the
/// frame's appearance; the input or Field remains the semantic owner.
#[derive(IntoElement)]
pub(crate) struct InputGroup {
    base: Stateful<Div>,
    control: AnyElement,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    axis: Axis,
    focused: bool,
    invalid: bool,
    palette: Palette,
}

impl InputGroup {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        control: impl IntoElement,
        palette: Palette,
    ) -> Self {
        Self {
            base: div()
                .id(id)
                .w_full()
                .min_w_0()
                .flex()
                .gap_2()
                .px_3()
                .py_1()
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .bg(palette.field)
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            control: control.into_any_element(),
            leading: None,
            trailing: None,
            axis: Axis::Horizontal,
            focused: false,
            invalid: false,
            palette,
        }
    }

    /// Horizontal addons flank the input; vertical addons sit above/below it.
    pub(crate) fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub(crate) fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub(crate) fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub(crate) fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    pub(crate) fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
        self
    }
}

impl Styled for InputGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for InputGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(Role::Group)
            .when(self.axis == Axis::Horizontal, |group| group.items_center())
            .when(self.axis == Axis::Vertical, |group| {
                group.flex_col().items_stretch()
            })
            .when(self.focused, |group| {
                group.border_color(self.palette.accent)
            })
            .when(self.invalid, |group| group.border_color(self.palette.error))
            .when_some(self.leading, |group, leading| {
                group.child(div().flex_shrink_0().child(leading))
            })
            .child(
                div()
                    .min_w_0()
                    .when(self.axis == Axis::Horizontal, |control| control.flex_1())
                    .child(self.control),
            )
            .when_some(self.trailing, |group, trailing| {
                group.child(div().flex_shrink_0().child(trailing))
            })
    }
}
