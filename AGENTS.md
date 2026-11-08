# Project guidance

GPUI Design is an independently maintained, source-owned headless UI foundation
for Rust and GPUI. Read the relevant implementation, its public exports, and its
tests before changing a behavior. The README describes the current scope.

## Implementation

- Implement the smallest complete change that satisfies the current requirement.
  Keep the project working end to end as capabilities are added.
- Keep behavior, state, focus, and accessibility in the library. Keep application
  layout, palettes, typography, and decorative assets in examples or consumer
  code. Input's minimal readable color defaults belong to its editing machinery.
- Give each concern a clear owner. Avoid speculative abstractions, configuration,
  fallback systems, and recovery layers.
- Remove obsolete APIs and paths. Do not add compatibility aliases or migrations.
- Check existing dependencies and their actual APIs before adding a package or
  recreating common functionality.
- Keep the GPUI snapshot family pinned together. Update the manifests and lockfile
  together, then verify the affected native targets.
- Preserve retained upstream copyright and license notices. Describe source
  provenance and substantial changes in the README.

## Verification

- Test behavior at the lowest layer that proves it. Use existing GPUI test
  contexts for keyboard, pointer, focus, and accessibility metadata contracts.
- Keep tests focused on observable behavior. Do not introduce a separate test
  framework or tests that merely repeat implementation details.
- Run checks for the affected scope. Repeat a passing check only after a relevant
  change, a new failure, a concrete unresolved concern, or a required gate.
- Run the integrated verification once for the completed change set. Use the
  commands in the README and the existing verification workflow.
- Report native window, IME, screen-reader, DPI, and visual observations separately
  from headless test results. Do not claim unperformed validation.

Use Git and brief current status notes for progress. Avoid duplicate source copies,
parallel history documents, and generated reports that repeat existing evidence.
