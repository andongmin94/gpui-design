//! Application-owned media framing using GPUI's existing aspect-ratio layout.

use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, Stateful, StyleRefinement, Styled, Window, div,
};
use gpui_design::{ObservedElement, TestSupportExt as _};

/// A clipped frame whose preferred ratio is width divided by height.
///
/// The parent supplies a definite width, or the application sets one with
/// `Styled`. Leave height automatic: explicit height and min/max constraints
/// follow GPUI's ordinary layout rules and can override the preferred ratio.
/// Content fills an absolute inner container, so intrinsic media dimensions do
/// not determine the frame's height. Images can choose their own object fit.
///
/// This wrapper adds no role, accessible name, focus stop, or retained state.
/// Children own their media semantics and interaction. The application supplies
/// background, rounding, and any other presentation through `Styled`.
#[derive(IntoElement)]
pub(crate) struct AspectRatio {
    base: ObservedElement<Stateful<Div>>,
    content: Div,
}

impl AspectRatio {
    /// `ratio` must be finite and greater than zero.
    pub(crate) fn new(id: impl Into<ElementId>, ratio: f32) -> Self {
        assert!(
            ratio.is_finite() && ratio > 0.,
            "aspect ratio must be finite and positive"
        );
        Self {
            base: div()
                .id(id)
                .test_support()
                .relative()
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .aspect_ratio(ratio)
                .overflow_hidden(),
            content: div().absolute().top_0().left_0().size_full(),
        }
    }
}

impl Styled for AspectRatio {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for AspectRatio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.content.extend(elements);
    }
}

impl RenderOnce for AspectRatio {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.child(self.content)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Context, Modifiers, MouseButton, Pixels, Render, TestAppContext, point, px,
    };
    use gpui_design::{Root, test_support};

    use super::*;

    struct Sample {
        width: Pixels,
        clicks: Rc<Cell<usize>>,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let clicks = self.clicks.clone();
            div().w(self.width).flex().flex_col().child(
                AspectRatio::new("media-frame", 16. / 9.).child(
                    div()
                        .id("oversized-media")
                        .test_support()
                        .size(px(1000.))
                        .on_mouse_down(MouseButton::Left, move |_, _, _| {
                            clicks.set(clicks.get() + 1);
                        }),
                ),
            )
        }
    }

    #[gpui::test]
    fn width_changes_preserve_ratio_and_clip_oversized_content(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| Sample {
                width: px(320.),
                clicks: Rc::new(Cell::new(0)),
            });
            Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Sample>()
            .unwrap();
        cx.update(|window, _| window.activate_window());

        for width in [320., 480.] {
            view.update(cx, |view, cx| {
                view.width = px(width);
                view.clicks.set(0);
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.run_until_parked();
            let bounds = cx.update(|window, _| {
                let frame = test_support::find(window, &[], &"media-frame".into()).unwrap();
                assert_eq!(frame.bounds().size.width, px(width));
                assert_eq!(frame.bounds().size.height, px(width * 9. / 16.));
                assert_eq!(frame.role(), None);
                assert_eq!(frame.focused(), None);
                let media = test_support::find(window, &[], &"oversized-media".into()).unwrap();
                assert_eq!(media.bounds().size.width, px(1000.));
                assert_eq!(media.bounds().size.height, px(1000.));
                frame.bounds()
            });

            cx.simulate_click(bounds.center(), Modifiers::default());
            cx.update(|_, cx| assert_eq!(view.read(cx).clicks.get(), 1));
            cx.simulate_click(
                point(bounds.right() + px(8.), bounds.center().y),
                Modifiers::default(),
            );
            cx.simulate_click(
                point(bounds.center().x, bounds.bottom() + px(8.)),
                Modifiers::default(),
            );
            cx.update(|_, cx| assert_eq!(view.read(cx).clicks.get(), 1));
        }
    }
}
