// Modified for GPUI Design: expose the existing multiline engine without editor
// presentation or a second focus owner.
use gpui::{App, Entity, IntoElement, RenderOnce, Window};

use super::{InputBaseState, TextareaMode};

/// State for editing ordinary multi-line text.
///
/// This is the shared editing engine in its multi-line kind. Code-editor
/// facilities such as languages, diagnostics, folding, and LSP do not exist on
/// this type — those methods live on [`super::EditorState`].
pub type TextareaState = InputBaseState<TextareaMode>;

/// An unstyled ordinary multi-line text input.
///
/// The retained state owns text, selection, undo, wrapping, scrolling, and the
/// native text-input interface. Enter inserts a newline by default; Tab and
/// Shift-Tab follow normal form navigation through [`crate::Root`].
///
/// Wrap this element in [`super::InputBase`] for application-owned frame styling.
/// Give the state an [`super::InputAccessibility`] with
/// [`gpui::Role::MultilineTextInput`] and a label; leave the surrounding frame's
/// role unset so the state remains the only text focus and semantic owner.
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
}

impl Textarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Textarea {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}
