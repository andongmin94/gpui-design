# Input module layout

Modified from GPUI Kit v0.9.0 for GPUI Design: `Input` and `Textarea` are the
supported rendered text controls. Both use the same editing engine for layout,
selection, clipboard, IME, undo, and text shaping. Their regression tests remain
with that shared engine.

- `base/` owns shared state, layout, cursor movement, selection, masking,
  native integration, and painting.
- `input/` provides the single-line `Input` element and `InputState`.
- `textarea/` provides the multiline `Textarea` element and `TextareaState`,
  including ordinary form Tab traversal.
- `editor/` retains shared display mapping, highlighting contracts, diagnostics,
  and language-provider types required by the engine. Its standalone renderer
  is compiled only for the inherited regression tests.

`mod.rs` defines the public input API. `EditorState` and editor support types
remain available through the input module; the standalone `Editor` renderer is
not a public component.
