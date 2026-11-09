# Presentation recipes

These fifteen recipes are reusable application source, used by the native
[two-style example](../crates/base/examples/two_styles.rs). They compose GPUI
layout with the existing controls and keep colors, spacing, and sizing in the
consumer's `Palette` or application-supplied `Styled` values.

## Copy into an application

1. Copy the desired files from
   [`crates/base/examples/recipes`](../crates/base/examples/recipes) into an
   application module, preserving the repository's license terms.
2. Declare the copied modules and re-export the types, following the small
   [`mod.rs`](../crates/base/examples/recipes/mod.rs). Copy only the modules needed
   by the application and list only those modules.
3. For recipes that import `use super::super::Palette`, use the application's
   token type and adapt the field names or recipe styling. The example's
   `Palette` and its compact and settings values show the fields used by these
   recipes. AspectRatio takes no palette; its consumer supplies the visual style.
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

## Kbd

[`kbd.rs`](../crates/base/examples/recipes/kbd.rs) supplies static individual
keycaps and combinations of keys. Each Kbd uses its visible text as the default
native Label name. Give symbolic keys a readable name explicitly:

```rust,ignore
Kbd::new("guide-up-key", "↑", palette)
    .accessibility_label("Arrow up");

KbdGroup::new("context-menu-shortcut", palette)
    .key(Kbd::new("context-menu-shift", "Shift", palette))
    .key(Kbd::new("context-menu-f10", "F10", palette));
```

`KbdGroup` preserves the supplied order and inserts `+` between keycaps. Its
native Group has no repeated name; each key retains its own readable Label.
Use a group for keys pressed together, such as Shift+Tab. Arrange independent
Kbd elements in ordinary application layout when showing alternatives, such as
Enter or Space, or the arrow keys.

Both types implement `Styled`. The palette supplies the keycap's field color,
border, corner radius, foreground, and font size, plus muted text for the group
separator. Applications own key labels and any platform-specific glyph choice.
The recipe has no focus stop, action, keybinding registration, or shortcut
dispatch; it displays the bindings already provided by the associated control.

The example's Help → Keyboard guide shows Tab, Shift+Tab, arrows, Enter/Space,
and Escape alongside descriptions of their existing behavior. The summary
displays Shift+F10 for its existing ContextMenu. Replacing plain hint text with
these keycaps does not add keyboard commands or another focus target.

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

## Avatar

[`avatar.rs`](../crates/base/examples/recipes/avatar.rs) presents a circular image
with explicit alternative text and application-supplied fallback text:

```rust,ignore
Avatar::new("preset-avatar", "Balanced preset artwork", "BL", palette)
    .image(preset_artwork)
    .size(rems(2.5))
```

`image` accepts GPUI's existing `ImageSource` conversions. Omitting it shows the
fallback immediately; an image error also shows the fallback. While an image is
pending, GPUI's native 200 ms loading delay applies before showing the fallback.
GPUI owns decoding, loading, caching, and redraws. The recipe adds no separate
loading task, timer, cache, or retry policy.

The root supplies one native Image role and the given accessible name in every
state. The image and fallback add no separate name or focus stop. Supply a name
that describes the image's meaning, rather than relying on the visible initials.
The application chooses fallback text and any localization; this recipe does not
derive initials from names. The default frame is 2.5 rem square, with palette
colors and circular clipping. `Styled` can change its size and colors; keep width
and height equal for a circle. Images use GPUI's cover fit. Status badges, avatar
groups, and interactive actions remain application compositions.

The example embeds [`preset-avatar.svg`](../crates/base/examples/assets/preset-avatar.svg)
for the Balanced preset. Other presets use explicit text such as `PR` and `HQ`,
and an empty selection uses `?`, with an accessible name derived from the current
preset label. The source is selected while rendering the existing HoverCard from
the retained Combobox state; there is no second selected-preset value. This
bundled artwork requires no network request. Applications supply their own assets
when copying the recipe.

## Aspect Ratio

[`aspect_ratio.rs`](../crates/base/examples/recipes/aspect_ratio.rs) provides a
clipped media frame using GPUI's existing aspect-ratio layout:

```rust,ignore
AspectRatio::new("recording-frame", 16. / 9.)
    .w(px(320.))
    .rounded(palette.radius)
    .bg(palette.field)
    .child(media)
```

The ratio is width divided by height and must be finite and positive. The default
width fills its parent; give that parent a definite width or set `.w(...)` on the
frame. Leave height automatic so GPUI can derive it from the width. Explicit
height or min/max constraints follow GPUI's ordinary layout rules and can
override its preferred ratio.

Children live in an absolute container that fills the frame, so their intrinsic
sizes do not determine its height. Oversized content is clipped, including
pointer interaction outside the frame. Supply `.size_full()` on an image or other
child that should fill the container, and choose image object fit on that child.
The frame adds no accessible role, name, focus stop, or retained state. Media
children supply their own accessible descriptions and interaction. Background,
rounding, and other presentation belong to the application through `Styled`.

The preset HoverCard shows an illustrative recording frame containing the current
session name. It uses 4:3 for Presentation and 16:9 for other presets or an empty
selection. Changing the retained preset changes the ratio on the next render;
editing the session name changes the displayed text. The supplied child has the
native Image name describing the format. The preview creates no recording and
does not add controls to HoverCard's noninteractive content.

## Sheet

[`sheet.rs`](../crates/base/examples/recipes/sheet.rs) presents existing Dialog
content in a full-height panel at the right edge of the window:

```rust,ignore
Sheet::new("session-settings", &dialog, &fields_scroll, "Session settings", palette)
    .description("Choose the session's drawing and toolbar settings.")
    .child(form_fields)
    .footer(
        DialogClose::new(&dialog)
            .trigger(|button| button.child("Done")),
    )
```

Keep the Sheet mounted under `gpui_design::Root` while closed. The application
owns and retains both the supplied `Entity<DialogState>` and `ScrollHandle`.
Configure initial focus on that existing DialogState and use its `open` and
`close` methods. Dialog continues to own modal focus containment, Escape and
backdrop dismissal, and return focus. The Sheet adds no open state or selection
model. Its title and optional description provide both the visible header and
the underlying Dialog's accessible name and description.

The preferred width is `palette.dialog_width`, limited to the viewport width.
The default popup fills the viewport height, has square corners, and uses one
border on its left edge. The header and optional footer stay outside a single
vertical ScrollArea body. Each `.child(...)` goes into that body, whose padding,
spacing, and scrollbar colors come from the consumer palette. The body takes
the remaining height and uses the application's retained scroll handle. On
reopening, native reveal of the initial focus target can adjust its offset.
The consumer must keep the header and footer's combined content within the
viewport height. Long form content belongs in the scrolling body.

`Styled` customizes the popup surface. Use `.w(...)` to change its preferred width
and retain the default maximum width, height, and flex layout to preserve the
Sheet's sizing contract. This recipe covers right-edge presentation; other edges,
drag gestures, and transition animation are outside its current scope. It does
not implement Drawer interaction.

The application supplies footer controls, including any `DialogClose` and its
disabled state. Keyboard interaction stays with the supplied controls and
Dialog's existing Tab containment. ScrollArea reveals focused descendants on
its enabled axis and provides native pointer scrolling; it adds no independent
scroll focus target or scroll-key bindings.

Both appearances now use Sheet for the existing Session settings form. The
example reuses its DialogState, initial name-input focus, body scroll handle,
Preferences/Help/Schedule/Presets tabs, and Done validation rule. Closing and
reopening keeps the same input, selection, and table state. The existing Edit
triggers and menu/command actions open that state; the recipe adds no separate
selection. Avatar and AspectRatio still render the retained preset in its
HoverCard preview. Search commands and Reset preview keep their centered Dialog
and AlertDialog presentations.
The example caps its deferred Select, Combobox, Calendar, and Help popup widths
against the window as well; those surfaces do not inherit the Sheet's width cap.

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

The existing Help flow checks Kbd's readable symbolic key name and absence of
new focus stops, alongside the composed Breadcrumb ancestor actions. The
context-menu flow exercises the existing shortcut displayed by its keycaps.
There is no separate keybinding test controller,
palette suite, or CI job for these static key hints. Native screen-reader speech
and desktop visual rendering remain separate validation work.

The media recipes add two focused regressions within the same composition target.
Avatar exercises absent, pending, failed, loaded, and cleared image sources while
checking its native name, focus neutrality, and fixed geometry; the loaded path
decodes the bundled SVG. AspectRatio exercises two container widths, oversized
content, and pointer clipping beyond the frame. The existing preset flow checks
that the retained choice drives the preview's artwork, frame ratio, and session
name while preserving the existing trigger and dismissal behavior. These checks
do not establish remote image transport, native screen-reader speech, desktop
visual rendering, or performance measurements.

Sheet coverage exercises edge placement, normal and narrow viewport widths,
fixed header/footer geometry, body scrolling, descendant focus reveal, and
closing and reopening with retained form state. The checks also exercise the
shared Dialog's backdrop wheel handling to keep underlying content from scrolling,
and opens Select in a narrow window to check the child popup's horizontal bounds.
Nested-modal checks cover focus containment and wheel isolation in the top dialog,
closing it while the Sheet remains open, and
returning through the Sheet to its original trigger. These are layout and
interaction checks in the existing GPUI test contexts; their scope does not
establish native screen-reader, platform IME, or desktop visual validation.
