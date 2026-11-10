//! Source-owned conversation presentation. Message data and reaction state are local to the app.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px, relative, rgb,
};
use gpui_design::{ObservedElement, TestSupportExt as _, TestTextExt as _};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BubbleSide {
    Incoming,
    Outgoing,
}

/// Copyable styling for a message, with grouped provenance and supplied actions.
/// The body is one paragraph; an attached reaction Button keeps its own behavior.
#[derive(IntoElement)]
pub(crate) struct Bubble {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    author: SharedString,
    time: SharedString,
    body: SharedString,
    side: BubbleSide,
    grouped: bool,
    action: Option<AnyElement>,
}

impl Bubble {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        author: impl Into<SharedString>,
        time: impl Into<SharedString>,
        body: impl Into<SharedString>,
        side: BubbleSide,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .flex(),
            id,
            author: author.into(),
            time: time.into(),
            body: body.into(),
            side,
            grouped: false,
            action: None,
        }
    }

    /// Visually joins a consecutive message by the same author, hiding repeated
    /// author text. The native Comment still names that author for this message.
    pub(crate) fn grouped(mut self, grouped: bool) -> Self {
        self.grouped = grouped;
        self
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl Styled for Bubble {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Bubble {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let outgoing = self.side == BubbleSide::Outgoing;
        let label = |part, text: SharedString| {
            div()
                .id((self.id.clone(), part))
                .test_support()
                .role(Role::Label)
                .aria_label(text.clone())
                .min_w_0()
                .whitespace_normal()
                .child(text)
        };
        let mut body = div()
            .id((self.id.clone(), "content"))
            .test_support()
            .role(Role::Comment)
            .max_w(relative(0.85))
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded(px(12.))
            .bg(if outgoing {
                rgb(0x1e40af)
            } else {
                rgb(0x1e293b)
            })
            .text_color(rgb(0xf1f5f9))
            .text_size(px(14.))
            .line_height(relative(1.4));
        if self.grouped {
            body = body.aria_label(self.author.clone());
        } else {
            body =
                body.child(label("author", self.author.clone()).font_weight(FontWeight::SEMIBOLD));
        }
        body = body
            .child(
                div()
                    .id((self.id.clone(), "body"))
                    .test_support()
                    .role(Role::Paragraph)
                    .aria_label(self.body.clone())
                    .min_w_0()
                    .whitespace_normal()
                    .child(self.body.clone().test_text(self.body)),
            )
            .child(
                label("time", self.time)
                    .text_size(px(12.))
                    .text_color(rgb(0xcbd5e1)),
            )
            .when_some(self.action, |body, action| {
                body.child(div().flex().flex_wrap().gap_1().child(action))
            });
        self.base
            .when(outgoing, |base| base.justify_end())
            .when(!self.grouped, |base| base.mt_2())
            .child(body)
    }
}
