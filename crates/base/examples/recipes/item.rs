//! Application-owned content row with a consumer-supplied trailing action.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use super::super::Palette;

/// A named row with static text and an optional action or control.
///
/// The row adds no focus stop, click handler, or selection state. Supplied
/// controls keep their own accessible names and keyboard/pointer behavior.
/// Use a container when the action slot needs several controls.
#[derive(IntoElement)]
pub(crate) struct Item {
    base: Stateful<Div>,
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    action: Option<AnyElement>,
    palette: Palette,
}

impl Item {
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
                .items_center()
                .gap(palette.gap)
                .px_3()
                .py_2()
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius)
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            id,
            title: title.into(),
            description: None,
            action: None,
            palette,
        }
    }

    pub(crate) fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl Styled for Item {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Item {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let text = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .id((self.id.clone(), "title"))
                    .role(Role::Label)
                    .aria_label(self.title.clone())
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title.clone()),
            )
            .when_some(self.description.clone(), |text, description| {
                text.child(
                    div()
                        .id((self.id.clone(), "description"))
                        .role(Role::Label)
                        .aria_label(description.clone())
                        .text_sm()
                        .text_color(self.palette.muted)
                        .child(description),
                )
            });

        self.base
            .role(Role::Group)
            .aria_label(self.title)
            .when_some(self.description, |item, description| {
                item.aria_description(description)
            })
            .child(text)
            .when_some(self.action, |item, action| {
                item.child(
                    div()
                        .id((self.id, "action"))
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .child(action),
                )
            })
    }
}
