//! Local, transport-free composition of Bubble, MessageScroller, and Questionnaire.

#[path = "recipes/bubble.rs"]
mod bubble;

use bubble::{Bubble, BubbleSide};
use gpui::{
    AnyElement, AppContext as _, Context, ElementId, Entity, FocusHandle, Focusable as _,
    FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Render, Role,
    SharedString, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_design::{
    Button, Input, InputAccessibility, InputBase, InputEditorStyle, InputState, Label,
    MessageScroller, MessageScrollerState, Questionnaire, QuestionnaireStage, QuestionnaireState,
};

struct LocalMessage {
    id: usize,
    body: SharedString,
    side: BubbleSide,
    time: SharedString,
    liked: bool,
}

pub(crate) struct ConversationCatalogue {
    messages: Vec<LocalMessage>,
    next_id: usize,
    scroller: Entity<MessageScrollerState>,
    compose: Entity<InputState>,
    answers: [Entity<InputState>; 2],
    questionnaire: QuestionnaireState<SharedString>,
    advance_focus: FocusHandle,
    back_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl ConversationCatalogue {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let scroller = cx.new(MessageScrollerState::new);
        let compose = Self::input("Message", "Write a local message", window, cx);
        let answers = [
            Self::input("Session title", "e.g. Recording review", window, cx),
            Self::input(
                "Review instructions",
                "What should the reviewer check?",
                window,
                cx,
            ),
        ];
        let subscriptions = vec![
            cx.observe(&scroller, |_, _, cx| cx.notify()),
            cx.observe(&compose, |_, _, cx| cx.notify()),
            cx.observe(&answers[0], |_, _, cx| cx.notify()),
            cx.observe(&answers[1], |_, _, cx| cx.notify()),
        ];
        Self {
            messages: (0..12).map(|id| LocalMessage {
                id,
                side: if id % 3 == 2 { BubbleSide::Outgoing } else { BubbleSide::Incoming },
                body: match id % 3 {
                    0 => "The recording is ready for review. Scroll up, then load history or receive a message to keep this reading position.",
                    1 => "Consecutive messages share an author. Reactions are ordinary application-owned Buttons.",
                    _ => "I will check the audio and captions. 질문의 답변은 뒤로 가도 유지됩니다.",
                }.into(),
                time: format!("10:{id:02}").into(), liked: false,
            }).collect(),
            next_id: 12, scroller, compose, answers,
            questionnaire: QuestionnaireState::new(2),
            advance_focus: cx.focus_handle(), back_focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    fn input(
        label: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder(placeholder.to_owned());
            let mut style = InputEditorStyle::default();
            style.foreground = rgb(0xf1f5f9).into();
            style.muted_foreground = rgb(0x94a3b8).into();
            style.caret = rgb(0x93c5fd).into();
            style.selection = rgb(0x1d4ed8).into();
            input.set_editor_style(style);
            input.set_accessibility(
                InputAccessibility::new(Some(Role::TextInput))
                    .with_label(Some(label.to_owned().into())),
            );
            input
        })
    }

    fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
        let label = label.into();
        Button::new(id)
            .accessibility_label(label.clone())
            .child(label)
            .px_3()
            .py_2()
            .border_1()
            .border_color(rgb(0x475569))
            .rounded(px(6.))
            .bg(rgb(0x1e293b))
            .text_color(rgb(0xf1f5f9))
            .hover(|style| style.bg(rgb(0x334155)))
            .focus_visible(|style| style.border_color(rgb(0x93c5fd)))
            .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
    }

    fn editor(id: impl Into<ElementId>, input: &Entity<InputState>) -> InputBase {
        InputBase::new(id)
            .role(None::<Role>)
            .semantic_owner::<Input>(input.entity_id())
            .h(px(38.))
            .w_full()
            .min_w_0()
            .px_2()
            .border_1()
            .border_color(rgb(0x64748b))
            .rounded(px(6.))
            .bg(rgb(0x0f172a))
            .child(Input::new(input))
    }

    fn push_message(&mut self, body: impl Into<SharedString>, side: BubbleSide) {
        self.messages.push(LocalMessage {
            id: self.next_id,
            body: body.into(),
            side,
            time: "Now · local".into(),
            liked: false,
        });
        self.next_id += 1;
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.compose.read(cx).value();
        if text.trim().is_empty() {
            return;
        }
        self.push_message(text, BubbleSide::Outgoing);
        self.compose.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        self.scroller
            .update(cx, |state, cx| state.jump_to_latest(window, cx));
        cx.notify();
    }

    fn focus_stage(&self, window: &mut Window, cx: &mut Context<Self>) {
        match self.questionnaire.stage() {
            QuestionnaireStage::Question(index) => {
                self.answers[index].focus_handle(cx).focus(window, cx)
            }
            QuestionnaireStage::Review => self.advance_focus.focus(window, cx),
            QuestionnaireStage::Complete => self.back_focus.focus(window, cx),
        }
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.questionnaire.stage() {
            QuestionnaireStage::Question(index) => {
                let value = self.answers[index].read(cx).value();
                // These rules belong to this example, not the headless controller.
                let validation = if value.trim().is_empty() {
                    Err("Enter an answer before continuing.".into())
                } else if index == 0 && value.chars().count() > 60 {
                    Err("Use a session title of at most 60 characters.".into())
                } else {
                    Ok(())
                };
                let accepted = self.questionnaire.submit_answer(value, validation);
                let error = self.questionnaire.error().cloned();
                self.answers[index].update(cx, |input, _| {
                    input.set_accessibility(
                        InputAccessibility::new(Some(Role::TextInput))
                            .with_label(Some(
                                if index == 0 {
                                    "Session title"
                                } else {
                                    "Review instructions"
                                }
                                .into(),
                            ))
                            .with_required(true)
                            .with_invalid(error.is_some())
                            .with_description(error),
                    );
                });
                if !accepted {
                    self.answers[index].focus_handle(cx).focus(window, cx);
                }
            }
            QuestionnaireStage::Review => {
                if self.questionnaire.complete() {
                    let title = self.questionnaire.answer(0).unwrap();
                    let instructions = self.questionnaire.answer(1).unwrap();
                    self.push_message(
                        format!("Review request: {title}\n{instructions}"),
                        BubbleSide::Outgoing,
                    );
                    self.scroller
                        .update(cx, |state, cx| state.jump_to_latest(window, cx));
                }
            }
            QuestionnaireStage::Complete => return,
        }
        self.focus_stage(window, cx);
        cx.notify();
    }

    fn render_conversation(&self, cx: &mut Context<Self>) -> AnyElement {
        let toolbar = div().flex().flex_wrap().gap_2()
            .child(Self::button("conversation-history", "Load older history").on_click(cx.listener(|this, _, _, cx| {
                let older = LocalMessage { id: this.next_id, body: "An earlier message loaded above the current reading position. Its height may differ from newer messages.".into(), side: BubbleSide::Incoming, time: "Earlier".into(), liked: false };
                this.next_id += 1;
                this.messages.insert(0, older);
                cx.notify();
            })))
            .child(Self::button("conversation-receive", "Receive message").on_click(cx.listener(|this, _, _, cx| {
                this.push_message("New local message. If you were reading history, the unread counter increases without moving that message out of view.", BubbleSide::Incoming);
                cx.notify();
            })))
            .child(Self::button("conversation-stream", "Append stream chunk").on_click(cx.listener(|this, _, _, cx| {
                if let Some(message) = this.messages.last_mut() {
                    message.body = format!("{} More streamed text grows the same message without counting it twice.", message.body).into();
                }
                cx.notify();
            })));
        let mut area = MessageScroller::new("conversation-messages", &self.scroller, cx)
            .accessibility_label("Local conversation history")
            .h(px(340.))
            .w_full()
            .content_styles(|style| style.flex().flex_col().gap_1().p_2())
            .border_1()
            .border_color(rgb(0x334155))
            .rounded(px(8.));
        for (index, message) in self.messages.iter().enumerate() {
            let id = message.id;
            let grouped = index > 0 && self.messages[index - 1].side == message.side;
            let author = if message.side == BubbleSide::Outgoing {
                "You"
            } else {
                "Reviewer"
            };
            let reaction = Self::button(
                ("conversation-reaction", id),
                if message.liked { "Liked · 1" } else { "Like" },
            )
            .selected(message.liked)
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(message) = this.messages.iter_mut().find(|message| message.id == id) {
                    message.liked = !message.liked;
                }
                cx.notify();
            }));
            area = area.item(
                id,
                Bubble::new(
                    ("conversation-bubble", id),
                    author,
                    message.time.clone(),
                    message.body.clone(),
                    message.side,
                )
                .grouped(grouped)
                .action(reaction),
            );
        }
        let unread = self.scroller.read(cx).unread_count();
        let following = self.scroller.read(cx).is_following();
        let jump = Self::button(
            "conversation-jump",
            if unread > 0 {
                format!("{unread} new · Jump to latest")
            } else {
                "Jump to latest".to_string()
            },
        )
        .disabled(following)
        .on_click(cx.listener(|this, _, window, cx| {
            this.scroller
                .update(cx, |state, cx| state.jump_to_latest(window, cx))
        }));
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Bubble + Message Scroller"),
            )
            .child(toolbar)
            .child(area)
            .child(div().flex().justify_end().child(jump))
            .child(Label::new(
                "conversation-compose-label",
                &self.compose.focus_handle(cx),
                "Message",
            ))
            .child(Self::editor("conversation-compose", &self.compose))
            .child(
                Self::button("conversation-send", "Send local message")
                    .disabled(self.compose.read(cx).value().trim().is_empty())
                    .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))),
            )
            .into_any_element()
    }

    fn render_questionnaire(&self, cx: &mut Context<Self>) -> AnyElement {
        let stage = self.questionnaire.stage();
        let label = match stage {
            QuestionnaireStage::Question(index) => format!("Question {} of 2", index + 1),
            QuestionnaireStage::Review => "Review your answers".to_string(),
            QuestionnaireStage::Complete => "Review request completed".to_string(),
        };
        let mut content = Questionnaire::new("conversation-questionnaire", label.clone())
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_1()
            .border_color(rgb(0x334155))
            .rounded(px(8.))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Questionnaire · create a review request"),
            )
            .child(label);
        match stage {
            QuestionnaireStage::Question(index) => {
                content = content
                    .child(Label::new(
                        ("question-label", index),
                        &self.answers[index].focus_handle(cx),
                        if index == 0 {
                            "Session title"
                        } else {
                            "Review instructions"
                        },
                    ))
                    .child(Self::editor(
                        ("question-editor", index),
                        &self.answers[index],
                    ))
                    .when_some(self.questionnaire.error().cloned(), |content, error| {
                        content.child(
                            div()
                                .text_color(rgb(0xfca5a5))
                                .child(gpui::text!(id = "question-error", error)),
                        )
                    });
            }
            QuestionnaireStage::Review | QuestionnaireStage::Complete => {
                content = content
                    .child(gpui::text!(
                        id = "question-review-session",
                        format!("Session: {}", self.questionnaire.answer(0).unwrap())
                    ))
                    .child(gpui::text!(
                        id = "question-review-instructions",
                        format!("Instructions: {}", self.questionnaire.answer(1).unwrap())
                    ));
                if stage == QuestionnaireStage::Complete {
                    content = content.child(gpui::text!(id = "question-completed", "Added to the local conversation. Back reopens the review with both answers retained."));
                }
            }
        }
        content
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Self::button("question-back", "Back")
                            .track_focus(&self.back_focus)
                            .disabled(!self.questionnaire.can_go_back())
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.questionnaire.back() {
                                    this.focus_stage(window, cx);
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        Self::button(
                            "question-next",
                            match stage {
                                QuestionnaireStage::Question(_) => "Continue",
                                QuestionnaireStage::Review => "Complete request",
                                QuestionnaireStage::Complete => "Completed",
                            },
                        )
                        .track_focus(&self.advance_focus)
                        .disabled(stage == QuestionnaireStage::Complete)
                        .on_click(cx.listener(|this, _, window, cx| this.advance(window, cx))),
                    ),
            )
            .into_any_element()
    }
}

impl Render for ConversationCatalogue {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_4()
            .text_size(px(14.))
            .text_color(rgb(0xf1f5f9))
            .child(self.render_conversation(cx))
            .child(self.render_questionnaire(cx))
    }
}
