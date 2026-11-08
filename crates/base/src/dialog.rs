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

        deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .id(("dialog", entity_id))
                    .test_support()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .items_center()
                    .justify_center()
                    .role(Role::Dialog)
                    .when_some(self.label, |this, label| this.aria_label(label))
                    .when_some(self.description, |this, description| {
                        this.aria_description(description)
                    })
                    .track_focus(&focus)
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
                    .refine_style(&self.style),
            ),
        )
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
