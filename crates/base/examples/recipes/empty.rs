//! Application-owned empty-state presentation with consumer-supplied actions.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use super::super::Palette;

/// A static explanation of missing content, with optional media and actions.
///
/// The group and its level-two heading are named by the title. It does not
/// announce updates, move focus, or add a Tab stop. Applications supply an
/// ordinary Button or other controls in the action slot and decide what they do.
/// Media should supply its own accessible name when it conveys information.
#[derive(IntoElement)]
pub(crate) struct EmptyState {
    base: Stateful<Div>,
    id: ElementId,
    title: SharedString,
    description: SharedString,
    media: Option<AnyElement>,
    action: Option<AnyElement>,
    palette: Palette,
}

impl EmptyState {
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
                .flex()
                .flex_col()
                .items_center()
                .gap(palette.gap)
                .p(palette.padding)
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .text_center()
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            id,
            title: title.into(),
            description: description.into(),
            media: None,
            action: None,
            palette,
        }
    }

    pub(crate) fn media(mut self, media: impl IntoElement) -> Self {
        self.media = Some(media.into_any_element());
        self
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl Styled for EmptyState {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        self.base
            .role(Role::Group)
            .aria_label(self.title.clone())
            .aria_description(self.description.clone())
            .when_some(self.media, |this, media| {
                this.child(
                    div()
                        .id((self.id.clone(), "media"))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(palette.muted)
                        .child(media),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .id((self.id.clone(), "title"))
                            .role(Role::Heading)
                            .aria_level(2)
                            .aria_label(self.title.clone())
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title),
                    )
                    .child(
                        div()
                            .id((self.id.clone(), "description"))
                            .role(Role::Label)
                            .aria_label(self.description.clone())
                            .text_color(palette.muted)
                            .child(self.description),
                    ),
            )
            .when_some(self.action, |this, action| {
                this.child(div().id((self.id, "action")).child(action))
            })
    }
}
