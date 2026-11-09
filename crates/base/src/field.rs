//! Label, help, and validation presentation associated with one retained input.

use gpui::{
    AnyElement, App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement,
    Refineable as _, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    StateStyle, StyledExt as _, TestSupportExt as _,
    input::{InputAccessibility, InputBaseState, InputModeKind},
};

/// An unstyled field associated with an existing Input or Textarea state.
///
/// The field renders its label, application-provided control children, help,
/// and optional error in that order. Applications supply layout, styles, and
/// validation; this component neither stores text nor validates it. Clicking
/// the label focuses the live enabled input without introducing another Tab
/// stop. The label's element ID is `(field_id, "label")`.
///
/// Label, error/help description, required, and invalid metadata are projected
/// onto the input's existing native node before its child renderer runs. Its
/// role, placeholder, external identifier, value exposure, and popup metadata
/// are preserved. An error is announced before help text. When no semantics
/// were configured, the input's mode supplies its natural text-input role.
///
/// Keep the associated input among this field's children. A composed input
/// renderer must likewise preserve metadata it does not own. Host application
/// content in [`crate::Root`] for normal form traversal.
#[derive(IntoElement)]
pub struct Field<M: InputModeKind> {
    id: ElementId,
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    input: Entity<InputBaseState<M>>,
    label: SharedString,
    description: Option<SharedString>,
    error: Option<SharedString>,
    required: bool,
    styles: FieldStyles,
    children: Vec<AnyElement>,
}

impl<M: InputModeKind> Field<M> {
    pub fn new(
        id: impl Into<ElementId>,
        input: &Entity<InputBaseState<M>>,
        label: impl Into<SharedString>,
    ) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            id,
            input: input.clone(),
            label: label.into(),
            description: None,
            error: None,
            required: false,
            styles: FieldStyles::default(),
            children: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Supplies an application validation error; `None` clears invalid state.
    pub fn error(mut self, error: Option<SharedString>) -> Self {
        self.error = error;
        self
    }

    /// Reports the requirement to accessibility clients; does not reject input.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn styles(mut self, build: impl FnOnce(FieldStyles) -> FieldStyles) -> Self {
        self.styles = build(self.styles);
        self
    }
}

/// Application-owned presentation of the field's text parts.
#[derive(Default)]
pub struct FieldStyles {
    label: StyleRefinement,
    description: StyleRefinement,
    error: StyleRefinement,
}

impl FieldStyles {
    pub fn label(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.label
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    pub fn description(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.description
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    pub fn error(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.error
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }
}

impl<M: InputModeKind> Styled for Field<M> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl<M: InputModeKind> ParentElement for Field<M> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl<M: InputModeKind> RenderOnce for Field<M> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let description = match (&self.error, &self.description) {
            (Some(error), Some(help)) => Some(format!("{error} {help}").into()),
            (Some(error), None) => Some(error.clone()),
            (None, help) => help.clone(),
        };
        self.input.update(cx, |state, _| {
            let accessibility = state.accessibility().cloned().unwrap_or_else(|| {
                InputAccessibility::new(Some(if M::MULTI_LINE {
                    Role::MultilineTextInput
                } else {
                    Role::TextInput
                }))
            });
            state.set_accessibility(
                accessibility
                    .with_label(Some(self.label.clone()))
                    .with_description(description)
                    .with_required(self.required)
                    .with_invalid(self.error.is_some()),
            );
        });

        let label_input = self.input;
        let label = div()
            .id((self.id.clone(), "label"))
            .test_support()
            .role(Role::Label)
            .aria_label(self.label.clone())
            .refine_style(&self.styles.label)
            .child(self.label)
            .on_click(move |_, window, cx| {
                label_input.update(cx, |state, cx| {
                    if !state.presentation().is_disabled() {
                        state.focus(window, cx);
                    }
                });
                cx.stop_propagation();
            });

        self.base
            .role(Role::Group)
            .child(label)
            .children(self.children)
            .when_some(self.description, |this, description| {
                this.child(
                    div()
                        .id((self.id.clone(), "description"))
                        .test_support()
                        .role(Role::Label)
                        .aria_label(description.clone())
                        .refine_style(&self.styles.description)
                        .child(description),
                )
            })
            .when_some(self.error, |this, error| {
                this.child(
                    div()
                        .id((self.id, "error"))
                        .test_support()
                        .role(Role::Label)
                        .aria_label(error.clone())
                        .refine_style(&self.styles.error)
                        .child(error),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, Element as _, Render as _, TestAppContext, accesskit};

    use super::*;
    use crate::InputState;

    #[gpui::test]
    fn feedback_preserves_the_native_inputs_existing_capabilities(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let input = cx.new(|cx| {
                let mut input = InputState::new(window, cx).default_value("private value");
                input.set_accessibility(
                    InputAccessibility::new(Some(Role::ComboBox))
                        .with_placeholder(Some("Find a session".into()))
                        .with_accessibility_id(Some("session-editor".into()))
                        .with_expanded(Some(true))
                        .with_value_exposed(false),
                );
                input
            });

            for invalid in [true, false] {
                let _ = Field::new("session", &input, "Session name")
                    .description("Choose a recognizable name.")
                    .required(true)
                    .error(invalid.then(|| "A name is required.".into()))
                    .render(window, cx);

                input.update(cx, |state, cx| {
                    let element = state.render(window, cx).into_element();
                    assert_eq!(element.a11y_role(), Some(Role::ComboBox));
                    let mut node = accesskit::Node::new(Role::ComboBox);
                    element.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Session name"));
                    assert_eq!(
                        node.description(),
                        Some(if invalid {
                            "A name is required. Choose a recognizable name."
                        } else {
                            "Choose a recognizable name."
                        }),
                    );
                    assert_eq!(node.placeholder(), Some("Find a session"));
                    assert_eq!(node.author_id(), Some("session-editor"));
                    assert_eq!(node.is_expanded(), Some(true));
                    assert!(node.is_required());
                    assert_eq!(node.invalid(), invalid.then_some(accesskit::Invalid::True));
                    assert_eq!(node.value(), None, "field must preserve hidden values");
                    assert_eq!(state.value(), "private value");
                });
            }

            input.update(cx, |state, cx| state.set_disabled(true, cx));
            let _ = Field::new("session", &input, "Session name").render(window, cx);
            input.update(cx, |state, cx| {
                let element = state.render(window, cx).into_element();
                let mut node = accesskit::Node::new(Role::ComboBox);
                element.write_a11y_info(&mut node);
                assert!(node.is_disabled());
                assert!(!node.is_required());
                assert_eq!(node.invalid(), None);
                assert_eq!(node.description(), None);
            });
        });
    }
}
