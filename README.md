# GPUI Design

A source-owned, headless UI foundation for Rust and GPUI.

The library owns control behavior, state, focus, and accessibility semantics.
Applications own their component source, layout, colors, typography, and assets.
Headless separates control behavior from application-owned presentation; the
examples render native GPUI windows.

This is an early foundation for an independent component ecosystem. It currently
includes 42 public library families and 23 copyable presentation recipes. An
executable example composes shared controls into compact tabbed and spacious
sidebar presentations. The source is maintained in this repository, with GPUI
kept as an exactly pinned rendering
dependency.

The [completion criteria and current work](docs/component-coverage.md#completion-criteria-and-current-work)
track behavior, accessibility, visual defaults, source distribution, and native
validation. Catalogue coverage alone is not a claim of shadcn/ui-level completeness.

## Current scope

| Control | Behavior supplied by the library | Application-owned presentation |
| --- | --- | --- |
| **Button** | Pointer and keyboard activation, disabled state, focus, accessible name and role, and optional current-page metadata | Content, spacing, shape, and state styling |
| **Toggle** | Controlled pressed state over Button's activation, focus, and disabled behavior | Content and pressed-state styling |
| **ToggleGroup** | Multiple or optional single pressed value, programmatic replacement, one Tab stop, orientation-aware navigation, and disabled items | Group layout and Toggle option styling |
| **Checkbox** | Controlled unchecked/checked/mixed state, activation, disabled state, and focus | Indicator, label, layout, and state styling |
| **Switch** | Controlled on/off state, activation, disabled state, and focus | Track, thumb, label, and state styling |
| **RadioGroup** | Single selection, programmatic set/clear, one Tab stop, arrow navigation, disabled options, and focus | Group layout and option rendering |
| **Input** | Text editing state, selection, clipboard, focus, and the native text-input interface | Field container, text style, caret, and surrounding controls |
| **Textarea** | Multiline editing over the same engine, retained text, wrapping, and normal form Tab traversal | Field container, dimensions, text style, and surrounding content |
| **Field** | Label focus, help/error descriptions, and required/invalid metadata for existing Input/Textarea-based controls | Layout, label/help/error styles, control rendering, and validation rules |
| **Label** | Named native text label that focuses an existing control when clicked, with an explicit disabled flag and no extra Tab stop | Text, layout, styling, target accessible name, and matching disabled state |
| **Select** | Fixed single selection, buffered typeahead, Home/End and arrow navigation, disabled options, commit, and focus | Trigger, popup content, and option rendering |
| **Combobox** | Editable single/multiple selection, grouped search results, replaceable options, keyboard/pointer commit, and draft cancellation | Query frame, suggestion surface, result rows, selected chips, and empty state |
| **Calendar** | Single/multiple/range date selection, separate focus, date-disabled rules, configurable week start, bounded navigation and RTL | Month controls, weekday/date labels, grid layout, and day styling |
| **DatePicker** | Calendar selection inside Popover, selected-date or retained-cursor initial focus, single/complete-range closing, dismissal, and disabled state | Formatted trigger, calendar/month controls, presets, and popup surface |
| **Slider** | One or more ordered numeric values, bounded steps, per-thumb focus and accessibility, keyboard/pointer adjustment, disabled state, vertical and RTL geometry | Track, fill, thumbs, sizing, and state styling |
| **Progress** | Controlled percentage or indeterminate progress, accessible name and numeric range | Track, fill, label, and any animation |
| **Breadcrumb** | Named navigation, ordered path items, and native current-page metadata | Path labels, ancestor actions, separators, layout, and styling |
| **Tabs** | Active panel, one tab-list stop, automatic or manual activation, orientation-aware keys, and disabled tabs | Tab-list layout, tab styling, and panel contents |
| **Accordion** | Single or multiple open sections, header navigation, disabled items, and focused-panel closure | Header, indicator, panel contents, and layout |
| **Collapsible** | One disclosure, active-only content, disabled trigger, and focus on content closure | Trigger content, indicator, region contents, and layout |
| **Tooltip** | Plain text description on Button hover/focus, delayed hover, Escape dismissal, and native tooltip positioning | Button and tooltip text-surface styling |
| **HoverCard** | Delayed rich preview on hover, trigger-to-card pointer travel, focus preview without moving focus, and dismissal | Button action, preview content, surface, and placement |
| **Popover** | Non-modal open/close state, corner or collision-aware side/alignment placement, focus, outside dismissal, and Escape | Trigger, popup contents, and surface styling |
| **DropdownMenu** | Plain, checked/mixed/radio and nested submenu semantics, navigation, leaf dispatch, dismissal, and focus | Trigger, menu surface, command rendering, and controlled checked values |
| **ContextMenu** | Secondary-click or keyboard opening, pointer placement, and the shared menu tree's commands, navigation, dismissal, and focus | Focusable target, menu surface, command rendering, and controlled checked values |
| **Command** | Text search, enabled-result navigation, pointer/Enter action dispatch, and highlighted-result scrolling | Query frame, result rows, empty state, and optional Dialog composition |
| **Dialog** | Open/close state, modal focus, initial focus, dismissal, and focus restoration | Backdrop, popup, title, description, and close control |
| **AlertDialog** | Dialog's modal lifecycle with alert semantics and blocked backdrop dismissal | Decision text, cancellation control, and explicit application confirmation |
| **Separator** | Semantic or decorative separation, with horizontal/vertical orientation | Length, thickness, color, and spacing |
| **Table** | Shared fixed/flexible columns, header/body/footer structure, native row/cell semantics, row selection and header sort metadata | Width, cell content, typography, borders, and data operations |
| **ScrollArea** | Native viewport, existing overlay scrollbar, retained offsets, and descendant reveal on enabled axes | Viewport dimensions, content layout, and scrollbar appearance |
| **Resizable** | Two-pane ratio, pointer/keyboard resizing, fractional limits, retained sizes, and native splitter semantics | Frame size, pane content, divider thickness, and grip styling |
| **Direction** | Scoped logical direction, nested overrides, and consistent horizontal keys/pointer geometry | Logical row layout through `Direction::row`, application spacing and typography |
| **InputOtp** | One numeric editor, native selection/paste/delete/undo, digit slots and completion events | Slot/group presentation and application verification |
| **Menubar** | One top-level focus entry, menu switching, opening and shared command trees | Menu labels, surfaces, commands and styling |
| **NavigationMenu** | Coordinated disclosures, roving focus and destination Link activation | Destination callbacks, panel content and layout |
| **Toast** | Retained lifetime, hover/focus/window pause and safe dismissal | Message, action controls, placement and stacking |
| **Drawer** | Dialog lifecycle plus bottom drag-to-dismiss through a dedicated handle | Backdrop, grip, surface, scrolling body and confirmation |
| **Carousel** | Active slide, bounded/looping navigation, discrete swipe, RTL and focus restoration | Slide content, viewport size, controls and indicators |
| **Chart** | Categorical series, visible-series domain, point inspection and exact native values | Copyable native line/bar/area plot, axes, legend, readout and Table |
| **MessageScroller** | Bottom following, keyed history anchoring, unread count and jump-to-latest | Stable message identities, content, dimensions and jump control |
| **Questionnaire** | Linear question/review/complete flow, validated answers and Back | Editors, prompts, validation, focus and submission |

Shared support includes the window root, popup positioning, input infrastructure,
and opt-in test observations. Internal editing machinery retained for Input is
not a separate editor product. Public APIs are still being developed.

The [crate exports](crates/base/src/lib.rs) define the public control and support
surface. The [input module](crates/base/src/input/mod.rs) intentionally also exposes
`EditorState`, display mapping, diagnostics and provider types used by its shared
engine; its standalone `Editor` renderer is available only to inherited tests.
`Input` and `Textarea` are the supported rendered text controls. Native test
observations require the `test-support` feature. Presentation recipes remain
consumer-owned `pub(crate)` source. `Theme` configures input colors, cursor blink
and scrollbars; application palettes own light/dark appearance.

### Copyable presentation recipes

The [recipe modules](crates/base/examples/recipes) are application-owned source
compositions, documented in the [presentation guide](docs/presentation-recipes.md).
Copy the desired module and adapt its styling and any palette import to your
application tokens.
These types live in the example, outside the `gpui_design` crate exports.

| Recipe | Supplied presentation | Existing example use |
| --- | --- | --- |
| **Card** | Named region with a heading, description, header action, body, and footer | Session summary and its controls |
| **Badge** | Static label with secondary and accent appearances | Cursor and toolbar settings |
| **Kbd** | Static named keycaps and `+`-joined key combinations | Existing keyboard-guide controls and the summary's Shift+F10 context-menu hint |
| **Empty** (`EmptyState`) | Named empty state with description, optional media, and an action slot | No matching commands, with Clear search returning focus to the query |
| **ButtonGroup** | Connected horizontal/vertical buttons with an optional text slot | Previous month, month label, and next month |
| **InputGroup** | Shared frame around an existing input, with leading/trailing addons and horizontal/vertical layout | Required name with a default-name action, multiline notes footer, and command-search prefix |
| **Item** | Named content row with a title, description, and trailing action | Current recording preset beside its existing preview/edit control |
| **Alert** | Inline title and description, with separate destructive styling and explicit urgency | Informational schedule notice and a missing-name warning after closing invalid settings |
| **Pagination** | Controlled previous/next buttons, current-page text, and empty/first/last-page boundaries | Moving through the preset catalogue |
| **DataTable** | Text filtering/sorting, pagination, page/row checkbox selection, column visibility, and stable-ID row actions | Browsing recording presets, selecting rows independently, and applying a preset |
| **Spinner** | Named indeterminate Progress with a rotating arc that respects GPUI's reduced-motion setting | The unknown-progress preview |
| **Skeleton** | Static decorative placeholder with application-controlled size and shape | The preview's unavailable percentage display |
| **Avatar** | Named circular image with explicit fallback text, using GPUI's image loader and cache | Preset artwork or initials in the existing HoverCard preview |
| **AspectRatio** | Clipped media frame with a finite positive width/height ratio and application-owned content | A recording-frame illustration driven by the retained preset and session name |
| **Sheet** | Four-edge modal presentation over Dialog, viewport-limited size, fixed header/footer, and one scrolling body | Session settings and catalogue help panels, using retained state |
| **Sidebar** | Named grouped navigation, ordinary Buttons, current-page metadata, and optional whole/group Collapsible state | Grouped Settings sections and the catalogue's collapsible project navigation |
| **Typography** | Named headings with explicit levels, paragraphs with native inline-code text runs, and independently named code elements | Help headings, state-ownership explanations, and the example's source identifier |
| **CodeBlock** | Read-only multiline code, an exact full-text Copy action, and bounded two-axis scrolling with keyboard access | A JSON snapshot of the retained session settings in Help |
| **Marker** | One named date/status label between decorative separators, with wrapping text and no focus stop | The currently selected Calendar date above Schedule's notes and recording plan |
| **Message** | Read-only comment with author/time metadata, a wrapping paragraph, and optional existing actions | Schedule's current session note, with Edit note returning to the shared Textarea |
| **Attachment** | Named file-metadata group with a filename, supplied media type and size, and optional existing actions | The current configuration JSON's in-memory metadata, with Inspect JSON focusing the existing CodeBlock |
| **Trace** | Expandable summary and named read-only details over the existing Collapsible lifecycle, with an optional action | Current form values in Help, with Close details returning focus to the summary |
| **Bubble** | Incoming/outgoing surfaces, grouped author/time labels and supplied reactions | Catalogue transcript with retained reaction Buttons |

The breadth target is the range of screens and interactions supported by
shadcn/ui. The [component coverage map](docs/component-coverage.md) compares all
66 entries in its current main catalogue with the actual public scope here.
It distinguishes behavior, source-owned presentation recipes, and larger
compositions. The 42 library families and 23 recipes supply 65 distinct
implementations; Native Select maps explicitly to the existing GPUI Select. All
66 reference entries now have an implemented scope or a platform mapping. The
map records each entry's limits; catalogue coverage does not establish complete
shadcn/ui feature parity or native desktop validation.

Input provides a small, readable baseline for unset text, caret, and selection
colors. Explicit `InputEditorStyle` values take precedence. Applications own the
controls' frame borders, radii, layout, design palette, and sizing scales.

Select accepts a fixed, ordered collection when `SelectState` is created. Each
option has a unique value; the state owns the selected value and keyboard cursor.
Typing selects an enabled prefix match while closed and highlights it while open.
Home/End highlight the first/last enabled option in an open popup without committing.
Command and Combobox share `SearchItem` values, labels, aliases, and disabled
flags and group labels. Combobox adds single or multiple committed values to the
shared query and result navigation; typing alone does not change selection.
Applications can replace items while preserving native query editing and stable
selected values. Fetching, request ordering and free-form values belong to the
application rather than a second input or network engine.

The [two-style example](crates/base/examples/two_styles.rs) applies a compact dark
presentation and a light settings presentation to shared form controls, with
different section navigation. Its styles live in application code, outside the
behavior layer. A Session actions menu and
the summary's ContextMenu open the settings Sheet or toggle application values.
The two hosts use separate MenuState entities with shared command definitions and
the same application callback. Show cursor and Keep toolbar visible are checked
menu commands whose indicators and native checked values read the same application
booleans as the settings Checkbox and Switch. Activation closes the menu before
the application updates those booleans. Right-click the summary or focus it and press
Shift-F10. Hovering or focusing the preset button previews the preset, date, and
notes status in HoverCard; normal button activation opens settings.
That preview uses Avatar for the bundled Balanced artwork and explicit fallback
text for other presets or an empty selection. AspectRatio frames the current
session name at 4:3 for Presentation and 16:9 otherwise. Both presentations read
the existing retained preset, so applying a preset updates the next preview.
The artwork is embedded from `examples/assets`; this illustrative recording
frame uses no network request and generates no recording.
Both appearances use a right-edge Sheet for Session settings. It retains the
existing DialogState, initial input focus, form values, and scroll handle. Its
preferred width is 22 rem in Compact and 46 rem in Settings, capped at the
viewport width; the title and Done footer stay outside the scrolling body.
Compact uses TabList/Tab/TabPanel with automatic arrow-key selection. Settings
uses Sidebar's ordinary Buttons and renders only the selected named Region:
Tab moves among enabled entries, while Enter, Space, or a click selects one.
Below a 640 px viewport width, that Sidebar stacks above its content without
changing its keyboard contract. Each appearance owns its separate retained
TabsState; a Sidebar must not share or reuse a state previously bound by TabList.
Preferences, Help, Schedule, and Presets keep their form values when the Sheet
closes and reopens. Help uses an Accordion whose keyboard section opens a guide
in a non-modal Popover.
Breadcrumb presents `Session settings > Help > Keyboard guide` in that guide.
Its ancestor Buttons either close the guide, select Preferences through
`TabsState::select_value`, and focus its navigation entry, or return to the Help
trigger; the current item is static text. Kbd displays the controls' existing
keys, including Shift+F10 beside the
summary's context-menu hint, without registering shortcuts.
The State ownership panel uses Typography for its explanations and CodeBlock for
a JSON snapshot of the current session settings. Copy writes that complete
snapshot through GPUI's clipboard API; the preview does not edit or apply it.
Attachment describes the same string as `session-configuration.json`, with
`application/json` and its current UTF-8 byte count. Inspect JSON focuses the
existing code viewport. The example identifies the content as in memory and not
saved; it does not create, open, upload, or download a file.
CodeBlock bounds long lines and bodies with the existing ScrollArea and keeps
code scrolling separate from the surrounding Sheet body.
Trace follows the configuration preview with an expandable view of the current
session, selected date, preset, drawing tool, cursor and toolbar settings, and
notes. It reads existing form state and retains one `CollapsibleState` for its
disclosure. Close details uses that lifecycle to return focus to the
summary. This view records no job execution or history.
A slider on the summary card drives a progress preview, using the same retained
value for its label, thumb position, and progress fill.
Toggle switches the progress preview to a named Spinner and a static Skeleton
placeholder while preserving the Slider value. Spinner uses GPUI's animation
wrapper and remains still when `App::reduce_motion()` is enabled. ToggleGroup
applies independent bold/italic emphasis to the summary text.
These controls live inside a Collapsible that starts closed and preserves their
retained values. The menu's Reset preview action opens an AlertDialog with Cancel
initially focused; only confirmation restores known progress at 40%.
Search commands opens a Command inside a centered Dialog. Its query matches
action labels, values, and keywords, then dispatches the same application actions
as the menu.
Its empty state can clear the query without dispatching a command or closing the
dialog. Card and Badge compose the summary; ButtonGroup shares the calendar
controls' framing while preserving each Button's normal Tab stop and action.
InputGroup frames the existing editors and addons without adding another text
input. An empty session name offers Use default, which restores `MiniCast` and
returns focus to that input; the action is disabled for a non-editable input.
Closing settings with a missing name shows an inline Alert on the summary.
Item presents the current preset beside its existing HoverCard/edit control.
The Presets tab uses DataTable and Pagination to browse the same four choices as
the Combobox. Search, column sorting, and page changes retain the current choice;
Use preset applies a stable row ID to the existing Combobox state. The unavailable
preset remains disabled. Query, sort, and page state survive closing the Sheet.
Session settings includes a retained multiline notes field and semantic separators;
its body scrolls when the available window height cannot hold all fields.
An editable recording-preset Combobox filters choices while retaining the last
committed preset. The session-name Field connects its visible label, help text,
and application-supplied required/error state to the native input. Standalone
Labels focus the Drawing tool Select and Preview percentage Slider without
opening a popup or changing a value. The application supplies each target's
accessible name and mirrors its disabled state on the Label. A text Tooltip
on the existing Edit button explains what the settings screen changes.
Schedule uses a DatePicker composed from Calendar and Popover, and a Table
reflecting the retained session name, date, and recording preset. Marker reads the
Calendar's selected date to label the boundary above the notes and plan. Its two
rules are decorative, leaving one meaningful label and the existing Tab order.
A vertical Resizable divides shared session notes and the plan table. The table
scrolls horizontally and vertically as its pane changes size, while the surrounding form
uses the Sheet's vertical ScrollArea. Date, notes, and pane ratio survive closing
the Sheet.
Below the panes, Message presents the same session note as read-only content from
You, with the last local edit time supplied by the application. Edit note focuses
the existing Textarea. Empty notes show EmptyState instead of an empty message.
The timestamp describes local editing; the example does not send or deliver notes
and keeps no separate message history or copy of the note.
An informational Alert explains that the example retains preferences without
scheduling an actual recording. Its date bounds are 2026–2027.

The separate [catalogue example](crates/base/examples/catalogue.rs) connects the
remaining families into five retained views. Navigation demonstrates coordinated
menus, destination links and collapsible grouped project navigation; Forms and feedback runs numeric code entry through
a confirmation Drawer into a pausing Toast, plus help Sheets at all four edges; Media and data supplies RTL-aware
Carousel slides and inspectable line/bar/area charts with an exact data table;
Conversation combines incoming/outgoing bubbles, anchored history/streaming,
reactions and a validated question/review/complete flow. Selection combines date
ranges, independent dates and grouped multiple recording targets. Its calendars
start weeks on Sunday, disable weekend selection and offer RTL layout. These examples are local
and make no network requests.

## Layers and source ownership

| Layer | Implementation and origin | Responsibility |
| --- | --- | --- |
| Rendering and application runtime | Zed's GPUI, consumed through the pinned `gpui-pre` snapshot packages | Windows, layout, drawing, text, events, entities, and focus infrastructure |
| Headless behavior | This repository's `gpui-design` crate, extracted from Longbridge's `gpui-base` 0.9.0 | Control state, interaction contracts, focus policy, and accessibility semantics |
| Presentation and composition | Application code, including the copyable `examples/recipes` modules used by `two_styles` | Colors, sizes, icons, component arrangement, and application actions |

`crates/base` is the directory name inherited from GPUI Kit. Its Cargo package is
now `gpui-design`; the selected source is maintained here. The original
[`gpui-base` README](https://github.com/longbridge/gpui-kit/blob/7591a35eced761e3613c01e71ff04c10129dae1a/crates/base/README.md)
already described an unstyled behavior foundation. GPUI Design builds on that
work with a narrower public surface and independently maintained state and focus
contracts. Upstream source changes are reviewed and adopted explicitly.

[`gpui-pre`](https://gpui-kit.com/docs/installation/#why-the-dependency-is-named-gpui-pre)
publishes version-aligned snapshots of Zed's GPUI. The pinned 0.3.11 package
records Zed revision `f816e1f9263e3a8ef63c4baaa02e1441ce1c6038`.

The controls listed above belong to the behavior layer. Compound controls reuse
that layer: Toggle and Collapsible triggers compose Button, ToggleGroup composes
Toggle, and AlertDialog composes Dialog. DropdownMenu and ContextMenu share
MenuState, MenuItem, MenuOption, and MenuEvent. Their common menu behavior composes
Popover; only the trigger interaction and requested placement differ.
`MenuOption::checked(bool)`, `check_state(CheckboxState)` and `radio_checked(bool)`
project application-owned state into native menu semantics. `MenuSubmenu` routes
leaf activation through the root MenuState; it does not add another selection store.
Command reuses Input for its query and can be hosted in Dialog. Combobox reuses
Command's filtering, result cursor, scrolling, and activation, with single or
multiple committed values and a suggestion surface positioned by Popup. Textarea renders the existing
multiline editing state. Field projects associated text into that native input's
semantics. Label uses the target's existing FocusHandle for click-to-focus;
the pinned GPUI API does not expose a native labelled-by relationship between
the sibling elements. Tooltip reuses Button and GPUI's native tooltip layer while retaining
hover/focus visibility. HoverCard combines Button with Popup positioning and a
retained hover/focus lifecycle without taking focus from the trigger.
Calendar composes Button for day activation and uses the already locked Chrono
date type for Gregorian arithmetic. DatePicker coordinates that Calendar with
Popover's focus and dismissal lifecycle; it has no duplicate date value or popup
engine. Resizable uses GPUI flex layout and native dragging for two retained panes.
Table uses shared GPUI column tracks and
native table semantics. ScrollArea composes GPUI's ScrollHandle viewport with the
retained Scrollbar implementation, without another scrolling engine.
Breadcrumb supplies native navigation/list structure and current-page metadata;
the consumer composes ordinary Buttons and owns their destination callbacks.
Appearance belongs in the application. The local `gpui-design-cli` copies selected
presentation recipes and their dependencies into consumer-owned modules. A remote
registry and separately published styled component package are not provided.

See the [form control guide](docs/form-controls.md) for state ownership, keyboard
contracts, and application-owned styling. Checkbox and Switch report the next
value through a callback; the application stores it and notifies its view.
RadioGroup retains selection and option focus in an entity, accepts fixed ordered
options with unique values, and changes selection immediately on arrow navigation.

The [navigation and overlay guide](docs/navigation-overlays.md) covers Breadcrumb, Tabs,
Accordion, Collapsible, Popover, DropdownMenu, ContextMenu, Command, Tooltip,
HoverCard, and AlertDialog. Inactive panels do
not contribute layout, focus, or accessibility nodes. Popover reuses the existing popup positioning
primitive and lets keyboard focus leave its contents within the enclosing window
or dialog. Both menu triggers use that lifecycle and close before dispatching a
command, so a command can open another surface without losing its new focus.
The [layout and data guide](docs/layout-and-data.md) explains shared Table columns,
ScrollArea composition, Resizable panes, and their current limits.

## Run the example

Install [Rust with rustup](https://rustup.rs/). The repository selects **Rust
1.99.1**, including rustfmt and Clippy, through `rust-toolchain.toml`.

On **Windows**, use the MSVC toolchain with the Visual Studio C++ build tools and
a Windows SDK. On **macOS**, install Xcode Command Line Tools with
`xcode-select --install`. On **Ubuntu 24.04**, install the native build dependencies:

```sh
sudo apt-get update
sudo apt-get install --no-install-recommends -y \
  build-essential pkg-config \
  libfontconfig1-dev libfreetype6-dev libwayland-dev \
  libxkbcommon-x11-dev libx11-xcb-dev libvulkan1
```

Then clone and run from a native desktop session:

```sh
git clone https://github.com/andongmin94/gpui-design.git
cd gpui-design
cargo run -p gpui-design --example two_styles --locked -- compact
cargo run -p gpui-design --example two_styles --locked -- settings
cargo run -p gpui-design --example catalogue --locked
cargo run -p gpui-design --example starter --locked
```

`compact` is the default when no appearance argument is supplied. Both commands
run the same session-settings interaction with application-owned styling.

Start with [`starter.rs`](crates/base/examples/starter.rs) for a small complete
application. It uses only public APIs: `init`, `Root`, a retained `InputState`,
`Field`, and `Button`. The application keeps its observation and event subscriptions,
owns the colors and layout, and handles both Enter and button activation. Copy
its startup and view into a consumer before adding the larger compositions.

In an existing Rust application, depend on the checked-out source and matching
GPUI snapshots, replacing the path with your checkout:

```toml
[dependencies]
gpui-design = { path = "../gpui-design/crates/base" }
gpui = { package = "gpui-pre", version = "=0.3.11" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.11", features = ["font-kit", "x11", "wayland", "runtime_shaders"] }

[features]
test-support = ["gpui-design/test-support"]
```

The feature declaration supports the test guards in copied recipe source;
enable it when running those GPUI tests. Use the repository's pinned Rust toolchain and native prerequisites above. Copy
the desired presentation recipes into the application using their
[dependency and token instructions](docs/presentation-recipes.md#copy-into-an-application).

The GPUI snapshot family is pinned to **`=0.3.11`**. Update those pins and
`Cargo.lock` together when changing the rendering dependency.

Copy application-owned presentation source with its required recipe modules,
light/dark tokens and license:

```sh
cargo run -p gpui-design-cli --locked -- list
cargo run -p gpui-design-cli --locked -- add data-table avatar --to ../my-app/src/ui
```

Declare `mod ui;` in the consumer. The tool preserves edits to previously installed
recipe and palette files; modified generated module declarations or new-file
conflicts stop before writing. It leaves Cargo dependencies to the consumer.
See the [source-copy guide](docs/presentation-recipes.md#copy-into-an-application).
The catalogue's Selection section demonstrates range/multiple dates, grouped
recording targets, removable selected chips and replaceable local option data.


## Work on the source

| Location | Responsibility |
| --- | --- |
| `crates/cli/` | Source-copy CLI and recipe dependency catalogue |
| `crates/base/src/` | The `gpui-design` library and its behavior-level tests |
| `crates/base/tests/` | Integration tests using the public API |
| `crates/base/examples/starter.rs` | Small complete application using only public APIs |
| `crates/base/examples/two_styles.rs` | Native example and application-owned styles |
| `crates/base/examples/presentation.rs` | Copyable light/dark application tokens used by the presentation recipes |
| `crates/base/examples/recipes/` | Twenty-three catalogue recipes and Chart's supporting presentation; copyable application-owned source |
| `crates/base/examples/assets/` | Embedded example artwork used by the preset preview and its image-decoding regression |
| `docs/form-controls.md` | Form control usage and interaction contracts |
| `docs/navigation-overlays.md` | Breadcrumb, Tabs, Accordion, command menus, and composed focus behavior |
| `docs/layout-and-data.md` | Table structure, ScrollArea composition, and Resizable panes |
| `docs/presentation-recipes.md` | Copying recipes, their slots, and application-owned styling |
| `docs/component-coverage.md` | Current shadcn catalogue comparison, supported scope, and next priorities |
| `Cargo.toml` / `Cargo.lock` | Workspace and matching dependency set |

Initialize the library with `gpui_design::init(cx)` and host application content
in `gpui_design::Root`. Use the starter as the small startup reference and the
other native examples for larger compositions.
Consumer applications can work directly with this source and supply their own
presentation while reusing the behavior layer.

## Verify changes

Run the checks for the completed change set:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features gpui-design/test-support --locked -- -D warnings
cargo test --workspace --features gpui-design/test-support --locked
cargo build --workspace --bins --examples --locked
```

The [Verify workflow](https://github.com/andongmin94/gpui-design/actions/workflows/verify.yml)
uses one matrix for Linux, Windows, and macOS. Platform-independent checks run
once; each platform still compiles its native dependencies and all examples.

| Check | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Formatting and Clippy | Yes | — | — |
| Full behavior and integration suite | Yes | — | — |
| Real consumer composition flows | Included above | Yes | Yes |
| Source CLI filesystem contracts | Included above | Yes | Yes |
| Isolated recipe tests | Included above | — | — |
| Native examples and source CLI builds | Yes | Yes | Yes |

Composition tests use representative session and catalogue flows instead of
duplicating every scenario for each palette. Add tests for distinct behavior and
observed regressions. Windows and macOS run the composition target with
`--skip two_styles::recipes::`, leaving isolated recipe tests to Linux while
retaining the real consumer flows and native builds. Workflow run results are
the source of automated status.

Headless tests exercise control behavior and exposed accessibility metadata.
Native Korean/CJK IME composition and candidate windows, screen-reader interaction,
mixed-DPI displays, and the visual appearance of both styles still require manual
validation on a real desktop. These checks have not been completed for this
initial version. Performance claims also require measurements of the actual app.

## Source and license

GPUI Design was extracted from [GPUI Kit](https://github.com/longbridge/gpui-kit),
**v0.9.0**, at commit
[`7591a35eced761e3613c01e71ff04c10129dae1a`](https://github.com/longbridge/gpui-kit/tree/7591a35eced761e3613c01e71ff04c10129dae1a).
It is independently maintained.

The initial changes narrow the upstream workspace to a headless library, remove
the styled catalog and unrelated applications, retain the editing and interaction
code required by the four controls, and add focused Select/Dialog composition,
two application styles, and project-specific verification.

The form layer adapts the upstream Checkbox and Switch roots without their
decorative parts, and adds retained selection and roving focus to RadioGroup.
Tabs adds retained active panels and keyboard navigation. Popover reuses the
positioning primitive with a retained non-modal focus and dismissal lifecycle.
Accordion adds single/multiple-open state and header navigation. The shared Menu core
composes Popover with command dispatch and a keyboard/pointer cursor;
DropdownMenu and ContextMenu provide button and secondary-click entry points.
Slider adapts the upstream pointer interaction to ordered linear values with
per-thumb native focus, value semantics and keyboard adjustment. Progress retains the controlled numeric
semantics while the application supplies its track and fill.
Toggle delegates activation to Button, and ToggleGroup adds retained multiple
selection and roving focus to Toggle options.
Collapsible adds retained visibility and content-closure focus to the upstream
disclosure contract, reusing Button for its trigger. AlertDialog adapts the
upstream Dialog specialization to the existing retained DialogState and parts.
Command adds searchable, replaceable and grouped actions over the existing Input engine.
Textarea exposes that engine's retained multiline state as a public control,
with form Tab traversal and disabled-input Tab skipping. Separator adds static
native semantics without imposing visual geometry.
Combobox extends shared `SearchItem` search and activation with editable drafts,
single/multiple committed selection, grouped and replaceable options, and an
anchored suggestion lifecycle. Calendar and DatePicker retain single, multiple
and inclusive-range values with independent keyboard navigation. Field connects visible
labels and application validation feedback to existing text-input semantics;
it does not own a separate form state or validation engine. Tooltip uses GPUI's
native tooltip rendering with Button focus, hover timing, and dismissal behavior.
HoverCard reuses Popup for rich, noninteractive previews, adding delayed pointer
transitions and focus-preserving dismissal without a second placement engine.

Software source is licensed under [Apache-2.0](LICENSE-APACHE). The original
Longbridge copyright and license notices are retained. Dependencies keep their
respective licenses.
