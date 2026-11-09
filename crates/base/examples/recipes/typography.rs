//! Application-owned text presentation using GPUI's native text layout.

use std::ops::Range;

use gpui::{
    App, Div, ElementId, FontWeight, HighlightStyle, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, StyledText, Window, div, relative,
};
use gpui_design::{ObservedElement, TestSupportExt as _, TestTextExt as _};

use super::super::Palette;

fn text_base(id: impl Into<ElementId>, palette: Palette) -> ObservedElement<Stateful<Div>> {
    div()
        .id(id)
        .test_support()
        .min_w_0()
        .max_w_full()
        .flex_shrink_0()
        .whitespace_normal()
        .text_size(palette.font_size)
        .text_color(palette.foreground)
        .line_height(relative(1.5))
}

/// A named native heading whose semantic level is independent of its appearance.
///
/// `level` must be positive, matching AccordionHeader's native heading contract.
/// The application chooses document hierarchy. Every level defaults to the same
/// semibold, 1.25-times-palette text size; Styled can change the visual hierarchy
/// without changing the accessible level or name. No focus stop is added.
#[derive(IntoElement)]
pub(crate) struct Heading {
    base: ObservedElement<Stateful<Div>>,
    text: SharedString,
    level: usize,
}

impl Heading {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        level: usize,
        palette: Palette,
    ) -> Self {
        assert!(level > 0, "Heading level must be positive");
        Self {
            base: text_base(id, palette)
                .text_size(palette.font_size * 1.25)
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(relative(1.25)),
            text: text.into(),
            level,
        }
    }
}

impl Styled for Heading {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Heading {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(Role::Heading)
            .aria_level(self.level)
            .aria_label(self.text.clone())
            .child(self.text.clone().test_text(self.text))
    }
}

/// One native paragraph containing plain text and inline code in source order.
///
/// Text and code fragments are concatenated exactly, including caller-supplied
/// spaces and newlines. GPUI shapes and wraps the entire string together, with
/// palette code-family/background overrides on UTF-8 code ranges. Styled affects
/// the paragraph's inherited text style; code keeps those two palette overrides.
/// The paragraph has one accessible name containing its complete text and adds
/// no focus stop. StyledText does not expose roles for individual ranges, so code
/// fragments do not create separate Code nodes or duplicate hidden text.
///
/// Give the containing layout a definite width for wrapping. Normal GPUI wrapping
/// handles long words and code; there is no truncation, line limit, parser,
/// selection, link action, or separate text renderer in this recipe.
#[derive(IntoElement)]
pub(crate) struct Paragraph {
    base: ObservedElement<Stateful<Div>>,
    text: String,
    code_ranges: Vec<Range<usize>>,
    palette: Palette,
}

impl Paragraph {
    pub(crate) fn new(id: impl Into<ElementId>, palette: Palette) -> Self {
        Self {
            base: text_base(id, palette),
            text: String::new(),
            code_ranges: Vec::new(),
            palette,
        }
    }

    pub(crate) fn text(mut self, text: impl AsRef<str>) -> Self {
        self.text.push_str(text.as_ref());
        self
    }

    pub(crate) fn code(mut self, code: impl AsRef<str>) -> Self {
        let start = self.text.len();
        self.text.push_str(code.as_ref());
        if self.text.len() > start {
            self.code_ranges.push(start..self.text.len());
        }
        self
    }
}

impl Styled for Paragraph {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Paragraph {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let text = SharedString::from(self.text);
        let highlights = self.code_ranges.iter().cloned().map(|range| {
            (
                range,
                HighlightStyle {
                    background_color: Some(self.palette.highlight),
                    ..Default::default()
                },
            )
        });
        let styled_text = StyledText::new(text.clone())
            .with_highlights(highlights)
            .with_font_family_overrides(
                self.code_ranges
                    .into_iter()
                    .map(|range| (range, self.palette.code_font_family.into())),
            );
        self.base
            .role(Role::Paragraph)
            .aria_label(text.clone())
            .child(styled_text.test_text(text))
    }
}

/// A standalone static code fragment with its own native Code role and name.
///
/// The palette supplies the requested font family; GPUI owns font resolution and
/// any platform fallback. Styled customizes this element's font and decoration.
/// Code wraps within the available width and adds no keyboard focus or action.
/// Use Paragraph::code for code inside a continuously shaped sentence.
#[derive(IntoElement)]
pub(crate) struct InlineCode {
    base: ObservedElement<Stateful<Div>>,
    text: SharedString,
}

impl InlineCode {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        palette: Palette,
    ) -> Self {
        Self {
            base: text_base(id, palette)
                .font_family(palette.code_font_family)
                .bg(palette.highlight)
                .rounded(palette.radius)
                .px_1(),
            text: text.into(),
        }
    }
}

impl Styled for InlineCode {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for InlineCode {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(Role::Code)
            .aria_label(self.text.clone())
            .child(self.text.clone().test_text(self.text))
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, Context, Element as _, Pixels, Render, TestAppContext, accesskit, canvas,
        px, size,
    };
    use gpui_design::{Root, test_support};

    use super::super::super::Appearance;
    use super::*;

    #[test]
    #[should_panic(expected = "Heading level must be positive")]
    fn a_heading_requires_a_positive_native_level() {
        Heading::new(
            "invalid-heading",
            "Invalid",
            0,
            Appearance::Compact.palette(),
        );
    }

    #[gpui::test]
    fn native_text_roles_names_and_heading_levels_are_preserved(cx: &mut TestAppContext) {
        struct Probe(Rc<RefCell<Vec<accesskit::Node>>>);
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let nodes = self.0.clone();
                canvas(
                    move |_, window, cx| {
                        fn native_node(element: impl IntoElement) -> accesskit::Node {
                            let element = element.into_element();
                            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                            element.write_a11y_info(&mut node);
                            node
                        }
                        let palette = Appearance::Compact.palette();
                        *nodes.borrow_mut() = vec![
                            native_node(
                                Heading::new("heading-two", "Session help", 2, palette)
                                    .render(window, cx),
                            ),
                            native_node(
                                Heading::new("heading-seven", "Nested details", 7, palette)
                                    .text_sm()
                                    .render(window, cx),
                            ),
                            native_node(
                                Paragraph::new("paragraph", palette)
                                    .text("설정 ")
                                    .code("")
                                    .code("session_name")
                                    .text(" stays readable.")
                                    .render(window, cx),
                            ),
                            native_node(
                                InlineCode::new("code", "session_name", palette).render(window, cx),
                            ),
                        ];
                    },
                    |_, _, _, _| {},
                )
            }
        }
        let nodes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let nodes = nodes.clone();
            move |_, _| Probe(nodes)
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let nodes = nodes.borrow();
        assert_eq!(nodes[0].role(), Role::Heading);
        assert_eq!(nodes[0].level(), Some(2));
        assert_eq!(nodes[0].label(), Some("Session help"));
        assert_eq!(nodes[1].role(), Role::Heading);
        assert_eq!(nodes[1].level(), Some(7));
        assert_eq!(nodes[1].label(), Some("Nested details"));
        assert_eq!(nodes[2].role(), Role::Paragraph);
        assert_eq!(nodes[2].label(), Some("설정 session_name stays readable."));
        assert_eq!(nodes[3].role(), Role::Code);
        assert_eq!(nodes[3].label(), Some("session_name"));
        assert!(
            nodes
                .iter()
                .all(|node| !node.supports_action(accesskit::Action::Focus))
        );
    }

    #[gpui::test]
    fn long_text_and_code_wrap_at_narrow_widths_in_both_appearances(cx: &mut TestAppContext) {
        const TITLE: &str =
            "Session settings and keyboard guidance for recording with long project names";
        const PREFIX: &str = "긴 프로젝트 이름에도 설정을 읽을 수 있습니다. Use ";
        const CODE: &str =
            "recording_session_name_with_a_long_unbroken_configuration_identifier_0123456789";
        const SUFFIX: &str = " to keep the session recognizable when the window becomes narrow. The complete instruction stays available across wrapped lines.";

        struct Sample {
            appearance: Appearance,
            width: Pixels,
        }
        impl Render for Sample {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let palette = self.appearance.palette();
                div()
                    .w(self.width)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Heading::new("heading", TITLE, 3, palette))
                    .child(
                        Paragraph::new("paragraph", palette)
                            .text(PREFIX)
                            .code(CODE)
                            .text(SUFFIX),
                    )
                    .child(InlineCode::new("code", CODE, palette))
            }
        }
        cx.update(gpui_design::init);
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (root, cx) = cx.add_window_view(|window, cx| {
                let sample = cx.new(|_| Sample {
                    appearance,
                    width: px(360.),
                });
                Root::new(sample, window, cx)
            });
            let sample = root
                .read_with(cx, |root, _| root.view().clone())
                .downcast::<Sample>()
                .unwrap();
            cx.simulate_resize(size(px(500.), px(1800.)));
            let mut wide_heights = Vec::new();
            for width in [360., 160.] {
                sample.update(cx, |sample, cx| {
                    sample.width = px(width);
                    cx.notify();
                });
                cx.update(|window, cx| window.draw(cx).clear(cx));
                cx.run_until_parked();
                cx.update(|window, cx| window.draw(cx).clear(cx));
                cx.update(|window, _| {
                    let paragraph = format!("{PREFIX}{CODE}{SUFFIX}");
                    for (index, (id, role, label)) in [
                        ("heading", Role::Heading, TITLE),
                        ("paragraph", Role::Paragraph, paragraph.as_str()),
                        ("code", Role::Code, CODE),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let id = ElementId::from(id);
                        let element = test_support::find(window, &[], &id).unwrap();
                        assert_eq!(element.role(), Some(role));
                        assert_eq!(element.label(), Some(label));
                        assert_eq!(element.focused(), None);
                        let text = test_support::texts(window)
                            .into_iter()
                            .find(|text| text.path().contains(&id))
                            .unwrap();
                        assert_eq!(text.text(), label);
                        assert!(text.visible());
                        assert!(text.bounds().size.width <= px(width) + px(1.));
                        assert!(text.bounds().size.height > px(0.));
                        if width == 360. {
                            wide_heights.push(text.bounds().size.height);
                        } else {
                            assert!(text.bounds().size.height > wide_heights[index]);
                        }
                    }
                    assert_eq!(
                        test_support::snapshots(window)
                            .iter()
                            .filter(|element| element.role() == Some(Role::Code))
                            .count(),
                        1,
                        "styled paragraph ranges must not add duplicate semantic code nodes"
                    );
                });
            }
        }
    }
}
