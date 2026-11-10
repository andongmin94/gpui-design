// Modified for GPUI Design: exercise the retained Button accessibility contract.
//! Disabled and form-validation semantics on a control's accessibility node.
//!
//! GPUI has no `aria_disabled` setter, so a disabled control would reach
//! assistive technology as one that accepts input. [`A11yDisabledExt`] wraps the
//! control's root element without adding a layout node, a focus stop or an
//! accessibility node. Inputs also use this same adapter for required and
//! invalid state, and selectable containers use it for multiselectable state,
//! which GPUI does not expose through its fluent builders.
use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, InteractiveElement,
    Interactivity, IntoElement, LayoutId, Pixels, Window, accesskit,
};

/// A control's root element whose accessibility node may be marked disabled.
pub(crate) struct A11yDisabled<E> {
    inner: E,
    disabled: bool,
    required: bool,
    invalid: bool,
    multiselectable: bool,
}

impl<E> A11yDisabled<E> {
    pub(crate) fn new(inner: E, disabled: bool) -> Self {
        Self {
            inner,
            disabled,
            required: false,
            invalid: false,
            multiselectable: false,
        }
    }

    pub(crate) fn with_validation(mut self, required: bool, invalid: bool) -> Self {
        self.required = required;
        self.invalid = invalid;
        self
    }

    fn with_multiselectable(mut self, multiselectable: bool) -> Self {
        self.multiselectable = multiselectable;
        self
    }
}

/// Marks a control's accessibility node disabled while `disabled` is true.
///
/// Call it at the end of the control's builder chain. With `test-support`, the
/// wrapper goes inside the observation, so its snapshot reports the flag too.
pub(crate) trait A11yDisabledExt: Sized {
    type Output: IntoElement;

    fn a11y_disabled(self, disabled: bool) -> Self::Output {
        self.a11y_selection(disabled, false)
    }

    /// Marks native list/grid selection semantics in the same root adapter.
    fn a11y_selection(self, disabled: bool, multiselectable: bool) -> Self::Output;
}

#[cfg(not(feature = "test-support"))]
impl<E: Element + InteractiveElement> A11yDisabledExt for E {
    type Output = A11yDisabled<E>;

    fn a11y_selection(self, disabled: bool, multiselectable: bool) -> Self::Output {
        A11yDisabled::new(self, disabled).with_multiselectable(multiselectable)
    }
}

#[cfg(feature = "test-support")]
impl<E: Element + InteractiveElement> A11yDisabledExt for gpui::Stateful<E> {
    type Output = A11yDisabled<Self>;

    fn a11y_selection(self, disabled: bool, multiselectable: bool) -> Self::Output {
        A11yDisabled::new(self, disabled).with_multiselectable(multiselectable)
    }
}

#[cfg(feature = "test-support")]
impl<E: Element<PrepaintState = Option<gpui::Hitbox>> + InteractiveElement> A11yDisabledExt
    for crate::test_support::Observed<E>
{
    type Output = crate::test_support::Observed<A11yDisabled<E>>;

    fn a11y_selection(self, disabled: bool, multiselectable: bool) -> Self::Output {
        self.map_inner(|inner| {
            A11yDisabled::new(inner, disabled).with_multiselectable(multiselectable)
        })
    }
}

impl<E: Element> IntoElement for A11yDisabled<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: InteractiveElement> InteractiveElement for A11yDisabled<E> {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}

impl<E: Element> Element for A11yDisabled<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.inner
            .prepaint(id, inspector, bounds, layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<gpui::Role> {
        self.inner.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if self.disabled {
            node.set_disabled();
        }
        if self.required {
            node.set_required();
        }
        if self.invalid {
            node.set_invalid(accesskit::Invalid::True);
        }
        if self.multiselectable {
            node.set_multiselectable();
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        self.inner.a11y_synthetic_children(prepaint, builder);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use gpui::{
        Context, IntoElement, Render, RenderOnce, StatefulInteractiveElement as _, TestAppContext,
        canvas,
    };

    use super::*;
    use crate::Button;

    /// The accessibility node a rendered control writes.
    fn node_of(element: impl IntoElement) -> accesskit::Node {
        let element = element.into_element();
        let mut node = accesskit::Node::new(element.a11y_role().unwrap_or(gpui::Role::Unknown));
        element.write_a11y_info(&mut node);
        node
    }

    type Captured = Arc<Mutex<Vec<(&'static str, bool, accesskit::Node)>>>;

    struct Probe {
        captured: Captured,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let captured = self.captured.clone();
            canvas(
                move |_, window, cx| {
                    let mut nodes = captured.lock().unwrap();
                    nodes.clear();
                    for disabled in [false, true] {
                        nodes.push((
                            "button",
                            disabled,
                            node_of(Button::new("button").disabled(disabled).render(window, cx)),
                        ));
                    }
                },
                |_, _, _, _| {},
            )
        }
    }

    #[gpui::test]
    fn a_disabled_control_marks_its_node_disabled(cx: &mut TestAppContext) {
        let captured: Captured = Arc::default();
        let (_, cx) = cx.add_window_view({
            let captured = captured.clone();
            move |_, _| Probe { captured }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let nodes = captured.lock().unwrap();
        assert_eq!(nodes.len(), 2);
        for (name, disabled, node) in nodes.iter() {
            assert_ne!(node.role(), gpui::Role::Unknown, "{name} wrote no node");
            assert_eq!(node.is_disabled(), *disabled, "{name}, disabled {disabled}");
        }
    }

    #[test]
    fn selection_metadata_uses_the_existing_root_node() {
        for role in [gpui::Role::ListBox, gpui::Role::Grid] {
            for multiple in [false, true] {
                let node = node_of(
                    gpui::div()
                        .id("selection")
                        .role(role)
                        .a11y_selection(false, multiple),
                );
                assert_eq!(node.role(), role);
                assert_eq!(node.is_multiselectable(), multiple);
                assert!(!node.is_disabled());
            }
        }
    }

    #[cfg(feature = "test-support")]
    struct Observed {
        disabled: bool,
    }

    #[cfg(feature = "test-support")]
    impl Render for Observed {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Button::new("button").disabled(self.disabled)
        }
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn the_test_support_snapshot_sees_the_flag(cx: &mut TestAppContext) {
        for disabled in [false, true] {
            let (_, cx) = cx.add_window_view(move |_, _| Observed { disabled });
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let snapshot = crate::test_support::find(window, &[], &"button".into())
                    .expect("button is observed");
                assert_eq!(snapshot.disabled(), disabled.then_some(true));
            });
        }
    }
}
