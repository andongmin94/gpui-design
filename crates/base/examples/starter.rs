//! A small native application using only GPUI Design's public API.
//! Run `cargo run -p gpui-design --example starter --locked`.

use gpui::{
    AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_design::{
    Button, Field, Input, InputBase, InputEditorStyle, InputState, Root, input::InputEvent,
};

struct Starter {
    name: Entity<InputState>,
    greeting: SharedString,
    // GPUI subscriptions stay active while their handles are retained.
    _input_observation: Subscription,
    _input_events: Subscription,
}

impl Starter {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Ada");
            input.set_editor_style(InputEditorStyle {
                foreground: rgb(0xf1f5f9).into(),
                muted_foreground: rgb(0x94a3b8).into(),
                selection: rgb(0x284568).into(),
                caret: rgb(0x93c5fd).into(),
                ..Default::default()
            });
            input
        });
        let input_observation = cx.observe(&name, |_, _, cx| cx.notify());
        let input_events = cx.subscribe(&name, |this, _, event, cx| {
            if matches!(
                event,
                InputEvent::PressEnter {
                    secondary: false,
                    shift: false
                }
            ) {
                this.greet(cx);
            }
        });
        name.update(cx, |input, cx| input.focus(window, cx));
        Self {
            name,
            greeting: "Your greeting will appear here.".into(),
            _input_observation: input_observation,
            _input_events: input_events,
        }
    }

    fn greet(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value();
        if !name.trim().is_empty() {
            self.greeting = format!("Hello, {}!", name.trim()).into();
            cx.notify();
        }
    }
}

impl Render for Starter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.name.focus_handle(cx).is_focused(window);
        let empty = self.name.read(cx).value().trim().is_empty();
        let input = InputBase::new("name-input")
            .role(None::<Role>)
            .semantic_owner::<Input>(self.name.entity_id())
            .focused(focused)
            .h(px(40.))
            .w_full()
            .px_3()
            .border_1()
            .border_color(rgb(0x64748b))
            .rounded_md()
            .bg(rgb(0x0f172a))
            .styles(|styles| styles.focused(|style| style.border_color(rgb(0x93c5fd))))
            .child(Input::new(&self.name));

        // Presentation stays in application code; Field supplies the input semantics.
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .bg(rgb(0x0b1220))
            .text_color(rgb(0xf1f5f9))
            .text_size(px(16.))
            .child(
                div()
                    .w_full()
                    .max_w(px(440.))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .id("starter-title")
                            .role(Role::Heading)
                            .aria_level(1)
                            .aria_label("Hello, GPUI")
                            .text_2xl()
                            .child("Hello, GPUI"),
                    )
                    .child(
                        Field::new("name-field", &self.name, "Your name")
                            .description("Enter a name, then press Enter or Tab to Say hello.")
                            .required(true)
                            .styles(|styles| {
                                styles
                                    .description(|style| style.text_sm().text_color(rgb(0x94a3b8)))
                            })
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(input),
                    )
                    .child(
                        Button::new("say-hello")
                            .accessibility_label("Say hello")
                            .disabled(empty)
                            .h(px(40.))
                            .w_full()
                            .rounded_md()
                            .bg(rgb(0x255c90))
                            .hover(|style| style.bg(rgb(0x306fa9)))
                            .focus_visible(|style| {
                                style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220))
                            })
                            .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
                            .on_click(cx.listener(|this, _, _, cx| this.greet(cx)))
                            .child("Say hello"),
                    )
                    .child(
                        div()
                            .id("starter-greeting")
                            .role(Role::Label)
                            .aria_label(self.greeting.clone())
                            .child(self.greeting.clone()),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_design::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("GPUI Design · Starter".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::centered(size(px(520.), px(360.)), cx)),
                ..WindowOptions::default()
            },
            |window, cx| {
                let starter = cx.new(|cx| Starter::new(window, cx));
                cx.new(|cx| Root::new(starter, window, cx))
            },
        )
        .expect("failed to open starter window");
        cx.activate(true);
    });
}
