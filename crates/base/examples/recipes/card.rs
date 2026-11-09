//! Application-owned card presentation. Copy this file with the consumer palette.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use super::super::Palette;

/// A named content region with a heading, body, and optional footer.
///
/// The title is a level-two native heading. Header actions and footer controls
/// retain their own interaction and focus behavior; the card adds no Tab stop.
/// Supply a container when a slot needs several elements.
#[derive(IntoElement)]
pub(crate) struct Card {
    base: Stateful<Div>,
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    header_action: Option<AnyElement>,
    content: Option<AnyElement>,
    footer: Option<AnyElement>,
    palette: Palette,
}

impl Card {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .flex()
                .flex_col()
                .gap(palette.gap)
                .p(palette.padding)
                .bg(palette.surface)
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            id,
            title: title.into(),
            description: None,
            header_action: None,
            content: None,
            footer: None,
            palette,
        }
    }

    pub(crate) fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub(crate) fn header_action(mut self, action: impl IntoElement) -> Self {
        self.header_action = Some(action.into_any_element());
        self
    }

    pub(crate) fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub(crate) fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl Styled for Card {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let header = div()
            .id((self.id.clone(), "header"))
            .flex()
            .items_start()
            .justify_between()
            .gap(palette.gap)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .id((self.id.clone(), "title"))
                            .role(Role::Heading)
                            .aria_level(2)
                            .aria_label(self.title.clone())
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title.clone()),
                    )
                    .when_some(self.description.clone(), |this, description| {
                        this.child(
                            div()
                                .id((self.id.clone(), "description"))
                                .role(Role::Label)
                                .aria_label(description.clone())
                                .text_color(palette.muted)
                                .child(description),
                        )
                    }),
            )
            .when_some(self.header_action, |this, action| {
                this.child(
                    div()
                        .id((self.id.clone(), "header-action"))
                        .flex_shrink_0()
                        .child(action),
                )
            });

        self.base
            .role(Role::Region)
            .aria_label(self.title)
            .when_some(self.description, |this, description| {
                this.aria_description(description)
            })
            .child(header)
            .when_some(self.content, |this, content| {
                this.child(div().id((self.id.clone(), "content")).child(content))
            })
            .when_some(self.footer, |this, footer| {
                this.child(
                    div()
                        .id((self.id, "footer"))
                        .pt(palette.gap)
                        .border_t_1()
                        .border_color(palette.border)
                        .child(footer),
                )
            })
    }
}
