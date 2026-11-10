//! A keyed conversation viewport over the ordinary, nonvirtualized ScrollArea.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Context, Element, ElementId, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ParentElement as _, Pixels, RenderOnce,
    ScrollHandle, SharedString, StyleRefinement, Styled, Window, px,
};

use crate::{ScrollArea, ScrollbarMode, ScrollbarStyles, StateStyle};

struct Frame {
    ids: Vec<ElementId>,
    top: Pixels,
}

/// Retained scrolling state for one conversation in one viewport.
///
/// Content and stable message identities belong to the application. Reuse this
/// state across appends, streamed edits, and prepended history; create a new state
/// when switching conversations. The first frame starts at the latest message.
pub struct MessageScrollerState {
    scroll: ScrollHandle,
    frame: Option<Frame>,
    following: bool,
    unread: usize,
    jump: bool,
}

impl MessageScrollerState {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            scroll: ScrollHandle::new(),
            frame: None,
            following: true,
            unread: 0,
            jump: false,
        }
    }

    pub fn scroll_handle(&self) -> &ScrollHandle {
        &self.scroll
    }

    /// Whether the last laid-out viewport was at its bottom edge.
    pub fn is_following(&self) -> bool {
        self.following
    }

    /// Appended messages received while the reader was away from the bottom.
    /// Edits to an existing message and prepended history do not increase this.
    pub fn unread_count(&self) -> usize {
        self.unread
    }

    /// Returns to the latest content and resumes following subsequent growth.
    /// The application's jump control retains focus; messages do not receive it.
    pub fn jump_to_latest(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.jump = true;
        self.following = true;
        self.unread = 0;
        self.scroll.scroll_to_bottom();
        cx.notify();
        window.refresh();
    }
}

/// An unstyled vertical message viewport with stable keyed scroll anchoring.
///
/// Give it a bounded height and add each message with [`Self::item`]. IDs must be
/// unique and stable within a conversation, including when loading older history.
/// Each item is a direct child of the existing ScrollArea. No virtual list,
/// transport, message model, new keyboard scope, or live region is introduced.
///
/// At the bottom, append/stream/resize follows the latest content. Away from it,
/// the first visible message keeps its pixel position even when earlier messages
/// are inserted or change height. If that message is removed, native scrolling
/// clamps the existing offset. Observe the retained state to update an app-owned
/// unread badge or jump Button. User scrolling back to the bottom clears unread.
#[derive(IntoElement)]
pub struct MessageScroller {
    area: ScrollArea,
    state: Entity<MessageScrollerState>,
    items: Vec<(ElementId, Rc<Cell<Option<LayoutId>>>)>,
}

impl MessageScroller {
    pub fn new(id: impl Into<ElementId>, state: &Entity<MessageScrollerState>, cx: &App) -> Self {
        Self {
            area: ScrollArea::new(id, state.read(cx).scroll_handle())
                .content_styles(|style| style.flex().flex_col()),
            state: state.clone(),
            items: Vec::new(),
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.area = self.area.accessibility_label(label);
        self
    }

    pub fn content_styles(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.area = self.area.content_styles(build);
        self
    }

    pub fn scrollbar_mode(mut self, mode: ScrollbarMode) -> Self {
        self.area = self.area.scrollbar_mode(mode);
        self
    }

    pub fn scrollbar_styles(
        mut self,
        build: impl FnOnce(ScrollbarStyles) -> ScrollbarStyles,
    ) -> Self {
        self.area = self.area.scrollbar_styles(build);
        self
    }

    pub fn item(mut self, id: impl Into<ElementId>, content: impl IntoElement) -> Self {
        let layout = Rc::new(Cell::new(None));
        self.items.push((id.into(), layout.clone()));
        self.area = self.area.child(MeasuredItem {
            child: content.into_any_element(),
            layout,
        });
        self
    }
}

impl Styled for MessageScroller {
    fn style(&mut self) -> &mut StyleRefinement {
        self.area.style()
    }
}

impl RenderOnce for MessageScroller {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        AnchoredArea {
            area: self.area.into_any_element(),
            state: self.state,
            items: self.items,
        }
    }
}

// Read the new item layout before ScrollArea prepaints. This lets its existing
// viewport clamp and paint the corrected offset in the same frame, avoiding an
// intermediate frame at the wrong reading position when history is prepended.
struct AnchoredArea {
    area: AnyElement,
    state: Entity<MessageScrollerState>,
    items: Vec<(ElementId, Rc<Cell<Option<LayoutId>>>)>,
}

impl IntoElement for AnchoredArea {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for AnchoredArea {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.area.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.state.update(cx, |state, cx| {
            let was_following = state.following;
            let old_unread = state.unread;
            let offset = state.scroll.offset();
            let following = state.jump
                || state.frame.is_none()
                || (offset.y + state.scroll.max_offset().y).abs() <= px(1.);
            if let Some(frame) = &state.frame {
                if !following {
                    if let Some(last) = frame.ids.last()
                        && let Some(index) = self.items.iter().position(|(id, _)| id == last)
                    {
                        state.unread += self.items.len() - index - 1;
                    }
                    let index = state.scroll.top_item();
                    if let Some(id) = frame.ids.get(index)
                        && let Some(old) = state.scroll.bounds_for_item(index)
                        && let Some((_, layout)) = self.items.iter().find(|(key, _)| key == id)
                        && let Some(layout) = layout.get()
                    {
                        let new = window.layout_bounds(layout);
                        let mut anchored = offset;
                        anchored.y += (old.top() - frame.top) - (new.top() - bounds.top());
                        state.scroll.set_offset(anchored);
                    }
                }
            }
            if following {
                state.scroll.scroll_to_bottom();
                state.unread = 0;
            }
            state.following = following;
            state.jump = false;
            state.frame = Some(Frame {
                ids: self.items.iter().map(|(id, _)| id.clone()).collect(),
                top: bounds.top(),
            });
            if old_unread != state.unread || was_following != state.following {
                cx.notify();
            }
        });
        self.area.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.area.paint(window, cx);
    }
}

struct MeasuredItem {
    child: AnyElement,
    layout: Rc<Cell<Option<LayoutId>>>,
}

impl IntoElement for MeasuredItem {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for MeasuredItem {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout = self.child.request_layout(window, cx);
        self.layout.set(Some(layout));
        (layout, ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Context, Render, TestAppContext, div, point};

    struct Conversation {
        state: Entity<MessageScrollerState>,
        items: Vec<(usize, f32)>,
    }
    impl Render for Conversation {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.items.iter().fold(
                MessageScroller::new("messages", &self.state, cx)
                    .w(px(200.))
                    .h(px(100.)),
                |area, &(id, height)| area.item(id, div().h(px(height)).flex_shrink_0()),
            )
        }
    }

    #[gpui::test]
    fn follow_stream_preserve_keyed_history_and_jump(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|_, cx| Conversation {
            state: cx.new(MessageScrollerState::new),
            items: (0..5).map(|id| (id, 50.)).collect(),
        });
        let state = view.read_with(cx, |view, _| view.state.clone());
        let scroll = state.read_with(cx, |state, _| state.scroll_handle().clone());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(scroll.offset().y, px(-150.));
        view.update(cx, |view, cx| {
            view.items[4].1 = 80.;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(scroll.offset().y, px(-180.));

        // Reader is partway through message 1, not aligned to an item edge.
        scroll.set_offset(point(px(0.), px(-65.)));
        view.update(cx, |view, cx| {
            view.items.insert(0, (8, 35.));
            view.items.push((9, 50.));
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(scroll.offset().y, px(-100.));
        assert_eq!(state.read_with(cx, |state, _| state.unread_count()), 1);
        view.update(cx, |view, cx| {
            view.items.last_mut().unwrap().1 = 110.;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(scroll.offset().y, px(-100.));
        assert_eq!(state.read_with(cx, |state, _| state.unread_count()), 1);
        cx.update(|window, cx| state.update(cx, |state, cx| state.jump_to_latest(window, cx)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(scroll.offset().y, -scroll.max_offset().y);
        assert_eq!(state.read_with(cx, |state, _| state.unread_count()), 0);
    }
}
