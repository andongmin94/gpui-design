# Layout and data

Table, ScrollArea, and Resizable provide reusable structure and native behavior. Applications
own their data, layout dimensions, colors, typography, and cell contents. The
[two-style example](../crates/base/examples/two_styles.rs) combines them in its
Schedule tab and uses ScrollArea for the surrounding form.

## Table

A Table defines columns once and applies them to its header, body, and footer.
Each row must provide exactly one cell per column. Use fixed widths for stable
tracks and positive flexible weights to divide the remaining width:

```rust
Table::new(
    "sessions",
    [TableColumn::fixed(px(180.)), TableColumn::flex(1.)],
)
.accessibility_label("Sessions")
.w_full()
.header(
    TableRow::new("sessions-header")
        .cell(TableCell::new("session-heading").p_2().child("Session"))
        .cell(TableCell::new("date-heading").p_2().child("Date")),
)
.row(
    TableRow::new("session-1")
        .cell(TableCell::new("session-1-name").row_header(true).p_2().child("Workshop"))
        .cell(TableCell::new("session-1-date").p_2().child("2026-11-09")),
)
.caption(TableCaption::new("sessions-caption").child("Scheduled sessions"))
```

`header` and `footer` accept one row each. Append body rows through `row` or
`rows`; `TableRow::cells` also accepts an iterator. An empty body is valid, so an
application can render its empty-state explanation alongside the table.

Every part implements `Styled`. Cells and captions accept arbitrary children;
an application can put an existing Button or Checkbox in a cell. Structural
table parts add no Tab stops and do not intercept those controls' activation.
Table owns the row's horizontal layout. Apply spacing inside cells rather than
adding horizontal row padding, margins, or gaps. Cell styles apply to an inner
content surface, preserving the shared outer column width even when header and
body padding differ.

| Part | Native semantics |
| --- | --- |
| Table | Table, row count, column count, and an explicit accessible label |
| Header/body/footer | RowGroup |
| Row | Row and its zero-based index |
| Header cell | ColumnHeader with row/column indices |
| Ordinary cell | Cell with row/column indices |
| Cell with `row_header(true)` | RowHeader, unless it belongs to the column-header row |
| Caption | Caption without a focus action |

Indices include the header and footer. A visible caption does not automatically
name the table; supply `accessibility_label` separately. Cells also accept an
explicit accessible label when their visible content does not provide one.

This is an ordinary table, without cell navigation or a data controller. Row or
column spans, automatic intrinsic column sizing, sorting, filtering, pagination,
row selection, and virtualization are not supplied. Data operations should be
composed over this structure when a real application needs them.

The copyable [DataTable recipe](presentation-recipes.md#data-table) adds text
filtering, single-column sorting, pagination, empty-state recovery, and stable-ID
row actions. The native example uses it to browse and apply recording presets;
its query and page state belong to the application, outside Table.

## ScrollArea

Retain a GPUI ScrollHandle in the view and give the viewport a bounded size on
its scrolling axes. The same handle drives native wheel/trackpad scrolling and
the existing overlay scrollbar:

```rust
// Store once in the view, not on each render.
let scroll = gpui::ScrollHandle::new();

ScrollArea::new("session-list", &scroll)
    .accessibility_label("Sessions")
    .h(px(240.))
    .w_full()
    .content_styles(|style| style.flex().flex_col())
    .children(rows)
```

The default axis is vertical. Use `.axis(ScrollbarAxis::Horizontal)` or `Both`
for other content. Give overflowing content a minimum/fixed extent or prevent
it from shrinking when appropriate. Ordinary GPUI styles size and decorate the
outer frame. Use `content_styles` for the viewport's child layout, spacing, and
padding. The viewport and overlay scrollbar use GPUI's normal rendering so
ancestor visibility and opacity apply to both.

`scrollbar_mode` accepts the existing Scrolling, Hover, or Always modes.
`scrollbar_styles` uses the existing typed track/thumb styles. The bar overlays
content and reserves no gutter, so leave application padding if its content
should not overlap the bar. For example, the plan table uses bottom/right
padding and always-visible scrollbars for its resizable viewport.

The public frame exposes the native ScrollView role and accepts an accessible label.
It adds no independent focus stop or key bindings: input arrows keep editing
text, and Calendar arrows keep navigating dates. Plain static content can be
scrolled with the pointer or the retained handle; a separate keyboard scrolling
control is not part of this contract.

Descendants can use GPUI's native `request_autoscroll` during prepaint to request
reveal. Input, Textarea, Calendar, and newly focused Button controls issue requests
when needed.
ScrollArea clamps the offset on its enabled axes and requests a new frame when
the viewport needs to move. A nested area's adjusted request continues to its
ancestors, and a sibling's pending request is preserved rather than used to
scroll the wrong viewport. Offsets on disabled axes are cleared.

ScrollArea reuses GPUI's ordinary scrolling and the retained Scrollbar pointer
implementation. It does not virtualize children, add elastic scrolling, or
anchor streaming/transcript content.

## A table inside a scroll area

Set the table's width or minimum width explicitly when columns should exceed
the viewport. The native example uses three fixed columns totaling 490 pixels
inside a ScrollArea supporting both axes. Its surrounding settings form scrolls vertically.
The nested table can move horizontally without altering the form's date or
scrolling unrelated siblings. Date selection and name/preset edits update the
table from the same retained state used by the other controls.

## Resizable

Resizable divides a bounded frame into two continuously mounted panes. Its
state owns the first pane's ratio, resize limits, and one divider focus handle.
The remaining share belongs to the second pane:

```rust
let split = cx.new(|cx| {
    ResizableState::new(Axis::Vertical, cx)
        .limits(0.3..=0.7)
        .default_ratio(0.5)
});

Resizable::new("notes-and-preview", &split)
    .w_full()
    .h(px(280.))
    .first(notes)
    .second(preview)
    .handle(|handle| {
        handle
            .accessibility_label("Space for session notes")
            .h(px(10.))
            .bg(divider_color)
    })
```

`Axis::Horizontal` places panes left/right and uses the handle's width.
`Axis::Vertical` places them top/bottom and uses its height. Set an actual handle
thickness and bound the frame's dimensions. The ratio divides the usable pane
space after that thickness, so a 50/50 split shares the available space equally
and stays proportional when the parent resizes. Frame padding is supported;
the internal layout has no gap, and main-axis divider margins are reserved.
Put spacing inside the application's pane contents.

The default ratio is 0.5 with limits 0.1–0.9. `limits` accepts finite fractions
with `0 < minimum < maximum < 1`, and `default_ratio` clamps the initial value.
For example, limits 0.3–0.7 guarantee both panes retain at least 30% of the usable
space. They are proportions, not pixel minimums. Panes clip overflow; use
ScrollArea inside a pane when its content should scroll. Nest independent
two-pane groups for more complex layouts.

| Input | Result |
| --- | --- |
| Drag the divider | Resize from the actual press position, bounded by the ratio limits |
| Left / Right in a horizontal group | Decrease / increase the first pane by two percentage points |
| Up / Down in a vertical group | Decrease / increase the first pane by two percentage points |
| Home / End | Reach the minimum / maximum first-pane ratio |
| Tab / Shift+Tab | Traverse the divider and pane controls in ordinary document order |

Only the divider owns the resize key context. Text editing, selection, and
scrolling inside panes retain their own behavior. Pointer dragging uses GPUI's
existing drag lifecycle and identifies its owning state, so independent or
nested groups do not react to another group's drag.

The divider has a native Splitter role with numeric value, limits, and step in
percent. Native orientation describes the divider line: a left/right layout has
a vertical divider, while a top/bottom layout has a horizontal divider. Give it
a label identifying the pane it adjusts and keep its grip children non-interactive.
The application supplies hover/focus/disabled styling and all pane content.

Observe the state to update presentation. `ratio()`, `min_ratio()`, `max_ratio()`,
`axis()`, `focus_handle()`, and `is_disabled()` expose its contract.
`set_ratio(ratio, window, cx)` clamps a finite value and emits
`ResizableEvent::Change` only when the ratio changes, including programmatic
updates. `set_disabled` blocks pointer, keyboard, and accessibility resizing and
normal Tab entry. A divider already focused retains its binding so Tab can leave;
application updates through `set_ratio` remain available.

The Schedule example reuses the same notes state as Preferences in the first
pane and puts its plan table in a two-axis ScrollArea in the second. Closing or
switching tabs preserves text and ratio because the application retains their
states. The component does not implement zero-size collapse, pixel constraints,
an arbitrary multi-pane solver, or automatic storage/persistence.

## Verification boundary

Focused tests cover column alignment, table metadata, interactive cell children,
native scrolling and reveal, divider dragging/keyboard behavior, and the
DatePicker-to-resizable-plan example flow. Existing
CI also builds the native example on Linux, Windows, and macOS. Automated GPUI
event/layout checks do not establish native visual, screen-reader, mixed-DPI, or
performance results.
