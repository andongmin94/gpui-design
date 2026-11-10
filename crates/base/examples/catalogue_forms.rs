//! A wholly local session-code flow using the retained form and modal primitives.

use gpui::{
    AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, SharedString, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_design::{
    Button, DialogBackdrop, DialogClose, DialogState, Drawer, DrawerState, InputOtp, InputOtpEvent,
    InputOtpState, Label, ScrollArea, Toast, ToastClose, ToastState,
};

const SESSION_CODE: &str = "742915";

pub struct CatalogueForms {
    otp: Entity<InputOtpState>,
    drawer: Entity<DrawerState>,
    toast: Entity<ToastState>,
    review_focus: FocusHandle,
    confirm_focus: FocusHandle,
    body_scroll: ScrollHandle,
    status: SharedString,
    confirmed: bool,
    _subscriptions: Vec<Subscription>,
}

impl CatalogueForms {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp = cx.new(|cx| InputOtpState::new(6, window, cx));
        let confirm_focus = cx.focus_handle();
        let dialog = cx.new(|cx| DialogState::new(cx).with_initial_focus(&confirm_focus));
        let drawer = cx.new(|cx| DrawerState::with_dialog(dialog, window, cx));
        let toast = cx.new(|cx| ToastState::new(window, cx));
        let subscriptions = vec![
            cx.observe(&otp, |_, _, cx| cx.notify()),
            cx.observe(&drawer, |_, _, cx| cx.notify()),
            cx.observe(&toast, |_, _, cx| cx.notify()),
            cx.subscribe(&otp, |this, _, _: &InputOtpEvent, cx| {
                this.confirmed = false;
                cx.notify();
            }),
        ];
        Self {
            otp,
            drawer,
            toast,
            review_focus: cx.focus_handle(),
            confirm_focus,
            body_scroll: ScrollHandle::new(),
            status: "Enter the displayed local code to continue.".into(),
            confirmed: false,
            _subscriptions: subscriptions,
        }
    }

    fn review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.otp.read(cx).value(cx).as_ref() == SESSION_CODE {
            self.drawer.update(cx, |state, cx| state.open(window, cx));
        } else {
            self.status = "The code does not match. Use the six digits shown above.".into();
            self.toast.update(cx, |state, cx| state.show(window, cx));
        }
        cx.notify();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirmed = true;
        self.status = "Session confirmed in this local example.".into();
        // Closing first lets the modal restore focus before Toast captures it.
        self.drawer.update(cx, |state, cx| state.close(window, cx));
        self.toast.update(cx, |state, cx| state.show(window, cx));
        cx.notify();
    }
}

impl Render for CatalogueForms {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input_focus = self.otp.read(cx).focus_handle(cx);
        let complete = self.otp.read(cx).is_complete(cx);
        let code = self.otp.read(cx).value(cx);
        let dialog = self.drawer.read(cx).dialog().clone();
        let status = self.status.clone();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .border_1()
            .border_color(rgb(0x334155))
            .rounded_lg()
            .child(div().text_lg().child("Session confirmation"))
            .child("Try numeric entry, paste, selection, Backspace and the arrow keys.")
            .child(format!("Local code: {SESSION_CODE}"))
            .child(Label::new(
                "session-code-label",
                &input_focus,
                "Session code",
            ))
            .child(
                InputOtp::new("session-code", &self.otp)
                    .accessibility_label("Session code")
                    .accessibility_description("Six digits. The local code shown above is 742915.")
                    .gap_2()
                    .slot(|slot, _, _| {
                        div()
                            .w(px(36.))
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(if slot.active || slot.selected {
                                rgb(0x93c5fd)
                            } else {
                                rgb(0x64748b)
                            })
                            .bg(if slot.active || slot.selected {
                                rgb(0x284568)
                            } else {
                                rgb(0x172033)
                            })
                            .rounded_md()
                            .when(slot.index == 2, |slot| slot.mr_2())
                            .child(slot.character.unwrap_or('_').to_string())
                    }),
            )
            .child(
                Button::new("review-session")
                    .track_focus(&self.review_focus)
                    .disabled(!complete)
                    .px_3()
                    .py_2()
                    .bg(rgb(0x255c90))
                    .rounded_md()
                    .hover(|style| style.bg(rgb(0x306fa9)))
                    .focus_visible(|style| style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220)))
                    .styles(|styles| styles.disabled(|style| style.opacity(0.45)))
                    .child("Review session")
                    .on_click(cx.listener(|this, _, window, cx| this.review(window, cx))),
            )
            .child(if self.confirmed {
                "This local session is confirmed. Editing the code starts a new confirmation."
            } else {
                "All activity stays in this window; no code is sent or remotely verified."
            })
            .child(
                Toast::new("session-notification", &self.toast)
                    .accessibility_label(status.clone())
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .bg(rgb(0x243c36))
                    .border_1()
                    .border_color(rgb(0x648a79))
                    .rounded_md()
                    .child(div().flex_1().child(status))
                    .child(
                        ToastClose::new("dismiss-session-notification", &self.toast).trigger(
                            |button| {
                                button
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .hover(|style| style.bg(rgb(0x334155)))
                                    .focus_visible(|style| {
                                        style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220))
                                    })
                                    .child("Dismiss")
                            },
                        ),
                    ),
            )
            .child(
                Drawer::new("session-confirmation", &self.drawer)
                    .accessibility_label("Confirm this local session")
                    .accessibility_description("Drag the handle down to cancel, or choose Confirm.")
                    .max_w(px(560.))
                    .bg(rgb(0x1c293c))
                    .text_color(rgb(0xf1f5f9))
                    .border_1()
                    .border_color(rgb(0x64748b))
                    .rounded_t_lg()
                    .backdrop(DialogBackdrop::new().size_full().bg(gpui::rgba(0x00000080)))
                    .handle(
                        div()
                            .h(px(32.))
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().w(px(48.)).h(px(5.)).rounded_full().bg(rgb(0x94a3b8))),
                    )
                    .child(
                        ScrollArea::new("session-review-body", &self.body_scroll)
                            .h(px(180.))
                            .content_styles(|style| style.flex().flex_col().gap_3().p_4())
                            .child(div().text_lg().child("Ready to confirm?"))
                            .child(format!("The entered code is {code}."))
                            .child("This changes only the retained state of this example.")
                            .child(
                                "The body keeps normal scrolling. Only the top grip starts a drag.",
                            )
                            .child("Escape, the backdrop and Cancel dismiss without confirming."),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .p_4()
                            .child(DialogClose::new(&dialog).trigger(|button| {
                                button
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .hover(|style| style.bg(rgb(0x334155)))
                                    .focus_visible(|style| {
                                        style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220))
                                    })
                                    .child("Cancel")
                            }))
                            .child(
                                Button::new("confirm-session")
                                    .track_focus(&self.confirm_focus)
                                    .px_3()
                                    .py_2()
                                    .bg(rgb(0x255c90))
                                    .rounded_md()
                                    .hover(|style| style.bg(rgb(0x306fa9)))
                                    .focus_visible(|style| {
                                        style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220))
                                    })
                                    .child("Confirm")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}
