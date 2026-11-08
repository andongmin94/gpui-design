# GPUI Design

A source-owned, headless UI foundation for Rust and GPUI.

The library owns control behavior, state, focus, and accessibility semantics.
Applications own their component source, layout, colors, typography, and assets.
Headless separates control behavior from application-owned presentation; the
examples render native GPUI windows.

This is an early foundation for an independent component ecosystem. It currently
includes eleven controls and an executable example that gives the same behavior
two visual treatments. The source is maintained in this repository, with GPUI
kept as an exactly pinned rendering dependency.

## Current scope

| Control | Behavior supplied by the library | Application-owned presentation |
| --- | --- | --- |
| **Button** | Pointer and keyboard activation, disabled state, focus, accessible name and role | Content, spacing, shape, and state styling |
| **Checkbox** | Controlled unchecked/checked/mixed state, activation, disabled state, and focus | Indicator, label, layout, and state styling |
| **Switch** | Controlled on/off state, activation, disabled state, and focus | Track, thumb, label, and state styling |
| **RadioGroup** | Single selection, one Tab stop, arrow navigation, disabled options, and focus | Group layout and option rendering |
| **Input** | Text editing state, selection, clipboard, focus, and the native text-input interface | Field container, text style, caret, and surrounding controls |
| **Select** | Single selection from fixed options, keyboard navigation, disabled options, commit, and focus | Trigger, popup content, and option rendering |
| **Tabs** | Active panel, one tab-list stop, automatic activation, orientation-aware keys, and disabled tabs | Tab-list layout, tab styling, and panel contents |
| **Accordion** | One collapsible section, header navigation, disabled items, and focused-panel closure | Header, indicator, panel contents, and layout |
| **Popover** | Non-modal open/close state, anchored placement, focus, outside dismissal, and Escape | Trigger, popup contents, and surface styling |
| **DropdownMenu** | Command activation, enabled-item navigation, single-character search, dismissal, and focus | Trigger, menu surface, and command rendering |
| **Dialog** | Open/close state, modal focus, initial focus, dismissal, and focus restoration | Backdrop, popup, title, description, and close control |

Shared support includes the window root, popup positioning, input infrastructure,
and opt-in test observations. Internal editing machinery retained for Input is
not a separate editor product. Public APIs are still being developed.

Input provides a small, readable baseline for unset text, caret, and selection
colors. Explicit `InputEditorStyle` values take precedence. Applications own the
controls' frame borders, radii, layout, design palette, and sizing scales.

Select accepts a fixed, ordered collection when `SelectState` is created. Each
option has a unique value; the state owns the selected value and keyboard cursor.

The [two-style example](crates/base/examples/two_styles.rs) applies a compact dark
presentation and a light settings presentation to the same controls. Its styles
live in application code, outside the behavior layer. A Session actions menu opens
the settings dialog or toggles application values. The dialog combines Preferences
and Help tabs, and retains values when it is closed and reopened. Help uses an
Accordion whose keyboard section opens a guide in a non-modal Popover.

See the [form control guide](docs/form-controls.md) for state ownership, keyboard
contracts, and application-owned styling. Checkbox and Switch report the next
value through a callback; the application stores it and notifies its view.
RadioGroup retains selection and option focus in an entity, accepts fixed ordered
options with unique values, and changes selection immediately on arrow navigation.

The [navigation and overlay guide](docs/navigation-overlays.md) covers Tabs,
Accordion, Popover, and DropdownMenu. Inactive panels do not contribute layout,
focus, or accessibility nodes. Popover reuses the existing popup positioning
primitive and lets keyboard focus leave its contents within the enclosing window
or dialog. DropdownMenu builds on that lifecycle and closes before dispatching a
command, so a command can open another surface without losing its new focus.

## Run the example

Install [Rust with rustup](https://rustup.rs/). The repository selects **Rust
1.99.1**, including rustfmt and Clippy, through `rust-toolchain.toml`.

On **Windows**, use the MSVC toolchain with the Visual Studio C++ build tools and
a Windows SDK. On **Ubuntu 24.04**, install the native build dependencies:

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
is configured for Windows and Ubuntu. It runs formatting, Clippy, behavior and
headless interaction tests, and a native example build. Its run results are the
source of automated verification status. macOS is not part of this initial CI
matrix and has not been validated for this extraction.

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

Software source is licensed under [Apache-2.0](LICENSE-APACHE). The original
Longbridge copyright and license notices are retained. Dependencies keep their
respective licenses.
