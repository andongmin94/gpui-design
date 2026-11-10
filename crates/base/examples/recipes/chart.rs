//! Copyable native line/bar/area presentation over the public headless Chart.

use gpui::{
    App, Bounds, ElementId, Entity, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    PathBuilder, Pixels, Point, RenderOnce, Role, StatefulInteractiveElement as _, Styled, Window,
    canvas, div, fill, point, px, rgb, size,
};
use gpui_design::{Chart, ChartState, Direction, Table, TableCell, TableColumn, TableRow, Toggle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChartKind {
    Line,
    Bar,
    Area,
}

impl ChartKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Bar => "Bar",
            Self::Area => "Area",
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct ChartRecipe {
    id: ElementId,
    state: Entity<ChartState>,
    kind: ChartKind,
    direction: Direction,
}

impl ChartRecipe {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        state: &Entity<ChartState>,
        kind: ChartKind,
        direction: Direction,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            kind,
            direction,
        }
    }
}

fn color(index: usize) -> Hsla {
    rgb([0x38bdf8, 0xfb923c, 0x4ade80, 0xc084fc][index % 4]).into()
}

struct PlotSeries {
    heights: Vec<f32>,
    color: Hsla,
}

struct Plot {
    series: Vec<PlotSeries>,
    categories: usize,
    baseline: f32,
    inspected: Option<usize>,
    kind: ChartKind,
    direction: Direction,
}

fn stroke(points: &[Point<Pixels>], color: Hsla, window: &mut Window) {
    let Some(&first) = points.first() else {
        return;
    };
    let mut path = PathBuilder::stroke(px(2.));
    path.move_to(first);
    for &point in &points[1..] {
        path.line_to(point);
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

impl Plot {
    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let series = &self.series;
        let categories = self.categories;
        let baseline = self.baseline;
        let inspected = self.inspected;
        let kind = self.kind;
        let direction = self.direction;
        if categories == 0 {
            return;
        }
        let x = |index: usize| {
            bounds.left()
                + bounds.size.width
                    * direction.physical_fraction((index as f32 + 0.5) / categories as f32)
        };
        let y = |height: f32| bounds.bottom() - bounds.size.height * height;
        for tick in 0..=4 {
            window.paint_quad(fill(
                Bounds::new(
                    point(bounds.left(), y(tick as f32 / 4.)),
                    size(bounds.size.width, px(1.)),
                ),
                rgb(0x334155),
            ));
        }
        window.paint_quad(fill(
            Bounds::new(
                point(bounds.left(), y(baseline)),
                size(bounds.size.width, px(1.)),
            ),
            rgb(0x94a3b8),
        ));
        if let Some(index) = inspected {
            window.paint_quad(fill(
                Bounds::new(
                    point(x(index), bounds.top()),
                    size(px(1.), bounds.size.height),
                ),
                rgb(0x64748b),
            ));
        }
        for series in series {
            let points: Vec<_> = series
                .heights
                .iter()
                .enumerate()
                .map(|(index, &height)| point(x(index), y(height)))
                .collect();
            match kind {
                ChartKind::Bar => {}
                ChartKind::Area => {
                    if let (Some(first), Some(last)) = (points.first(), points.last()) {
                        let mut area = PathBuilder::fill();
                        area.move_to(point(first.x, y(baseline)));
                        for &point in &points {
                            area.line_to(point);
                        }
                        area.line_to(point(last.x, y(baseline)));
                        area.close();
                        if let Ok(path) = area.build() {
                            window.paint_path(path, series.color.opacity(0.2));
                        }
                    }
                    stroke(&points, series.color, window);
                }
                ChartKind::Line => stroke(&points, series.color, window),
            }
            if kind != ChartKind::Bar {
                for (index, &point) in points.iter().enumerate() {
                    let radius = if inspected == Some(index) {
                        px(4.)
                    } else {
                        px(2.5)
                    };
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point - gpui::point(radius, radius),
                                size(radius * 2., radius * 2.),
                            ),
                            series.color,
                        )
                        .corner_radii(radius),
                    );
                }
            }
        }
        if kind == ChartKind::Bar && !series.is_empty() {
            let group_width = bounds.size.width / categories as f32 * 0.7;
            let width = group_width / series.len() as f32;
            for (series_index, series) in series.iter().enumerate() {
                let slot = if direction.is_rtl() {
                    self.series.len() - 1 - series_index
                } else {
                    series_index
                };
                for (index, &height) in series.heights.iter().enumerate() {
                    let left = x(index) - group_width / 2. + width * slot as f32;
                    let top = y(height).min(y(baseline));
                    let height = (y(height) - y(baseline)).abs();
                    window.paint_quad(fill(
                        Bounds::new(point(left, top), size((width - px(2.)).max(px(1.)), height)),
                        series.color,
                    ));
                }
            }
        }
    }
}

impl RenderOnce for ChartRecipe {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let categories = model.categories();
        let data = model.series();
        let range = model.range();
        let baseline = model.normalized_value(0.);
        let inspected = model.inspected_index();
        let description = model.inspected_description();
        let visible: Vec<_> = (0..data.len())
            .map(|index| model.is_visible(index))
            .collect();
        let plotted: Vec<_> = data
            .iter()
            .enumerate()
            .filter(|(index, _)| visible[*index])
            .map(|(index, series)| PlotSeries {
                heights: series
                    .values()
                    .iter()
                    .map(|&value| model.normalized_value(value))
                    .collect(),
                color: color(index),
            })
            .collect();
        let direction = self.direction;
        let drawing = Plot {
            series: plotted,
            categories: categories.len(),
            baseline,
            inspected,
            kind: self.kind,
            direction,
        };
        let plot = Chart::new(
            (self.id.clone(), "plot"),
            &self.state,
            "Transfer samples in megabytes",
        )
        .direction(direction)
        .h(px(200.))
        .w_full()
        .border_1()
        .border_color(if model.focus_handle().is_focused(window) {
            rgb(0x93c5fd)
        } else {
            rgb(0x334155)
        })
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| drawing.paint(bounds, window),
            )
            .size_full(),
        );
        let mut legend = direction.row(div()).flex_wrap().gap_2();
        for (index, series) in data.iter().enumerate() {
            let state = self.state.clone();
            legend = legend.child(
                Toggle::new((self.id.clone(), format!("legend-{index}")))
                    .accessibility_label(format!("Show {}", series.label()))
                    .pressed(visible[index])
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(color(index))
                    .text_color(if visible[index] {
                        color(index)
                    } else {
                        rgb(0x94a3b8).into()
                    })
                    .child(format!(
                        "{} {}",
                        if visible[index] { "●" } else { "○" },
                        series.label()
                    ))
                    .on_change(move |visible, _, window, cx| {
                        state.update(cx, |state, cx| {
                            state.set_visible(index, visible, window, cx);
                        })
                    }),
            );
        }
        let mut table = Table::new(
            (self.id.clone(), "table"),
            (0..=data.len()).map(|_| TableColumn::flex(1.)),
        )
        .accessibility_label("All transfer values in megabytes, including hidden series")
        .w_full()
        .border_1()
        .border_color(rgb(0x334155));
        let mut header = TableRow::new((self.id.clone(), "header")).cell(
            TableCell::new((self.id.clone(), "category-header"))
                .accessibility_label("Day")
                .p_2()
                .child("Day"),
        );
        for (index, series) in data.iter().enumerate() {
            header = header.cell(
                TableCell::new((self.id.clone(), format!("series-{index}")))
                    .accessibility_label(series.label().clone())
                    .p_2()
                    .child(series.label().clone()),
            );
        }
        table = table.header(header);
        for (index, category) in categories.iter().enumerate() {
            let mut row = TableRow::new((self.id.clone(), format!("row-{index}"))).cell(
                TableCell::new((self.id.clone(), format!("category-{index}")))
                    .row_header(true)
                    .accessibility_label(category.clone())
                    .p_2()
                    .child(category.clone()),
            );
            for (series_index, series) in data.iter().enumerate() {
                row = row.cell(
                    TableCell::new((self.id.clone(), format!("value-{index}-{series_index}")))
                        .accessibility_label(series.values()[index].to_string())
                        .p_2()
                        .child(series.values()[index].to_string()),
                );
            }
            table = table.row(row);
        }
        let ticks = div()
            .w(px(56.))
            .h(px(200.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .justify_between()
            .text_color(rgb(0x94a3b8))
            .text_size(px(12.))
            .child(format!("{:.0}", range.1))
            .child(format!("{:.0}", (range.0 + range.1) / 2.))
            .child(format!("{:.0}", range.0));
        let labels = direction
            .row(div())
            .w_full()
            .children(categories.iter().map(|category| {
                div()
                    .flex_1()
                    .min_w_0()
                    .text_center()
                    .text_size(px(12.))
                    .child(category.clone())
            }));
        div().w_full().min_w_0().flex().flex_col().gap_2()
            .child(legend)
            .child(div().flex().gap_2().child(ticks).child(div().min_w_0().flex_1().flex().flex_col().gap_2().child(plot).child(labels)))
            .child(div().id((self.id, "readout")).role(Role::Status).aria_label(description.clone()).text_color(rgb(0xe2e8f0)).child(description))
            .child("Hover a category, or focus the plot and use Left/Right, Home/End. Legend buttons hide a series; the table keeps all values.")
            .child(table)
    }
}
