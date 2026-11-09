//! Application-owned loading motion over the shared Progress semantics.

use std::{
    f32::consts::{FRAC_PI_2, TAU},
    time::Duration,
};

use gpui::{
    Animation, AnimationExt as _, App, ElementId, IntoElement, ParentElement as _, PathBuilder,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, canvas, div, point, px, rems,
};
use gpui_design::Progress;

use super::super::Palette;

/// A named indeterminate Progress with a rotating, decorative arc.
///
/// The application mounts this while an operation has unknown progress, and
/// unmounts it when that operation finishes. GPUI's animation wrapper owns the
/// frame requests and renders a static arc when `App::reduce_motion` is enabled.
/// There is no separate timer, loading state, focus stop, or live announcement.
/// `Styled` controls the root; its size and text color also control the arc.
#[derive(IntoElement)]
pub(crate) struct Spinner {
    base: Progress,
    id: ElementId,
}

impl Spinner {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: Progress::new(id.clone())
                .indeterminate(true)
                .accessibility_label(label)
                .size(rems(1.))
                .flex_shrink_0()
                .text_color(palette.accent),
            id,
        }
    }
}

impl Styled for Spinner {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.child(div().size_full().with_animation(
            (self.id, "rotation"),
            Animation::new(Duration::from_secs(1)).repeat(),
            |visual, phase| {
                visual.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let diameter = bounds.size.width.min(bounds.size.height);
                            if diameter <= px(0.) {
                                return;
                            }
                            let stroke = diameter / 8.;
                            let radius = (diameter - stroke) / 2.;
                            let center = bounds.center();
                            let position = |angle: f32| {
                                center + point(radius * angle.cos(), radius * angle.sin())
                            };
                            let start = phase * TAU - FRAC_PI_2;
                            let mut arc = PathBuilder::stroke(stroke);
                            arc.move_to(position(start));
                            arc.arc_to(
                                point(radius, radius),
                                px(0.),
                                true,
                                true,
                                position(start + 0.75 * TAU),
                            );
                            let color = window.text_style().color;
                            window.paint_path(arc.build().expect("valid spinner arc"), color);
                        },
                    )
                    .size_full(),
                )
            },
        ))
    }
}
