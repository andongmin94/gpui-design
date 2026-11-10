//! A retained modal lifecycle with application-owned presentation.
//! Modified for GPUI Design: retained focus ownership replaces styled-host callbacks.

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, InteractiveElement as _,
    IntoElement, KeyBinding, MouseButton, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, WeakFocusHandle, Window, anchored,
    deferred, div, point, prelude::FluentBuilder as _, px,
};
use smallvec::SmallVec;

use crate::{FocusTrapElement as _, StyledExt as _, TestSupportExt as _, actions::Cancel};

const CONTEXT: &str = "Dialog";

/// The interaction that closed a dialog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogChangeReason {
    Cancel,
    BackdropPress,
    CloseButton,
    Imperative,
}

/// A notification emitted after the retained lifecycle has changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogEvent {
    Opened,
    Closed(DialogChangeReason),
}

/// Owns a dialog's open state, stable focus scope and return target.
///
/// Keep the matching [`Dialog`] mounted while closed. Opening focuses the
/// supplied initial control, or the dialog scope if none was supplied. Closing
/// restores the previous live focus target only while focus still belongs to
/// this dialog; it never steals focus back from another surface.
pub struct DialogState {
    open: bool,
    focus: FocusHandle,
    initial_focus: Option<FocusHandle>,
    previous_focus: Option<WeakFocusHandle>,
}

impl DialogState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            focus: cx.focus_handle(),
            initial_focus: None,
            previous_focus: None,
        }
    }

    /// Chooses a control rendered inside the dialog for initial focus.
    pub fn with_initial_focus(mut self, focus: &FocusHandle) -> Self {
        self.initial_focus = Some(focus.clone());
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.previous_focus = window.focused(cx).map(|focus| focus.downgrade());
        self.open = true;
        self.initial_focus
            .as_ref()
            .unwrap_or(&self.focus)
            .focus(window, cx);
        cx.emit(DialogEvent::Opened);
        cx.notify();
        window.refresh();
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_with_reason(DialogChangeReason::Imperative, window, cx);
    }

    fn close_with_reason(
        &mut self,
        reason: DialogChangeReason,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.open {
            return;
        }
        self.open = false;
        let owns_focus = self.focus.contains_focused(window, cx)
            || self.focus.is_focused(window)
            // Opening and closing before a paint has no new focus tree yet.
            || self
                .initial_focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window));
        let previous = self.previous_focus.take().and_then(|focus| focus.upgrade());
        if owns_focus {
            match previous {
                Some(focus) => focus.focus(window, cx),
                None => window.blur(cx),
            }
        }
        cx.emit(DialogEvent::Closed(reason));
        cx.notify();
        window.refresh();
    }
}

impl EventEmitter<DialogEvent> for DialogState {}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))]);
}

/// An unstyled modal host. [`DialogState`] owns its retained lifecycle.
///
/// The host supplies viewport geometry and focus containment. Applications
/// supply the backdrop, popup, accessible name, and every visual style. Mount
/// the application inside [`crate::Root`] for Tab/Shift-Tab containment.
#[derive(IntoElement)]
pub struct Dialog {
    state: Entity<DialogState>,
    style: StyleRefinement,
    label: Option<SharedString>,
    description: Option<SharedString>,
    role: Role,
    keyboard: bool,
    overlay_closable: bool,
    backdrop: Option<AnyElement>,
    popup: Option<AnyElement>,
}

impl Dialog {
    pub fn new(state: &Entity<DialogState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            label: None,
            description: None,
            role: Role::Dialog,
            keyboard: true,
            overlay_closable: true,
            backdrop: None,
            popup: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn accessibility_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    // Composed modal controls reuse this host's retained focus and dismissal.
    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub fn backdrop(mut self, element: impl IntoElement) -> Self {
        self.backdrop = Some(element.into_any_element());
        self
    }

    pub fn popup(mut self, element: impl IntoElement) -> Self {
        self.popup = Some(element.into_any_element());
        self
    }

    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.keyboard = close;
        self
    }

    pub fn close_on_backdrop_press(mut self, close: bool) -> Self {
        self.overlay_closable = close;
        self
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Dialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.state.read(cx).is_open() {
            return div().into_any_element();
        }
        let focus = self.state.read(cx).focus_handle().clone();
        let entity_id = self.state.entity_id();
        let viewport = window.viewport_size();
        let escape_state = self.state.clone();
        let backdrop_state = self.state;
        let overlay_closable = self.overlay_closable;

        let host = div()
            .id(("dialog", entity_id))
            .absolute()
            .top_0()
            .left_0()
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .items_center()
            .justify_center()
            .occlude()
            .role(self.role)
            .when_some(self.label, |this, label| this.aria_label(label))
            .when_some(self.description, |this, description| {
                this.aria_description(description)
            })
            .focus_trap(("dialog", entity_id), &focus)
            .when(self.keyboard, |this| {
                this.key_context(CONTEXT)
                    .on_action(move |_: &Cancel, window, cx| {
                        cx.stop_propagation();
                        escape_state.update(cx, |state, cx| {
                            state.close_with_reason(DialogChangeReason::Cancel, window, cx);
                        });
                    })
            })
            .when_some(self.backdrop, |this, backdrop| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .on_scroll(|_, _, cx| cx.stop_propagation())
                        .on_any_mouse_down(move |event, window, cx| {
                            cx.stop_propagation();
                            if event.button == MouseButton::Left && overlay_closable {
                                backdrop_state.update(cx, |state, cx| {
                                    state.close_with_reason(
                                        DialogChangeReason::BackdropPress,
                                        window,
                                        cx,
                                    );
                                });
                            }
                        })
                        .child(backdrop),
                )
            })
            .children(self.popup)
            .refine_style(&self.style);
        let host = crate::a11y_disabled::A11yDisabled::new(host, false)
            .with_modal(true)
            .test_support()
            .track_focus(&focus);

        deferred(anchored().position(point(px(0.), px(0.))).child(host))
            .with_priority(10)
            .into_any_element()
    }
}

macro_rules! dialog_part {
    ($(#[$meta:meta])* $name:ident, $id:literal, $occlude:literal) => {
        $(#[$meta])*
        #[derive(IntoElement)]
        pub struct $name {
            style: StyleRefinement,
            children: SmallVec<[AnyElement; 2]>,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    style: StyleRefinement::default(),
                    children: SmallVec::new(),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
                div()
                    .id($id)
                    .test_support()
                    .when($occlude, |this| this.occlude())
                    .children(self.children)
                    .refine_style(&self.style)
            }
        }
    };
}

dialog_part!(
    /// Application-owned surface behind the modal popup.
    DialogBackdrop,
    "dialog-backdrop",
    false
);
dialog_part!(
    /// Application-owned popup; blocks pointer events from reaching the backdrop.
    DialogPopup,
    "dialog-popup",
    true
);
dialog_part!(
    /// Visual title slot. Name the modal with `Dialog::accessibility_label`.
    DialogTitle,
    "dialog-title",
    false
);
dialog_part!(
    /// Visual description slot.
    DialogDescription,
    "dialog-description",
    false
);

/// A close button with behavior supplied by the dialog's retained state.
#[derive(IntoElement)]
pub struct DialogClose {
    state: Entity<DialogState>,
    trigger: Option<AnyElement>,
}

impl DialogClose {
    pub fn new(state: &Entity<DialogState>) -> Self {
        Self {
            state: state.clone(),
            trigger: None,
        }
    }

    /// Supplies the presentation for a keyboard- and pointer-accessible button.
    pub fn trigger<E: IntoElement>(mut self, build: impl FnOnce(crate::Button) -> E) -> Self {
        let state = self.state.clone();
        self.trigger = Some(
            build(
                crate::Button::new(("dialog-close", state.entity_id()))
                    .accessibility_label("Close")
                    .on_click(move |_, window, cx| {
                        state.update(cx, |state, cx| {
                            state.close_with_reason(DialogChangeReason::CloseButton, window, cx);
                        });
                    }),
            )
            .into_any_element(),
        );
        self
    }
}

impl RenderOnce for DialogClose {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().children(self.trigger)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Modifiers, Render, ScrollDelta, ScrollEvent, ScrollHandle, TestAppContext,
    };

    use super::*;
    use crate::{Button, Root, ScrollArea};

    struct NoBackdropHarness {
        dialog: Entity<DialogState>,
        background_activations: Rc<Cell<usize>>,
        background_scrolls: Rc<Cell<usize>>,
        popup_activations: Rc<Cell<usize>>,
    }

    impl Render for NoBackdropHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let background = self.background_activations.clone();
            let background_scrolls = self.background_scrolls.clone();
            let popup = self.popup_activations.clone();
            div()
                .size_full()
                .child(
                    Button::new("background-button")
                        .size_full()
                        .on_scroll(move |_, _, _| {
                            background_scrolls.set(background_scrolls.get() + 1)
                        })
                        .on_click(move |_, _, _| background.set(background.get() + 1)),
                )
                .child(
                    Dialog::new(&self.dialog)
                        .accessibility_label("Undecorated dialog")
                        .popup(
                            DialogPopup::new().w(px(200.)).h(px(120.)).child(
                                Button::new("popup-button")
                                    .size_full()
                                    .on_click(move |_, _, _| popup.set(popup.get() + 1)),
                            ),
                        ),
                )
        }
    }

    #[gpui::test]
    fn modal_without_a_backdrop_blocks_background_pointer_activation(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let dialog = cx.new(DialogState::new);
        let background_activations = Rc::new(Cell::new(0));
        let background_scrolls = Rc::new(Cell::new(0));
        let popup_activations = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let dialog = dialog.clone();
            let background_activations = background_activations.clone();
            let background_scrolls = background_scrolls.clone();
            let popup_activations = popup_activations.clone();
            move |window, cx| {
                let content = cx.new(|_| NoBackdropHarness {
                    dialog,
                    background_activations,
                    background_scrolls,
                    popup_activations,
                });
                Root::new(content, window, cx)
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let background_position = point(px(8.), px(8.));
        cx.simulate_click(background_position, Modifiers::default());
        assert_eq!(background_activations.get(), 1);
        let wheel = || ScrollEvent {
            position: background_position,
            delta: ScrollDelta::Pixels(point(px(0.), px(-20.))),
            ..Default::default()
        };
        cx.simulate_event(wheel());
        assert_eq!(background_scrolls.get(), 1);
        cx.update(|window, cx| {
            dialog.update(cx, |state, cx| state.open(window, cx));
            window.draw(cx).clear(cx);
        });
        cx.simulate_click(background_position, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(dialog.read(cx).is_open());
        });
        assert_eq!(background_activations.get(), 1);
        cx.simulate_event(wheel());
        assert_eq!(
            background_scrolls.get(),
            1,
            "the undecorated modal also blocks background wheel input"
        );

        let popup_position = cx.update(|window, _| {
            let viewport = window.viewport_size();
            point(viewport.width / 2., viewport.height / 2.)
        });
        cx.simulate_click(popup_position, Modifiers::default());
        assert_eq!(popup_activations.get(), 1);
        #[cfg(feature = "test-support")]
        cx.update(|window, _| {
            let node =
                crate::test_support::find(window, &[], &("dialog", dialog.entity_id()).into())
                    .unwrap();
            assert_eq!(node.role(), Some(Role::Dialog));
            assert!(node.modal());
        });
    }

    struct ScrollIsolationHarness {
        dialog: Entity<DialogState>,
        background: ScrollHandle,
        body: ScrollHandle,
        background_presses: Rc<Cell<usize>>,
    }

    impl Render for ScrollIsolationHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let background_presses = self.background_presses.clone();
            div()
                .size_full()
                .child(
                    ScrollArea::new("background", &self.background)
                        .size_full()
                        .child(div().w_full().h(px(2000.)).flex_shrink_0().on_mouse_down(
                            MouseButton::Left,
                            move |_, _, _| {
                                background_presses.set(background_presses.get() + 1);
                            },
                        )),
                )
                .child(
                    Dialog::new(&self.dialog)
                        .accessibility_label("Scroll isolation")
                        .backdrop(DialogBackdrop::new().size_full())
                        .popup(
                            DialogPopup::new().w(px(200.)).h(px(120.)).child(
                                ScrollArea::new("dialog-body", &self.body)
                                    .size_full()
                                    .child(div().w_full().h(px(400.)).flex_shrink_0()),
                            ),
                        ),
                )
        }
    }

    #[gpui::test]
    fn modal_wheel_isolation_preserves_body_scrolling_and_backdrop_dismissal(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let dialog = cx.new(DialogState::new);
        let background = ScrollHandle::new();
        let body = ScrollHandle::new();
        let background_presses = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let dialog = dialog.clone();
            let background = background.clone();
            let body = body.clone();
            let background_presses = background_presses.clone();
            move |window, cx| {
                let content = cx.new(|_| ScrollIsolationHarness {
                    dialog,
                    background,
                    body,
                    background_presses,
                });
                Root::new(content, window, cx)
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let backdrop_position = point(
            background.bounds().left() + px(8.),
            background.bounds().top() + px(8.),
        );
        let wheel_down = point(px(0.), px(-40.));

        // Establish that this point and wheel event scroll the real background
        // while the dialog is closed, rather than testing an inert viewport.
        cx.simulate_event(ScrollEvent {
            position: backdrop_position,
            delta: ScrollDelta::Pixels(wheel_down),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let background_offset = background.offset();
        assert_eq!(background_offset, wheel_down);

        cx.update(|window, cx| {
            dialog.update(cx, |state, cx| state.open(window, cx));
            window.draw(cx).clear(cx);
        });
        assert!(!body.bounds().contains(&backdrop_position));

        cx.simulate_event(ScrollEvent {
            position: body.bounds().center(),
            delta: ScrollDelta::Pixels(wheel_down),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(body.offset(), wheel_down);
        assert_eq!(background.offset(), background_offset);

        // An exhausted body must not pass the wheel to the background.
        let body_end = point(px(0.), -body.max_offset().y);
        body.set_offset(body_end);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_event(ScrollEvent {
            position: body.bounds().center(),
            delta: ScrollDelta::Pixels(wheel_down),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(body.offset(), body_end);
        assert_eq!(background.offset(), background_offset);

        cx.simulate_event(ScrollEvent {
            position: backdrop_position,
            delta: ScrollDelta::Pixels(wheel_down),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let background_after_backdrop_wheel = background.offset();
        assert_eq!(body.offset(), body_end);

        // Blocking wheel events must leave backdrop dismissal working and
        // must not turn the same press into a background activation.
        cx.simulate_click(backdrop_position, Modifiers::default());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(!dialog.read(cx).is_open());
        });
        assert_eq!(background_presses.get(), 0);
        assert_eq!(
            background_after_backdrop_wheel, background_offset,
            "a wheel over the modal backdrop must not move the background"
        );
    }
}
