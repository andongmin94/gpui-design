# Presentation recipes

These four modules are reusable application source, used by the native
[two-style example](../crates/base/examples/two_styles.rs). They compose GPUI
layout with the existing controls and keep colors, spacing, and sizing in the
consumer's `Palette`.

## Copy into an application

1. Copy the desired files from
   [`crates/base/examples/recipes`](../crates/base/examples/recipes) into an
   application module, preserving the repository's license terms.
2. Declare the copied modules and re-export the types, following the small
   [`mod.rs`](../crates/base/examples/recipes/mod.rs). Copy only the modules needed
   by the application and list only those modules.
3. Change `use super::super::Palette` to the application's token type and adapt
   the field names or recipe styling. The example's `Palette` and its compact
   and settings values show every field used by these recipes.
4. Import the types from that application module. They use `pub(crate)` visibility
   and are not exported from `gpui_design`. No extra dependency is required.

The source in this repository is the distribution mechanism. There is no recipe
registry, installer, or separate theme provider. The existing crate continues to
own interactive behavior; the application owns the copied recipe source.

## Card

[`card.rs`](../crates/base/examples/recipes/card.rs) supplies a named region with a
level-two heading. The description accepts text; the header action, content,
and footer each accept one element, so the application can compose several
children in a `div()`.

```rust,ignore
Card::new("session-card", "Session", palette)
    .description("Recording preferences")
    .header_action(Badge::new("toolbar-status", "Toolbar pinned", palette).accent(true))
    .content(session_summary)
    .footer(session_controls)
    .w_full()
```

The card adds no focus stop. Controls placed in its slots retain their own
keyboard and pointer behavior. Its `Styled` implementation allows frame overrides;
change the copied source to fit different heading levels or header/footer layouts.

## Badge

[`badge.rs`](../crates/base/examples/recipes/badge.rs) is a static named label.
The default appearance is secondary; `.accent(true)` uses the palette's accent
colors. The example displays actual cursor and toolbar settings with these two
appearances.

```rust,ignore
Badge::new("cursor-status", "Cursor visible", palette).accent(true)
```

Badge adds no Tab stop, button action, pressed state, or live announcement. Its
text must explain the state independently of color. Interactive chips should use
an existing Button or Toggle with application styling.

## Empty

[`empty.rs`](../crates/base/examples/recipes/empty.rs) exports `EmptyState` to
avoid confusion with GPUI's `Empty` element. It supplies a named group, level-two
heading, description, and optional media/action slots.

```rust,ignore
EmptyState::new("empty-results", "No results", "Try another search.", palette)
    .media(Badge::new("result-count", "0 results", palette))
    .action(clear_search_button)
```

The recipe does not own search, loading, notification, or focus state. Supplied
media owns its accessible description when it conveys information. The example
places this recipe beside `CommandList`, inside `Command`, so its action is an
ordinary Button outside the listbox. Clear search calls `CommandState::set_query`
and returns focus to the existing query input. The dialog remains open and no
command is dispatched. `Styled` can override the outer frame.

## Button Group

[`button_group.rs`](../crates/base/examples/recipes/button_group.rs) gives related
Buttons one frame and decorative separators. It accepts existing `Button`
instances, applies connected styling, and preserves their identity, actions,
disabled state, and individual Tab stops.

```rust,ignore
ButtonGroup::new("calendar-month", "Calendar month", palette)
    .axis(Axis::Horizontal)
    .button(previous_month_button)
    .text("November 2026")
    .button(next_month_button)
    .w_full()
```

Horizontal is the default; `Axis::Vertical` stacks the same parts. `text` is a
static label with no focus stop. `Styled` allows frame and size overrides.
Focus uses an accent fill because the group owns the outer border. The group
adds no arrow-key navigation or pressed-value state; `ToggleGroup` remains the
control for multiple pressed values with roving focus.

## Verification scope

The existing composition suite runs the same recipe modules as the native
example. Its command-search flow covers empty-state recovery, and its date-picker
flow covers traversal and activation of the grouped month buttons. Static shapes
and palette values have no separate test suite. The existing three-platform CI
continues to check composition and native compilation; those results do not
establish native screen-reader, IME, visual, or performance validation.
