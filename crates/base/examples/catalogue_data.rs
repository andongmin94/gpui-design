//! Native chart, directional layout, media trim, and a local review carousel.

#[path = "recipes/chart.rs"]
mod chart;

use chart::{ChartKind, ChartRecipe};
use gpui::{
    AnyElement, AppContext as _, Axis, Bounds, Context, ElementId, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString, Styled,
    Subscription, Window, canvas, div, fill, point, prelude::FluentBuilder as _, px, relative, rgb,
    size,
};
use gpui_design::{
    Button, Carousel, CarouselControl, CarouselSlide, CarouselState, ChartSeries, ChartState,
    Direction, DirectionProvider, Slider, SliderState, SliderThumb,
};

pub(crate) struct DataCatalogue {
    chart: Entity<ChartState>,
    carousel: Entity<CarouselState>,
    trim: Entity<SliderState>,
    trim_axis: Axis,
    chart_kind: ChartKind,
    direction: Direction,
    reviewed: [bool; 3],
    _subscriptions: Vec<Subscription>,
}

impl DataCatalogue {
    pub(crate) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let chart = cx.new(|cx| {
            ChartState::new(
                ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
                [
                    ChartSeries::new("Uploaded", [32., 55., 41., 72., 48., 64.]),
                    ChartSeries::new("Removed", [-12., -18., -8., -30., -16., -24.]),
                ],
                cx,
            )
        });
        let carousel = cx.new(|cx| {
            CarouselState::new(["Recording frame", "Audio waveform", "Caption review"], cx)
        });
        let trim = cx.new(|cx| SliderState::new(0.0..=100.0, 1., [20., 80.], cx));
        let subscriptions = vec![
            cx.observe(&chart, |_, _, cx| cx.notify()),
            cx.observe(&carousel, |_, _, cx| cx.notify()),
            cx.observe(&trim, |_, _, cx| cx.notify()),
        ];
        Self {
            chart,
            carousel,
            trim,
            trim_axis: Axis::Horizontal,
            chart_kind: ChartKind::Line,
            direction: Direction::Ltr,
            reviewed: [false; 3],
            _subscriptions: subscriptions,
        }
    }

    fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
        let label = label.into();
        Button::new(id)
            .accessibility_label(label.clone())
            .child(label)
            .px_3()
            .py_2()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(0x475569))
            .bg(rgb(0x1e293b))
            .text_color(rgb(0xf1f5f9))
            .hover(|style| style.bg(rgb(0x334155)))
            .focus_visible(|style| style.border_color(rgb(0x93c5fd)))
            .styles(|styles| {
                styles
                    .selected(|style| style.bg(rgb(0x1e40af)))
                    .disabled(|style| style.opacity(0.45))
            })
    }

    fn render_trim(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self.trim.read(cx);
        let start = state.percentage(0);
        let end = state.percentage(1);
        let horizontal = self.trim_axis == Axis::Horizontal;
        let first = if horizontal {
            self.direction
                .physical_fraction(start)
                .min(self.direction.physical_fraction(end))
        } else {
            start
        };
        let mut rail = Slider::new("media-trim", &self.trim)
            .accessibility_label("Media trim range in seconds")
            .axis(self.trim_axis)
            .direction(self.direction)
            .relative()
            .when(horizontal, |this| this.w_full().h(px(32.)))
            .when(!horizontal, |this| this.w(px(32.)).h(px(160.)))
            .when(state.is_disabled(), |this| this.opacity(0.45))
            .child(
                div()
                    .absolute()
                    .rounded_full()
                    .bg(rgb(0x475569))
                    .when(horizontal, |this| {
                        this.left_0()
                            .right_0()
                            .top(relative(0.5))
                            .h(px(4.))
                            .mt(px(-2.))
                    })
                    .when(!horizontal, |this| {
                        this.top_0()
                            .bottom_0()
                            .left(relative(0.5))
                            .w(px(4.))
                            .ml(px(-2.))
                    }),
            )
            .child(
                div()
                    .absolute()
                    .rounded_full()
                    .bg(rgb(0x38bdf8))
                    .when(horizontal, |this| {
                        this.left(relative(first))
                            .w(relative(end - start))
                            .top(relative(0.5))
                            .h(px(4.))
                            .mt(px(-2.))
                    })
                    .when(!horizontal, |this| {
                        this.bottom(relative(start))
                            .h(relative(end - start))
                            .left(relative(0.5))
                            .w(px(4.))
                            .ml(px(-2.))
                    }),
            );
        for index in 0..2 {
            let fraction = state.percentage(index);
            rail = rail.child(
                SliderThumb::new(&self.trim, index)
                    .accessibility_label(if index == 0 {
                        "Trim start in seconds"
                    } else {
                        "Trim end in seconds"
                    })
                    .absolute()
                    .size(px(16.))
                    .rounded_full()
                    .border_2()
                    .border_color(rgb(0x0f172a))
                    .bg(rgb(0x38bdf8))
                    .focus_visible(|style| style.border_color(rgb(0xf1f5f9)).bg(rgb(0x1d4ed8)))
                    .when(horizontal, |this| {
                        this.left(relative(self.direction.physical_fraction(fraction)))
                            .top(relative(0.5))
                            .ml(px(-8.))
                            .mt(px(-8.))
                    })
                    .when(!horizontal, |this| {
                        this.bottom(relative(fraction))
                            .left(relative(0.5))
                            .ml(px(-8.))
                            .mb(px(-8.))
                    }),
            );
        }
        let values = state.values();
        div().w_full().flex().flex_col().gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Slider · media trim selection"))
            .child("Choose the start and end of a 100-second preview. Tab reaches each handle; arrow keys adjust it without crossing the other handle.")
            .child(Self::button("media-trim-axis", if horizontal { "Orientation: Horizontal" } else { "Orientation: Vertical" })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.trim_axis = if this.trim_axis == Axis::Horizontal { Axis::Vertical } else { Axis::Horizontal };
                    cx.notify();
                })))
            .child(div().w_full().p_3().child(rail))
            .child(format!("Start: {:.0} s · End: {:.0} s · Selected: {:.0} s", values[0], values[1], values[1] - values[0]))
            .into_any_element()
    }

    fn render_carousel(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.carousel.read(cx).selected_index().unwrap();
        let mut carousel = Carousel::new(
            "data-carousel",
            &self.carousel,
            "Local media review previews",
        )
        .direction(self.direction)
        .w_full()
        .min_w_0()
        .flex()
        .flex_col();
        for index in 0..3 {
            let preview = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    window.paint_quad(fill(bounds, rgb(0x0f172a)).corner_radii(px(8.)));
                    match index {
                        0 => {
                            // An application-owned mock recording frame and timeline.
                            let inset = bounds.inset(px(16.));
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        inset.origin,
                                        size(inset.size.width * 0.25, inset.size.height * 0.7),
                                    ),
                                    rgb(0x334155),
                                )
                                .corner_radii(px(4.)),
                            );
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(inset.left() + inset.size.width * 0.3, inset.top()),
                                        size(inset.size.width * 0.7, inset.size.height * 0.7),
                                    ),
                                    rgb(0x1e40af),
                                )
                                .corner_radii(px(4.)),
                            );
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(inset.left(), inset.bottom() - px(10.)),
                                    size(inset.size.width, px(5.)),
                                ),
                                rgb(0x38bdf8),
                            ));
                        }
                        1 => {
                            // Local illustrative amplitude samples, not a decoder.
                            for sample in 0..40 {
                                let amplitude = ((sample as f32 * 0.7).sin().abs() * 0.7 + 0.1)
                                    * bounds.size.height;
                                let x =
                                    bounds.left() + bounds.size.width * (sample as f32 + 0.5) / 40.;
                                window.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(x, bounds.center().y - amplitude / 2.),
                                            size(px(3.), amplitude),
                                        ),
                                        rgb(0x4ade80),
                                    )
                                    .corner_radii(px(1.5)),
                                );
                            }
                        }
                        _ => {
                            for (row, width) in [0.8, 0.6, 0.9, 0.7].into_iter().enumerate() {
                                window.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(
                                                bounds.left() + px(20.),
                                                bounds.top() + px(16.) + px(24.) * row as f32,
                                            ),
                                            size((bounds.size.width - px(40.)) * width, px(10.)),
                                        ),
                                        rgb(0xc084fc),
                                    )
                                    .corner_radii(px(3.)),
                                );
                            }
                        }
                    }
                },
            )
            .h(px(128.))
            .w_full();
            let title = self.carousel.read(cx).labels()[index].clone();
            let description = [
                "Check the framing and timeline. This preview is drawn locally with GPUI.",
                "Check the illustrative audio waveform. Review state survives switching slides.",
                "Check the illustrative captions and mark this item reviewed.",
            ][index];
            let review = Self::button(
                ("review-media", index),
                if self.reviewed[index] {
                    "Reviewed"
                } else {
                    "Mark reviewed"
                },
            )
            .selected(self.reviewed[index])
            .on_click(cx.listener(move |this, _, _, cx| {
                this.reviewed[index] = !this.reviewed[index];
                cx.notify();
            }));
            carousel = carousel.child(
                CarouselSlide::new(&self.carousel, index)
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .bg(rgb(0x1e293b))
                    .rounded(px(8.))
                    .child(preview)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(gpui::text!(id = ("media-description", index), description))
                    .child(review),
            );
        }
        let mut pagination = self.direction.row(div()).gap_2();
        for index in 0..3 {
            pagination = pagination.child(
                Self::button(("media-page", index), format!("{}", index + 1))
                    .accessibility_label(format!("Show media slide {} of 3", index + 1))
                    .selected(selected == index)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.carousel.update(cx, |state, cx| {
                            state.select(index, window, cx);
                        });
                    })),
            );
        }
        div().flex().flex_col().gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Carousel · local media review"))
            .child("Focus the carousel for arrows/Home/End; swipe a slide or use the controls. Each slide keeps its application-owned review state.")
            .child(carousel)
            .child(self.direction.row(div()).gap_2().items_center()
                .child(CarouselControl::previous("media-previous", &self.carousel).px_3().py_2().rounded(px(6.)).border_1().border_color(rgb(0x64748b)).child("Previous"))
                .child(pagination)
                .child(CarouselControl::next("media-next", &self.carousel).px_3().py_2().rounded(px(6.)).border_1().border_color(rgb(0x64748b)).child("Next")))
            .child(format!("Slide {} of 3 · {} reviewed", selected + 1, self.reviewed.iter().filter(|reviewed| **reviewed).count()))
            .into_any_element()
    }
}

impl Render for DataCatalogue {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut modes = self.direction.row(div()).gap_2().flex_wrap();
        for kind in [ChartKind::Line, ChartKind::Bar, ChartKind::Area] {
            modes = modes.child(
                Self::button((ElementId::from("chart-mode"), kind.label()), kind.label())
                    .selected(self.chart_kind == kind)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.chart_kind = kind;
                        cx.notify();
                    })),
            );
        }
        let direction = Self::button(
            "data-direction",
            if self.direction.is_rtl() {
                "Direction: RTL"
            } else {
                "Direction: LTR"
            },
        )
        .on_click(cx.listener(|this, _, _, cx| {
            this.direction = if this.direction.is_rtl() {
                Direction::Ltr
            } else {
                Direction::Rtl
            };
            cx.notify();
        }));
        DirectionProvider::new(self.direction)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_4()
            .text_size(px(14.))
            .text_color(rgb(0xf1f5f9))
            .child(direction)
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Chart · illustrative transfer samples (MB)"),
            )
            .child(modes)
            .child(ChartRecipe::new(
                "transfer-chart",
                &self.chart,
                self.chart_kind,
                self.direction,
            ))
            .child(self.render_carousel(cx))
            .child(self.render_trim(cx))
    }
}
