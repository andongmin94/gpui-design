//! Application-owned navigation built from the existing TabsState and Button.

use gpui::{
    App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, rems,
};
use gpui_design::{Button, ObservedElement, TestSupportExt as _};

use super::super::Palette;

/// A navigation sidebar whose selection is owned by an existing `TabsState`.
///
/// Unlike `TabList`, this recipe uses ordinary independent Buttons:
/// Tab and Shift-Tab traverse enabled entries, Enter/Space activate the
/// focused entry, and arrow keys do not change the selected section.
///
/// `Button::selected` controls presentation; `Button::current` separately exposes
/// the current page to assistive technology. The navigation landmark and its
/// Buttons retain their native roles, without tab selection semantics.
///
/// The caller must retain the supplied `TabsState` and render only the
/// content corresponding to its selected item. Each Button uses that state's
/// `tab_focus_handle(index)` for explicit focus restoration. Do not reuse this
/// state with `TabList`, `Tab`, or `TabPanel`: TabList installs automatic focus
/// selection, while this Sidebar activates only on Enter/Space or a click.
#[derive(IntoElement)]
pub(crate) struct Sidebar {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    state: Entity<gpui_design::TabsState>,
    label: SharedString,
    palette: Palette,
}

impl Sidebar {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        state: &Entity<gpui_design::TabsState>,
        label: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .w(rems(10.))
                .min_w_0()
                .flex()
                .flex_col()
                .items_stretch()
                .gap_1()
                .text_size(palette.font_size)
                .text_color(palette.foreground),
            id,
            state: state.clone(),
            label: label.into(),
            palette,
        }
    }
}

impl Styled for Sidebar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let selected = model.selected_index();
        let items = model.items().to_vec();
        let palette = self.palette;

        let buttons = items.into_iter().enumerate().map(|(index, item)| {
            let value = item.value().clone();
            let label = item.label().clone();
            let disabled = item.is_disabled();
            let is_selected = selected == Some(index);
            let state = self.state.clone();

            Button::new((self.id.clone(), index.to_string()))
                .accessibility_label(label.clone())
                .track_focus(model.tab_focus_handle(index))
                .selected(is_selected)
                .current(is_selected)
                .disabled(disabled)
                .w_full()
                .min_w_0()
                .justify_start()
                .items_start()
                .px_2()
                .py_2()
                .rounded(palette.radius)
                .text_color(palette.foreground)
                .when(is_selected, |button| button.bg(palette.highlight))
                .when(disabled, |button| button.opacity(0.45))
                .focus_visible(|style| style.border_1().border_color(palette.accent))
                .child(div().min_w_0().whitespace_normal().child(label))
                .on_click(move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.select_value(value.as_ref(), window, cx);
                    });
                })
        });

        self.base
            .role(Role::Navigation)
            .aria_label(self.label)
            .children(buttons)
    }
}
