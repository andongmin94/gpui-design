//! Application-owned inline notices with explicit urgency semantics.

use gpui::{
    App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div,
};
use gpui_design::{ObservedElement, TestSupportExt as _};

use super::super::Palette;

/// An inline notice with a title and description, without a focus stop.
///
/// Ordinary notices use the native Note role. Applications can explicitly mark
/// urgent content with the Alert role; destructive styling does not imply that
/// urgency. This recipe does not schedule announcements or own dismissal state.
#[derive(IntoElement)]
pub(crate) struct Alert {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    title: SharedString,
    description: SharedString,
    palette: Palette,
    destructive: bool,
    urgent: bool,
}

impl Alert {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .bg(palette.field)
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            id,
            title: title.into(),
            description: description.into(),
            palette,
            destructive: false,
            urgent: false,
        }
    }

    /// Changes presentation only; use `urgent` to change the native role.
    pub(crate) fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self.base = self
            .base
            .border_color(if destructive {
                self.palette.error
            } else {
                self.palette.border
            })
            .text_color(if destructive {
                self.palette.error
            } else {
                self.palette.foreground
            });
        self
    }

    /// Exposes urgent content as an Alert instead of an informational Note.
    ///
    /// The native accessibility adapter determines how that role is presented;
    /// the recipe does not implement a live-region delivery mechanism.
    pub(crate) fn urgent(mut self, urgent: bool) -> Self {
        self.urgent = urgent;
        self
    }
}

impl Styled for Alert {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Alert {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(if self.urgent { Role::Alert } else { Role::Note })
            .aria_label(self.title.clone())
            .aria_description(self.description.clone())
            .child(
                div()
                    .id((self.id.clone(), "title"))
                    .role(Role::Label)
                    .aria_label(self.title.clone())
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title),
            )
            .child(
                div()
                    .id((self.id, "description"))
                    .role(Role::Label)
                    .aria_label(self.description.clone())
                    .text_sm()
                    .text_color(if self.destructive {
                        self.palette.error
                    } else {
                        self.palette.muted
                    })
                    .child(self.description),
            )
    }
}
