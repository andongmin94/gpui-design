//! A confirmation surface composed from the retained Dialog host.
//! Adapted from GPUI Kit v0.9.0's AlertDialog specialization; GPUI Design reuses
//! DialogState and the existing Dialog parts instead of callback-owned lifecycle.

use gpui::{
    App, Entity, IntoElement, RenderOnce, Role, SharedString, StyleRefinement, Styled, Window,
};

use crate::{Dialog, DialogState};

/// An unstyled alert dialog for a short decision that needs an explicit response.
///
/// Supply an accessible name and description, a backdrop, and a popup containing
/// cancellation and confirmation controls. Reuse [`crate::DialogBackdrop`],
/// [`crate::DialogPopup`], [`crate::DialogTitle`], and [`crate::DialogDescription`]
/// for presentation. All visual styles belong to the application.
///
/// Create the retained state with [`DialogState::with_initial_focus`] pointing at
/// the cancellation button. [`crate::DialogClose`] supplies its close behavior;
/// an ordinary [`crate::Button`] can perform the application's confirmation and
/// call [`DialogState::close`]. Escape cancels, while backdrop presses never
/// dismiss this host. Closing alone does not execute the confirmation action.
///
/// Keep this host mounted while closed and host the window in [`crate::Root`].
/// The existing Dialog owns focus containment, initial focus, and restoration.
#[derive(IntoElement)]
pub struct AlertDialog {
    dialog: Dialog,
}

impl AlertDialog {
    pub fn new(state: &Entity<DialogState>) -> Self {
        Self {
            dialog: Dialog::new(state)
                .role(Role::AlertDialog)
                .close_on_backdrop_press(false),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.dialog = self.dialog.accessibility_label(label);
        self
    }

    pub fn accessibility_description(mut self, description: impl Into<SharedString>) -> Self {
        self.dialog = self.dialog.accessibility_description(description);
        self
    }

    pub fn backdrop(mut self, element: impl IntoElement) -> Self {
        self.dialog = self.dialog.backdrop(element);
        self
    }

    pub fn popup(mut self, element: impl IntoElement) -> Self {
        self.dialog = self.dialog.popup(element);
        self
    }
}

impl Styled for AlertDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        self.dialog.style()
    }
}

impl RenderOnce for AlertDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.dialog.render(window, cx)
    }
}
