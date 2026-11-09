# Component coverage

Reference catalogue checked **2026-11-09**:
[shadcn/ui All Components](https://ui.shadcn.com/docs/components).
Its main catalogue contains **66 entries**, each listed once below. The separate
AI Elements catalogue is outside this comparison; conversation components already
in the main catalogue remain included.

GPUI Design currently documents **23 public component families**. This is not
23/66 feature parity: a family can exist with a deliberately limited contract,
and several shadcn entries describe presentation or combinations of other parts.
These counts do not measure performance, accessibility conformance, or completion
of native desktop validation.

The target is comparable application-building breadth: shared behavior in the
library, and reusable presentation and screen compositions whose source belongs
to the application. The groups below describe our implementation responsibilities,
not shadcn's internal architecture. The current two-style example demonstrates
composition; a separately distributed styled catalogue and installer are not yet
available.

## Status meanings

- **Available:** a public family and consumer example cover the stated scope.
- **Support only:** useful lower-level code exists, but the catalogue-level
  component is not available.
- **Pending:** no supported public component or reusable recipe is provided yet.
- **Platform mapping:** the browser-specific contract needs a deliberate GPUI
  equivalent; a name alias would not supply that behavior.

## Behavior and semantic foundations

These components own interaction, state, focus, or a reusable semantic contract.
Their appearance remains application-owned. Exact implemented APIs are described
in the [form guide](form-controls.md) and
[navigation and overlay guide](navigation-overlays.md).

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Accordion | Available | Fixed items, one open section, independent header navigation, and active-only panels. |
| Alert Dialog | Available | `AlertDialog` reuses Dialog state and focus; backdrop dismissal is blocked and confirmation belongs to the application. |
| Button | Available | Native pointer/keyboard activation, disabled state, focus, and accessible naming; content and styles belong to the application. |
| Checkbox | Available | Controlled checked/unchecked/mixed state; the application supplies its indicator and label presentation. |
| Collapsible | Available | One retained disclosure, Button trigger, active-only content, and focus handling when content closes. |
| Combobox | Available | Editable search over fixed options, one committed selection, keyboard/pointer commit, and draft cancellation on dismissal. Free-form values, async sources, multiple selection, and groups are not included. |
| Command | Available | Fixed command items, text filtering, keyboard/pointer actions, and composition with Dialog. Async sources, fuzzy ranking, and groups are not included. |
| Context Menu | Pending | Context-menu triggers and placement are not supplied by the current trigger-button menu. |
| Dialog | Available | Modal open/close, initial focus, containment, dismissal, and return focus; the application supplies its surface and contents. |
| Direction | Pending | A consistent RTL layout and navigation policy has not been established. Text rendering alone does not establish this support. |
| Dropdown Menu | Available | Fixed plain commands, enabled-item navigation, single-character search, and dismissal. Submenus and checked/radio menu items are not included. |
| Field | Available | Label focus, help/error descriptions, and required/invalid metadata for existing Input/Textarea-based controls, including a Combobox query. Validation remains application-owned; arbitrary control types are not yet covered. |
| Hover Card | Pending | Hover/focus opening and dismissal need a distinct lifecycle beyond manually opening a Popover. |
| Input | Available | Single-line editing, selection, clipboard, focus, and the native input interface through the retained editing engine. |
| Input OTP | Pending | Segmented entry and paste distribution are not provided by ordinary Input. |
| Label | Pending | Field provides associated labels for text inputs. A standalone label component for the wider control set is not yet supplied. |
| Menubar | Pending | Coordinated focus and menu switching across multiple top-level menus are not implemented. |
| Native Select | Platform mapping | The reference is an HTML select. Define the required native GPUI experience before adding another family; do not duplicate Select under an alias. |
| Navigation Menu | Pending | Navigation items and coordinated popup interaction are not supplied by Tabs or standalone DropdownMenu. |
| Popover | Available | Anchored non-modal content, retained open state, dismissal, normal focus traversal, and return focus. |
| Progress | Available | Controlled known progress or an indeterminate state; the application owns fill geometry and animation. |
| Radio Group | Available | Fixed items, exclusive selection, roving Tab entry, and immediate arrow selection. |
| Resizable | Pending | A pane-divider component with pointer/keyboard resizing and size constraints is not implemented. |
| Scroll Area | Support only | Public Scrollbar types and handles exist. A documented viewport-and-scrollbar component is still missing. |
| Select | Available | One committed value from fixed ordered options, with a separate open-popup cursor. Editable search and multiple selection are not included. |
| Separator | Available | Horizontal/vertical, semantic or decorative separation; the application supplies its visual style. |
| Slider | Available | One bounded linear value and one thumb, with stepped keyboard and pointer adjustment. |
| Switch | Available | Controlled boolean state, native activation, disabled state, and focus; the application supplies track and thumb. |
| Tabs | Available | Fixed items, automatic activation, orientation-aware navigation, and active-only panels. |
| Textarea | Available | Public multiline renderer over the existing `TextareaState` editing engine; no second editor implementation. |
| Toast | Pending | Notification lifetime, dismissal, and presentation are not provided by the current overlays. |
| Toggle | Available | Controlled pressed state composed from Button's activation and focus behavior. |
| Toggle Group | Available | Fixed multiple-selection items and roving focus; moving focus does not change the pressed values. |
| Tooltip | Available | Plain text descriptions for a Button, delayed pointer hover, immediate keyboard focus, Escape dismissal, and native GPUI tooltip positioning. Rich content and touch long-press behavior are not included. |

## Presentation and composition recipes

These entries primarily need reusable source compositions with consistent visual
defaults and appropriate semantics. Existing GPUI layout and controls should do
the work; adding a new state controller for each visual name is unnecessary.
They remain pending until a usable recipe is actually provided.

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Alert | Pending | Inline message/status presentation and semantic guidance have not been packaged. |
| Aspect Ratio | Pending | No documented proportion-preserving media recipe. |
| Avatar | Pending | Image/fallback presentation and its loading-state composition are not provided. |
| Badge | Pending | No reusable status-label presentation or variants. |
| Breadcrumb | Pending | Navigation hierarchy, current-location semantics, and action wiring need a recipe. |
| Button Group | Pending | Buttons exist; shared group layout and connected styling are not yet supplied. |
| Card | Pending | Examples contain surfaces, but reusable header/content/footer parts are not supplied. |
| Empty | Pending | No consistent empty-state composition with description and optional actions. |
| Input Group | Pending | Input and Button exist; addons, inline actions, and shared framing need a recipe. |
| Item | Pending | No reusable content/action row composition. |
| Kbd | Pending | Keyboard shortcut text is application content without a reusable presentation recipe. |
| Pagination | Pending | Page controls and current-page presentation have not been composed into a supported recipe. |
| Sheet | Pending | Dialog provides modal behavior; edge placement and sheet presentation are not yet offered. |
| Skeleton | Pending | No reusable loading-placeholder shapes or reduced-motion presentation recipe. |
| Spinner | Pending | No public activity-indicator presentation with naming guidance. |
| Table | Pending | Basic table structure, cell layout, and row/column semantics are not yet exposed. |
| Typography | Pending | Examples choose text styles locally; no reusable heading/body/code presentation set is provided. |

## Larger combined capabilities

These entries combine lower-level behavior and presentation into more complete
workflows. They remain part of the breadth target, and should grow from working
shared components rather than independent implementations of the same input,
popup, focus, and scrolling machinery.

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Attachment | Pending | File/media metadata, progress, and actions have no reusable attachment composition. |
| Bubble | Pending | Conversation surfaces, grouping, and reactions have no published composition. |
| Calendar | Pending | Date-grid navigation and date-selection behavior are not implemented. |
| Carousel | Pending | Coordinated item navigation and viewport behavior are not supplied. |
| Chart | Pending | No chart rendering and interaction integration is provided. |
| Code Block | Pending | Internal editing machinery does not constitute a public code-display, copy, and presentation component. |
| Data Table | Pending | Table structure and reusable sorting/filtering/selection compositions are missing. |
| Date Picker | Pending | Popover is available, but Calendar and date-value integration are missing. |
| Drawer | Pending | Modal infrastructure exists; drawer interaction and presentation are not implemented. |
| Marker | Pending | Conversation status notes and labeled separators have no published recipe. |
| Message | Pending | Message-row layout, metadata, and actions have no reusable composition. |
| Message Scroller | Pending | Transcript anchoring and streaming/history scroll behavior are not implemented by the scrollbar support alone. |
| Questionnaire | Pending | Individual form controls exist; multi-step question flow and response composition are missing. |
| Sidebar | Pending | A navigation shell with disclosure and layout behavior is not yet provided. |
| Trace | Pending | Shared disclosure controls exist; expandable activity/tool/source rows have no reusable composition. |

## Next priorities

1. Add basic Table structure before data operations, and Calendar navigation
   and selection before date pickers. Build Data Table and Date Picker from
   those working public foundations.
2. Complete a composed Scroll Area before specialized scrolling views, and add
   Resizable for application panes. Reuse existing viewport, scrolling, and
   pointer infrastructure rather than creating another layout runtime.
3. Publish source-owned presentation recipes alongside those working flows,
   starting with common surfaces, field framing, grouped actions, and empty
   states. Keep palette and sizing decisions outside the behavior layer.
4. Build the larger conversation and data compositions on these shared parts.
   Track their actual supported scope here rather than counting aliases or
   unfinished wrappers as coverage.

A supported entry needs a public contract and a consumer example that works end
to end. Verify each new behavior at the layer that owns it, extend existing
composition flows when appropriate, and keep the existing CI platform split.
Native IME, screen-reader, visual, and performance observations must be reported
separately from automated behavior checks.

## Reference distinctions

The [shadcn introduction](https://ui.shadcn.com/docs) describes source ownership,
composition, distribution, and visual defaults. Comparable breadth includes
those consumer-facing recipes as well as headless behavior.

The current [Combobox](https://ui.shadcn.com/docs/components/base/combobox)
documentation covers an editable autocomplete and additional selection variants;
the implemented Combobox here covers fixed single selection. The other variants
remain gaps in this family rather than extra completed components.
[Command](https://ui.shadcn.com/docs/components/base/command) includes a dialog
composition rather than requiring a second independent command engine.

[Date Picker](https://ui.shadcn.com/docs/components/base/date-picker) is composed
from calendar and popup parts.
[Data Table](https://ui.shadcn.com/docs/components/base/data-table) is a guide to
building application-specific tables over shared foundations.
[Native Select](https://ui.shadcn.com/docs/components/base/native-select) explicitly
wraps a browser HTML control, so its platform contract requires adaptation.
