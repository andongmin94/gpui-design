//! A wholly local session-code flow using the retained form and modal primitives.

#[path = "recipes/sheet.rs"]
mod sheet;
use sheet::Sheet;
pub(crate) use sheet::SheetSide;

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
    pub(crate) sheet: Entity<DialogState>,
    pub(crate) sheet_side: SheetSide,
    pub(crate) sheet_triggers: [FocusHandle; 4],
    sheet_close: FocusHandle,
    pub(crate) sheet_scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl CatalogueForms {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp = cx.new(|cx| InputOtpState::new(6, window, cx));
        let confirm_focus = cx.focus_handle();
        let dialog = cx.new(|cx| DialogState::new(cx).with_initial_focus(&confirm_focus));
        let drawer = cx.new(|cx| DrawerState::with_dialog(dialog, window, cx));
        let toast = cx.new(|cx| ToastState::new(window, cx));
        let sheet_close = cx.focus_handle();
        let sheet = cx.new(|cx| DialogState::new(cx).with_initial_focus(&sheet_close));
        let subscriptions = vec![
            cx.observe(&sheet, |_, _, cx| cx.notify()),
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
            sheet,
            sheet_side: SheetSide::Right,
            sheet_triggers: std::array::from_fn(|_| cx.focus_handle()),
            sheet_close,
            sheet_scroll: ScrollHandle::new(),
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
        let palette = super::Appearance::Compact.palette();
        let close_focus = self.sheet_close.clone();
        let help = Sheet::new("session-help", &self.sheet, &self.sheet_scroll, "Session help", palette)
            .side(self.sheet_side)
            .description("Keyboard guidance and the current local session state.")
            .footer(DialogClose::new(&self.sheet).trigger(move |button| button
                .accessibility_label("Close help")
                .track_focus(&close_focus).px_3().py_2().rounded_md()
                .bg(palette.field).text_color(palette.foreground)
                .focus_visible(|style| style.bg(palette.accent).text_color(palette.accent_foreground))
                .child("Close help")))
            .child(format!("Entered code: {}", if code.is_empty() { "Empty" } else { code.as_ref() }))
            .child(self.status.clone())
            .children([
                ("Enter digits", "Type the six-digit code displayed in the form. The editor accepts digits and keeps its native text selection."),
                ("Paste a code", "Paste digits into the editor. The existing selection is replaced using the same native editing behavior."),
                ("Edit a digit", "Use Left and Right to move the caret. Backspace or Delete removes digits without leaving the editor."),
                ("Select text", "Use the platform Select All shortcut or Shift with arrow keys. Copy and paste use the application's clipboard."),
                ("Review", "Tab to Review session and press Enter or Space. A matching code opens the confirmation drawer."),
                ("Confirm", "Choose Confirm in the drawer. The local example marks the session as confirmed and shows a notification."),
                ("Cancel", "Escape, Cancel or the drawer backdrop dismisses the review. Its grip can also be dragged down."),
                ("Keep context", "The form's code remains in its retained input state when help closes. No request or confirmation is sent to a server."),
            ].into_iter().map(|(title, body)| div().flex().flex_col().gap_1()
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
                .child(div().whitespace_normal().child(body))));
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
                    .accessibility_label("Review session")
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
                                    .accessibility_label("Dismiss")
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
                                    .accessibility_label("Cancel")
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
                                    .accessibility_label("Confirm")
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
            .child(
                div().flex().flex_wrap().gap_2().children(
                    [
                        (SheetSide::Top, "Help from top"),
                        (SheetSide::Right, "Help from right"),
                        (SheetSide::Bottom, "Help from bottom"),
                        (SheetSide::Left, "Help from left"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (side, label))| {
                        Button::new(("session-help-edge", index))
                            .accessibility_label(label)
                            .track_focus(&self.sheet_triggers[index])
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(palette.field)
                            .text_color(palette.foreground)
                            .hover(|style| style.bg(palette.highlight))
                            .focus_visible(|style| {
                                style
                                    .bg(palette.accent)
                                    .text_color(palette.accent_foreground)
                            })
                            .child(label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.sheet_side = side;
                                this.sheet.update(cx, |state, cx| state.open(window, cx));
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child(help)
    }
}
