//! A bottom modal surface with a dedicated drag-to-dismiss handle.

use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, EventEmitter, FocusHandle,
    InteractiveElement as _, IntoElement, MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement,
    Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Subscription, Window, canvas, div,
    prelude::FluentBuilder as _, px,
};

use crate::{
    Dialog, DialogEvent, DialogPopup, DialogState, ElementExt as _, StyledExt as _,
    TestSupportExt as _,
};

/// Retained pointer geometry around the existing modal lifecycle.
///
/// DialogState remains the sole owner of open state, focus trapping, initial
/// focus and restoration. Only presses on the dedicated handle start dragging;
/// scrolling and editing inside the body retain their normal native behavior.
pub struct DrawerState {
    dialog: Entity<DialogState>,
    drag_start: Option<Pixels>,
    offset: Pixels,
    height: Pixels,
    _subscriptions: Vec<Subscription>,
}

impl DrawerState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_dialog(cx.new(DialogState::new), window, cx)
    }

    /// Adopts an existing modal state, including its configured initial focus.
    /// Mount that state in this Drawer only, not in a second Dialog as well.
    pub fn with_dialog(
        dialog: Entity<DialogState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = vec![
            cx.observe(&dialog, |_, _, cx| cx.notify()),
            cx.subscribe(&dialog, |state, _, event: &DialogEvent, cx| {
                if matches!(event, DialogEvent::Closed(_)) {
                    state.drag_start = None;
                    state.offset = px(0.);
                }
                cx.emit(*event);
                cx.notify();
            }),
            cx.observe_window_activation(window, |state, window, cx| {
                if !window.is_window_active() {
                    state.cancel_drag(window, cx);
                }
            }),
        ];
        Self {
            dialog,
            drag_start: None,
            offset: px(0.),
            height: px(0.),
            _subscriptions: subscriptions,
        }
    }

    pub fn dialog(&self) -> &Entity<DialogState> {
        &self.dialog
    }

    pub fn is_open(&self, cx: &App) -> bool {
        self.dialog.read(cx).is_open()
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.dialog.read(cx).focus_handle().clone()
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_drag(window, cx);
        self.dialog.update(cx, |dialog, cx| dialog.open(window, cx));
    }

    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_drag(window, cx);
        self.dialog
            .update(cx, |dialog, cx| dialog.close(window, cx));
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_start.is_some()
    }

    pub fn drag_offset(&self) -> Pixels {
        self.offset
    }

    fn start_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        if self.is_open(cx) {
            self.drag_start = Some(y);
            self.offset = px(0.);
            cx.notify();
        }
    }

    fn drag_to(&mut self, y: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(start) = self.drag_start {
            self.offset = (y - start).clamp(px(0.), self.height);
            cx.notify();
            window.refresh();
        }
    }

    fn finish_drag(&mut self, y: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        if self.drag_start.is_none() {
            return;
        }
        self.drag_to(y, window, cx);
        // A deliberate downward drag through a third of the panel dismisses;
        // a shorter drag snaps back. A click never dismisses a zero-height panel.
        let dismiss = self.height > px(0.) && self.offset >= self.height / 3.;
        self.cancel_drag(window, cx);
        if dismiss {
            self.dialog
                .update(cx, |dialog, cx| dialog.close(window, cx));
        }
    }

    fn cancel_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.drag_start.take().is_some() || self.offset != px(0.) {
            self.offset = px(0.);
            cx.notify();
            window.refresh();
        }
    }
}

impl EventEmitter<DialogEvent> for DrawerState {}

/// A bottom-aligned modal whose panel follows downward pointer movement.
///
/// [`Styled`] controls the panel, and normal children form its body. Supply a
/// visible, noninteractive grip with [`Self::handle`]; its entire bounds become
/// the drag target. Native mouse capture keeps tracking outside that target.
/// Releasing past one third of the panel height closes the modal; shorter or
/// cancelled drags reset it. Keyboard users keep Dialog's Escape and focus
/// trap, and may use a regular `DialogClose` with [`DrawerState::dialog`].
///
/// Placement and clipping are structural. The application owns colors, panel
/// dimensions, a backdrop, content scrolling and the handle's appearance.
#[derive(IntoElement)]
pub struct Drawer {
    id: ElementId,
    state: Entity<DrawerState>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    handle: Option<AnyElement>,
    backdrop: Option<AnyElement>,
    label: Option<SharedString>,
    description: Option<SharedString>,
    escape: bool,
    backdrop_closable: bool,
}

impl Drawer {
    pub fn new(id: impl Into<ElementId>, state: &Entity<DrawerState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            handle: None,
            backdrop: None,
            label: None,
            description: None,
            escape: true,
            backdrop_closable: true,
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

    pub fn handle(mut self, handle: impl IntoElement) -> Self {
        self.handle = Some(handle.into_any_element());
        self
    }

    pub fn backdrop(mut self, backdrop: impl IntoElement) -> Self {
        self.backdrop = Some(backdrop.into_any_element());
        self
    }

    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.escape = close;
        self
    }

    pub fn close_on_backdrop_press(mut self, close: bool) -> Self {
        self.backdrop_closable = close;
        self
    }
}

impl Styled for Drawer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Drawer {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl RenderOnce for Drawer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let model = self.state.read(cx);
        let dialog = model.dialog.clone();
        let offset = model.offset;
        let drag = self.state.clone();
        let geometry = self.state.clone();
        let movement = self.state.clone();
        let release = self.state;
        let panel = DialogPopup::new()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .max_w_full()
            .max_h_full()
            .refine_style(&self.style)
            .top(offset)
            .on_prepaint(move |bounds, _, cx| {
                geometry.update(cx, |state, _| state.height = bounds.size.height);
            })
            .when_some(self.handle, |this, handle| {
                this.child(
                    div()
                        .id((self.id, "handle"))
                        .test_support()
                        .flex_shrink_0()
                        .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                            cx.stop_propagation();
                            drag.update(cx, |state, cx| state.start_drag(event.position.y, cx));
                        })
                        .child(handle),
                )
            })
            .children(self.children)
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, _| {
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase.capture() && movement.read(cx).is_dragging() {
                                movement.update(cx, |state, cx| {
                                    if event.dragging() {
                                        state.drag_to(event.position.y, window, cx);
                                    } else {
                                        state.cancel_drag(window, cx);
                                    }
                                });
                                cx.stop_propagation();
                            }
                        });
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                            if phase.capture()
                                && event.button == MouseButton::Left
                                && release.read(cx).is_dragging()
                            {
                                release.update(cx, |state, cx| {
                                    state.finish_drag(event.position.y, window, cx);
                                });
                                cx.stop_propagation();
                            }
                        });
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            );
        Dialog::new(&dialog)
            .items_end()
            .overflow_hidden()
            .close_on_escape(self.escape)
            .close_on_backdrop_press(self.backdrop_closable)
            .when_some(self.label, |this, label| this.accessibility_label(label))
            .when_some(self.description, |this, description| {
                this.accessibility_description(description)
            })
            .when_some(self.backdrop, |this, backdrop| this.backdrop(backdrop))
            .popup(panel)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Modifiers, Render, TestAppContext, point};

    use super::*;

    struct Harness {
        state: Entity<DrawerState>,
        trigger: FocusHandle,
        _subscription: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    crate::Button::new("open")
                        .track_focus(&self.trigger)
                        .child("Open"),
                )
                .child(
                    Drawer::new("drawer", &self.state)
                        .accessibility_label("Session review")
                        .h(px(240.))
                        .w(px(320.))
                        .backdrop(div().size_full())
                        .handle(
                            div()
                                .debug_selector(|| "drawer-grip".into())
                                .h(px(30.))
                                .w_full()
                                .child("Drag down"),
                        )
                        .child(
                            crate::Button::new("body-action")
                                .debug_selector(|| "body-action".into())
                                .h(px(40.))
                                .child("Body action"),
                        ),
                )
        }
    }

    #[gpui::test]
    fn only_handle_drag_dismisses_after_threshold_and_returns_modal_focus(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let state = cx.new(|cx| DrawerState::new(window, cx));
                Harness {
                    _subscription: cx.observe(&state, |_, _, cx| cx.notify()),
                    state,
                    trigger: cx.focus_handle(),
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
            view.read(cx).trigger.clone().focus(window, cx);
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.open(window, cx));
            window.draw(cx).clear(cx);
        });
        let handle = cx.debug_bounds("drawer-grip").unwrap().center();
        let short = point(handle.x, handle.y + px(30.));
        cx.simulate_mouse_down(handle, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(short, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_up(short, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            let state = view.read(cx).state.read(cx);
            assert!(state.is_open(cx));
            assert_eq!(state.drag_offset(), px(0.));
            window.draw(cx).clear(cx);
        });
        let body = cx.debug_bounds("body-action").unwrap().center();
        let end = point(handle.x, handle.y + px(120.));
        cx.simulate_mouse_down(body, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            assert!(view.read(cx).state.read(cx).is_open(cx));
            window.draw(cx).clear(cx);
        });
        cx.simulate_mouse_down(handle, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            assert!(!view.read(cx).state.read(cx).is_open(cx));
            assert!(!view.read(cx).state.read(cx).is_dragging());
            assert!(view.read(cx).trigger.is_focused(window));
        });
        cx.simulate_resize(gpui::size(px(260.), px(180.)));
        cx.update(|window, cx| {
            let state = view.read(cx).state.clone();
            state.update(cx, |state, cx| state.open(window, cx));
            window.draw(cx).clear(cx);
            assert!(
                state.read(cx).height <= window.viewport_size().height,
                "the drawer must fit the viewport so its grip stays reachable"
            );
        });
        assert!(cx.debug_bounds("drawer-grip").unwrap().top() >= px(0.));
    }
}
