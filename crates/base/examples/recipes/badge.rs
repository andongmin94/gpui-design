//! Application-owned presentation for a static status label.

use super::super::Palette;
use gpui::{
    App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, Role,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, relative,
};

/// A non-interactive label. Accent changes appearance, not accessibility state.
#[derive(IntoElement)]
pub(crate) struct Badge {
    id: ElementId,
    text: SharedString,
    palette: Palette,
    accent: bool,
}

impl Badge {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            palette,
            accent: false,
        }
    }

    pub(crate) fn accent(mut self, accent: bool) -> Self {
        self.accent = accent;
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        div()
            .id(self.id)
            .role(Role::Label)
            .aria_label(self.text.clone())
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .px_2()
            .py_1()
            .rounded(palette.radius)
            .border_1()
            .border_color(palette.border)
            .bg(palette.field)
            .text_color(palette.muted)
            .text_size(palette.font_size)
            .line_height(relative(1.))
            .when(self.accent, |badge| {
                badge
                    .border_color(palette.accent)
                    .bg(palette.accent)
                    .text_color(palette.accent_foreground)
            })
            .child(self.text)
    }
}
