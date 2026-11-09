//! Connected application buttons with ordinary independent keyboard focus.

use gpui::{
    AnyElement, App, Axis, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};
use gpui_design::{Button, Separator, StyledExt as _};

use super::super::Palette;

/// A named visual group, with no group focus stop or arrow-key navigation.
#[derive(IntoElement)]
pub(crate) struct ButtonGroup {
    id: ElementId,
    label: SharedString,
    palette: Palette,
    axis: Axis,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl ButtonGroup {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            palette,
            axis: Axis::Horizontal,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub(crate) fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Keeps the supplied button's identity, actions, disabled state, and focus.
    pub(crate) fn button(mut self, button: Button) -> Self {
        let palette = self.palette;
        self.children.push(
            button
                .h(palette.control_height)
                .px_3()
                .flex_shrink_0()
                .rounded_none()
                .border_0()
                .bg(palette.field)
                .text_color(palette.foreground)
                .hover(|style| style.bg(palette.highlight))
                // The group owns the shared border, so focus uses the fill.
                .focus_visible(|style| {
                    style
                        .bg(palette.accent)
                        .text_color(palette.accent_foreground)
                })
                .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
                .into_any_element(),
        );
        self
    }

    /// Inserts a static label between controls without adding a focus stop.
    pub(crate) fn text(mut self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        self.children.push(
            div()
                .id(("text", self.children.len()))
                .role(Role::Label)
                .aria_label(text.clone())
                .h(self.palette.control_height)
                .px_3()
                .flex()
                .flex_auto()
                .min_w_0()
                .items_center()
                .justify_center()
                .child(text)
                .into_any_element(),
        );
        self
    }
}

impl Styled for ButtonGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let mut group = div()
            .id(self.id)
            .role(Role::Group)
            .aria_label(self.label)
            .flex()
            .items_stretch()
            .when(self.axis == Axis::Vertical, |group| group.flex_col())
            .rounded(palette.radius)
            .border_1()
            .border_color(palette.border)
            .overflow_hidden()
            .bg(palette.field)
            .text_color(palette.foreground)
            .text_size(palette.font_size)
            .refine_style(&self.style);

        for (index, child) in self.children.into_iter().enumerate() {
            if index > 0 {
                let separator = Separator::new(("separator", index))
                    .axis(self.axis.invert())
                    .decorative(true)
                    .flex_shrink_0()
                    .bg(palette.border);
                group = group.child(match self.axis {
                    Axis::Horizontal => separator.w(px(1.)),
                    Axis::Vertical => separator.h(px(1.)),
                });
            }
            group = group.child(child);
        }
        group
    }
}
