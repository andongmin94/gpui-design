//! Application-owned edge presentation over the retained Dialog lifecycle.

use gpui::{
    AnyElement, App, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, ScrollHandle, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_design::{
    Dialog, DialogBackdrop, DialogDescription, DialogPopup, DialogState, DialogTitle, ScrollArea,
    StyledExt as _, TestSupportExt as _,
};

use super::super::Palette;

/// The viewport edge occupied by a Sheet.
#[allow(dead_code)] // Consumers may retain only the default right-edge presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SheetSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

/// An edge modal with a fixed header/footer and scrolling body.
///
/// Keep this host mounted under Root while closed. The application's DialogState
/// owns opening, initial focus, Escape/backdrop dismissal, and focus restoration.
/// Retain the supplied ScrollHandle with the application as well. Children go
/// into one vertical ScrollArea, which adds no keyboard focus stop or bindings.
///
/// Right (the default) and left fill the viewport height; top and bottom fill its
/// width. The preferred size on the other axis is palette.dialog_width, capped
/// at the viewport. Styled customizes the popup, not the full-viewport host; use
/// .w(...) for left/right or .h(...) for top/bottom to change that preferred size.
/// Preserve the full axis, maximum dimensions, and flex layout for this contract.
/// The default has square corners and one inner-edge border. Header/footer must
/// fit within the popup height; the body takes the remaining space. Drag and
/// animation are outside this recipe's scope.
#[derive(IntoElement)]
pub(crate) struct Sheet {
    dialog: Dialog,
    style: StyleRefinement,
    side: SheetSide,
    body: ScrollArea,
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    footer: Option<AnyElement>,
    palette: Palette,
}

impl Sheet {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        state: &Entity<DialogState>,
        scroll: &ScrollHandle,
        title: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        let title = title.into();
        Self {
            dialog: Dialog::new(state)
                .accessibility_label(title.clone())
                .backdrop(
                    DialogBackdrop::new()
                        .absolute()
                        .inset_0()
                        .bg(gpui::black())
                        .opacity(0.45),
                ),
            style: StyleRefinement::default(),
            side: SheetSide::default(),
            body: ScrollArea::new((id.clone(), "body"), scroll)
                .accessibility_label(format!("{title} content"))
                .flex_1()
                .scrollbar_styles(|styles| {
                    styles.thumb(|thumb| thumb.bg(palette.muted).radius(px(3.)))
                })
                .content_styles(|style| {
                    style.flex().flex_col().gap(palette.gap).p(palette.padding)
                }),
            id,
            title,
            description: None,
            footer: None,
            palette,
        }
    }

    #[allow(dead_code)] // Optional placement for consumers of the copyable recipe.
    pub(crate) fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    pub(crate) fn description(mut self, description: impl Into<SharedString>) -> Self {
        let description = description.into();
        self.dialog = self.dialog.accessibility_description(description.clone());
        self.description = Some(description);
        self
    }

    pub(crate) fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl Styled for Sheet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Sheet {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let popup = DialogPopup::new()
            .max_w_full()
            .max_h_full()
            .min_w_0()
            .min_h_0()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(palette.surface)
            .text_color(palette.foreground)
            .text_size(palette.font_size)
            .border_color(palette.border);
        let (dialog, popup) = match self.side {
            SheetSide::Right => (
                self.dialog.justify_end().items_stretch(),
                popup.w(palette.dialog_width).h_full().border_l_1(),
            ),
            SheetSide::Left => (
                self.dialog.justify_start().items_stretch(),
                popup.w(palette.dialog_width).h_full().border_r_1(),
            ),
            SheetSide::Top => (
                self.dialog.flex_col().justify_start().items_stretch(),
                popup.w_full().h(palette.dialog_width).border_b_1(),
            ),
            SheetSide::Bottom => (
                self.dialog.flex_col().justify_end().items_stretch(),
                popup.w_full().h(palette.dialog_width).border_t_1(),
            ),
        };
        let popup = popup.refine_style(&self.style);
        let header = div()
            .id((self.id.clone(), "header"))
            .test_support()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(palette.gap)
            .p(palette.padding)
            .border_b_1()
            .border_color(palette.border)
            .child(DialogTitle::new().text_lg().child(self.title))
            .when_some(self.description, |header, description| {
                header.child(
                    DialogDescription::new()
                        .text_color(palette.muted)
                        .child(description),
                )
            });
        dialog.popup(popup.child(header).child(self.body).when_some(
            self.footer,
            |popup, footer| {
                popup.child(
                    div()
                        .id((self.id, "footer"))
                        .test_support()
                        .flex_shrink_0()
                        .p(palette.padding)
                        .border_t_1()
                        .border_color(palette.border)
                        .child(footer),
                )
            },
        ))
    }
}
