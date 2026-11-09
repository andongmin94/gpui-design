//! Application-owned circular image presentation with explicit alternative text.

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, ImageSource, InteractiveElement as _, IntoElement,
    ObjectFit, ParentElement as _, RenderOnce, Role, SharedString, Stateful,
    StatefulInteractiveElement as _, StyleRefinement, Styled, StyledImage as _, Window, div, img,
    rems,
};
use gpui_design::{ObservedElement, TestSupportExt as _, TestTextExt as _};

use super::super::Palette;

/// A named, noninteractive avatar with application-supplied fallback text.
///
/// GPUI owns image loading, caching, and redraws. Its native loading delay also
/// applies here; the fallback appears immediately for an absent or failed image,
/// and after GPUI's loading delay while a supplied image is pending.
/// The wrapper owns one Image role and accessible name in every state. Neither
/// the image nor its fallback adds another name, action, or focus stop.
///
/// The default circular frame is 2.5 rem square. Use Styled to set its size and
/// colors; keep width and height equal. The application chooses the fallback
/// text, so initials and localization follow its own content rules.
#[derive(IntoElement)]
pub(crate) struct Avatar {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    label: SharedString,
    fallback: SharedString,
    source: Option<ImageSource>,
}

impl Avatar {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        fallback: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .size(rems(2.5))
                .flex_shrink_0()
                .rounded_full()
                .overflow_hidden()
                .bg(palette.highlight)
                .text_color(palette.foreground)
                .text_size(palette.font_size)
                .font_weight(FontWeight::SEMIBOLD),
            id,
            label: label.into(),
            fallback: fallback.into(),
            source: None,
        }
    }

    pub(crate) fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.source = Some(source.into());
        self
    }
}

fn fallback(id: ElementId, text: SharedString) -> AnyElement {
    div()
        .id((id, "fallback"))
        .test_support()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .child(text.clone())
        .test_text(text)
        .into_any_element()
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Avatar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let content = match self.source {
            Some(source) => {
                let loading_id = self.id.clone();
                let loading_text = self.fallback.clone();
                let error_id = self.id.clone();
                img(source)
                    .id((self.id, "image"))
                    .size_full()
                    .rounded_full()
                    .object_fit(ObjectFit::Cover)
                    .with_loading(move || fallback(loading_id.clone(), loading_text.clone()))
                    .with_fallback(move || fallback(error_id.clone(), self.fallback.clone()))
                    .test_support()
                    .into_any_element()
            }
            None => fallback(self.id, self.fallback),
        };
        self.base
            .role(Role::Image)
            .aria_label(self.label)
            .child(content)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::{cell::RefCell, rc::Rc, time::Duration};

    use gpui::{
        AppContext as _, Context, Image, ImageCacheError, ImageFormat, Render, TestAppContext,
        VisualTestContext, px,
    };
    use gpui_design::{Root, test_support};

    use super::super::super::Appearance;
    use super::*;

    struct Sample {
        source: Option<ImageSource>,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut avatar = Avatar::new(
                "avatar",
                "Recording preset artwork",
                "BL",
                Appearance::Compact.palette(),
            )
            .size(px(40.));
            if let Some(source) = self.source.clone() {
                avatar = avatar.image(source);
            }
            avatar
        }
    }

    fn draw(cx: &mut VisualTestContext, phase: &str, expect_fallback: bool, expect_image: bool) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, _| {
            let avatar = test_support::find(window, &[], &"avatar".into()).unwrap();
            assert_eq!(avatar.role(), Some(Role::Image));
            assert_eq!(avatar.label(), Some("Recording preset artwork"));
            assert_eq!(avatar.focused(), None);
            assert_eq!(avatar.bounds().size.width, px(40.));
            assert_eq!(avatar.bounds().size.height, px(40.));
            let fallback_id = (ElementId::from("avatar"), "fallback").into();
            let fallback = test_support::find(window, &[], &fallback_id);
            assert_eq!(
                fallback.is_some(),
                expect_fallback,
                "fallback during {phase}"
            );
            if let Some(fallback) = fallback {
                assert!(fallback.visible());
                assert_eq!(fallback.bounds(), avatar.bounds());
                assert_eq!(fallback.role(), None);
                assert_eq!(fallback.label(), None);
                assert_eq!(fallback.focused(), None);
                assert!(test_support::texts(window).iter().any(|text| {
                    text.path().contains(&fallback_id) && text.visible() && text.text() == "BL"
                }));
            }
            let image_id = (ElementId::from("avatar"), "image").into();
            let image = test_support::find(window, &[], &image_id);
            assert_eq!(image.is_some(), expect_image, "image during {phase}");
            if let Some(image) = image {
                assert!(image.visible());
                assert_eq!(image.bounds(), avatar.bounds());
            }
        });
    }

    #[gpui::test]
    fn image_transitions_preserve_fallback_geometry_and_native_name(cx: &mut TestAppContext) {
        cx.update(gpui_design::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let sample = cx.new(|_| Sample { source: None });
            Root::new(sample, window, cx)
        });
        let sample = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Sample>()
            .unwrap();
        draw(cx, "absent source", true, false);

        let result = Rc::new(RefCell::new(None));
        let image_result = result.clone();
        sample.update(cx, |sample, cx| {
            sample.source = Some(ImageSource::from(move |_: &mut Window, _: &mut App| {
                image_result.borrow().clone()
            }));
            cx.notify();
        });
        draw(cx, "initial loading", false, true);
        cx.run_until_parked();
        // GPUI Img uses a wall-clock Instant for its loading delay, even with
        // the test dispatcher. Advancing only the executor clock is insufficient.
        std::thread::sleep(Duration::from_millis(201));
        cx.executor().advance_clock(Duration::from_millis(201));
        draw(cx, "delayed loading", true, true);

        *result.borrow_mut() = Some(Err(ImageCacheError::Asset("Unavailable image".into())));
        sample.update(cx, |_, cx| cx.notify());
        draw(cx, "failed source", true, true);

        let loaded = cx.update(|_, cx| {
            Image::from_bytes(
                ImageFormat::Svg,
                include_bytes!("../assets/preset-avatar.svg").to_vec(),
            )
            .to_image_data(cx.svg_renderer())
            .expect("bundled preset artwork must decode")
        });
        *result.borrow_mut() = Some(Ok(loaded));
        sample.update(cx, |_, cx| cx.notify());
        draw(cx, "loaded source", false, true);

        sample.update(cx, |sample, cx| {
            sample.source = None;
            cx.notify();
        });
        draw(cx, "cleared source", true, false);
    }
}
