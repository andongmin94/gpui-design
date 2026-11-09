# GPUI Design

A source-owned, headless UI foundation for Rust and GPUI.

The library owns control behavior, state, focus, and accessibility semantics.
Applications own their component source, layout, colors, typography, and assets.
Headless separates control behavior from application-owned presentation; the
examples render native GPUI windows.

This is an early foundation for an independent component ecosystem. It currently
includes 26 component families and an executable example that gives the same behavior
two visual treatments. The source is maintained in this repository, with GPUI
kept as an exactly pinned rendering dependency.

## Current scope

| Control | Behavior supplied by the library | Application-owned presentation |
| --- | --- | --- |
| **Button** | Pointer and keyboard activation, disabled state, focus, accessible name and role | Content, spacing, shape, and state styling |
| **Toggle** | Controlled pressed state over Button's activation, focus, and disabled behavior | Content and pressed-state styling |
| **ToggleGroup** | Multiple pressed values, one Tab stop, orientation-aware navigation, and disabled items | Group layout and Toggle option styling |
| **Checkbox** | Controlled unchecked/checked/mixed state, activation, disabled state, and focus | Indicator, label, layout, and state styling |
| **Switch** | Controlled on/off state, activation, disabled state, and focus | Track, thumb, label, and state styling |
| **RadioGroup** | Single selection, one Tab stop, arrow navigation, disabled options, and focus | Group layout and option rendering |
| **Input** | Text editing state, selection, clipboard, focus, and the native text-input interface | Field container, text style, caret, and surrounding controls |
| **Textarea** | Multiline editing over the same engine, retained text, wrapping, and normal form Tab traversal | Field container, dimensions, text style, and surrounding content |
| **Field** | Label focus, help/error descriptions, and required/invalid metadata for existing Input/Textarea-based controls | Layout, label/help/error styles, control rendering, and validation rules |
| **Select** | Single selection from fixed options, keyboard navigation, disabled options, commit, and focus | Trigger, popup content, and option rendering |
| **Combobox** | Editable search over fixed options, separate committed selection, keyboard/pointer commit, and draft cancellation on dismissal | Query frame, suggestion surface, result rows, and empty state |
| **Calendar** | Single date selection, separate focused date, bounded month/year navigation, and a Monday-first grid | Month controls, weekday/date labels, grid layout, and day styling |
| **Slider** | One numeric value, bounded steps, keyboard and pointer adjustment, disabled state, and focus | Track, fill, thumb, sizing, and state styling |
| **Progress** | Controlled percentage or indeterminate progress, accessible name and numeric range | Track, fill, label, and any animation |
| **Tabs** | Active panel, one tab-list stop, automatic activation, orientation-aware keys, and disabled tabs | Tab-list layout, tab styling, and panel contents |
| **Accordion** | One collapsible section, header navigation, disabled items, and focused-panel closure | Header, indicator, panel contents, and layout |
| **Collapsible** | One disclosure, active-only content, disabled trigger, and focus on content closure | Trigger content, indicator, region contents, and layout |
| **Tooltip** | Plain text description on Button hover/focus, delayed hover, Escape dismissal, and native tooltip positioning | Button and tooltip text-surface styling |
| **Popover** | Non-modal open/close state, anchored placement, focus, outside dismissal, and Escape | Trigger, popup contents, and surface styling |
| **DropdownMenu** | Command activation, enabled-item navigation, single-character search, dismissal, and focus | Trigger, menu surface, and command rendering |
| **Command** | Text search, enabled-result navigation, pointer/Enter action dispatch, and highlighted-result scrolling | Query frame, result rows, empty state, and optional Dialog composition |
| **Dialog** | Open/close state, modal focus, initial focus, dismissal, and focus restoration | Backdrop, popup, title, description, and close control |
| **AlertDialog** | Dialog's modal lifecycle with alert semantics and blocked backdrop dismissal | Decision text, cancellation control, and explicit application confirmation |
| **Separator** | Semantic or decorative separation, with horizontal/vertical orientation | Length, thickness, color, and spacing |
| **Table** | Shared fixed/flexible columns, header/body/footer structure, and native row/cell semantics | Width, cell content, typography, borders, and data operations |
| **ScrollArea** | Native viewport, existing overlay scrollbar, retained offsets, and descendant reveal on enabled axes | Viewport dimensions, content layout, and scrollbar appearance |

Shared support includes the window root, popup positioning, input infrastructure,
and opt-in test observations. Internal editing machinery retained for Input is
not a separate editor product. Public APIs are still being developed.

The breadth target is the range of screens and interactions supported by
shadcn/ui. The [component coverage map](docs/component-coverage.md) compares all
66 entries in its current main catalogue with the actual public scope here.
It distinguishes behavior, source-owned presentation recipes, and larger
compositions. Each of the 26 available families has a defined contract; the map
records its current limits and the work still needed for comparable coverage.

Input provides a small, readable baseline for unset text, caret, and selection
colors. Explicit `InputEditorStyle` values take precedence. Applications own the
controls' frame borders, radii, layout, design palette, and sizing scales.

Select accepts a fixed, ordered collection when `SelectState` is created. Each
option has a unique value; the state owns the selected value and keyboard cursor.
Command and Combobox share `SearchItem` values, labels, aliases, and disabled
flags. Combobox adds one committed selection to the shared query and result
navigation; typing alone does not change that selection. Its current scope is
fixed single selection, without free-form values, async sources, or groups.

The [two-style example](crates/base/examples/two_styles.rs) applies a compact dark
presentation and a light settings presentation to the same controls. Its styles
live in application code, outside the behavior layer. A Session actions menu opens
the settings dialog or toggles application values. The dialog combines Preferences,
Help, and Schedule tabs, and retains values when it is closed and reopened. Help uses an
Accordion whose keyboard section opens a guide in a non-modal Popover.
A slider on the summary card drives a progress preview, using the same retained
value for its label, thumb position, and progress fill.
Toggle switches the progress preview to an unknown amount, and ToggleGroup applies
independent bold/italic emphasis to the summary text.
These controls live inside a Collapsible that starts closed and preserves their
retained values. The menu's Reset preview action opens an AlertDialog with Cancel
initially focused; only confirmation restores known progress at 40%.
Search commands opens a Command inside Dialog. Its query matches action labels,
values, and keywords, then dispatches the same application actions as the menu.
Session settings includes a retained multiline notes field and semantic separators;
its body scrolls when the available window height cannot hold all fields.
An editable recording-preset Combobox filters choices while retaining the last
committed preset. The session-name Field connects its visible label, help text,
and application-supplied required/error state to the native input. A text Tooltip
on the existing Edit button explains what the settings screen changes.
Schedule uses a single-date Calendar and a Table reflecting the retained session
name, date, and recording preset. The table sits in a horizontal ScrollArea, while
the form uses a vertical ScrollArea that reveals focused inputs and calendar days.
The example's date bounds are 2026–2027; it does not schedule an actual recording.

## Layers and source ownership

| Layer | Implementation and origin | Responsibility |
| --- | --- | --- |
| Rendering and application runtime | Zed's GPUI, consumed through the pinned `gpui-pre` snapshot packages | Windows, layout, drawing, text, events, entities, and focus infrastructure |
| Headless behavior | This repository's `gpui-design` crate, extracted from Longbridge's `gpui-base` 0.9.0 | Control state, interaction contracts, focus policy, and accessibility semantics |
| Presentation and composition | Application code, currently demonstrated in `two_styles` | Colors, sizes, icons, component arrangement, and application actions |

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
Toggle, DropdownMenu composes Popover, and AlertDialog composes Dialog.
Command reuses Input for its query and can be hosted in Dialog. Combobox reuses
Command's filtering, result cursor, scrolling, and activation, with one committed
value and a suggestion surface positioned by Popup. Textarea renders the existing
multiline editing state. Field projects associated text into that native input's
semantics. Tooltip reuses Button and GPUI's native tooltip layer while retaining
hover/focus visibility.
Calendar composes Button for day activation and uses the already locked Chrono
date type for Gregorian arithmetic. Table uses shared GPUI column tracks and
native table semantics. ScrollArea composes GPUI's ScrollHandle viewport with the
retained Scrollbar implementation, without another scrolling engine.
Appearance belongs in the application. A separately distributed
styled component catalog, installation CLI, and registry have not been added.

See the [form control guide](docs/form-controls.md) for state ownership, keyboard
contracts, and application-owned styling. Checkbox and Switch report the next
value through a callback; the application stores it and notifies its view.
RadioGroup retains selection and option focus in an entity, accepts fixed ordered
options with unique values, and changes selection immediately on arrow navigation.

The [navigation and overlay guide](docs/navigation-overlays.md) covers Tabs,
Accordion, Collapsible, Popover, DropdownMenu, Command, Tooltip, and AlertDialog. Inactive panels do
not contribute layout, focus, or accessibility nodes. Popover reuses the existing popup positioning
primitive and lets keyboard focus leave its contents within the enclosing window
or dialog. DropdownMenu builds on that lifecycle and closes before dispatching a
command, so a command can open another surface without losing its new focus.
The [layout and data guide](docs/layout-and-data.md) explains shared Table columns,
ScrollArea composition, and their current limits.

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
```

`compact` is the default when no appearance argument is supplied. Both commands
run the same session-settings interaction with application-owned styling.

The GPUI snapshot family is pinned to **`=0.3.11`**. Update those pins and
`Cargo.lock` together when changing the rendering dependency.

## Work on the source

| Location | Responsibility |
| --- | --- |
| `crates/base/src/` | The `gpui-design` library and its behavior-level tests |
| `crates/base/tests/` | Integration tests using the public API |
| `crates/base/examples/two_styles.rs` | Native example and application-owned styles |
| `docs/form-controls.md` | Form control usage and interaction contracts |
| `docs/navigation-overlays.md` | Tabs, Accordion, command menus, and composed focus behavior |
| `docs/layout-and-data.md` | Table structure, shared columns, and ScrollArea composition |
| `docs/component-coverage.md` | Current shadcn catalogue comparison, supported scope, and next priorities |
| `Cargo.toml` / `Cargo.lock` | Workspace and matching dependency set |

Initialize the library with `gpui_design::init(cx)` and host application content
in `gpui_design::Root`. Use the native example as the complete startup reference.
Consumer applications can work directly with this source and supply their own
presentation while reusing the behavior layer.

## Verify changes

Run the checks for the completed change set:

```sh
cargo fmt --all -- --check
cargo clippy -p gpui-design --all-targets --features test-support --locked -- -D warnings
cargo test -p gpui-design --features test-support --locked
cargo build -p gpui-design --example two_styles --locked
```

The [Verify workflow](https://github.com/andongmin94/gpui-design/actions/workflows/verify.yml)
uses one matrix for Linux, Windows, and macOS. Platform-independent checks run
once; each platform still compiles its native dependencies and example.

| Check | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Formatting and Clippy | Yes | — | — |
| Full behavior and integration suite | Yes | — | — |
| Public-API composition tests | Included above | Yes | Yes |
| Native example build | Yes | Yes | Yes |

Composition tests use representative flows across the two appearances instead of
duplicating every scenario for each palette. Add tests for distinct behavior and
observed regressions. Workflow run results are the source of automated status.

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
Accordion adds single-open state and header navigation; DropdownMenu composes
Popover with command dispatch and a keyboard/pointer cursor.
Slider adapts the upstream pointer interaction to a single linear value with
retained focus and keyboard adjustment. Progress retains the controlled numeric
semantics while the application supplies its track and fill.
Toggle delegates activation to Button, and ToggleGroup adds retained multiple
selection and roving focus to Toggle options.
Collapsible adds retained visibility and content-closure focus to the upstream
disclosure contract, reusing Button for its trigger. AlertDialog adapts the
upstream Dialog specialization to the existing retained DialogState and parts.
Command adds fixed-action search and activation over the existing Input engine.
Textarea exposes that engine's retained multiline state as a public control,
with form Tab traversal and disabled-input Tab skipping. Separator adds static
native semantics without imposing visual geometry.
Combobox extends shared `SearchItem` search and activation with editable drafts,
committed selection, and an anchored suggestion lifecycle. Field connects visible
labels and application validation feedback to existing text-input semantics;
it does not own a separate form state or validation engine. Tooltip uses GPUI's
native tooltip rendering with Button focus, hover timing, and dismissal behavior.

Software source is licensed under [Apache-2.0](LICENSE-APACHE). The original
Longbridge copyright and license notices are retained. Dependencies keep their
respective licenses.
