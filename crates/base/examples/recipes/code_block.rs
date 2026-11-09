//! Read-only code presentation over native text, Button, and ScrollArea.

use gpui::{
    App, ClipboardItem, Div, ElementId, FocusHandle, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, RenderOnce, Role, ScrollHandle, SharedString, Stateful,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div, px, relative, rems,
};
use gpui_design::{
    Button, ElementExt as _, ObservedElement, ScrollArea, ScrollbarAxis, TestSupportExt as _,
    TestTextExt as _,
};

use super::super::Palette;

/// A bounded, read-only code viewport with one whole-source copy action.
///
/// The application retains the ScrollHandle and FocusHandle. The header stays
/// outside the two-axis ScrollArea. Native GPUI text keeps explicit line breaks
/// without soft wrapping, truncation, parsing, or editing; copying uses the
/// supplied string exactly, including empty text and its original line endings.
/// The code's native Code role exposes that complete string as its name.
///
/// Tab visits Copy and then the code viewport. With the viewport focused, plain
/// arrow keys move by one line, PageUp/PageDown by its height, and Home/End reach
/// the top-left/bottom-right. Modified keys, Tab, and Escape keep propagating.
/// Pointer scrolling over the code is contained there, including at its edges;
/// moving outside the viewport permits the surrounding area to scroll normally.
///
/// Styled controls the complete frame, whose default height is 12 rem. Supply a
/// definite available width and preserve the bounded flex layout. The palette
/// supplies code font and colors; GPUI owns shaping, tabs, and font resolution.
/// There is no copied-state timer, selection, syntax highlighter, or editor.
#[derive(IntoElement)]
pub(crate) struct CodeBlock {
    base: ObservedElement<Stateful<Div>>,
    id: ElementId,
    code: SharedString,
    label: SharedString,
    scroll: ScrollHandle,
    focus: FocusHandle,
    palette: Palette,
}

impl CodeBlock {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        code: impl Into<SharedString>,
        label: impl Into<SharedString>,
        scroll: &ScrollHandle,
        focus: &FocusHandle,
        palette: Palette,
    ) -> Self {
        let id = id.into();
        let label = label.into();
        Self {
            base: div()
                .id(id.clone())
                .test_support()
                .role(Role::Group)
                .aria_label(label.clone())
                .w_full()
                .min_w_0()
                .h(rems(12.))
                .min_h_0()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .overflow_hidden()
                .text_size(palette.font_size)
                .text_color(palette.foreground)
                .bg(palette.surface)
                .border_1()
                .border_color(palette.border)
                .rounded(palette.radius),
            id,
            code: code.into(),
            label,
            scroll: scroll.clone(),
            focus: focus.clone(),
            palette,
        }
    }
}

impl Styled for CodeBlock {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CodeBlock {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let palette = self.palette;
        let copy_source = self.code.clone();
        let copy = Button::new((self.id.clone(), "copy"))
            .accessibility_label(format!("Copy {}", self.label))
            .flex_shrink_0()
            .h(palette.control_height)
            .px_2()
            .border_1()
            .border_color(palette.border)
            .rounded(palette.radius)
            .focus_visible(|style| style.border_color(palette.accent))
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copy_source.to_string()));
            })
            .child("Copy");
        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_shrink_0()
            .px_2()
            .py_1()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .whitespace_normal()
                    .child(self.label.clone()),
            )
            .child(copy);

        let area = ScrollArea::new((self.id.clone(), "scroll"), &self.scroll)
            .accessibility_label(format!("{} viewport", self.label))
            .axis(ScrollbarAxis::Both)
            .size_full()
            .content_styles(|style| style.flex().flex_col().items_start().p_2())
            .scrollbar_styles(|styles| styles.thumb(|thumb| thumb.bg(palette.muted).radius(px(3.))))
            .child(
                div()
                    .min_w_full()
                    .flex_shrink_0()
                    .font_family(palette.code_font_family)
                    .text_size(palette.font_size)
                    .line_height(relative(1.5))
                    .whitespace_nowrap()
                    .child(self.code.clone().test_text(self.code.clone())),
            );
        let pointer_focus = self.focus.clone();
        let keyboard_focus = self.focus.clone();
        let reveal_focus = self.focus.clone();
        let scroll = self.scroll;
        let body = div()
            .id((self.id, "code"))
            .test_support()
            .role(Role::Code)
            .aria_label(self.code)
            .track_focus(&self.focus.tab_stop(true))
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .border_t_1()
            .border_color(palette.border)
            .focus_visible(|style| style.border_color(palette.accent))
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                pointer_focus.focus(window, cx);
            })
            .on_key_down(move |event, window, cx| {
                if !keyboard_focus.is_focused(window)
                    || event.keystroke.modifiers.number_of_modifiers() != 0
                {
                    return;
                }
                let mut offset = scroll.offset();
                let maximum = scroll.max_offset();
                let line = (palette.font_size * 1.5).to_pixels(window.rem_size());
                let page = scroll.bounds().size.height;
                match event.keystroke.key.as_str() {
                    "left" => offset.x += line,
                    "right" => offset.x -= line,
                    "up" => offset.y += line,
                    "down" => offset.y -= line,
                    "pageup" => offset.y += page,
                    "pagedown" => offset.y -= page,
                    "home" => offset = gpui::Point::default(),
                    "end" => offset = -maximum,
                    _ => return,
                }
                offset.x = offset.x.clamp(-maximum.x, px(0.));
                offset.y = offset.y.clamp(-maximum.y, px(0.));
                scroll.set_offset(offset);
                window.refresh();
                cx.stop_propagation();
            })
            .child(area)
            .on_prepaint(move |bounds, window, _| {
                window.with_global_id("code-block-focus-reveal".into(), |id, window| {
                    window.with_element_state(id, |was_focused: Option<bool>, window| {
                        let focused = reveal_focus.is_focused(window);
                        if focused && was_focused != Some(true) {
                            window.request_autoscroll(bounds);
                        }
                        ((), focused)
                    });
                });
            });
        self.base.child(header).child(body)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Context, Entity, KeyDownEvent, KeyUpEvent, Keystroke, Pixels, Render,
        ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point,
    };
    use gpui_design::{Root, test_support};

    use super::super::super::Appearance;
    use super::*;

    struct Sample {
        appearance: Appearance,
        code: SharedString,
        width: Pixels,
        scroll: ScrollHandle,
        outer: ScrollHandle,
        focus: FocusHandle,
        before: FocusHandle,
        after: FocusHandle,
        unhandled: Rc<Cell<usize>>,
    }

    impl Render for Sample {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let unhandled = self.unhandled.clone();
            div()
                .on_key_down(move |_, _, _| unhandled.set(unhandled.get() + 1))
                .child(
                    ScrollArea::new("code-parent", &self.outer)
                        .w(self.width)
                        .h(px(250.))
                        .content_styles(|style| style.flex().flex_col().gap_2().p_3())
                        .child(
                            Button::new("before-code")
                                .track_focus(&self.before)
                                .h(px(32.))
                                .flex_shrink_0()
                                .child("Before"),
                        )
                        .child(CodeBlock::new(
                            "sample-code",
                            self.code.clone(),
                            "Configuration",
                            &self.scroll,
                            &self.focus,
                            self.appearance.palette(),
                        ))
                        .child(
                            Button::new("after-code")
                                .track_focus(&self.after)
                                .h(px(32.))
                                .flex_shrink_0()
                                .child("After"),
                        )
                        .child(div().h(px(100.)).flex_shrink_0()),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        appearance: Appearance,
        code: String,
    ) -> (&mut VisualTestContext, Entity<Sample>) {
        cx.update(gpui_design::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let sample = cx.new(|cx| Sample {
                appearance,
                code: code.into(),
                width: px(360.),
                scroll: ScrollHandle::new(),
                outer: ScrollHandle::new(),
                focus: cx.focus_handle(),
                before: cx.focus_handle(),
                after: cx.focus_handle(),
                unhandled: Rc::new(Cell::new(0)),
            });
            Root::new(sample, window, cx)
        });
        let sample = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Sample>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        draw(cx);
        (cx, sample)
    }

    fn part(name: &'static str) -> ElementId {
        (ElementId::from("sample-code"), name).into()
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn activate(cx: &mut VisualTestContext, key: &str) {
        let keystroke =
            cx.update(|_, cx| Keystroke::parse_with_os(key, cx.platform_info().os).unwrap());
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
        draw(cx);
    }

    #[gpui::test]
    fn copy_preserves_empty_unicode_and_line_endings_without_moving_focus(cx: &mut TestAppContext) {
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (cx, sample) = harness(cx, appearance, String::new());
            for source in [
                "",
                "설정 = \"긴 이름\"\r\n\tpath = \"C:\\recordings\"\n\nlast = true\n",
                "  updated = \"보존된 공백과 한글\"\r\n\r\n",
            ] {
                sample.update(cx, |sample, cx| {
                    sample.code = source.to_string().into();
                    sample.width = px(180.);
                    cx.notify();
                });
                cx.update(|window, cx| {
                    // TestAppContext uses an in-memory TestPlatform clipboard.
                    cx.write_to_clipboard(ClipboardItem::new_string("sentinel".into()));
                    let before = sample.read(cx).before.clone();
                    before.focus(window, cx);
                });
                draw(cx);
                cx.simulate_keystrokes("tab");
                draw(cx);
                cx.update(|window, _| {
                    let frame = test_support::find(window, &[], &"sample-code".into()).unwrap();
                    assert_eq!(frame.role(), Some(Role::Group));
                    assert_eq!(frame.label(), Some("Configuration"));
                    let viewport = test_support::find(window, &[], &part("scroll")).unwrap();
                    assert_eq!(viewport.role(), Some(Role::ScrollView));
                    assert_eq!(viewport.label(), Some("Configuration viewport"));
                    let copy = test_support::find(window, &[], &part("copy")).unwrap();
                    assert_eq!(copy.role(), Some(Role::Button));
                    assert_eq!(copy.label(), Some("Copy Configuration"));
                    assert_eq!(copy.focused(), Some(true));
                    let code = test_support::find(window, &[], &part("code")).unwrap();
                    assert_eq!(code.role(), Some(Role::Code));
                    assert_eq!(code.label(), Some(source));
                    let text = test_support::texts(window)
                        .into_iter()
                        .find(|text| text.path().contains(&part("code")))
                        .unwrap();
                    assert_eq!(text.text(), source);
                });
                for key in ["enter", "space"] {
                    activate(cx, key);
                    cx.update(|window, cx| {
                        // ClipboardItem::text() also returns None for an empty
                        // string entry; inspect the payload to prove it replaced
                        // the sentinel with the complete source, including "".
                        let clipboard = cx.read_from_clipboard().unwrap();
                        let [gpui::ClipboardEntry::String(entry)] = clipboard.entries.as_slice()
                        else {
                            panic!("copy must write exactly one string entry");
                        };
                        assert_eq!(entry.text.as_str(), source);
                        assert_eq!(
                            test_support::find(window, &[], &part("copy"))
                                .unwrap()
                                .focused(),
                            Some(true)
                        );
                    });
                }
                // The contained code hitbox must leave the header's copy button
                // reachable by pointer even when its caption wraps narrowly.
                let copy_bounds = cx.update(|window, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string("pointer sentinel".into()));
                    test_support::find(window, &[], &part("copy"))
                        .unwrap()
                        .bounds()
                });
                cx.simulate_click(copy_bounds.center(), gpui::Modifiers::default());
                draw(cx);
                cx.update(|_, cx| {
                    assert_eq!(
                        cx.read_from_clipboard(),
                        Some(ClipboardItem::new_string(source.to_string()))
                    );
                });
                cx.simulate_keystrokes("tab");
                draw(cx);
                cx.update(|window, cx| assert!(sample.read(cx).focus.is_focused(window)));
                cx.simulate_keystrokes("tab");
                draw(cx);
                cx.update(|window, cx| assert!(sample.read(cx).after.is_focused(window)));
                cx.simulate_keystrokes("shift-tab");
                draw(cx);
                cx.update(|window, cx| assert!(sample.read(cx).focus.is_focused(window)));
            }
        }
    }

    #[gpui::test]
    fn long_code_keeps_lines_and_contains_pointer_and_keyboard_scrolling(cx: &mut TestAppContext) {
        let source = (0..30)
            .map(|line| {
                format!(
                    "{line:02}: {}{}\n",
                    "긴한글설정".repeat(12),
                    "long_configuration_value_".repeat(8)
                )
            })
            .collect::<String>();
        for appearance in [Appearance::Compact, Appearance::Settings] {
            let (cx, sample) = harness(cx, appearance, source.clone());
            let mut wide_height = None;
            for width in [360., 180., 360.] {
                sample.update(cx, |sample, cx| {
                    sample.width = px(width);
                    cx.notify();
                });
                draw(cx);
                draw(cx);
                cx.update(|window, cx| {
                    let sample = sample.read(cx);
                    let code = test_support::find(window, &[], &part("code")).unwrap();
                    let frame = test_support::find(window, &[], &"sample-code".into()).unwrap();
                    assert!(frame.bounds().size.width <= px(width));
                    assert!(code.bounds().size.width > px(0.));
                    assert!(code.bounds().right() <= frame.bounds().right());
                    let text = test_support::texts(window)
                        .into_iter()
                        .find(|text| text.path().contains(&part("code")))
                        .unwrap();
                    assert_eq!(text.text(), source);
                    assert!(text.bounds().size.width > code.bounds().size.width);
                    assert!(sample.scroll.max_offset().x > px(0.));
                    assert!(sample.scroll.max_offset().y > px(0.));
                    match wide_height {
                        Some(height) => assert_eq!(text.bounds().size.height, height),
                        None => wide_height = Some(text.bounds().size.height),
                    }
                });
            }

            cx.update(|window, cx| {
                let focus = sample.read(cx).focus.clone();
                focus.focus(window, cx);
            });
            draw(cx);
            draw(cx);
            let (scroll, outer, unhandled) = sample.read_with(cx, |sample, _| {
                (
                    sample.scroll.clone(),
                    sample.outer.clone(),
                    sample.unhandled.clone(),
                )
            });
            let parent_offset = outer.offset();
            unhandled.set(0);
            cx.simulate_keystrokes("right down");
            draw(cx);
            assert!(scroll.offset().x < px(0.) && scroll.offset().y < px(0.));
            assert_eq!(outer.offset(), parent_offset);
            assert_eq!(unhandled.get(), 0);
            cx.simulate_keystrokes("end");
            draw(cx);
            assert_eq!(scroll.offset(), -scroll.max_offset());
            cx.simulate_keystrokes("right down pagedown");
            draw(cx);
            assert_eq!(scroll.offset(), -scroll.max_offset());
            assert_eq!(outer.offset(), parent_offset);
            cx.simulate_keystrokes("home left up pageup");
            draw(cx);
            assert_eq!(scroll.offset(), gpui::Point::default());
            cx.simulate_keystrokes("pagedown");
            draw(cx);
            assert_eq!(scroll.offset().y, -scroll.bounds().size.height);
            cx.simulate_keystrokes("ctrl-down escape");
            draw(cx);
            assert_eq!(unhandled.get(), 2);
            assert_eq!(outer.offset(), parent_offset);

            for delta in [point(px(-10_000.), px(0.)), point(px(0.), px(-10_000.))] {
                cx.simulate_event(ScrollEvent {
                    position: scroll.bounds().center(),
                    delta: ScrollDelta::Pixels(delta),
                    ..Default::default()
                });
                draw(cx);
                assert_eq!(outer.offset(), parent_offset);
            }
            assert_eq!(scroll.offset(), -scroll.max_offset());
            cx.simulate_event(ScrollEvent {
                position: scroll.bounds().center(),
                delta: ScrollDelta::Pixels(point(px(0.), px(-100.))),
                ..Default::default()
            });
            draw(cx);
            assert_eq!(scroll.offset(), -scroll.max_offset());
            assert_eq!(outer.offset(), parent_offset);
            cx.simulate_event(ScrollEvent {
                position: point(outer.bounds().left() + px(1.), outer.bounds().center().y),
                delta: ScrollDelta::Pixels(point(px(0.), px(-10_000.))),
                ..Default::default()
            });
            draw(cx);
            assert_eq!(outer.offset().y, -outer.max_offset().y);
            assert_eq!(scroll.offset(), -scroll.max_offset());
            cx.update(|window, cx| assert!(sample.read(cx).focus.is_focused(window)));
        }
    }
}
