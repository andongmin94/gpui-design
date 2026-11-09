# Layout and data

Table and ScrollArea provide reusable structure and native behavior. Applications
own their data, layout dimensions, colors, typography, and cell contents. The
[two-style example](../crates/base/examples/two_styles.rs) combines both in its
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
should not overlap the bar. For example, the horizontal plan table uses bottom
padding and an always-visible scrollbar.

The public frame exposes the native ScrollView role and accepts an accessible label.
It adds no independent focus stop or key bindings: input arrows keep editing
text, and Calendar arrows keep navigating dates. Plain static content can be
scrolled with the pointer or the retained handle; a separate keyboard scrolling
control is not part of this contract.

Descendants can use GPUI's native `request_autoscroll` during prepaint to request
reveal. Input, Textarea, and Calendar already issue requests when needed.
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
inside a horizontal ScrollArea. Its surrounding settings form scrolls vertically.
The nested table can move horizontally without altering the form's date or
scrolling unrelated siblings. Date selection and name/preset edits update the
table from the same retained state used by the other controls.

## Verification boundary

Focused tests cover column alignment, table metadata, interactive cell children,
native scrolling and reveal, and the calendar-to-table example flow. Existing
CI also builds the native example on Linux, Windows, and macOS. Automated GPUI
event/layout checks do not establish native visual, screen-reader, mixed-DPI, or
performance results.
