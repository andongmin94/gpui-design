# Presentation recipes

These eleven recipes are reusable application source, used by the native
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

Some recipes compose other recipe modules. Copy those modules and their re-exports
alongside the desired recipe:

| Recipe | Other recipe modules required |
| --- | --- |
| Pagination | ButtonGroup |
| DataTable | InputGroup, EmptyState, Pagination, and ButtonGroup |

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

## Pagination

[`pagination.rs`](../crates/base/examples/recipes/pagination.rs) composes existing
Button and ButtonGroup controls into named native navigation. The application
supplies the zero-based page index, page count, and callback.

```rust,ignore
Pagination::new("preset-pages", "Recording preset pages", page, page_count, palette)
    .on_change(move |page, _, cx| {
        table_state.update(cx, |state, cx| state.set_page(page, cx));
    })
```

For a nonempty collection, the page index must be less than the count. Empty
results use index zero and count zero, display `No pages`, and disable both
buttons. Otherwise the text is `Page N of M`, Previous is disabled on the first
page, and Next is disabled on the last. The buttons preserve ordinary Tab and
activation behavior, including focus on a button that becomes disabled after its
action. The recipe requests a page change; it owns no retained page or data.
`Styled` controls the frame. Numbered links, ellipsis, routing, and fetching are
outside this recipe's scope.

## Data Table

[`data_table.rs`](../crates/base/examples/recipes/data_table.rs) provides a working
client-side text-table composition. It uses the existing native Input/Field,
Table, ScrollArea, and Button, plus the InputGroup, EmptyState, and Pagination
recipes. Columns and immutable rows belong to the consuming application.

```rust,ignore
let state = cx.new(|cx| {
    DataTableState::new(
        [
            DataTableColumn::new("Preset", px(180.)),
            DataTableColumn::new("Description", px(280.)),
        ],
        rows, // DataTableRow::new(stable_id, [name, description]).disabled(false)
        2,    // Rows per page; must be positive.
        window,
        cx,
    )
});

DataTable::new("preset-catalogue", &state, palette)
    .accessibility_label("Recording presets")
    .action_label("Use preset")
    .selected_value(current_preset)
    .on_action(move |id, window, cx| {
        preset.update(cx, |state, cx| {
            state.set_selected_value(Some(id.as_ref()), window, cx);
        });
    })
```

Keep the state entity in the application and observe it to redraw the parent.
Apply the application's `InputEditorStyle` to `query_input()`, as the two-style
example does. A row has a unique stable ID and one text value per column. The
initial display order is the supplied row order. Header Buttons toggle one column
between ascending and descending case-insensitive text order; equal keys retain
their original order.

The native input owns the query. Every whitespace-separated term must appear in
the combined cell text, ignoring case. Matching/sorted indices are derived when
the query or sort changes. Both changes return to the first page; moving focus
does not reset it. `visible_rows()`, `filtered_count()`, `page_index()`, and
`page_count()` expose the current view. `set_page` clamps to the available range,
and page/filter/sort changes return the row viewport to its vertical start.

The example derives its four rows from the same preset definitions used by the
Combobox. `.selected_value` supplies that Combobox's current value each render;
the table keeps no second selected value. A row action passes the stable ID to
the application even after filtering, sorting, or paging. Current and unavailable
rows have disabled action Buttons. The selected value remains in the application
when its row is filtered out or appears on another page.

An empty result replaces the row viewport with EmptyState and Clear filter.
Clearing restores the rows and query focus without applying a row action. With
results, focus follows query → sort Buttons → enabled row actions → enabled page
Buttons. Horizontal and vertical ScrollArea scrolling reveals focused controls;
there is no grid-style arrow navigation. Query, sort, and page survive unmounting
the view while its state entity is retained.

Native Table indices describe the displayed page. Header Button names announce
the current sorting description and next action; native sort-direction metadata
is not supplied by the pinned Table/GPUI API. The columns are fixed-width text,
with one application action column. Numeric/date ordering, custom cell renderers,
multi-column sorting, multiple row selection, column hiding/reordering, mutable
row sources, remote data, and virtualization are outside the current scope.
Adapt the copied source when an application needs different data operations.

## Spinner

[`spinner.rs`](../crates/base/examples/recipes/spinner.rs) adds a rotating arc to
the existing indeterminate Progress. Supply the operation's accessible name:

```rust,ignore
Spinner::new("preview-progress", "Preview progress", palette)
    .size(rems(1.25))
    .text_color(palette.accent)
```

The default size is one rem and the default color is the consumer's accent.
`Styled` controls the root; the arc follows its actual bounds and text color.
Progress supplies the sole native ProgressIndicator role, label, and numeric
range, with no current numeric value. The visual arc adds no separate role,
accessible name, or Tab stop. This is an indeterminate progress indicator; it
does not provide an automatic live announcement.

GPUI's `AnimationExt` owns the one-second repeating animation. It renders a
static arc without requesting animation frames when `App::reduce_motion()` is
enabled. Applications can set that existing preference with
`App::set_reduce_motion(bool)`; automatic synchronization with an operating
system preference is outside this recipe. There is no recipe-specific animation
controller, retained loading state, or timer.

Mount Spinner while the application considers an operation pending and remove
it when that operation finishes. In the example, the existing Unknown progress
toggle replaces the determinate bar with Spinner and Skeleton in the same
bounded row. Both progress presentations keep the `preview-progress` ID and
accessible name. The Slider retains its known value while the unknown state is
shown, and Reset preview returns to known progress at 40%. This remains a
controlled preview; the recipe does not start a recording or a background task.

## Skeleton

[`skeleton.rs`](../crates/base/examples/recipes/skeleton.rs) is a static decorative
placeholder. The application supplies its dimensions and decides when to replace
it with ready content:

```rust,ignore
Skeleton::new("preview-placeholder", palette)
    .w(px(180.))
    .h(px(16.))
    .rounded(px(4.))
```

The default is a full-width line as high as the consumer's font size, using its
border color, corner radius, and 0.35 opacity. Ordinary `Styled` refinements can
make text lines, blocks, or circular placeholders. The surface owns no timer or
animation, so it remains still under every motion preference.

Skeleton adds no native role, accessible name, children, or focus stop. Name the
loading operation separately, as the example does with Spinner. It owns no
loading state or content measurement; a collection of placeholders is ordinary
application layout. The example shows one where the percentage fill is not yet
available and removes it when known progress is restored.

## Verification scope

The existing composition suite runs the same recipe modules as the native
example. Its command-search flow covers empty-state recovery, and its date-picker
flow covers traversal and activation of the grouped month buttons. Static shapes
and palette values have no separate test suite. The existing field/preset flow
covers missing-name recovery, the disabled addon for a read-only input, urgent
inline semantics, and the Item's retained preview/edit behavior. The date-picker
flow also checks that the ordinary schedule notice remains a Note. The existing
three-platform CI continues to check composition and native compilation. One
additional composed flow covers the preset table's sorting, filter/empty recovery,
page boundaries, stable-ID action, and retained choice. There is no separate data
test framework or duplicated platform-specific flow. These results do not
establish native screen-reader, IME, visual, or performance validation.

The existing preview flow also covers named Spinner rendering, a decorative
non-focusable Skeleton, GPUI reduced-motion rendering, reset cancellation,
restoring the percentage fill, and removing the indicators when their disclosure
closes. The loading recipes add no test function or CI job; animation scheduling
remains GPUI's responsibility rather than a second test controller here.
