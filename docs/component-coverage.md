# Component coverage

Reference catalogue checked **2026-11-11**:
[shadcn/ui All Components](https://ui.shadcn.com/docs/components).
Its main catalogue contains **66 entries**, each listed once below. The separate
AI Elements catalogue is outside this comparison; conversation components already
in the main catalogue remain included.

GPUI Design documents **42 public library families and 23 copyable
presentation/composition recipes**, supplying **65 distinct implementations**.
The remaining catalogue entry, Native Select, is explicitly **mapped to Select**
for GPUI. All **66 reference entries have an implemented scope or platform
mapping** below; none remains unassigned or Pending.

This is catalogue coverage, not complete shadcn/ui feature parity. Each contract
has explicit limits, and the HTML-specific mapping does not implement a browser
control or an operating-system picker. These counts do not measure performance,
accessibility conformance, or completion of native desktop validation.

The target is comparable application-building breadth: shared behavior in the
library, and reusable presentation and screen compositions whose source belongs
to the application. The groups below describe our implementation responsibilities,
not shadcn's internal architecture. The `two_styles` and `catalogue` native examples
use the reusable [presentation recipes](presentation-recipes.md). Recipe source is distributed in
this repository; a local source CLI installs recipes, dependencies, tokens and
license into consumer-owned modules. A remote registry is not provided.

## Status meanings

- **Available:** a public family and consumer example cover the stated scope.
- **Recipe:** reusable application source and a working consumer example cover
  the stated presentation; the recipe is outside the library's public exports.
- **Mapped:** an explicit GPUI platform equivalent supplies the native application
  need; the browser-only contract is documented instead of hidden behind an alias.

## Behavior and semantic foundations

These components own interaction, state, focus, or a reusable semantic contract.
Their appearance remains application-owned. Exact implemented APIs are described
in the [form guide](form-controls.md),
[navigation and overlay guide](navigation-overlays.md), and
[layout and data guide](layout-and-data.md).

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Accordion | Available | Fixed items, single/multiple open sections, independent header navigation, and active-only panels. |
| Alert Dialog | Available | `AlertDialog` reuses Dialog state and focus; backdrop dismissal is blocked and confirmation belongs to the application. |
| Breadcrumb | Available | Named native Navigation, List, and ListItem structure with `aria_current(Page)` on the current item. The application supplies its path, ancestor Buttons, navigation callbacks, separators, and styling; no routing or focus controller. |
| Button | Available | Native pointer/keyboard activation, disabled state, focus, accessible naming, and explicit current-page metadata independent of selected styling; content and styles belong to the application. |
| Checkbox | Available | Controlled checked/unchecked/mixed state; the application supplies its indicator and label presentation. |
| Collapsible | Available | One retained disclosure, Button trigger, active-only content, and focus handling when content closes. |
| Combobox | Available | Editable single/multiple selection, grouped results, removable selected values and replaceable items that preserve native query/focus/IME and surviving choices. Fetching/request ordering remain application-owned; no free-form values or network manager. |
| Command | Available | Replaceable command items, named result groups, text filtering, keyboard/pointer actions and Dialog composition. Native query and a surviving highlight are preserved on item replacement; fetching and fuzzy ranking are not supplied. |
| Context Menu | Available | Secondary-click and keyboard opening, focusable target, shared plain/checked/mixed/radio commands and nested submenus with leaf dispatch, dismissal and focus restoration. Values are application-owned. Target content is noninteractive; touch long-press is not supplied. |
| Dialog | Available | Modal open/close, native modal flag, background pointer/wheel isolation with or without a decorative backdrop, initial focus, containment, dismissal and return focus; the application supplies its surface and contents. |
| Direction | Available | Scoped `DirectionProvider`, nested overrides, logical row layout, and inherited horizontal navigation for Tabs, RadioGroup, ToggleGroup, Slider, Carousel, Chart, Menubar and NavigationMenu. Slider pointer geometry also mirrors. Application layout uses `Direction::row`; GPUI owns text shaping and editing. |
| Dropdown Menu | Available | Button entry to shared menus, application-controlled checked/mixed/radio semantics, enabled-item navigation, single-character search and nested submenu navigation. Leaf activation closes the menu tree before emitting one root action; no second selection store. |
| Field | Available | Label focus, help/error descriptions, and required/invalid metadata for existing Input/Textarea-based controls, including a Combobox query. Validation remains application-owned; arbitrary control types are not yet covered. |
| Hover Card | Available | Noninteractive rich preview on delayed hover or trigger focus, delayed closing across trigger/card pointer travel, dismissal, disabled state, and retained-state unmount cleanup. Opening and dismissal preserve existing focus; interactive preview controls and touch long-press are not supplied. |
| Input | Available | Single-line editing, selection, clipboard, focus, and the native input interface through the retained editing engine. |
| Input OTP | Available | One retained numeric editor with application-rendered slots, selection, clipboard, deletion, undo, disabled/read-only states and completion events. Full resulting pasted value must fit and contain ASCII digits; no transport or code verification. |
| Label | Available | Named native plain-text label with click-to-focus through a supplied existing FocusHandle, an explicit disabled flag, and no additional Tab stop. The application keeps the target mounted and supplies its accessible name and matching disabled state. No activation forwarding or native labelled-by relationship; Field retains its richer text-input association. |
| Menubar | Available | Coordinated top-level menus with one Tab entry, logical Left/Right and Home/End movement, Down/Up opening, open-menu pointer switching and shared Menu command semantics. |
| Native Select | Mapped | Use the existing GPUI `Select` for one value from fixed options, keyboard selection, disabled options and native accessibility metadata. HTML form submission, optgroup markup and browser/OS picker UI are not GPUI contracts. No duplicate NativeSelect export. |
| Navigation Menu | Available | Named navigation with destination Links, explicit disclosure entries, coordinated popups, roving focus, logical arrows and open-menu pointer switching. Popup contents remain ordinary groups with normal Tab traversal; navigation destinations are application-owned. |
| Popover | Available | Non-modal content with corner or collision-aware side/alignment placement, retained open state, dismissal, normal focus traversal, and return focus. |
| Progress | Available | Controlled known progress or an indeterminate state; the application owns fill geometry and animation. |
| Radio Group | Available | Fixed items, exclusive selection, roving Tab entry, and immediate arrow selection. |
| Resizable | Available | Two retained panes, horizontal/vertical arrangement, pointer/keyboard resizing, fractional limits, and native splitter values. Groups can nest; collapse, pixel-based minimums, automatic persistence, and arbitrary multi-pane solving are not supplied. |
| Scroll Area | Available | Native viewport with existing overlay scrollbar, vertical/horizontal/both axes, retained offsets, and scoped descendant reveal. No virtualization, streaming anchoring, or separate keyboard scroll focus. |
| Select | Available | Fixed ordered options, one committed value, separate popup cursor, Home/End, buffered Unicode label typeahead and atomic programmatic set/clear. Closed typing commits; open typing only highlights. Editable query, item replacement and multiple selection are not included. |
| Separator | Available | Horizontal/vertical, semantic or decorative separation; the application supplies its visual style. |
| Slider | Available | Ordered single/range/multiple values with per-thumb focus and numeric semantics, nearest-thumb pointer adjustment, noncrossing steps, vertical and RTL geometry. |
| Switch | Available | Controlled boolean state, native activation, disabled state, and focus; the application supplies track and thumb. |
| Tabs | Available | Fixed items, automatic or manual activation, orientation-aware navigation, and active-only panels. |
| Textarea | Available | Public multiline renderer over the existing `TextareaState` editing engine; no second editor implementation. |
| Toast | Available | Retained notification lifetime and explicit/timeout dismissal; hover, contained focus and inactive windows pause remaining time. Showing never takes focus; explicit focused dismissal restores prior focus. Applications own message, actions and stacking; no queue service. |
| Toggle | Available | Controlled pressed state composed from Button's activation and focus behavior. |
| Toggle Group | Available | Fixed items with multiple or optional single selection and roving focus; moving focus does not change the pressed values. |
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
| Sheet | Recipe | Top/right/bottom/left presentation using the existing Dialog lifecycle, viewport-limited preferred size, fixed header/footer and one vertical ScrollArea body. The application retains DialogState/ScrollHandle and supplies header/footer content that fits the viewport. No drag interaction, animation or separate modal/scroll-key controller. |
| Skeleton | Recipe | Static decorative placeholder with application-controlled width, height, rounding, and color. No focus stop, native role/name, shimmer, or loading state. |
| Spinner | Recipe | Named indeterminate Progress with a rotating arc, size/color styling, and GPUI App reduced-motion support. The application controls mounting; no independent task, timer, or automatic live announcement. |
| Table | Available | Shared fixed/flexible columns, header/body/footer rows, caption and native table/row/cell semantics, with optional application-supplied row selection and header sort direction. No spans, sorting/filtering engine, selection controller or virtualization. |
| Typography | Recipe | Named Heading with an explicit positive level, Paragraph with native styled text runs for inline code, and independently named InlineCode with native Code semantics. Paragraph exposes one complete accessible name; its code ranges have visual styling without separate Code nodes. Palette, font choice, width, and visual size belong to the application. No Markdown parser, syntax highlighting, copy action, or editor. |

## Larger combined capabilities

These entries combine lower-level behavior and presentation into more complete
workflows. They remain part of the breadth target, and should grow from working
shared components rather than independent implementations of the same input,
popup, focus, and scrolling machinery.

| Catalogue entry | Status | Current scope or gap |
| --- | --- | --- |
| Attachment | Recipe | Static native Group named by its full filename, with a visible media-type/size description and optional existing actions. Help derives in-memory JSON metadata from the same string as CodeBlock; Inspect JSON focuses that code viewport. Formatting and action behavior belong to the application. No file access, upload/download, progress, preview renderer, or container focus stop. |
| Bubble | Recipe | Incoming/outgoing conversation surfaces, grouped author/time metadata and supplied reaction Buttons, demonstrated in the local transcript. Styling, reaction values and message content remain application-owned; no chat transport. |
| Calendar | Available | Single/multiple/inclusive range selection, separate focus, six weeks with configurable first weekday, disabled-date predicate, bounded navigation, RTL and native multiselectable metadata. Unavailable in-bounds dates remain keyboard-discoverable but cannot be selected. No non-Gregorian arithmetic, clock or timezone conversion. |
| Carousel | Available | Fixed single-slide viewport, bounded or looping previous/next, horizontal/vertical keys, RTL, discrete pointer swipe, active-only slides and safe focus when a slide disappears. App-owned media/actions; no autoplay, inertia or multi-slide sizing engine. |
| Chart | Available | Headless categorical data, visible series and pointer/keyboard point inspection; copyable native line/bar/area plot with axes, legend Toggles, exact-value readout and same-data Table. Negative values and zero baselines are supported. No time/log scales, stacking, pie/radar charts or external chart engine. |
| Code Block | Recipe | Read-only multiline code with a full-text native Code name, a Button that copies the original string, and bounded two-axis ScrollArea overflow. The application retains its scroll/focus handles. Focused-code arrow/page/Home/End navigation and contained wheel scrolling preserve parent boundaries. No parser, syntax highlighting, editor, selection model, line numbers, or copy-feedback state. |
| Data Table | Recipe | Fixed text columns and immutable stable-ID rows, case-insensitive filtering, one-column text sorting, pagination, page/row checkbox selection, column visibility, empty-state recovery and application row actions. Selection survives filtering/paging and remains independent of the application's current choice. Native row selection and header sort metadata are supplied. No numeric/date comparators, column reordering, server data or virtualization. |
| Date Picker | Available | Single/multiple/range Calendar values inside Popover; single/complete-range activation closes before events, partial/multiple values stay open. Preset Buttons use the same programmatic setter. Trigger formatting and appearance are application-owned; text parsing and preset fetching are not supplied. |
| Drawer | Available | Bottom modal surface reusing Dialog focus, Escape and backdrop lifecycle, plus a dedicated downward drag handle with threshold dismissal and snap-back. Body scrolling/editing stays independent; no snap-point or nested-drag engine. |
| Marker | Recipe | Static date/status boundary with one named native Label between two decorative Separators. The text wraps within a bounded width; its content and formatting belong to the application. Schedule reads the retained Calendar selection without another date or timeline state. No focus stop, action, or automatic live announcement. |
| Message | Recipe | Read-only native Comment with application-supplied author/time Labels, a wrapping Paragraph body, and an optional existing action. Schedule presents the retained session note and its local edit time, with a Button returning focus to the shared Textarea. No message focus stop, delivery state, transport, history, streaming, or automatic live announcement. |
| Message Scroller | Available | Existing ScrollArea with stable message keys, initial/append/stream bottom following, same-frame preservation of a visible history anchor, unread count and jump-to-latest. No virtualization, transport or forced focus changes. |
| Questionnaire | Available | Typed application-owned answers with a linear question/review/complete state, validation-gated advance, Back and retained answers. The example connects real Input editors and app-owned focus changes; no schema interpreter or conditional branching. |
| Sidebar | Recipe | Named navigation with ordinary Buttons, explicit activation and current-page metadata over dedicated TabsState; static groups and optional whole/group Collapsible state preserve selection and restore focus when entries unmount. Settings and the catalogue stack navigation above content below 640 px. No routing, icon rail, arbitrary nested menu tree or reuse of a state previously bound by TabList. |
| Trace | Recipe | Application-supplied summary, named read-only Paragraph details, and an optional existing action over retained Collapsible state. Shared Button activation, expanded/disabled metadata, active-only content, and focus on closure are preserved. Help presents current form values rather than recorded operations. No execution history, streaming, parser, separate disclosure state, or scroll controller. |

## Completion criteria and current work

The target is shadcn/ui-level usability across behavior, accessibility, visual
defaults, and source distribution on native GPUI. The 66-entry catalogue counts
scope coverage only. It is not a completion percentage or a release-readiness
claim. `Available`, `Recipe`, and `Mapped` above do not mean parity is complete.

Work is tracked here and in Git, without a separate progress report. A capability
is complete when its documented consumer contract is implemented, exercised in a
native example, covered at the appropriate existing test layer, and accurately
documented. Platform observations are recorded separately from automated checks.

| Priority | Work | Exit condition | Current state |
| --- | --- | --- | --- |
| 1 | Existing correctness and core interaction gaps | Fix observed failures; support single-selection ToggleGroup and multiple-open Accordion; verify native values, keyboard/pointer activation, disabled state, and focus on closure | Implemented with focused tests and catalogue consumers |
| 2 | Remaining common component contracts | Audit the official component examples against each scope above; implement manual Tabs activation, popup side/alignment, range inputs/date selection, and the remaining consumer-required selection/table/navigation capabilities with focused tests and examples | Listed common gaps implemented; core retained-value, composition, source-distribution and native-metadata audits completed. Feature limits remain explicit in the scope table |
| 3 | Visual defaults and accessibility | Consistent visible keyboard focus, disabled/hover/selected/error states, reusable application-owned tokens, narrow-window layout, and usable light/dark presentations | Catalogue focus and copyable light/dark tokens implemented; native Calendar/Sheet smoke performed; unnamed catalogue buttons/links corrected with regression coverage; wider native review pending |
| 4 | Source distribution and developer experience | A small complete starter, compilable public examples, explicit supported API boundary, and a source-copy workflow that resolves recipe dependencies and tokens | Starter, doctest gate, application tokens and dependency-aware source CLI implemented; supported input/control boundary and consumer test feature documented; unused ThemeAppearance removed |
| 5 | Release validation | Existing formatting, lint, full behavior suite and native example builds pass; representative native IME, screen-reader, DPI, and visual flows have platform-specific observations | Not complete |

The current Windows checkpoint passed 702 behavior/integration tests, five CLI
tests, 16 doctests (two existing ignored), formatting, Clippy and affected native
example/CLI builds. The [three-platform run for 8c538034](https://github.com/andongmin94/gpui-design/actions/runs/42852815564)
also passed: full checks on Linux, representative composition/CLI tests and native
builds on Windows/macOS. Modal metadata/input-isolation follow-up is being verified.
All 24 copied recipe modules and their test-feature compilation have standalone
consumer coverage; source dependencies, exports, assets and 59 local document
links were audited. [Draft PR #1](https://github.com/andongmin94/gpui-design/pull/1)
contains the implementation commits and current validation status.

Existing tests cover the documented selection/overlay contracts, retained-value
setters, unavailable Calendar days/RTL, table selection/visibility, Sidebar closure,
native read-only metadata, multiline/scrolled UTF-16 point lookup and modal flags
(including non-modal Popover), plus background pointer/wheel isolation without
a decorative backdrop. Composition
checks include Sidebar at 320/980 px and all Sheet edges at 320×520/980×840.

Native Windows observations cover starter editing/Tab/Enter, the Selection view
and range popup, unavailable Sunday focus/blocked activation, and top Sheet
layout/body scrolling/focus return. UI Automation names were inspected, corrected
where absent, and re-observed in Navigation/Selection. These are limited visual
and metadata observations, not screen-reader speech verification.

Still open: actual Korean/CJK IME composition/candidates/commit/cancel, screen-reader
speech and actions, mixed DPI, and wider desktop visual review. The automation
tool does not support the Hangul key, so that native flow needs manual Korean
mode selection. Headless handler tests do
not establish native IME behavior. The overall goal remains incomplete.
The official [Accordion multiple example](https://ui.shadcn.com/docs/components/base/accordion#multiple),
[Toggle Group usage](https://ui.shadcn.com/docs/components/base/toggle-group#usage),
[Slider range](https://ui.shadcn.com/docs/components/base/slider#range),
[Calendar](https://ui.shadcn.com/docs/components/base/calendar), and
[Combobox](https://ui.shadcn.com/docs/components/base/combobox) provide concrete
behavior references. The [Data Table guide](https://ui.shadcn.com/docs/components/base/data-table)
and [Sidebar](https://ui.shadcn.com/docs/components/base/sidebar) supply the
selection, column visibility and collapse/group composition references.
The [Sheet reference](https://ui.shadcn.com/docs/components/base/sheet) describes
four-edge placement. Calendar's unavailable-date focus follows the
[APG disabled-grid guidance](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/#focusability-of-disabled-controls);
its application predicate and week order remain explicit native contracts.
The [introduction](https://ui.shadcn.com/docs) and
[theming guide](https://ui.shadcn.com/docs/theming) also make visual defaults and
source reuse part of the target. Browser-specific APIs require an explicit native
equivalent rather than an extra alias.

Use
`cargo run -p gpui-design --example two_styles --locked -- settings` for the
session controls and `cargo run -p gpui-design --example catalogue --locked` for
navigation, OTP/Drawer/Toast, media/chart, and conversation/questionnaire flows.

Native IME, screen-reader, visual, DPI, and performance observations remain
separate from automated checks. No unperformed platform validation is considered
complete, and no performance claim is made without measurements.

## Reference distinctions

The [shadcn introduction](https://ui.shadcn.com/docs) describes source ownership,
composition, distribution, and visual defaults. Comparable breadth includes
those consumer-facing recipes as well as headless behavior.

The current [Combobox](https://ui.shadcn.com/docs/components/base/combobox)
documentation covers an editable autocomplete and additional selection variants;
the implementation here supplies grouped single/multiple selection and item
replacement. Applications retain chip presentation and asynchronous request
ownership; these capabilities deepen the same family rather than add components.
[Command](https://ui.shadcn.com/docs/components/base/command) includes a dialog
composition rather than requiring a second independent command engine.

[Date Picker](https://ui.shadcn.com/docs/components/base/date-picker) is composed
from calendar and popup parts.
[Data Table](https://ui.shadcn.com/docs/components/base/data-table) is a guide to
building application-specific tables over shared foundations.
[Native Select](https://ui.shadcn.com/docs/components/base/native-select) explicitly
wraps a browser HTML control, so its platform contract requires adaptation.
