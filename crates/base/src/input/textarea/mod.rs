// Modified for GPUI Design: retain the shared multiline state without a separate control.
use super::{InputBaseState, TextareaMode};

/// State for editing ordinary multi-line text.
///
/// This is the shared editing engine in its multi-line kind. Code-editor
/// facilities such as languages, diagnostics, folding, and LSP do not exist on
/// this type — those methods live on [`super::EditorState`].
pub type TextareaState = InputBaseState<TextareaMode>;
