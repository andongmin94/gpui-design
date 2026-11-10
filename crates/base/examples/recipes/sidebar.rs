//! Application-owned navigation composed from TabsState, Button and Collapsible.

use gpui::{
    AnyElement, App, Div, ElementId, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, rems,
};
use gpui_design::{
    Button, Collapsible, CollapsibleContent, CollapsibleState, CollapsibleTrigger, ObservedElement,
    TestSupportExt as _,
};

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
///
/// Optional named groups and whole-sidebar/group disclosures compose the existing
/// `CollapsibleState`. The caller retains and observes those states. Closing a
/// disclosure removes its navigation Buttons and returns contained focus to its
/// trigger without changing the current section. Ungrouped entries render first;
/// each group lists existing, unique item values in its desired display order.
/// Expand a closed disclosure before programmatically focusing an entry in it.
#[derive(IntoElement)]
pub(crate) struct Sidebar {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    state: Entity<gpui_design::TabsState>,
    label: SharedString,
    palette: Palette,
    collapse: Option<Entity<CollapsibleState>>,
    groups: Vec<SidebarGroup>,
}

struct SidebarGroup {
    label: SharedString,
    values: Vec<SharedString>,
    collapse: Option<Entity<CollapsibleState>>,
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
            collapse: None,
            groups: Vec::new(),
        }
    }

    /// Keeps a native disclosure trigger mounted when navigation is hidden.
    #[allow(dead_code)] // Optional in consumers that use only flat navigation.
    pub(crate) fn collapsible(mut self, state: &Entity<CollapsibleState>) -> Self {
        self.collapse = Some(state.clone());
        self
    }

    pub(crate) fn group(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.groups.push(SidebarGroup {
            label: label.into(),
            values: values.into_iter().map(Into::into).collect(),
            collapse: None,
        });
        self
    }

    #[allow(dead_code)] // Optional in consumers that use only static groups.
    pub(crate) fn collapsible_group(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = impl Into<SharedString>>,
        state: &Entity<CollapsibleState>,
    ) -> Self {
        self.groups.push(SidebarGroup {
            label: label.into(),
            values: values.into_iter().map(Into::into).collect(),
            collapse: Some(state.clone()),
        });
        self
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
        let mut grouped_values = Vec::new();
        for value in self.groups.iter().flat_map(|group| &group.values) {
            assert!(
                !grouped_values.contains(value),
                "Sidebar group item values must be unique"
            );
            grouped_values.push(value.clone());
        }

        let button = |index: usize| {
            let item = &items[index];
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
        };

        let mut content = div().w_full().min_w_0().flex().flex_col().gap_1().children(
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| !grouped_values.contains(item.value()))
                .map(|(index, _)| button(index)),
        );
        for (index, group) in self.groups.iter().enumerate() {
            let id = (self.id.clone(), format!("group-{index}"));
            let buttons = group.values.iter().map(|value| {
                let index = items
                    .iter()
                    .position(|item| item.value() == value)
                    .expect("Sidebar groups must reference existing item values");
                button(index)
            });
            let group_content: AnyElement = if let Some(state) = &group.collapse {
                Collapsible::new(id, state)
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(disclosure_trigger(state, group.label.clone(), palette, cx))
                    .child(
                        CollapsibleContent::new(state)
                            .accessibility_label(group.label.clone())
                            .w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(buttons),
                    )
                    .into_any_element()
            } else {
                div()
                    .id(id)
                    .test_support()
                    .role(Role::Group)
                    .aria_label(group.label.clone())
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .id((self.id.clone(), format!("group-{index}-heading")))
                            .test_support()
                            .role(Role::Heading)
                            .aria_level(3)
                            .aria_label(group.label.clone())
                            .min_w_0()
                            .px_2()
                            .py_1()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(palette.muted)
                            .child(group.label.clone()),
                    )
                    .children(buttons)
                    .into_any_element()
            };
            content = content.child(group_content);
        }

        let content = if let Some(state) = self.collapse {
            Collapsible::new((self.id.clone(), "disclosure"), &state)
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(disclosure_trigger(&state, self.label.clone(), palette, cx))
                .child(
                    CollapsibleContent::new(&state)
                        .accessibility_label(self.label.clone())
                        .w_full()
                        .min_w_0()
                        .child(content),
                )
                .into_any_element()
        } else {
            content.into_any_element()
        };

        self.base
            .role(Role::Navigation)
            .aria_label(self.label)
            .child(content)
    }
}

fn disclosure_trigger(
    state: &Entity<CollapsibleState>,
    label: SharedString,
    palette: Palette,
    cx: &App,
) -> CollapsibleTrigger {
    let indicator = if state.read(cx).is_open() { "−" } else { "+" };
    CollapsibleTrigger::new(state)
        .accessibility_label(label.clone())
        .w_full()
        .min_w_0()
        .justify_start()
        .items_start()
        .gap_2()
        .px_2()
        .py_2()
        .rounded(palette.radius)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(palette.foreground)
        .hover(|style| style.bg(palette.highlight))
        .focus_visible(|style| style.border_1().border_color(palette.accent))
        .when(state.read(cx).is_disabled(), |trigger| {
            trigger.opacity(0.45)
        })
        .child(div().flex_1().min_w_0().whitespace_normal().child(label))
        .child(indicator)
}
