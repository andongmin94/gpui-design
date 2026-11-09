//! Application-controlled previous/next navigation over a known page count.

use std::rc::Rc;

use gpui::{
    App, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
};
use gpui_design::{Button, ObservedElement, TestSupportExt as _};

use super::super::Palette;
use super::ButtonGroup;

type PageChange = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A named navigation region using ordinary independent Button focus.
///
/// The application supplies a zero-based page index and the total page count.
/// A nonempty collection requires `page_index < page_count`; an empty collection
/// uses index zero and displays `No pages`. The callback requests another page;
/// the application owns the retained index and its data. No arrows, selection
/// controller, numbered links, or extra Tab stop are introduced here.
#[derive(IntoElement)]
pub(crate) struct Pagination {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    label: SharedString,
    page_index: usize,
    page_count: usize,
    palette: Palette,
    on_change: Option<PageChange>,
}

impl Pagination {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        page_index: usize,
        page_count: usize,
        palette: Palette,
    ) -> Self {
        assert!(
            if page_count == 0 {
                page_index == 0
            } else {
                page_index < page_count
            },
            "page index must identify an available page, or be zero for no pages"
        );
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support().w_full(),
            id,
            label: label.into(),
            page_index,
            page_count,
            palette,
            on_change: None,
        }
    }

    pub(crate) fn on_change(
        mut self,
        on_change: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }
}

impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Pagination {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let page_index = self.page_index;
        let has_previous = self.page_count > 0 && page_index > 0;
        let has_next = self.page_count > 0 && page_index < self.page_count - 1;
        let page_label = if self.page_count == 0 {
            "No pages".to_string()
        } else {
            format!("Page {} of {}", page_index + 1, self.page_count)
        };
        let previous = Button::new((self.id.clone(), "previous"))
            .accessibility_label("Previous page")
            .disabled(!has_previous)
            .when_some(
                self.on_change.clone().filter(|_| has_previous),
                |button, change| {
                    button.on_click(move |_, window, cx| change(page_index - 1, window, cx))
                },
            )
            .child("Previous");
        let next = Button::new((self.id.clone(), "next"))
            .accessibility_label("Next page")
            .disabled(!has_next)
            .when_some(self.on_change.filter(|_| has_next), |button, change| {
                button.on_click(move |_, window, cx| change(page_index + 1, window, cx))
            })
            .child("Next");

        self.base
            .role(Role::Navigation)
            .aria_label(self.label)
            .child(
                ButtonGroup::new((self.id, "controls"), "Page controls", self.palette)
                    .w_full()
                    .button(previous)
                    .text(page_label)
                    .button(next),
            )
    }
}
