//! Application-owned, static decoration for content that is not ready yet.

use gpui::{
    App, Div, ElementId, InteractiveElement as _, IntoElement, RenderOnce, Stateful,
    StyleRefinement, Styled, Window, div,
};
use gpui_design::{ObservedElement, TestSupportExt as _};

use super::super::Palette;

/// A decorative placeholder with no loading state, focus, or accessible name.
///
/// The default is a full-width text line using the consumer's font size and
/// corner radius. Set width, height, and rounding through Styled for other
/// shapes. The application decides when to replace it with actual content and
/// names the loading operation separately, for example with Spinner.
///
/// This surface stays still: it owns no animation or repeating redraw request.
#[derive(IntoElement)]
pub(crate) struct Skeleton {
    base: ObservedElement<Stateful<Div>>,
}

impl Skeleton {
    pub(crate) fn new(id: impl Into<ElementId>, palette: Palette) -> Self {
        Self {
            base: div()
                .id(id)
                .test_support()
                .w_full()
                .h(palette.font_size)
                .flex_shrink_0()
                .rounded(palette.radius)
                .bg(palette.border)
                .opacity(0.35),
        }
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
    }
}
