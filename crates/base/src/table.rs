//! Shared column geometry and native semantics for ordinary data tables.

use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, accesskit::SortDirection, div, prelude::FluentBuilder as _, px,
};

use crate::{StyledExt as _, TestSupportExt as _, a11y_disabled::A11yDisabledExt as _};

/// One column track, shared by every row in a [`Table`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableColumn {
    width: ColumnWidth,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColumnWidth {
    Fixed(Pixels),
    Flex(f32),
}

impl TableColumn {
    /// A fixed-width column that does not shrink when its table overflows.
    pub fn fixed(width: Pixels) -> Self {
        assert!(f32::from(width).is_finite() && width >= px(0.));
        Self {
            width: ColumnWidth::Fixed(width),
        }
    }

    /// A column receiving this positive share of the space after fixed columns.
    pub fn flex(weight: f32) -> Self {
        assert!(weight.is_finite() && weight > 0.);
        Self {
            width: ColumnWidth::Flex(weight),
        }
    }
}

/// An unstyled table with one shared column definition.
///
/// Header, body, and footer render as native row groups. Header cells have the
/// ColumnHeader role; ordinary cells have Cell or an explicitly chosen RowHeader
/// role. Indices are zero-based, matching AccessKit, and include header/footer
/// rows. Tables and their structural parts introduce no keyboard focus stops.
/// Interactive children retain their own normal focus and activation behavior.
///
/// Every row must contain exactly one cell per column. Column widths belong to
/// this table, while cell padding and content styles belong to the application.
/// Rows reserve their horizontal layout for these tracks: apply spacing inside
/// cells, not with row padding, margins, or gaps. Give the table an explicit
/// minimum width and compose it with ScrollArea when columns must overflow a
/// narrower viewport. This component does not virtualize rows or manage data
/// sorting, filtering, selection, or spans.
#[derive(IntoElement)]
pub struct Table {
    id: ElementId,
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    columns: Vec<TableColumn>,
    header: Option<TableRow>,
    rows: Vec<TableRow>,
    footer: Option<TableRow>,
    caption: Option<TableCaption>,
    label: Option<SharedString>,
}

impl Table {
    pub fn new(id: impl Into<ElementId>, columns: impl IntoIterator<Item = TableColumn>) -> Self {
        let columns: Vec<_> = columns.into_iter().collect();
        assert!(!columns.is_empty(), "a table needs at least one column");
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            id,
            columns,
            header: None,
            rows: Vec::new(),
            footer: None,
            caption: None,
            label: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the single column-header row.
    pub fn header(mut self, row: TableRow) -> Self {
        self.header = Some(row);
        self
    }

    pub fn row(mut self, row: TableRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = TableRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Sets a single footer row, such as a total or summary.
    pub fn footer(mut self, row: TableRow) -> Self {
        self.footer = Some(row);
        self
    }

    /// Adds a caption after the table's row groups.
    ///
    /// Supply [`Self::accessibility_label`] separately; a visible caption does
    /// not automatically become the table's native accessible name.
    pub fn caption(mut self, caption: TableCaption) -> Self {
        self.caption = Some(caption);
        self
    }
}

impl Styled for Table {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Table {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let row_count = self.rows.len()
            + usize::from(self.header.is_some())
            + usize::from(self.footer.is_some());
        let body_start = usize::from(self.header.is_some());
        let footer_index = body_start + self.rows.len();
        self.base
            .role(Role::Table)
            .aria_row_count(row_count)
            .aria_column_count(self.columns.len())
            .when_some(self.label, |base, label| base.aria_label(label))
            .flex()
            .flex_col()
            .when_some(self.header, |base, row| {
                base.child(
                    div()
                        .id((self.id.clone(), "header"))
                        .test_support()
                        .role(Role::RowGroup)
                        .w_full()
                        .flex_shrink_0()
                        .child(row.render_row(&self.columns, 0, true)),
                )
            })
            .child(
                div()
                    .id((self.id.clone(), "body"))
                    .test_support()
                    .role(Role::RowGroup)
                    .w_full()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .children(self.rows.into_iter().enumerate().map(|(index, row)| {
                        row.render_row(&self.columns, body_start + index, false)
                    })),
            )
            .when_some(self.footer, |base, row| {
                base.child(
                    div()
                        .id((self.id.clone(), "footer"))
                        .test_support()
                        .role(Role::RowGroup)
                        .w_full()
                        .flex_shrink_0()
                        .child(row.render_row(&self.columns, footer_index, false)),
                )
            })
            .when_some(self.caption, |base, caption| base.child(caption))
    }
}

/// A table-owned row. Build its cells with [`Self::cell`] or [`Self::cells`].
///
/// The owning Table supplies the shared column tracks and native row index.
/// Style backgrounds, borders, and height here; horizontal spacing belongs to
/// cells so every row retains the same column geometry.
pub struct TableRow {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    cells: Vec<TableCell>,
}

impl TableRow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
            cells: Vec::new(),
        }
    }

    pub fn cell(mut self, cell: TableCell) -> Self {
        self.cells.push(cell);
        self
    }

    pub fn cells(mut self, cells: impl IntoIterator<Item = TableCell>) -> Self {
        self.cells.extend(cells);
        self
    }

    /// Exposes application-owned row selection without adding a focus stop.
    pub fn selected(mut self, selected: bool) -> Self {
        self.base = self.base.aria_selected(selected);
        self
    }

    fn render_row(
        self,
        columns: &[TableColumn],
        row: usize,
        header: bool,
    ) -> impl IntoElement + use<> {
        assert_eq!(
            self.cells.len(),
            columns.len(),
            "every table row needs one cell per column"
        );
        self.base
            .role(Role::Row)
            .aria_row_index(row)
            .flex()
            .flex_row()
            .flex_nowrap()
            .flex_shrink_0()
            .w_full()
            .min_w_0()
            .px_0()
            .mx_0()
            .gap_0()
            .children(
                self.cells
                    .into_iter()
                    .zip(columns.iter().copied())
                    .enumerate()
                    .map(|(column, (cell, track))| cell.render_cell(track, row, column, header)),
            )
    }
}

impl Styled for TableRow {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

/// A cell whose contents and presentation belong to the application.
///
/// Styles apply to the content surface inside a structural column track, so
/// different header/body padding cannot change column alignment. The outer
/// track owns the supplied ID, cell role, and row/column indices.
pub struct TableCell {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    row_header: bool,
    label: Option<SharedString>,
    sort_direction: Option<SortDirection>,
}

impl TableCell {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            row_header: false,
            label: None,
            sort_direction: None,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marks a body or footer cell as a row header. Column-header rows take precedence.
    pub fn row_header(mut self, row_header: bool) -> Self {
        self.row_header = row_header;
        self
    }

    /// Exposes the active sort direction on a column-header cell.
    ///
    /// `None` omits the property. Body and footer cells ignore this value.
    pub fn sort_direction(mut self, direction: Option<SortDirection>) -> Self {
        self.sort_direction = direction;
        self
    }

    fn render_cell(
        self,
        track: TableColumn,
        row: usize,
        column: usize,
        header: bool,
    ) -> impl IntoElement {
        let role = if header {
            Role::ColumnHeader
        } else if self.row_header {
            Role::RowHeader
        } else {
            Role::Cell
        };
        let base = self
            .base
            .role(role)
            .aria_row_index(row)
            .aria_column_index(column)
            .when_some(self.label, |base, label| base.aria_label(label))
            .flex()
            .flex_col()
            .min_w_0()
            .flex_shrink_0();
        let base = match track.width {
            ColumnWidth::Fixed(width) => base.w(width),
            ColumnWidth::Flex(weight) => base.flex_basis(px(0.)).flex_grow(weight),
        };
        base.child(
            div()
                .min_w_0()
                .w_full()
                .flex_1()
                .refine_style(&self.style)
                .children(self.children),
        )
        .a11y_sort(self.sort_direction.filter(|_| header))
    }
}

impl Styled for TableCell {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TableCell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// A noninteractive caption rendered after a Table's rows.
#[derive(IntoElement)]
pub struct TableCaption {
    base: crate::ObservedElement<gpui::Stateful<Div>>,
}

impl TableCaption {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id).test_support(),
        }
    }
}

impl Styled for TableCaption {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for TableCaption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for TableCaption {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.role(Role::Caption).flex_shrink_0()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Element as _, TestAppContext, accesskit};

    use super::*;

    #[gpui::test]
    fn table_parts_expose_native_structure_without_focus_actions(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let row = || TableRow::new("row").cell(TableCell::new("cell").child("Value"));
            let table = Table::new("table", [TableColumn::flex(1.)])
                .accessibility_label("Sessions")
                .header(row())
                .row(row())
                .footer(row())
                .render(window, cx)
                .into_element();
            assert_eq!(table.a11y_role(), Some(Role::Table));
            let mut node = accesskit::Node::new(Role::Table);
            table.write_a11y_info(&mut node);
            assert_eq!(node.label(), Some("Sessions"));
            assert_eq!(node.row_count(), Some(3));
            assert_eq!(node.column_count(), Some(1));
            assert!(!node.supports_action(accesskit::Action::Focus));

            let row = row()
                .render_row(&[TableColumn::fixed(px(120.))], 2, false)
                .into_element();
            assert_eq!(row.a11y_role(), Some(Role::Row));
            let mut node = accesskit::Node::new(Role::Row);
            row.write_a11y_info(&mut node);
            assert_eq!(node.row_index(), Some(2));
            assert!(!node.supports_action(accesskit::Action::Focus));

            for (header, row_header, role) in [
                (true, false, Role::ColumnHeader),
                (false, true, Role::RowHeader),
                (false, false, Role::Cell),
            ] {
                let cell = TableCell::new("cell")
                    .row_header(row_header)
                    .accessibility_label("A session")
                    .render_cell(TableColumn::flex(1.), 2, 1, header)
                    .into_element();
                assert_eq!(cell.a11y_role(), Some(role));
                let mut node = accesskit::Node::new(role);
                cell.write_a11y_info(&mut node);
                assert_eq!(node.row_index(), Some(2));
                assert_eq!(node.column_index(), Some(1));
                assert_eq!(node.label(), Some("A session"));
                assert!(!node.supports_action(accesskit::Action::Focus));
                assert!(!node.supports_action(accesskit::Action::Click));
            }
            let caption = TableCaption::new("caption")
                .child("Scheduled sessions")
                .render(window, cx)
                .into_element();
            assert_eq!(caption.a11y_role(), Some(Role::Caption));
        });
    }

    #[gpui::test]
    fn selection_and_sort_direction_use_existing_native_nodes(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|_, _| {
            for selected in [false, true] {
                let row = TableRow::new("row")
                    .selected(selected)
                    .cell(TableCell::new("value"))
                    .render_row(&[TableColumn::flex(1.)], 1, false)
                    .into_element();
                let mut node = accesskit::Node::new(Role::Row);
                row.write_a11y_info(&mut node);
                assert_eq!(node.is_selected(), Some(selected));
                assert!(!node.supports_action(accesskit::Action::Focus));
            }
            for direction in [
                None,
                Some(SortDirection::Ascending),
                Some(SortDirection::Descending),
            ] {
                for header in [false, true] {
                    let cell = TableCell::new("heading")
                        .sort_direction(direction)
                        .render_cell(TableColumn::flex(1.), 0, 0, header)
                        .into_element();
                    let role = if header {
                        Role::ColumnHeader
                    } else {
                        Role::Cell
                    };
                    let mut node = accesskit::Node::new(role);
                    cell.write_a11y_info(&mut node);
                    assert_eq!(cell.a11y_role(), Some(role));
                    assert_eq!(node.sort_direction(), direction.filter(|_| header));
                    assert!(!node.supports_action(accesskit::Action::Click));
                }
            }
        });
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn shared_tracks_align_cells_and_keep_child_button_traversal(cx: &mut TestAppContext) {
        use gpui::{AppContext as _, Context, FocusHandle, Modifiers, Render, VisualTestContext};
        use std::{cell::Cell, rc::Rc};

        use crate::{Button, Root, test_support};

        struct Sample {
            before: FocusHandle,
            action: FocusHandle,
            after: FocusHandle,
            clicks: Rc<Cell<usize>>,
        }

        impl Render for Sample {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let clicks = self.clicks.clone();
                div()
                    .flex()
                    .flex_col()
                    .child(
                        Button::new("before")
                            .track_focus(&self.before)
                            .child("Before"),
                    )
                    .child(
                        Table::new(
                            "sample-table",
                            [
                                TableColumn::fixed(px(90.)),
                                TableColumn::flex(1.),
                                TableColumn::flex(2.),
                            ],
                        )
                        .w(px(450.))
                        .header(
                            TableRow::new("header").cells([
                                TableCell::new("head-0")
                                    .sort_direction(Some(SortDirection::Ascending))
                                    .p_3()
                                    .child("Code"),
                                TableCell::new("head-1").p_3().child("Name"),
                                TableCell::new("head-2").p_3().child("Action"),
                            ]),
                        )
                        .row(
                            TableRow::new("body-row").selected(true).cells([
                                TableCell::new("body-0").p_1().child("AB12"),
                                TableCell::new("body-1").p_1().child("A long session name"),
                                TableCell::new("body-2").p_1().child(
                                    Button::new("table-action")
                                        .track_focus(&self.action)
                                        .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                                        .child("Open"),
                                ),
                            ]),
                        ),
                    )
                    .child(Button::new("after").track_focus(&self.after).child("After"))
            }
        }

        fn draw(cx: &mut VisualTestContext) {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }

        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Sample {
                before: cx.focus_handle(),
                action: cx.focus_handle(),
                after: cx.focus_handle(),
                clicks: Rc::new(Cell::new(0)),
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Sample>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            let before = view.read(cx).before.clone();
            before.focus(window, cx);
        });
        draw(cx);
        cx.run_until_parked();
        cx.update(|window, _| {
            for index in 0..3 {
                let header =
                    test_support::find(window, &[], &format!("head-{index}").into()).unwrap();
                let body =
                    test_support::find(window, &[], &format!("body-{index}").into()).unwrap();
                assert_eq!(header.bounds().origin.x, body.bounds().origin.x);
                assert_eq!(header.bounds().size.width, body.bounds().size.width);
                assert_eq!(header.role(), Some(Role::ColumnHeader));
                assert_eq!(body.role(), Some(Role::Cell));
                assert_eq!(body.focused(), None);
                let expected = [90., 120., 240.][index];
                assert_eq!(body.bounds().size.width, px(expected));
            }
            assert_eq!(
                test_support::find(window, &[], &"head-0".into())
                    .unwrap()
                    .sort_direction(),
                Some(SortDirection::Ascending),
            );
            assert_eq!(
                test_support::find(window, &[], &"body-row".into())
                    .unwrap()
                    .selected(),
                Some(true),
            );
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).action.is_focused(window)));
        let point = cx.update(|window, _| {
            test_support::find(window, &[], &"table-action".into())
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Modifiers::default());
        draw(cx);
        cx.update(|_, cx| assert_eq!(view.read(cx).clicks.get(), 1));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
    }
}
