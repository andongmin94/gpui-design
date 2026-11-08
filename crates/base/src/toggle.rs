//! A controlled pressed button composed from the existing Button primitive.
//! Derived from GPUI Kit v0.9.0's toggle contract; GPUI Design delegates native
//! activation, focus, disabled behavior, and geometry to Button.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Toggled, Window, prelude::FluentBuilder as _,
};

use crate::Button;

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// An unstyled toggle whose pressed value belongs to the application.
///
/// Activation requests the inverse of [`Self::pressed`]. Store the requested
/// value and render it back to accept a change. The existing [`Button`] owns
/// pointer and keyboard activation, focus, disabled handling, and layout geometry.
#[derive(IntoElement)]
pub struct Toggle {
    button: Button,
    pressed: bool,
    on_change: Option<ChangeHandler>,
}

impl Toggle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            button: Button::new(id),
            pressed: false,
            on_change: None,
        }
    }

    /// Sets the controlled pressed state without requesting a change.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.button = self.button.disabled(disabled);
        self
    }

    /// Keep this name stable; the pressed metadata describes the current state.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.button = self.button.accessibility_label(label);
        self
    }

    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.button = self.button.track_focus(focus);
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.button = self.button.tab_index(tab_index);
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.button = self.button.tab_stop(tab_stop);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for Toggle {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}

impl ParentElement for Toggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(elements);
    }
}

impl InteractiveElement for Toggle {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.button.interactivity()
    }
}

impl StatefulInteractiveElement for Toggle {}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pressed = self.pressed;
        self.button
            .aria_toggled(if pressed {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(self.on_change, |button, on_change| {
                button.on_click(move |event, window, cx| {
                    on_change(!pressed, event, window, cx);
                })
            })
            .render(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui::{
        Context, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, Role,
        TestAppContext, accesskit, px,
    };

    use super::*;

    #[gpui::test]
    fn activation_requests_the_inverse_without_changing_controlled_state(cx: &mut TestAppContext) {
        struct Harness {
            pressed: bool,
            focus: FocusHandle,
            requests: Rc<RefCell<Vec<bool>>>,
        }
        impl Render for Harness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let requests = self.requests.clone();
                Toggle::new("toggle")
                    .debug_selector(|| "toggle".into())
                    .pressed(self.pressed)
                    .track_focus(&self.focus)
                    .accessibility_label("Unknown progress")
                    .w(px(160.))
                    .h(px(32.))
                    .on_change(move |next, _, _, _| requests.borrow_mut().push(next))
                    .child("Unknown progress")
            }
        }
        let requests = Rc::new(RefCell::new(Vec::new()));
        let (view, cx) = cx.add_window_view({
            let requests = requests.clone();
            move |_, cx| Harness {
                pressed: false,
                focus: cx.focus_handle(),
                requests,
            }
        });
        cx.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        let point = cx.debug_bounds("toggle").unwrap().center();
        cx.simulate_click(point, Modifiers::default());
        cx.simulate_click(point, Modifiers::default());
        assert_eq!(*requests.borrow(), [true, true]);
        view.update(cx, |view, cx| {
            assert!(
                !view.pressed,
                "a request does not mutate the application's value"
            );
            view.pressed = true;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let keystroke =
            cx.update(|_, cx| Keystroke::parse_with_os("space", cx.platform_info().os).unwrap());
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
        assert_eq!(*requests.borrow(), [true, true, false]);
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(view.pressed);
            let element = Toggle::new("native-toggle")
                .pressed(view.pressed)
                .track_focus(&view.focus)
                .accessibility_label("Unknown progress")
                .render(window, cx)
                .into_element();
            assert_eq!(element.a11y_role(), Some(Role::Button));
            let mut node = accesskit::Node::new(Role::Button);
            element.write_a11y_info(&mut node);
            assert_eq!(node.toggled(), Some(Toggled::True));
            assert_eq!(node.label(), Some("Unknown progress"));
        });
    }
}
