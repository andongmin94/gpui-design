//! Application-owned text-table operations over shared native controls.

use std::{cmp::Reverse, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Div, ElementId, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, RenderOnce, Role, ScrollHandle, SharedString,
    Stateful, StyleRefinement, Styled, Subscription, Window, div, point,
    prelude::FluentBuilder as _, px,
};
use gpui_design::{
    Button, Field, Input, InputBase, InputState, ScrollArea, ScrollbarAxis, Table, TableCaption,
    TableCell, TableColumn, TableRow,
};

use super::super::Palette;
use super::{EmptyState, InputGroup, Pagination};

/// One fixed-width text column. Values are sorted as case-insensitive text.
pub(crate) struct DataTableColumn {
    label: SharedString,
    width: Pixels,
}

impl DataTableColumn {
    pub(crate) fn new(label: impl Into<SharedString>, width: Pixels) -> Self {
        assert!(f32::from(width).is_finite() && width > px(0.));
        Self {
            label: label.into(),
            width,
        }
    }
}

/// An immutable row with a stable application ID and one text value per column.
pub(crate) struct DataTableRow {
    id: SharedString,
    cells: Vec<SharedString>,
    disabled: bool,
}

impl DataTableRow {
    pub(crate) fn new(
        id: impl Into<SharedString>,
        cells: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            cells: cells.into_iter().map(Into::into).collect(),
            disabled: false,
        }
    }

    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(crate) fn id(&self) -> &SharedString {
        &self.id
    }

    pub(crate) fn cells(&self) -> &[SharedString] {
        &self.cells
    }
}

/// Retained client-side filtering, one-column text sorting, and pagination.
///
/// The native InputState is the query's source of truth. Matching indices are
/// derived only when query text or sort order changes. Each whitespace-separated
/// query term must occur somewhere in the row, ignoring case. Sorting preserves
/// source order for equal keys. Rows stay immutable and selection belongs to the
/// consuming application. Observe this entity when rendering it from a parent.
pub(crate) struct DataTableState {
    columns: Vec<DataTableColumn>,
    rows: Vec<DataTableRow>,
    input: Entity<InputState>,
    // The last text processed by the derived filter, not a second query value.
    filtered_query: SharedString,
    filtered: Vec<usize>,
    sort_column: Option<usize>,
    descending: bool,
    page: usize,
    page_size: usize,
    scroll: ScrollHandle,
    _input_observation: Subscription,
}

impl DataTableState {
    pub(crate) fn new(
        columns: impl IntoIterator<Item = DataTableColumn>,
        rows: impl IntoIterator<Item = DataTableRow>,
        page_size: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let columns: Vec<_> = columns.into_iter().collect();
        let rows: Vec<_> = rows.into_iter().collect();
        assert!(!columns.is_empty(), "a data table needs text columns");
        assert!(page_size > 0, "page size must be positive");
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(
                row.cells.len(),
                columns.len(),
                "each row needs one value per column"
            );
            assert!(
                rows[..index].iter().all(|other| other.id != row.id),
                "row IDs must be unique"
            );
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search all columns"));
        let input_observation = cx.observe(&input, |state, _, cx| {
            state.sync_query(cx);
            // Input focus and editing presentation also affect the frame.
            cx.notify();
        });
        Self {
            filtered: (0..rows.len()).collect(),
            columns,
            rows,
            input,
            filtered_query: SharedString::default(),
            sort_column: None,
            descending: false,
            page: 0,
            page_size,
            scroll: ScrollHandle::new(),
            _input_observation: input_observation,
        }
    }

    pub(crate) fn query_input(&self) -> &Entity<InputState> {
        &self.input
    }

    pub(crate) fn page_index(&self) -> usize {
        self.page
    }

    pub(crate) fn page_count(&self) -> usize {
        self.filtered.len().div_ceil(self.page_size)
    }

    pub(crate) fn filtered_count(&self) -> usize {
        self.filtered.len()
    }

    pub(crate) fn visible_rows(&self) -> impl Iterator<Item = &DataTableRow> {
        self.filtered
            .iter()
            .skip(self.page * self.page_size)
            .take(self.page_size)
            .map(|&index| &self.rows[index])
    }

    pub(crate) fn set_page(&mut self, page: usize, cx: &mut Context<Self>) {
        let page = page.min(self.page_count().saturating_sub(1));
        if page != self.page {
            self.page = page;
            self.reset_vertical_scroll();
            cx.notify();
        }
    }

    fn sync_query(&mut self, cx: &Context<Self>) {
        let query = self.input.read(cx).value();
        if query != self.filtered_query {
            self.filtered_query = query;
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        let terms: Vec<_> = self
            .filtered_query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        self.filtered = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let text = row
                    .cells
                    .iter()
                    .map(|cell| cell.as_ref())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                terms
                    .iter()
                    .all(|term| text.contains(term))
                    .then_some(index)
            })
            .collect();
        if let Some(column) = self.sort_column {
            if self.descending {
                self.filtered.sort_by_cached_key(|&index| {
                    Reverse(self.rows[index].cells[column].to_lowercase())
                });
            } else {
                self.filtered
                    .sort_by_cached_key(|&index| self.rows[index].cells[column].to_lowercase());
            }
        }
        self.page = 0;
        self.reset_vertical_scroll();
    }

    fn reset_vertical_scroll(&self) {
        self.scroll
            .set_offset(point(self.scroll.offset().x, px(0.)));
    }

    fn sort_by_column(&mut self, column: usize, cx: &mut Context<Self>) {
        self.descending = self.sort_column == Some(column) && !self.descending;
        self.sort_column = Some(column);
        self.rebuild();
        cx.notify();
    }

    fn clear_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        self.sync_query(cx);
        cx.notify();
    }
}

type RowAction = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;

/// A copyable text table with an application-controlled current row and action.
///
/// This composes Field/Input, Table, ScrollArea, EmptyState, and Pagination.
/// Header and row buttons keep ordinary Tab order; there is no grid focus or
/// row-selection controller. Native table indices describe the displayed page.
/// Sorting direction is exposed by the header button's accessible name.
#[derive(IntoElement)]
pub(crate) struct DataTable {
    id: ElementId,
    base: Stateful<Div>,
    state: Entity<DataTableState>,
    palette: Palette,
    label: SharedString,
    action_label: SharedString,
    selected: Option<SharedString>,
    on_action: Option<RowAction>,
}

impl DataTable {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        state: &Entity<DataTableState>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(palette.gap),
            id,
            state: state.clone(),
            palette,
            label: "Data table".into(),
            action_label: "Choose".into(),
            selected: None,
            on_action: None,
        }
    }

    pub(crate) fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub(crate) fn action_label(mut self, label: impl Into<SharedString>) -> Self {
        self.action_label = label.into();
        self
    }

    pub(crate) fn selected_value(mut self, selected: Option<SharedString>) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn on_action(
        mut self,
        action: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(action));
        self
    }

    fn button(palette: Palette, button: Button) -> Button {
        button
            .h(palette.control_height)
            .px_2()
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .bg(palette.field)
            .text_color(palette.foreground)
            .hover(|style| style.bg(palette.highlight))
            .focus_visible(|style| style.border_color(palette.accent))
            .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
    }
}

impl Styled for DataTable {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DataTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let palette = self.palette;
        let query = state.query_input();
        let input = InputBase::new((self.id.clone(), "query"))
            .role(None::<Role>)
            .semantic_owner::<Input>(query.entity_id())
            .h(palette.control_height)
            .w_full()
            .child(Input::new(query));
        let filter = Field::new(
            (self.id.clone(), "filter"),
            query,
            format!("Filter {}", self.label),
        )
        .flex()
        .flex_col()
        .gap_1()
        .child(
            InputGroup::new((self.id.clone(), "query-group"), input, palette)
                .focused(query.focus_handle(cx).is_focused(window)),
        );

        let mut header = TableRow::new((self.id.clone(), "header")).bg(palette.highlight);
        for (index, column) in state.columns.iter().enumerate() {
            let (current, next, marker) = if state.sort_column != Some(index) {
                ("not sorted", "ascending", "↕")
            } else if state.descending {
                ("sorted descending", "ascending", "↓")
            } else {
                ("sorted ascending", "descending", "↑")
            };
            let owner = self.state.clone();
            header = header.cell(
                TableCell::new((self.id.clone(), format!("heading-{index}")))
                    .accessibility_label(column.label.clone())
                    .p_1()
                    .child(Self::button(
                        palette,
                        Button::new((self.id.clone(), format!("sort-{index}")))
                            .accessibility_label(format!(
                                "{}, {}; sort {}",
                                column.label, current, next
                            ))
                            .on_click(move |_, _, cx| {
                                owner.update(cx, |state, cx| state.sort_by_column(index, cx))
                            })
                            .child(format!("{} {marker}", column.label)),
                    )),
            );
        }
        header = header.cell(
            TableCell::new((self.id.clone(), "action-heading"))
                .p_2()
                .accessibility_label("Action")
                .child("Action"),
        );

        let columns = state
            .columns
            .iter()
            .map(|column| TableColumn::fixed(column.width))
            .chain(std::iter::once(TableColumn::fixed(px(130.))));
        let width = state
            .columns
            .iter()
            .fold(px(130.), |width, column| width + column.width);
        let mut table = Table::new((self.id.clone(), "table"), columns)
            .accessibility_label(self.label.clone())
            .w(width)
            .header(header);
        for row in state.visible_rows() {
            let row_id: ElementId = (self.id.clone(), format!("row-{}", row.id())).into();
            let current = self.selected.as_ref() == Some(row.id());
            let mut rendered = TableRow::new(row_id.clone())
                .border_b_1()
                .border_color(palette.border);
            if current {
                rendered = rendered.bg(palette.highlight);
            }
            for (index, cell) in row.cells().iter().enumerate() {
                rendered = rendered.cell(
                    TableCell::new((row_id.clone(), format!("column-{index}")))
                        .row_header(index == 0)
                        .accessibility_label(cell.clone())
                        .p_2()
                        .child(cell.clone()),
                );
            }
            let value = row.id().clone();
            let action = self.on_action.clone();
            let label = if current {
                format!("{}: current", row.cells()[0])
            } else {
                format!("{}: {}", self.action_label, row.cells()[0])
            };
            let button = Self::button(
                palette,
                Button::new((row_id.clone(), "action"))
                    .accessibility_label(label)
                    .disabled(row.disabled || current || action.is_none())
                    .on_click(move |_, window, cx| {
                        if let Some(action) = &action {
                            action(value.clone(), window, cx);
                        }
                    })
                    .child(if current {
                        SharedString::from("Current")
                    } else {
                        self.action_label.clone()
                    }),
            );
            table = table
                .row(rendered.cell(TableCell::new((row_id, "action-cell")).p_1().child(button)));
        }
        table = table.caption(
            TableCaption::new((self.id.clone(), "caption"))
                .py_2()
                .text_sm()
                .text_color(palette.muted)
                .child(format!("{} matching rows", state.filtered_count())),
        );

        let empty = state.filtered_count() == 0;
        let page = state.page_index();
        let page_count = state.page_count();
        let viewport = ScrollArea::new((self.id.clone(), "scroll"), &state.scroll)
            .accessibility_label(format!("{} rows", self.label))
            .axis(ScrollbarAxis::Both)
            .w_full()
            .h(px(200.))
            .content_styles(|style| style.pb(px(16.)).pr(px(16.)))
            .scrollbar_styles(|styles| styles.thumb(|thumb| thumb.bg(palette.muted).radius(px(3.))))
            .child(table);
        let clear = self.state.clone();
        let pagination = self.state.clone();
        self.base
            .child(filter)
            .when(!empty, |base| base.child(viewport))
            .when(empty, |base| {
                base.child(
                    EmptyState::new(
                        (self.id.clone(), "empty"),
                        "No matching rows",
                        "Change the filter or clear it to show every row.",
                        palette,
                    )
                    .action(Self::button(
                        palette,
                        Button::new((self.id.clone(), "clear-filter"))
                            .accessibility_label("Clear filter")
                            .on_click(move |_, window, cx| {
                                clear.update(cx, |state, cx| state.clear_filter(window, cx))
                            })
                            .child("Clear filter"),
                    )),
                )
            })
            .child(
                Pagination::new(
                    (self.id, "pagination"),
                    format!("{} pages", self.label),
                    page,
                    page_count,
                    palette,
                )
                .on_change(move |page, _, cx| {
                    pagination.update(cx, |state, cx| state.set_page(page, cx))
                }),
            )
    }
}
