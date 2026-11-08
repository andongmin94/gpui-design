//! Controlled progress semantics with application-owned presentation.
// Modified for GPUI Design from GPUI Kit v0.9.0: retain the percentage root,
// remove decorative part wrappers, and reject non-finite values.

use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

use crate::{StyledExt as _, TestSupportExt as _};

/// An unstyled progress indicator with an application-controlled percentage.
///
/// The application supplies the track, fill, and any indeterminate animation as
/// children. This root supplies the accessible role, name, and numeric range;
/// it has no keyboard focus or Tab stop of its own.
#[derive(IntoElement)]
pub struct Progress {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    style: StyleRefinement,
    value: f32,
    indeterminate: bool,
    accessibility_label: Option<SharedString>,
    children: SmallVec<[AnyElement; 2]>,
}

impl Progress {
    /// Creates an indicator with known progress of zero percent.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
            style: StyleRefinement::default(),
            value: 0.,
            indeterminate: false,
            accessibility_label: None,
            children: SmallVec::new(),
        }
    }

    /// Sets the controlled percentage, clamped to `0..=100`.
    ///
    /// # Panics
    /// Panics if `value` is not finite.
    pub fn value(mut self, value: f32) -> Self {
        assert!(value.is_finite(), "Progress value must be finite");
        self.value = value.clamp(0., 100.);
        self
    }

    /// Omits the current numeric value while the amount of progress is unknown.
    /// This does not choose or animate the visual presentation.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Names the operation whose progress is being reported.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Styled for Progress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Progress {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Progress {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Progress {}

impl RenderOnce for Progress {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(Role::ProgressIndicator)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(100.)
            .when(!self.indeterminate, |this| {
                this.aria_numeric_value(self.value as f64)
            })
            .children(self.children)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Element as _, TestAppContext, accesskit};

    use super::*;

    #[gpui::test]
    fn known_and_unknown_progress_expose_the_numeric_contract(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            for (value, indeterminate, expected) in [
                (-10., false, Some(0.)),
                (42.5, false, Some(42.5)),
                (120., false, Some(100.)),
                (42.5, true, None),
            ] {
                let element = Progress::new("progress")
                    .value(value)
                    .indeterminate(indeterminate)
                    .accessibility_label("Preview progress")
                    .render(window, cx)
                    .into_element();
                assert_eq!(element.a11y_role(), Some(Role::ProgressIndicator));
                let mut node = accesskit::Node::new(Role::ProgressIndicator);
                element.write_a11y_info(&mut node);
                assert_eq!(node.label(), Some("Preview progress"));
                assert_eq!(node.min_numeric_value(), Some(0.));
                assert_eq!(node.max_numeric_value(), Some(100.));
                assert_eq!(node.numeric_value(), expected);
            }
        });
    }
}
