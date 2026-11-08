//! Window content and modal focus navigation.
//! Modified for GPUI Design: removed styled-layer plugins and rich-text hosting.
use crate::StyledExt;
use gpui::{
    AnyView, App, Context, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    StyleRefinement, Styled, Window, actions, div,
};

actions!(root, [Tab, TabPrev]);
const CONTEXT: &str = "Root";

/// Scan at most one cycle of the rendered tab order, preserving containment
/// regardless of how many controls exist behind a modal. GPUI exposes traversal
/// but not candidate enumeration. This scan is synchronous: only the final
/// focus path is published at the next draw, not the background candidates.
fn advance_focus(window: &mut Window, cx: &mut App, advance: fn(&mut Window, &mut App)) {
    let Some(container) = crate::active_focus_trap(window, cx) else {
        advance(window, cx);
        return;
    };

    advance(window, cx);
    // GPUI's tab order can cycle short of the whole window (a stop whose
    // previous stop is itself), so the walk ends at the first repeated stop,
    // not only on a return to the one it began with.
    let mut visited = Vec::new();
    while !container.contains_focused(window, cx) {
        let focused = window.focused(cx);
        if visited.contains(&focused) {
            // No tab stop in the trap is reachable. Its own handle remains a
            // safe focus target even when excluded from ordinary tab order.
            container.focus(window, cx);
            break;
        }
        visited.push(focused);
        advance(window, cx);
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
    ]);
}

/// The window's content and overlay host, independent of any styled component library.
pub struct Root {
    view: AnyView,
    style: StyleRefinement,
}

impl Root {
    pub fn new(view: impl Into<AnyView>, window: &mut Window, _: &mut Context<Self>) -> Self {
        #[cfg(all(target_os = "macos", not(test)))]
        crate::install_window_hit_test_forwarder(window);
        #[cfg(not(all(target_os = "macos", not(test))))]
        let _ = window;
        Self {
            view: view.into(),
            style: StyleRefinement::default(),
        }
    }

    /// The original application content entity.
    pub fn view(&self) -> &AnyView {
        &self.view
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        window
            .root::<Self>()
            .flatten()
            .expect("window must have a GPUI Design Root")
            .read(cx)
    }
    pub fn update<R>(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    ) -> R {
        let root = window
            .root::<Self>()
            .flatten()
            .expect("window must have a GPUI Design Root");
        root.update(cx, |root, cx| f(root, window, cx))
    }
    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        advance_focus(window, cx, Window::focus_next);
    }

    fn on_action_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        advance_focus(window, cx, Window::focus_prev);
    }
}
impl Styled for Root {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(feature = "test-support")]
        crate::test_support::begin_frame(window);
        #[cfg(not(feature = "test-support"))]
        let _ = window;
        div()
            .id("root")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_tab))
            .on_action(cx.listener(Self::on_action_tab_prev))
            .relative()
            .size_full()
            .child(self.view.clone())
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FocusTrapElement as _;
    use gpui::{AppContext, FocusHandle, Subscription, TestAppContext, VisualTestContext, px};
    use std::{cell::Cell, rc::Rc};

    struct TrapContent {
        background: Vec<FocusHandle>,
        container: FocusHandle,
        controls: Vec<FocusHandle>,
        _focus_subscriptions: Vec<Subscription>,
    }

    impl Render for TrapContent {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .tab_group()
                .size_full()
                .children(self.background.iter().enumerate().map(|(index, handle)| {
                    div()
                        .id(("background", index))
                        .track_focus(handle)
                        .tab_index(0)
                        .size(px(1.))
                }))
                .child(
                    div()
                        .id("trap")
                        .track_focus(&self.container)
                        .tab_stop(false)
                        .children(self.controls.iter().enumerate().map(|(index, handle)| {
                            div()
                                .id(("control", index))
                                .track_focus(handle)
                                .tab_index(0)
                                .size(px(1.))
                        }))
                        .focus_trap("modal", &self.container),
                )
        }
    }

    fn trap_harness(
        cx: &mut TestAppContext,
        control_count: usize,
    ) -> (
        &mut VisualTestContext,
        FocusHandle,
        Vec<FocusHandle>,
        Vec<FocusHandle>,
        Rc<Cell<usize>>,
    ) {
        cx.update(crate::init);
        let (container, controls, background) = cx.update(|cx| {
            (
                cx.focus_handle(),
                (0..control_count)
                    .map(|_| cx.focus_handle().tab_stop(true))
                    .collect::<Vec<_>>(),
                // Regression: traversing past the modal must cross more than
                // the former 100-attempt limit before reaching its other end.
                (0..256)
                    .map(|_| cx.focus_handle().tab_stop(true))
                    .collect::<Vec<_>>(),
            )
        });
        let background_focuses = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let container = container.clone();
            let controls = controls.clone();
            let background = background.clone();
            let background_focuses = background_focuses.clone();
            move |window, cx| {
                let content = cx.new(|cx| {
                    let subscriptions = background
                        .iter()
                        .map(|handle| {
                            let focuses = background_focuses.clone();
                            cx.on_focus(handle, window, move |_, _, _| {
                                focuses.set(focuses.get() + 1)
                            })
                        })
                        .collect();
                    TrapContent {
                        background,
                        container,
                        controls,
                        _focus_subscriptions: subscriptions,
                    }
                });
                Root::new(content, window, cx)
            }
        });
        cx.update(|window, _| window.activate_window());
        cx.executor().run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, container, controls, background, background_focuses)
    }

    fn assert_trapped_focus(
        cx: &mut VisualTestContext,
        container: &FocusHandle,
        expected: &FocusHandle,
    ) {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.focused(cx).as_ref(), Some(expected));
            assert!(container.contains_focused(window, cx));
        });
    }

    #[gpui::test]
    fn focus_trap_wraps_forward_past_more_than_100_background_stops(cx: &mut TestAppContext) {
        let (cx, container, controls, _, background_focuses) = trap_harness(cx, 2);
        cx.update(|window, cx| {
            controls[1].focus(window, cx);
            window.draw(cx).clear(cx);
        });

        cx.dispatch_action(Tab);
        assert_trapped_focus(cx, &container, &controls[0]);
        cx.dispatch_action(Tab);
        assert_trapped_focus(cx, &container, &controls[1]);
        assert_eq!(background_focuses.get(), 0);
    }

    #[gpui::test]
    fn focus_trap_wraps_backward_past_more_than_100_background_stops(cx: &mut TestAppContext) {
        let (cx, container, controls, _, background_focuses) = trap_harness(cx, 2);
        cx.update(|window, cx| {
            controls[0].focus(window, cx);
            window.draw(cx).clear(cx);
        });

        cx.dispatch_action(TabPrev);
        assert_trapped_focus(cx, &container, &controls[1]);
        cx.dispatch_action(TabPrev);
        assert_trapped_focus(cx, &container, &controls[0]);
        assert_eq!(background_focuses.get(), 0);
    }

    #[gpui::test]
    fn focus_trap_without_tab_stops_keeps_its_container_focused(cx: &mut TestAppContext) {
        let (cx, container, _, _, background_focuses) = trap_harness(cx, 0);
        cx.update(|window, cx| {
            container.focus(window, cx);
            window.draw(cx).clear(cx);
        });

        cx.dispatch_action(Tab);
        assert_trapped_focus(cx, &container, &container);
        cx.dispatch_action(TabPrev);
        assert_trapped_focus(cx, &container, &container);
        assert_eq!(background_focuses.get(), 0);
    }

    thread_local! {
        static SCRIPTED_STOPS: std::cell::RefCell<Vec<FocusHandle>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    /// Moves focus to the next scripted stop, in place of `focus_prev`. Running
    /// out fails the test, so a search that never stops cannot hang it.
    fn scripted_advance(window: &mut Window, cx: &mut App) {
        SCRIPTED_STOPS
            .with(|stops| stops.borrow_mut().pop())
            .expect("the search kept advancing after a stop repeated")
            .focus(window, cx);
    }

    #[gpui::test]
    fn focus_trap_search_ends_at_a_short_cycle_on_its_container(cx: &mut TestAppContext) {
        let (cx, container, controls, background, _) = trap_harness(cx, 2);
        cx.update(|window, cx| {
            controls[0].focus(window, cx);
            window.draw(cx).clear(cx);
        });
        // A tab order in which the second stop is its own neighbor never
        // returns to the first. Popped from the end: first, second, second.
        let (first, second) = (background[0].clone(), background[1].clone());
        SCRIPTED_STOPS.with(|stops| *stops.borrow_mut() = vec![second.clone(), second, first]);

        cx.update(|window, cx| advance_focus(window, cx, scripted_advance));
        assert_trapped_focus(cx, &container, &container);
        assert!(SCRIPTED_STOPS.with(|stops| stops.borrow().is_empty()));
    }

    #[gpui::test]
    fn inactive_focus_trap_does_not_capture_background_navigation(cx: &mut TestAppContext) {
        let (cx, _, _, background, background_focuses) = trap_harness(cx, 2);
        cx.update(|window, cx| {
            background[0].focus(window, cx);
            window.draw(cx).clear(cx);
        });
        // Prove that the observers used by the containment regressions really
        // receive native focus notifications when a background control is used.
        assert_eq!(background_focuses.get(), 1);

        cx.dispatch_action(Tab);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.focused(cx), Some(background[1].clone()));
        });
        cx.dispatch_action(TabPrev);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.focused(cx), Some(background[0].clone()));
        });
        assert_eq!(background_focuses.get(), 3);
    }

    mod nested_traps {
        use super::*;

        struct NestedTraps {
            containers: Vec<FocusHandle>,
            controls: Vec<[FocusHandle; 2]>,
            depth: usize,
        }

        impl Render for NestedTraps {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let mut child = div().into_any_element();
                for index in (0..self.depth).rev() {
                    let control = |side: usize| {
                        div()
                            .id(("nested-control", index * 2 + side))
                            .track_focus(&self.controls[index][side])
                            .tab_index(0)
                            .size(px(10.))
                    };
                    child = div()
                        .tab_stop(false)
                        .child(control(0))
                        .child(child)
                        .child(control(1))
                        .focus_trap(("nested-trap", index), &self.containers[index])
                        .into_any_element();
                }
                div().tab_group().size_full().child(child)
            }
        }

        #[gpui::test]
        fn nested_traps_keep_the_innermost_scope_across_close_and_reopen(cx: &mut TestAppContext) {
            cx.update(crate::init);
            let (containers, controls) = cx.update(|cx| {
                (
                    (0..6).map(|_| cx.focus_handle()).collect::<Vec<_>>(),
                    (0..6)
                        .map(|_| {
                            [
                                cx.focus_handle().tab_stop(true),
                                cx.focus_handle().tab_stop(true),
                            ]
                        })
                        .collect::<Vec<_>>(),
                )
            });
            let (root, cx) = cx.add_window_view({
                let containers = containers.clone();
                let controls = controls.clone();
                move |window, cx| {
                    let view = cx.new(|_| NestedTraps {
                        containers,
                        controls,
                        depth: 6,
                    });
                    Root::new(view, window, cx)
                }
            });
            let view = root.read_with(cx, |root, _| {
                root.view.clone().downcast::<NestedTraps>().unwrap()
            });

            // Retain every container handle while inner scopes disappear and
            // return. Rendered containment, not registration order or ownership
            // lifetime, must decide which remaining scope handles Tab.
            for depth in [6, 5, 4, 3, 2, 1, 2, 3, 4, 5, 6] {
                view.update(cx, |view, cx| {
                    view.depth = depth;
                    cx.notify();
                });
                let index = depth - 1;
                cx.update(|window, cx| {
                    controls[index][1].focus(window, cx);
                    window.draw(cx).clear(cx);
                });
                cx.dispatch_action(Tab);
                assert_trapped_focus(cx, &containers[index], &controls[index][0]);
                cx.dispatch_action(TabPrev);
                assert_trapped_focus(cx, &containers[index], &controls[index][1]);
            }
        }
    }
}
