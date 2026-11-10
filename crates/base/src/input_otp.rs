//! Segmented numeric entry with one retained native text editor.

use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable as _, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    RenderOnce, Role, SharedString, StyleRefinement, Styled, Subscription, Window, div,
};

use crate::{
    Input, InputBase, TestSupportExt as _,
    input::{InputAccessibility, InputEvent, InputState},
};

/// A committed user edit. Completion means all slots contain a digit, not that
/// the application has verified the code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputOtpEvent {
    pub value: SharedString,
    pub complete: bool,
}

/// One ASCII-digit code, backed by the same editor as [`Input`].
///
/// Paste, selection, navigation, undo, deletion, IME and native accessibility
/// belong to that editor. A paste is accepted only when its entire resulting
/// value is numeric and fits the slots; excess text is never silently dropped.
/// The component does not send or verify codes.
pub struct InputOtpState {
    input: Entity<InputState>,
    length: usize,
    _subscriptions: Vec<Subscription>,
}

impl InputOtpState {
    /// Creates a fixed number of digit slots. `length` must be nonzero.
    pub fn new(length: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        assert!(length > 0, "InputOtp needs at least one digit");
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .validate(move |value, _| {
                    value.len() <= length && value.bytes().all(|byte| byte.is_ascii_digit())
                })
                .placeholder("")
        });
        let subscriptions = vec![
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.subscribe(&input, |state, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value();
                    cx.emit(InputOtpEvent {
                        complete: value.len() == state.length,
                        value,
                    });
                }
            }),
        ];
        Self {
            input,
            length,
            _subscriptions: subscriptions,
        }
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn value(&self, cx: &App) -> SharedString {
        self.input.read(cx).value()
    }

    pub fn is_complete(&self, cx: &App) -> bool {
        self.value(cx).len() == self.length
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }

    /// Access to the existing editor for selection, focus, native text input
    /// integration and `InputEvent::PressEnter` subscriptions. Keep its numeric
    /// validator intact; change the code through [`Self::set_value`].
    pub fn input(&self) -> &Entity<InputState> {
        &self.input
    }

    /// Replaces the code without emitting a user edit or preserving undo.
    /// Returns false for non-ASCII digits or a value longer than the slots.
    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let value = value.into();
        if value.len() > self.length || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        self.input
            .update(cx, |input, cx| input.set_value(value, window, cx));
        cx.notify();
        true
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_disabled(disabled, cx));
    }

    pub fn set_readonly(&mut self, readonly: bool, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_readonly(readonly, cx));
    }
}

impl EventEmitter<InputOtpEvent> for InputOtpState {}

/// Presentation facts for one slot. Slots never own independent editor or
/// focus state. Use `index` to add application-specific grouping and spacing.
#[derive(Clone, Copy, Debug)]
pub struct InputOtpSlot {
    pub index: usize,
    pub character: Option<char>,
    pub active: bool,
    pub selected: bool,
    pub disabled: bool,
    pub readonly: bool,
}

type SlotRenderer = Rc<dyn Fn(InputOtpSlot, &mut Window, &mut App) -> AnyElement>;

/// A single accessible input with application-rendered visual digit slots.
///
/// The native editor spans the frame with transparent paint. The visual slots
/// delegate clicks to its selection and display its current value/caret state;
/// they have no separate text-input handlers or Tab stops. Supply noninteractive
/// slot content, including its active/selected appearance, using [`Self::slot`].
/// All frame and slot colors, dimensions and grouping belong to the consumer.
#[derive(IntoElement)]
pub struct InputOtp {
    id: ElementId,
    base: InputBase,
    state: Entity<InputOtpState>,
    label: Option<SharedString>,
    description: Option<SharedString>,
    slot: SlotRenderer,
}

impl InputOtp {
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputOtpState>) -> Self {
        let id = id.into();
        Self {
            base: InputBase::new(id.clone()).role(None::<Role>),
            id,
            state: state.clone(),
            label: None,
            description: None,
            slot: Rc::new(|slot, _, _| {
                div()
                    .child(slot.character.unwrap_or('_').to_string())
                    .into_any_element()
            }),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn accessibility_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn slot<R: IntoElement>(
        mut self,
        render: impl Fn(InputOtpSlot, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.slot = Rc::new(move |slot, window, cx| render(slot, window, cx).into_any_element());
        self
    }
}

impl Styled for InputOtp {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for InputOtp {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let length = model.length;
        let input = model.input.clone();
        let value = input.read(cx).value();
        let selection = input.read(cx).selected_range();
        let cursor = input.read(cx).cursor();
        let presentation = input.read(cx).presentation();
        let disabled = presentation.is_disabled();
        let readonly = presentation.is_readonly();
        let focus = input.focus_handle(cx);
        let focused = focus.is_focused(window);
        input.update(cx, |input, _| {
            let mut accessibility = input
                .accessibility()
                .cloned()
                .unwrap_or_else(|| InputAccessibility::new(Some(Role::TextInput)));
            if let Some(label) = self.label {
                accessibility = accessibility.with_label(Some(label));
            }
            if let Some(description) = self.description {
                accessibility = accessibility.with_description(Some(description));
            }
            input.set_accessibility(accessibility);
        });
        let slots = (0..length).map(|index| {
            let input = input.clone();
            let slot = InputOtpSlot {
                index,
                character: value.as_bytes().get(index).map(|byte| char::from(*byte)),
                active: focused && cursor.min(length - 1) == index,
                selected: focused && selection.contains(&index),
                disabled,
                readonly,
            };
            div()
                .id((self.id.clone(), index.to_string()))
                .test_support()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    input.update(cx, |input, cx| {
                        if !input.presentation().is_disabled() {
                            input.focus(window, cx);
                            let length = input.value().len();
                            let start = index.min(length);
                            input.set_selected_range(start..(start + 1).min(length), cx);
                        }
                    });
                })
                .child((self.slot)(slot, window, cx))
        });
        self.base
            .semantic_owner::<Input>(input.entity_id())
            .focused(focused)
            .disabled(disabled)
            .relative()
            .flex()
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .size_full()
                    .opacity(0.)
                    .child(Input::new(&input)),
            )
            .children(slots)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{ClipboardItem, Modifiers, Render, TestAppContext, px};

    use super::*;

    struct Harness {
        state: Entity<InputOtpState>,
        after: FocusHandle,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    InputOtp::new("otp", &self.state)
                        .accessibility_label("Verification code")
                        .slot(|slot, _, _| {
                            div()
                                .debug_selector(move || format!("otp-slot-{}", slot.index))
                                .w(px(36.))
                                .h(px(40.))
                                .child(slot.character.unwrap_or('_').to_string())
                        }),
                )
                .child(
                    crate::Button::new("after")
                        .track_focus(&self.after)
                        .child("After"),
                )
        }
    }

    #[gpui::test]
    fn one_editor_owns_paste_selection_deletion_and_slot_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let state = cx.new(|cx| InputOtpState::new(6, window, cx));
                Harness {
                    _subscription: cx.observe(&state, |_, _, cx| cx.notify()),
                    state,
                    after: cx.focus_handle(),
                }
            });
            crate::Root::new(view, window, cx)
        });
        let view = root
            .read_with(cx, |root, _| root.view().clone())
            .downcast::<Harness>()
            .unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            view.read(cx)
                .state
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
            window.draw(cx).clear(cx);
            cx.write_to_clipboard(ClipboardItem::new_string("123456".into()));
        });
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-v"
        } else {
            "ctrl-v"
        });
        cx.simulate_input("7");
        cx.simulate_input("x");
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(cx), "123456"));
        cx.simulate_keystrokes("left backspace");
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(cx), "12346"));
        cx.simulate_input("9");
        cx.update(|window, cx| {
            assert_eq!(view.read(cx).state.read(cx).value(cx), "123496");
            assert!(view.read(cx).state.read(cx).is_complete(cx));
            window.draw(cx).clear(cx);
        });
        let slot = cx.debug_bounds("otp-slot-1").unwrap().center();
        cx.simulate_click(slot, Modifiers::default());
        cx.simulate_input("8");
        cx.update(|_, cx| assert_eq!(view.read(cx).state.read(cx).value(cx), "183496"));
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            assert!(
                view.read(cx).after.is_focused(window),
                "slots add no Tab stops"
            );
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.set_disabled(true, cx));
            window.draw(cx).clear(cx);
        });
        cx.simulate_click(slot, Modifiers::default());
        cx.update(|window, cx| assert!(view.read(cx).after.is_focused(window)));
    }
}
