//! Standalone labels that focus an existing control without activating it.

use gpui::{
    App, Div, ElementId, FocusHandle, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{TestSupportExt as _, a11y_disabled::A11yDisabledExt as _};

/// An unstyled text label that focuses an application-supplied control.
///
/// Clicking the enabled label focuses its target without changing its value,
/// toggling it, or opening a popup. The label itself has no focus handle or Tab
/// stop. Keep the target mounted in the same window and supply its rendered
/// disabled state through [`Self::disabled`].
///
/// The label exposes its own native Label role and name. Set the target's
/// accessible name separately, normally from the same text; this component
/// does not establish a native `labelled_by` relationship. Text-input labels
/// with help and validation metadata are provided by [`crate::Field`].
#[derive(IntoElement)]
pub struct Label {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    target: FocusHandle,
    text: SharedString,
    disabled: bool,
}

impl Label {
    pub fn new(
        id: impl Into<ElementId>,
        target: &FocusHandle,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            base: div().id(id).test_support(),
            target: target.clone(),
            text: text.into(),
            disabled: false,
        }
    }

    /// Mirrors the target's disabled state, suppressing label activation.
    ///
    /// The caller owns this state and must update it when the target changes.
    /// A disabled label neither moves focus nor activates a containing control.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let target = self.target;
        self.base
            .role(Role::Label)
            .aria_label(self.text.clone())
            .child(self.text)
            .when(disabled, |label| {
                label.on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
            })
            .when(!disabled, |label| {
                label.on_click(move |_, window, cx| {
                    target.focus(window, cx);
                    cx.stop_propagation();
                })
            })
            .a11y_disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Element as _, TestAppContext, accesskit};

    use super::*;

    #[gpui::test]
    fn native_label_has_a_name_and_only_an_enabled_click_action(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let target = cx.focus_handle();
            for disabled in [false, true] {
                let element = Label::new("label", &target, "Drawing tool")
                    .disabled(disabled)
                    .render(window, cx)
                    .into_element();
                assert_eq!(element.a11y_role(), Some(Role::Label));
                let mut node = accesskit::Node::new(Role::Label);
                element.write_a11y_info(&mut node);
                assert_eq!(node.label(), Some("Drawing tool"));
                assert_eq!(node.is_disabled(), disabled);
                assert_eq!(node.supports_action(accesskit::Action::Click), !disabled);
                assert!(!node.supports_action(accesskit::Action::Focus));
            }
        });
    }
}
