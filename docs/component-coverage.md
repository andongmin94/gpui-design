# Component coverage

Reference catalogue checked **2026-11-09**:
[shadcn/ui All Components](https://ui.shadcn.com/docs/components).
Its main catalogue contains **66 entries**, each listed once below. The separate
AI Elements catalogue is outside this comparison; conversation components already
in the main catalogue remain included.

GPUI Design currently documents **31 public library families and twenty-one copyable
presentation/composition recipes**, covering **52 catalogue entries** within the scopes below.
This is not 52/66 feature parity: a family can exist with a deliberately limited
contract, and several shadcn entries describe combinations of other parts.
These counts do not measure performance, accessibility conformance, or completion
of native desktop validation.

The target is comparable application-building breadth: shared behavior in the
library, and reusable presentation and screen compositions whose source belongs
to the application. The groups below describe our implementation responsibilities,
not shadcn's internal architecture. The two-style example uses the reusable
[presentation recipes](presentation-recipes.md). Recipe source is distributed in
this repository; a registry and installer are not yet available.

## Status meanings

- **Available:** a public family and consumer example cover the stated scope.
- **Recipe:** reusable application source and a working consumer example cover
  the stated presentation; the recipe is outside the library's public exports.
- **Pending:** no supported public component or reusable recipe is provided yet.
- **Platform mapping:** the browser-specific contract needs a deliberate GPUI
  equivalent; a name alias would not supply that behavior.

## Behavior and semantic foundations

These components own interaction, state, focus, or a reusable semantic contract.
Their appearance remains application-owned. Exact implemented APIs are described
in the [form guide](form-controls.md),
[navigation and overlay guide](navigation-overlays.md), and
[layout and data guide](layout-and-data.md).

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Accordion | Available | Fixed items, one open section, independent header navigation, and active-only panels. |
| Alert Dialog | Available | `AlertDialog` reuses Dialog state and focus; backdrop dismissal is blocked and confirmation belongs to the application. |
| Breadcrumb | Available | Named native Navigation, List, and ListItem structure with `aria_current(Page)` on the current item. The application supplies its path, ancestor Buttons, navigation callbacks, separators, and styling; no routing or focus controller. |
| Button | Available | Native pointer/keyboard activation, disabled state, focus, and accessible naming; content and styles belong to the application. |
| Checkbox | Available | Controlled checked/unchecked/mixed state; the application supplies its indicator and label presentation. |
| Collapsible | Available | One retained disclosure, Button trigger, active-only content, and focus handling when content closes. |
| Combobox | Available | Editable search over fixed options, one committed selection, keyboard/pointer commit, and draft cancellation on dismissal. Free-form values, async sources, multiple selection, and groups are not included. |
| Command | Available | Fixed command items, text filtering, keyboard/pointer actions, and composition with Dialog. Async sources, fuzzy ranking, and groups are not included. |
| Context Menu | Available | Secondary-click placement, Shift-F10/Menu-key opening, a focusable target, and shared Menu command navigation, dismissal, and focus restoration. Target content is noninteractive; submenus, checked/radio commands, and touch long-press are not supplied. |
| Dialog | Available | Modal open/close, initial focus, containment, dismissal, and return focus; the application supplies its surface and contents. |
| Direction | Pending | A consistent RTL layout and navigation policy has not been established. Text rendering alone does not establish this support. |
| Dropdown Menu | Available | A Button entry point to the shared Menu state, fixed plain commands, enabled-item navigation, single-character search, and dismissal. Submenus and checked/radio menu items are not included. |
| Field | Available | Label focus, help/error descriptions, and required/invalid metadata for existing Input/Textarea-based controls, including a Combobox query. Validation remains application-owned; arbitrary control types are not yet covered. |
| Hover Card | Available | Noninteractive rich preview on delayed hover or trigger focus, delayed closing across trigger/card pointer travel, dismissal, disabled state, and retained-state unmount cleanup. Opening and dismissal preserve existing focus; interactive preview controls and touch long-press are not supplied. |
| Input | Available | Single-line editing, selection, clipboard, focus, and the native input interface through the retained editing engine. |
| Input OTP | Pending | Segmented entry and paste distribution are not provided by ordinary Input. |
| Label | Pending | Field provides associated labels for text inputs. A standalone label component for the wider control set is not yet supplied. |
| Menubar | Pending | Coordinated focus and menu switching across multiple top-level menus are not implemented. |
| Native Select | Platform mapping | The reference is an HTML select. Define the required native GPUI experience before adding another family; do not duplicate Select under an alias. |
| Navigation Menu | Pending | Navigation items and coordinated popup interaction are not supplied by Tabs or standalone DropdownMenu. |
| Popover | Available | Anchored non-modal content, retained open state, dismissal, normal focus traversal, and return focus. |
| Progress | Available | Controlled known progress or an indeterminate state; the application owns fill geometry and animation. |
| Radio Group | Available | Fixed items, exclusive selection, roving Tab entry, and immediate arrow selection. |
| Resizable | Available | Two retained panes, horizontal/vertical arrangement, pointer/keyboard resizing, fractional limits, and native splitter values. Groups can nest; collapse, pixel-based minimums, automatic persistence, and arbitrary multi-pane solving are not supplied. |
| Scroll Area | Available | Native viewport with existing overlay scrollbar, vertical/horizontal/both axes, retained offsets, and scoped descendant reveal. No virtualization, streaming anchoring, or separate keyboard scroll focus. |
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
The published recipes live in `crates/base/examples/recipes`, keep visual styling
in the consumer, and are composed into the native two-style example.

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Alert | Recipe | Inline title/description, default informational Note semantics, explicit urgent Alert semantics, and independent destructive styling. No notification queue, dismissal state, or live-region delivery controller. |
| Aspect Ratio | Recipe | Finite positive width/height ratio through GPUI layout, a width-controlled frame, and clipped application-owned content. The wrapper adds no role, name, or focus stop; media semantics and object fit belong to its children. Explicit height or min/max constraints follow GPUI's ordinary layout rules. |
| Avatar | Recipe | Named circular image with explicit application-supplied fallback text for absent, pending, or failed sources. GPUI owns loading, its loading delay, and caching; the wrapper preserves one Image name and no focus stop. No initials generator, status badge, group layout, or separate loading controller. |
| Badge | Recipe | Static named label with secondary/accent appearances. No click, toggle, or automatic live announcement. |
| Button Group | Recipe | Connected horizontal/vertical buttons, shared border, decorative separators, and optional static text. Existing Button actions, disabled states, and individual Tab stops are preserved; no roving selection model. |
| Card | Recipe | Named region, level-two heading, description, header action, content, and footer slots. Layout and palette belong to the application; no card focus stop or state controller. |
| Empty | Recipe | Named static group with heading, description, optional media, and supplied actions. The command-search example clears the query and restores input focus through its existing state. No loading controller or automatic live announcement. |
| Input Group | Recipe | Shared frame for an existing InputBase, CommandInput, or other supplied control, leading/trailing addons, horizontal/vertical arrangement, and focus/error presentation. Native editing, Field metadata, addon actions, and disabled policy remain with the supplied controls and application. No separate editor or focus controller. |
| Item | Recipe | Named row with a title, description, and supplied trailing action. No whole-row activation, selection, media/header/footer slots, or list controller. |
| Kbd | Recipe | Static named keycaps with readable-label overrides for symbols and `+`-joined combinations. No focus stop, shortcut registration, event dispatch, or automatic platform-glyph conversion. |
| Pagination | Recipe | Controlled previous/next Buttons, current-page text, named navigation, and zero/first/last-page boundaries. No numbered page links, ellipsis, routing, or data loading. |
| Sheet | Recipe | Right-edge, full-height modal presentation using the existing Dialog lifecycle, a viewport-limited preferred width, fixed header/footer, and one vertical ScrollArea body. The application retains DialogState and ScrollHandle and supplies header/footer content that fits the viewport. No other edge placement, drag interaction, animation, or separate modal/scroll-key controller. |
| Skeleton | Recipe | Static decorative placeholder with application-controlled width, height, rounding, and color. No focus stop, native role/name, shimmer, or loading state. |
| Spinner | Recipe | Named indeterminate Progress with a rotating arc, size/color styling, and GPUI App reduced-motion support. The application controls mounting; no independent task, timer, or automatic live announcement. |
| Table | Available | Shared fixed/flexible columns, a header row, body rows, footer, caption, and native table/row/cell semantics. No spans, sorting, filtering, selection controller, or virtualization. |
| Typography | Recipe | Named Heading with an explicit positive level, Paragraph with native styled text runs for inline code, and independently named InlineCode with native Code semantics. Paragraph exposes one complete accessible name; its code ranges have visual styling without separate Code nodes. Palette, font choice, width, and visual size belong to the application. No Markdown parser, syntax highlighting, copy action, or editor. |

## Larger combined capabilities

These entries combine lower-level behavior and presentation into more complete
workflows. They remain part of the breadth target, and should grow from working
shared components rather than independent implementations of the same input,
popup, focus, and scrolling machinery.

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Attachment | Recipe | Static native Group named by its full filename, with a visible media-type/size description and optional existing actions. Help derives in-memory JSON metadata from the same string as CodeBlock; Inspect JSON focuses that code viewport. Formatting and action behavior belong to the application. No file access, upload/download, progress, preview renderer, or container focus stop. |
| Bubble | Pending | Conversation surfaces, grouping, and reactions have no published composition. |
| Calendar | Available | Single-date selection, separate focused date, six Monday-first weeks, month/year navigation, and inclusive date bounds. Range/multiple selection, arbitrary disabled-day rules, and locale/week-start configuration are not included. |
| Carousel | Pending | Coordinated item navigation and viewport behavior are not supplied. |
| Chart | Pending | No chart rendering and interaction integration is provided. |
| Code Block | Recipe | Read-only multiline code with a full-text native Code name, a Button that copies the original string, and bounded two-axis ScrollArea overflow. The application retains its scroll/focus handles. Focused-code arrow/page/Home/End navigation and contained wheel scrolling preserve parent boundaries. No parser, syntax highlighting, editor, selection model, line numbers, or copy-feedback state. |
| Data Table | Recipe | Fixed text columns and immutable stable-ID rows, case-insensitive filtering, one-column ascending/descending text sorting, pagination, empty-state recovery, and application row actions. Current choice is supplied by the application. No multi-row selection, numeric/date comparators, column hiding/reordering, server data, virtualization, or native sort-direction metadata. |
| Date Picker | Available | A single Calendar date inside Popover, current selected-day focus, activation commit-and-close, dismissal, and disabled state. Trigger formatting and calendar appearance belong to the application; text parsing, ranges, multiple selection, and presets are not supplied. |
| Drawer | Pending | Modal infrastructure exists; drawer interaction and presentation are not implemented. |
| Marker | Recipe | Static date/status boundary with one named native Label between two decorative Separators. The text wraps within a bounded width; its content and formatting belong to the application. Schedule reads the retained Calendar selection without another date or timeline state. No focus stop, action, or automatic live announcement. |
| Message | Recipe | Read-only native Comment with application-supplied author/time Labels, a wrapping Paragraph body, and an optional existing action. Schedule presents the retained session note and its local edit time, with a Button returning focus to the shared Textarea. No message focus stop, delivery state, transport, history, streaming, or automatic live announcement. |
| Message Scroller | Pending | Transcript anchoring and streaming/history scroll behavior are not implemented by the scrollbar support alone. |
| Questionnaire | Pending | Individual form controls exist; multi-step question flow and response composition are missing. |
| Sidebar | Pending | A navigation shell with disclosure and layout behavior is not yet provided. |
| Trace | Recipe | Application-supplied summary, named read-only Paragraph details, and an optional existing action over retained Collapsible state. Shared Button activation, expanded/disabled metadata, active-only content, and focus on closure are preserved. Help presents current form values rather than recorded operations. No execution history, streaming, parser, separate disclosure state, or scroll controller. |

## Next priorities

1. Extend data-table operations only where an application needs them, including
   typed ordering or row selection. Preserve shared focus and activation behavior
   instead of duplicating those public foundations.
2. Extend the source-owned recipe set alongside working flows. Sidebar is a
   candidate for a navigation shell around the existing settings sections,
   composing shared Tabs, Buttons, and disclosure behavior where needed. Define
   its layout and keyboard contract without adding a second navigation state.
   This candidate is recorded for future work.
3. Extend menu composition where real screens need it, starting with checked
   commands and submenus. Continue composing panes with Resizable/ScrollArea;
   virtualization and transcript anchoring remain separate work.
4. Build the larger conversation and data compositions on these shared parts.
   Track their actual supported scope here rather than counting aliases or
   unfinished wrappers as coverage.

A supported entry needs a documented library API or copyable recipe and a consumer
example that works end to end. Verify each new behavior at the layer that owns it,
extend existing composition flows when appropriate, and keep the existing CI
platform split.
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
