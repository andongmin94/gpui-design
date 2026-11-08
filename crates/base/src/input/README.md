# Input module layout

Modified from GPUI Kit v0.9.0 for GPUI Design: `Input` is the supported
rendered text control. The shared editing engine and its existing regression
tests remain together because single-line input uses their layout, selection,
IME, undo, and text-shaping behavior.

- `base/` owns shared state, layout, cursor movement, selection, masking,
  native integration, and painting.
- `input/` provides the single-line `Input` element and `InputState`.
- `textarea/` retains a shared multiline state alias for the editing engine.
- `editor/` retains shared display mapping, highlighting contracts, diagnostics,
  and language-provider types required by the engine. Its standalone renderer
  is compiled only for the inherited regression tests.

`mod.rs` defines the public input API. The retained editor and multiline support
types do not constitute separately supported controls in this initial scope.
