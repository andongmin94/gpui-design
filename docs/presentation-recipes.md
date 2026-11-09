# Presentation recipes

These seven modules are reusable application source, used by the native
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

## Input Group

[`input_group.rs`](../crates/base/examples/recipes/input_group.rs) supplies one
frame around an existing control and optional leading/trailing elements. The
constructor takes the control itself, so its native input renderer, value,
selection, and input interface stay with the existing editor.

```rust,ignore
let control = InputBase::new("session-name")
    .role(None::<Role>)
    .semantic_owner::<Input>(input.entity_id())
    .h(palette.control_height)
    .w_full()
    .child(Input::new(&input));

InputGroup::new("session-name-group", control, palette)
    .focused(input.focus_handle(cx).is_focused(window))
    .invalid(name_invalid)
    .trailing(default_name_button)
```

Place the group inside an existing `Field` to preserve the input's label, help,
required state, and error metadata. Keep the native editor directly below its
`InputBase` semantic owner; the group surrounds that complete control. Remove the
control's old outer border and padding when the group supplies them. `CommandInput`
can likewise be supplied directly, preserving its search and input semantics.

Horizontal is the default. `.leading(element)` and `.trailing(element)` appear
before and after the control. `.axis(Axis::Vertical)` places them above and below
it; the example uses a trailing keyboard hint below the existing Textarea.
Each slot accepts one element, which can be an application-owned container.
Traversal follows the supplied leading/control/trailing order; put action buttons
in the trailing slot when the input should precede them in the Tab sequence.

The frame adds no Tab stop or click-to-focus handler. Supplied buttons retain
their focus and callbacks. `focused` and `invalid` style the border only, with
invalid taking precedence; native invalid metadata remains the Field's job.
The application chooses which addons are enabled. In the example, Use default
appears only for a missing session name and is disabled when the input is not
editable. Its existing input state receives `MiniCast` and regains focus when the
action runs. The recipe does not create a second text value or infer mutation
permissions from its child elements. `Styled` can override the frame.

## Item

[`item.rs`](../crates/base/examples/recipes/item.rs) groups a title, optional
description, and optional trailing action in a content row.

```rust,ignore
Item::new("recording-preset-item", "Recording preset", palette)
    .description(selected_preset_label)
    .action(preset_preview_and_edit)
```

Its native Group is named by the title. The text column can shrink while the
action keeps its required width. The example supplies the existing HoverCard
trigger, which previews the preset on hover/focus and opens settings on ordinary
activation. Its focus handle, button name, and return-focus behavior are retained.

The row has no whole-row click, Tab stop, selected value, or list navigation.
Supplied controls own their semantics and actions. Use Field for associated input
labels and validation. Media, header/footer slots, and a separate ItemGroup are
outside the current recipe's scope. `Styled` can change the frame and spacing.

## Alert

[`alert.rs`](../crates/base/examples/recipes/alert.rs) presents an inline title and
description. Ordinary content uses the native `Note` role; `.urgent(true)` selects
the native `Alert` role. `.destructive(true)` changes colors independently of that
urgency decision.

```rust,ignore
Alert::new(
    "session-name-alert",
    "Session name is missing",
    "Open Edit session and enter a name or use the default name.",
    palette,
)
.destructive(true)
.urgent(true)
```

The Schedule panel uses the default informational form. Closing settings with a
missing name shows the urgent form on the summary. Reopening settings removes it
while Field supplies the input's validation details, avoiding two visible copies
of the same error. Once the name is restored, the summary warning is absent.

Alert does not move focus, open a modal, own dismissal state, or schedule repeated
announcements. The native accessibility adapter determines how its role is
presented; exposing that role is not evidence of native screen-reader delivery.
Use AlertDialog for a modal decision that requires explicit confirmation. The
current inline recipe has no action or media slot; `Styled` controls its frame.

## Verification scope

The existing composition suite runs the same recipe modules as the native
example. Its command-search flow covers empty-state recovery, and its date-picker
flow covers traversal and activation of the grouped month buttons. Static shapes
and palette values have no separate test suite. The existing field/preset flow
covers missing-name recovery, the disabled addon for a read-only input, urgent
inline semantics, and the Item's retained preview/edit behavior. The date-picker
flow also checks that the ordinary schedule notice remains a Note. The existing
three-platform CI continues to check composition and native compilation; those
results do not establish native screen-reader, IME, visual, or performance
validation.
