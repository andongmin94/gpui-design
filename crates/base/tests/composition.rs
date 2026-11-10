//! Exercises the same consumer code as the native two_styles example.
//! Each distinct flow runs once, across representative compact and settings appearances.
//! GPUI's input simulation does not run a platform IME or screen reader.

// Each native binary owns its module tree; this test exercises both binaries'
// source directly, including their shared palette and Sidebar recipe imports.
#![allow(clippy::duplicate_mod)]

#[path = "../examples/two_styles.rs"]
mod two_styles;

#[path = "../examples/catalogue.rs"]
mod catalogue;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    AppContext as _, Entity, EntityInputHandler as _, Focusable as _, KeyDownEvent, KeyUpEvent,
    Keystroke, Modifiers, MouseButton, Role, ScrollDelta, ScrollEvent, TestAppContext,
    VisualTestContext, point, px,
};
use gpui_design::{
    CalendarSelection, ComboboxEvent, CommandEvent, MenuEvent, NaiveDate, RadioGroupEvent, Root,
    SelectEvent, TabsEvent, test_support,
};
use two_styles::{Appearance, Demo};

fn harness(
    cx: &mut TestAppContext,
    appearance: Appearance,
) -> (&mut VisualTestContext, Entity<Demo>) {
    cx.update(gpui_design::init);
    let (root, cx) = cx.add_window_view(move |window, cx| {
        let demo = cx.new(|cx| Demo::new(appearance, window, cx));
        Root::new(demo, window, cx)
    });
    let demo = root
        .read_with(cx, |root, _| root.view().clone())
        .downcast::<Demo>()
        .expect("example content");
    cx.update(|window, cx| {
        window.activate_window();
        let trigger = demo.read(cx).edit_hint.read(cx).focus_handle().clone();
        trigger.focus(window, cx);
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    (cx, demo)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn activate_key(cx: &mut VisualTestContext, key: &str) {
    // GPUI's native controls activate on key-up; simulate_keystrokes sends only down.
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

fn open_dialog(cx: &mut VisualTestContext, demo: &Entity<Demo>) {
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.dialog.read(cx).is_open());
        assert!(demo.input.focus_handle(cx).is_focused(window));
    });
}

// Both navigation presentations share selection state; Sidebar focus alone does not select.
fn focus_section(window: &mut gpui::Window, cx: &mut gpui::App, demo: &Entity<Demo>, index: usize) {
    let tabs = demo.read(cx).tabs.clone();
    tabs.update(cx, |state, cx| {
        let value = state.items()[index].value().clone();
        state.select_value(value, window, cx);
        state.tab_focus_handle(index).focus(window, cx);
    });
}

#[gpui::test]
fn sheet_geometry_scrolling_and_nested_modal_preserve_focus(cx: &mut TestAppContext) {
    let parts = |window: &gpui::Window| {
        let bounds = |part: &'static str| {
            let id = (gpui::ElementId::from("session-settings"), part).into();
            test_support::find(window, &[], &id).unwrap().bounds()
        };
        let done = test_support::snapshots(window)
            .into_iter()
            .find(|item| item.label() == Some("Done"))
            .unwrap();
        (
            bounds("header"),
            bounds("body"),
            bounds("footer"),
            done.bounds(),
        )
    };
    // Both palettes have different widths and padding; exercise those layout risks.
    for (appearance, preferred_rems) in [(Appearance::Compact, 22.), (Appearance::Settings, 46.)] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(760.), px(780.)));
        draw(cx);
        open_dialog(cx, &demo);

        cx.update(|window, _| {
            let host = test_support::snapshots(window)
                .into_iter()
                .find(|item| item.role() == Some(Role::Dialog))
                .unwrap();
            assert_eq!(host.label(), Some("Session settings"));
            assert_eq!(
                host.description(),
                Some("Choose the session's drawing and toolbar settings.")
            );
            let body_id = (gpui::ElementId::from("session-settings"), "body").into();
            let body = test_support::find(window, &[], &body_id).unwrap();
            assert_eq!(body.role(), Some(Role::ScrollView));
            assert_eq!(body.focused(), None);
        });

        for (width, height) in [(760., 780.), (320., 480.), (760., 360.)] {
            cx.simulate_resize(gpui::size(px(width), px(height)));
            draw(cx);
            draw(cx);
            let (header, body, footer, done) = cx.update(|window, _| {
                let popup = test_support::find(window, &[], &"dialog-popup".into())
                    .unwrap()
                    .bounds();
                let expected_width = (window.rem_size() * preferred_rems).min(px(width));
                assert!((popup.right() - px(width)).abs() < px(1.));
                assert!(popup.top().abs() < px(1.));
                assert!((popup.size.height - px(height)).abs() < px(1.));
                assert!((popup.size.width - expected_width).abs() < px(1.));
                let parts = parts(window);
                assert!(parts.1.top() >= parts.0.bottom());
                assert!(parts.1.bottom() <= parts.2.top());
                assert!(parts.1.size.height > px(0.));
                assert!(parts.3.top() >= parts.2.top());
                assert!(parts.3.bottom() <= px(height));
                parts
            });
            let wheel_position = point(body.left() + px(2.), body.center().y);
            for delta in [-10_000., -10_000., 10_000., 10_000.] {
                cx.simulate_event(ScrollEvent {
                    position: wheel_position,
                    delta: ScrollDelta::Pixels(point(px(-1_000.), px(delta))),
                    ..Default::default()
                });
                draw(cx);
                cx.update(|window, cx| {
                    let scroll = &demo.read(cx).fields_scroll;
                    let expected = if delta < 0. {
                        -scroll.max_offset().y
                    } else {
                        px(0.)
                    };
                    assert!((scroll.offset().y - expected).abs() < px(1.));
                    assert_eq!(scroll.offset().x, px(0.));
                    let after = parts(window);
                    assert_eq!((after.0, after.2, after.3), (header, footer, done));
                });
            }
            if width == 320. {
                cx.simulate_keystrokes("tab down");
                draw(cx);
                cx.update(|window, _| {
                    let list = test_support::snapshots(window)
                        .into_iter()
                        .find(|item| item.role() == Some(Role::ListBox))
                        .unwrap();
                    assert!(
                        list.bounds().left() >= px(0.) && list.bounds().right() <= px(width),
                        "select popup {:?} must fit the narrow viewport",
                        list.bounds()
                    );
                });
                cx.simulate_keystrokes("escape shift-tab");
                draw(cx);
                cx.update(|window, cx| {
                    assert!(!demo.read(cx).select.read(cx).is_open());
                    assert!(demo.read(cx).dialog.read(cx).is_open());
                    assert!(demo.read(cx).input.focus_handle(cx).is_focused(window));
                });
            }
        }

        if matches!(appearance, Appearance::Compact) {
            cx.simulate_keystrokes("escape");
            draw(cx);
            cx.update(|window, cx| {
                assert!(!demo.read(cx).dialog.read(cx).is_open());
                assert!(
                    demo.read(cx)
                        .edit_hint
                        .read(cx)
                        .focus_handle()
                        .is_focused(window)
                );
            });
            continue;
        }

        // One interaction flow is enough after both layouts have been checked.
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        });
        cx.simulate_input("Retained sheet session");
        draw(cx);
        let body = cx.update(|window, _| parts(window).1);
        cx.simulate_event(ScrollEvent {
            position: point(body.left() + px(2.), body.center().y),
            delta: ScrollDelta::Pixels(point(px(0.), px(-10_000.))),
            ..Default::default()
        });
        draw(cx);
        cx.update(|window, cx| {
            assert!(demo.read(cx).fields_scroll.max_offset().y > px(0.));
            let input = test_support::find(window, &[], &"session-name".into()).unwrap();
            assert!(input.bounds().bottom() <= body.top());
        });
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, _| {
            assert!(
                test_support::snapshots(window)
                    .iter()
                    .any(|item| { item.label() == Some("Done") && item.focused() == Some(true) })
            );
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        draw(cx); // Focus reveal is applied to the retained viewport on the next frame.
        cx.update(|window, cx| {
            assert!(demo.read(cx).input.focus_handle(cx).is_focused(window));
            let input = test_support::find(window, &[], &"session-name".into()).unwrap();
            assert!(input.visible());
            // Input requests reveal of its native editing viewport, which is
            // centered inside the taller application-owned control frame.
            let editing = demo.read(cx).input.read(cx).input_bounds();
            assert!(editing.size.height > px(0.));
            assert!(editing.top() >= body.top());
            assert!(editing.bottom() <= body.bottom());
            let scroll = &demo.read(cx).fields_scroll;
            assert!(scroll.offset().y > -scroll.max_offset().y);
        });

        // Open the existing confirmation from inside the Sheet, retaining its return target.
        cx.update(|window, cx| {
            let reset = demo.read(cx).reset_dialog.clone();
            reset.update(cx, |state, cx| state.open(window, cx));
        });
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(demo.dialog.read(cx).is_open());
            assert!(demo.reset_dialog.read(cx).is_open());
            assert!(demo.reset_cancel_focus.is_focused(window));
        });
        for (key, cancel_focused) in [
            ("shift-tab", false),
            ("tab", true),
            ("tab", false),
            ("tab", true),
        ] {
            cx.simulate_keystrokes(key);
            draw(cx);
            cx.update(|window, cx| {
                let demo = demo.read(cx);
                assert!(
                    demo.reset_dialog
                        .read(cx)
                        .focus_handle()
                        .contains_focused(window, cx)
                );
                assert_eq!(demo.reset_cancel_focus.is_focused(window), cancel_focused);
                assert_eq!(
                    test_support::find(window, &[], &"confirm-preview-reset".into())
                        .unwrap()
                        .focused(),
                    Some(!cancel_focused),
                );
            });
        }
        cx.update(|window, cx| assert!(demo.read(cx).reset_cancel_focus.is_focused(window)));
        let sheet_offset = cx.update(|_, cx| demo.read(cx).fields_scroll.offset());
        // The top modal also blocks wheel input over the Sheet behind it.
        cx.simulate_event(ScrollEvent {
            position: point(body.right() - px(2.), body.center().y),
            delta: ScrollDelta::Pixels(point(px(0.), px(-100.))),
            ..Default::default()
        });
        draw(cx);
        cx.update(|_, cx| assert_eq!(demo.read(cx).fields_scroll.offset(), sheet_offset));
        // This point is outside both surfaces: the child backdrop must block the Sheet's.
        cx.simulate_click(point(px(2.), px(2.)), Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(demo.dialog.read(cx).is_open());
            assert!(demo.reset_dialog.read(cx).is_open());
            assert!(demo.reset_cancel_focus.is_focused(window));
        });
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(!demo.reset_dialog.read(cx).is_open());
            assert!(demo.dialog.read(cx).is_open());
            assert!(demo.input.focus_handle(cx).is_focused(window));
        });
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(!demo.dialog.read(cx).is_open());
            assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
        });
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            assert_eq!(
                demo.read(cx).input.read(cx).value(),
                "Retained sheet session"
            );
            assert!(demo.read(cx).input.focus_handle(cx).is_focused(window));
        });
        cx.simulate_click(point(px(2.), px(2.)), Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(!demo.dialog.read(cx).is_open());
            assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
        });
    }
}

#[gpui::test]
fn compact_controls_complete_the_keyboard_flow(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Compact);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let stroke_changes = Rc::new(RefCell::new(Vec::new()));
    cx.update(|_, cx| {
        let changes = changes.clone();
        let select = demo.read(cx).select.clone();
        cx.subscribe(&select, move |_, event: &SelectEvent, _| {
            let SelectEvent::Change(value) = event;
            changes.borrow_mut().push(value.clone());
        })
        .detach();
        let stroke = demo.read(cx).stroke.clone();
        let stroke_changes = stroke_changes.clone();
        cx.subscribe(&stroke, move |_, event: &RadioGroupEvent, _| {
            let RadioGroupEvent::Change(value) = event;
            stroke_changes.borrow_mut().push(value.clone());
        })
        .detach();
    });
    open_dialog(cx, &demo);

    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    });
    cx.simulate_input("미니캐스트");
    draw(cx);
    cx.update(|window, cx| {
        let name = test_support::find(window, &[], &"session-name".into()).unwrap();
        assert_eq!(name.role(), Some(Role::TextInput));
        assert_eq!(name.label(), Some("Session name"));
        assert_eq!(name.value(), Some("미니캐스트"));
        assert_eq!(name.focused(), Some(true));
        assert_eq!(demo.read(cx).input.read(cx).value(), "미니캐스트");
    });

    let label_point = cx.update(|window, _| {
        let label = test_support::find(window, &[], &"drawing-tool-label".into()).unwrap();
        assert_eq!(label.role(), Some(Role::Label));
        assert_eq!(label.label(), Some("Drawing tool"));
        assert_eq!(label.focused(), None);
        label.bounds().center()
    });
    cx.simulate_click(label_point, Modifiers::default());
    draw(cx);
    cx.update(|window, cx| {
        let select = demo.read(cx).select.read(cx);
        assert!(select.focus_handle().is_focused(window));
        assert!(
            !select.is_open(),
            "a label focuses without activating Select"
        );
        assert_eq!(select.selected_index(), Some(0));
    });
    assert!(changes.borrow().is_empty());
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(demo.read(cx).input.focus_handle(cx).is_focused(window));
    });

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .select
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    cx.simulate_keystrokes("down");
    draw(cx);
    cx.update(|window, cx| {
        let select = demo.read(cx).select.read(cx);
        assert!(select.is_open());
        assert!(select.content_focus_handle().is_focused(window));
        assert_eq!(select.highlighted_index(), Some(0));
        let trigger = test_support::find(window, &[], &"drawing-tool".into()).unwrap();
        assert_eq!(trigger.role(), Some(Role::ComboBox));
        assert_eq!(trigger.label(), Some("Drawing tool"));
        assert_eq!(trigger.expanded(), Some(true));
    });
    cx.simulate_keystrokes("down");
    draw(cx);
    cx.update(|_, cx| {
        // The intervening Laser option is disabled.
        assert_eq!(demo.read(cx).select.read(cx).highlighted_index(), Some(2));
    });
    cx.simulate_keystrokes("enter");
    draw(cx);
    cx.update(|window, cx| {
        let select = demo.read(cx).select.read(cx);
        assert!(!select.is_open());
        assert_eq!(
            select.selected_item().unwrap().value().as_ref(),
            "rectangle"
        );
        assert!(select.focus_handle().is_focused(window));
    });
    assert_eq!(
        changes.borrow().as_slice(),
        &[gpui::SharedString::from("rectangle")]
    );

    cx.simulate_keystrokes("down");
    draw(cx);
    cx.simulate_keystrokes("up");
    draw(cx);
    cx.update(|_, cx| assert_eq!(demo.read(cx).select.read(cx).highlighted_index(), Some(0)));
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(
            demo.dialog.read(cx).is_open(),
            "Select must consume the first Escape"
        );
        assert!(!demo.select.read(cx).is_open());
        assert_eq!(
            demo.select.read(cx).selected_index(),
            Some(2),
            "Escape must not commit the cursor"
        );
        assert!(demo.select.read(cx).focus_handle().is_focused(window));
    });

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .tabs
                .read(cx)
                .tab_focus_handle(0)
                .is_focused(window)
        );
        let list = test_support::find(window, &[], &"session-sections".into()).unwrap();
        assert_eq!(list.role(), Some(Role::TabList));
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let checkbox = test_support::find(window, &[], &"show-cursor".into()).unwrap();
        assert_eq!(checkbox.role(), Some(Role::CheckBox));
        assert_eq!(checkbox.label(), Some("Show cursor"));
        assert_eq!(checkbox.checked(), Some(true));
        assert_eq!(checkbox.focused(), Some(true));
    });
    activate_key(cx, "space");
    cx.update(|window, cx| {
        assert!(!demo.read(cx).show_cursor);
        assert_eq!(
            test_support::find(window, &[], &"show-cursor".into())
                .unwrap()
                .checked(),
            Some(false)
        );
    });

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let switch = test_support::find(window, &[], &"keep-toolbar".into()).unwrap();
        assert_eq!(switch.role(), Some(Role::Switch));
        assert_eq!(switch.label(), Some("Keep toolbar visible"));
        assert_eq!(switch.checked(), Some(false));
        assert_eq!(switch.focused(), Some(true));
    });
    activate_key(cx, "space");
    cx.update(|window, cx| {
        assert!(demo.read(cx).keep_toolbar);
        assert_eq!(
            test_support::find(window, &[], &"keep-toolbar".into())
                .unwrap()
                .checked(),
            Some(true)
        );
    });

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let stroke = demo.read(cx).stroke.read(cx);
        assert!(stroke.option_focus_handle(0).is_focused(window));
        let group = test_support::find(window, &[], &"stroke-width".into()).unwrap();
        assert_eq!(group.role(), Some(Role::RadioGroup));
        assert_eq!(group.label(), Some("Stroke width"));
    });
    cx.simulate_keystrokes("right");
    draw(cx);
    cx.update(|window, cx| {
        let stroke = demo.read(cx).stroke.read(cx);
        assert_eq!(stroke.selected_index(), Some(2), "skip the disabled width");
        assert!(stroke.option_focus_handle(2).is_focused(window));
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::RadioButton)
                && item.label() == Some("Bold")
                && item.checked() == Some(true)
                && item.focused() == Some(true)
        }));
    });
    activate_key(cx, "space");
    assert_eq!(
        stroke_changes.borrow().as_slice(),
        &[gpui::SharedString::from("bold")],
        "reselecting the current radio emits no duplicate change"
    );
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx).notes.focus_handle(cx).is_focused(window),
            "the radio group has one Tab stop"
        );
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .stroke
                .read(cx)
                .option_focus_handle(2)
                .is_focused(window),
            "returning to the group focuses its selected option"
        );
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).dialog.read(cx).is_open());
        assert!(
            demo.read(cx)
                .edit_hint
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    assert_eq!(changes.borrow().len(), 1);

    // Reopening retains all consumer state and renews the initial focus contract.
    open_dialog(cx, &demo);
    cx.update(|_, cx| {
        let demo = demo.read(cx);
        assert_eq!(demo.input.read(cx).value(), "미니캐스트");
        assert_eq!(demo.select.read(cx).selected_index(), Some(2));
        assert!(!demo.select.read(cx).is_open());
        assert!(!demo.show_cursor);
        assert!(demo.keep_toolbar);
        assert_eq!(demo.stroke.read(cx).selected_index(), Some(2));
    });
}

#[gpui::test]
fn select_dismissal_preserves_tab_pointer_and_dialog_focus(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Compact);
    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab down");
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.select.read(cx).is_open());
        assert!(
            demo.dialog
                .read(cx)
                .focus_handle()
                .contains_focused(window, cx)
        );
        assert!(
            test_support::snapshots(window).iter().any(|item| {
                item.label() == Some("Preferences") && item.focused() == Some(true)
            }),
            "Tab from the popup advances after its trigger"
        );
    });
    cx.simulate_keystrokes("shift-tab down");
    draw(cx);
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.select.read(cx).is_open());
        assert!(demo.input.focus_handle(cx).is_focused(window));
    });

    cx.simulate_keystrokes("tab down");
    draw(cx);
    let input_point = cx.update(|window, _| {
        test_support::find(window, &[], &"session-name".into())
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(input_point, Modifiers::default());
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.select.read(cx).is_open());
        assert!(
            demo.input.focus_handle(cx).is_focused(window),
            "outside dismissal must not steal the clicked input's focus"
        );
    });

    cx.simulate_keystrokes("tab down");
    draw(cx);
    // The popup covers the footer. Close it before clicking the visible Done
    // button; an occluded coordinate must never click through the option list.
    cx.simulate_keystrokes("escape");
    draw(cx);
    let done_point = cx.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|item| item.label() == Some("Done"))
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(done_point, Modifiers::default());
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(!demo.select.read(cx).is_open());
        assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
    });
    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab down");
    draw(cx);
    // A backdrop press dismisses both scopes without returning to a hidden trigger.
    cx.simulate_click(
        gpui::point(gpui::px(2.), gpui::px(2.)),
        Modifiers::default(),
    );
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(!demo.select.read(cx).is_open());
        assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
    });

    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab down");
    draw(cx);
    // Programmatic focus changes have no outside-click or Tab handler. The
    // focus-out lifecycle must notify after drawing so the popup disappears.
    let notifications = Rc::new(Cell::new(0));
    cx.update(|_, cx| {
        let select = demo.read(cx).select.clone();
        let notifications = notifications.clone();
        cx.observe(&select, move |_, _| {
            notifications.set(notifications.get() + 1)
        })
        .detach();
    });
    cx.update(|window, cx| {
        let input_focus = demo.read(cx).input.focus_handle(cx);
        input_focus.focus(window, cx);
    });
    draw(cx);
    cx.run_until_parked();
    assert!(
        notifications.get() > 0,
        "focus-out close must notify outside the draw phase"
    );
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).select.read(cx).is_open());
        assert!(demo.read(cx).input.focus_handle(cx).is_focused(window));
        assert!(
            !test_support::snapshots(window)
                .iter()
                .any(|item| item.role() == Some(Role::ListBox))
        );
    });

    cx.simulate_keystrokes("tab down");
    draw(cx);
    cx.update(|window, cx| {
        let dialog = demo.read(cx).dialog.clone();
        dialog.update(cx, |state, cx| state.close(window, cx));
    });
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(
            !demo.select.read(cx).is_open(),
            "closing the parent must close its popup state"
        );
        assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
    });

    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab down");
    draw(cx);
    cx.deactivate_window();
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|_, cx| {
        assert!(
            !demo.read(cx).select.read(cx).is_open(),
            "deactivation must dismiss the popup even when its focus handle is retained"
        );
        assert!(demo.read(cx).dialog.read(cx).is_open());
    });
}

#[gpui::test]
fn settings_sidebar_and_popover_preserve_nested_focus(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Settings);
    let changes = Rc::new(RefCell::new(Vec::new()));
    cx.update(|_, cx| {
        let tabs = demo.read(cx).tabs.clone();
        let changes = changes.clone();
        cx.subscribe(&tabs, move |_, event: &TabsEvent, _| {
            let TabsEvent::Change(value) = event;
            changes.borrow_mut().push(value.clone());
        })
        .detach();
    });
    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab tab");
    draw(cx);
    cx.update(|window, cx| {
        let navigation = test_support::find(window, &[], &"session-sections".into()).unwrap();
        assert_eq!(navigation.role(), Some(Role::Navigation));
        assert_eq!(navigation.label(), Some("Session sections"));
        assert!(
            demo.read(cx)
                .tabs
                .read(cx)
                .tab_focus_handle(0)
                .is_focused(window)
        );
    });
    cx.simulate_keystrokes("right down home end");
    draw(cx);
    cx.update(|window, cx| {
        let tabs = demo.read(cx).tabs.read(cx);
        assert_eq!(tabs.selected_index(), Some(0));
        assert!(tabs.tab_focus_handle(0).is_focused(window));
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let tabs = demo.read(cx).tabs.read(cx);
        assert_eq!(
            tabs.selected_index(),
            Some(0),
            "focus does not select a section"
        );
        assert!(
            tabs.tab_focus_handle(2).is_focused(window),
            "skip disabled Sync"
        );
    });
    assert!(changes.borrow().is_empty());
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let tabs = demo.read(cx).tabs.read(cx);
        assert_eq!(tabs.selected_index(), Some(2));
        assert!(tabs.tab_focus_handle(2).is_focused(window));
        assert!(test_support::find(window, &[], &"show-cursor".into()).is_none());
        let region_id = (gpui::ElementId::from("session-region"), "2".to_string()).into();
        let region = test_support::find(window, &[], &region_id).unwrap();
        assert_eq!(region.role(), Some(Role::Region));
        assert_eq!(region.label(), Some("Help"));
        let disabled_id = (gpui::ElementId::from("session-sections"), "1".to_string()).into();
        let disabled = test_support::find(window, &[], &disabled_id).unwrap();
        assert_eq!(disabled.role(), Some(Role::Button));
        assert_eq!(disabled.label(), Some("Sync (unavailable)"));
        assert_eq!(disabled.disabled(), Some(true));
        assert!(!test_support::snapshots(window).iter().any(|item| {
            matches!(
                item.role(),
                Some(Role::Tab | Role::TabList | Role::TabPanel)
            )
        }));
        let heading = test_support::find(window, &[], &"help-title".into()).unwrap();
        assert_eq!(heading.role(), Some(Role::Heading));
        assert_eq!(heading.label(), Some("Help"));
        assert_eq!(heading.focused(), None);
        let description =
            test_support::find(window, &[], &"help-keyboard-description".into()).unwrap();
        assert_eq!(description.role(), Some(Role::Paragraph));
        assert_eq!(
            description.label(),
            Some("Use the keyboard guide to explore these controls.")
        );
        assert_eq!(description.focused(), None);
    });
    assert_eq!(
        changes.borrow().as_slice(),
        &[gpui::SharedString::from("help")]
    );

    for width in [320., 760.] {
        cx.simulate_resize(gpui::size(px(width), px(780.)));
        draw(cx);
        draw(cx);
        cx.update(|window, cx| {
            let navigation = test_support::find(window, &[], &"session-sections".into())
                .unwrap()
                .bounds();
            let region_id = (gpui::ElementId::from("session-region"), "2".to_string()).into();
            let region = test_support::find(window, &[], &region_id)
                .unwrap()
                .bounds();
            let body_id = (gpui::ElementId::from("session-settings"), "body").into();
            let body = test_support::find(window, &[], &body_id).unwrap().bounds();
            for bounds in [navigation, region] {
                assert!(bounds.left() >= body.left());
                assert!(bounds.right() <= body.right() + px(1.));
            }
            if width < 640. {
                assert!(navigation.bottom() <= region.top());
            } else {
                assert!(navigation.right() <= region.left());
            }
            let tabs = demo.read(cx).tabs.read(cx);
            assert_eq!(tabs.selected_index(), Some(2));
            assert!(tabs.tab_focus_handle(2).is_focused(window));
        });
    }

    cx.simulate_keystrokes("tab tab tab tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .help
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let help = demo.read(cx).help.read(cx);
        assert!(help.is_open());
        assert!(help.content_focus_handle().is_focused(window));
        let trigger = test_support::find(window, &[], &"keyboard-help".into()).unwrap();
        assert_eq!(trigger.role(), Some(Role::Button));
        assert_eq!(trigger.expanded(), Some(true));
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::Dialog) && item.label() == Some("Keyboard help")
        }));
        let path = test_support::find(window, &[], &"keyboard-guide-path".into()).unwrap();
        assert_eq!(path.role(), Some(Role::Navigation));
        assert_eq!(path.label(), Some("Help location"));
        let arrow = test_support::find(window, &[], &"help-up-key".into()).unwrap();
        assert_eq!(arrow.role(), Some(Role::Label));
        assert_eq!(arrow.label(), Some("Arrow up"));
        let heading = test_support::find(window, &[], &"keyboard-guide-title".into()).unwrap();
        assert_eq!(heading.role(), Some(Role::Heading));
        assert_eq!(heading.label(), Some("Keyboard guide"));
        assert_eq!(heading.focused(), None);
    });

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let settings = test_support::find(window, &[], &"help-to-settings".into()).unwrap();
        assert_eq!(settings.focused(), Some(true));
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let parent = test_support::find(window, &[], &"help-to-parent".into()).unwrap();
        assert_eq!(parent.focused(), Some(true));
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.help.read(cx).is_open());
        assert_eq!(demo.tabs.read(cx).selected_index(), Some(2));
        assert!(demo.help.read(cx).focus_handle().is_focused(window));
    });
    activate_key(cx, "enter");
    // Only ancestor actions and Close help join Tab order; typography and keycaps do not.
    cx.simulate_keystrokes("tab tab tab");
    draw(cx);
    cx.update(|window, _| {
        let close = test_support::find(window, &[], &"close-help".into()).unwrap();
        assert_eq!(close.focused(), Some(true));
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).help.read(cx).is_open());
        assert!(demo.read(cx).dialog.read(cx).is_open());
        assert!(
            test_support::snapshots(window).iter().any(|item| {
                item.label() == Some("State ownership") && item.focused() == Some(true)
            }),
            "Tab exits the non-modal popup after its trigger inside the dialog"
        );
    });

    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    activate_key(cx, "space");
    cx.simulate_keystrokes("tab shift-tab");
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let help = demo.read(cx).help.read(cx);
        assert!(!help.is_open());
        assert!(help.focus_handle().is_focused(window));
    });

    activate_key(cx, "enter");
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.help.read(cx).is_open());
        assert!(
            demo.dialog.read(cx).is_open(),
            "Popover consumes the first Escape"
        );
        assert!(demo.help.read(cx).focus_handle().is_focused(window));
    });
    activate_key(cx, "enter");
    let input_point = cx.update(|window, _| {
        test_support::find(window, &[], &"session-name".into())
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(input_point, Modifiers::default());
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.help.read(cx).is_open());
        assert!(demo.input.focus_handle(cx).is_focused(window));
    });

    // A hidden region has no Tab stops; its application-owned values survive remounting.
    cx.simulate_keystrokes("tab tab");
    draw(cx);
    activate_key(cx, "space");
    cx.simulate_keystrokes("tab tab tab tab");
    draw(cx);
    activate_key(cx, "space");
    cx.update(|_, cx| assert!(!demo.read(cx).show_cursor));
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, _| {
        assert!(test_support::find(window, &[], &"show-cursor".into()).is_none());
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    activate_key(cx, "space");
    cx.simulate_keystrokes("tab tab tab tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).show_cursor);
        let checkbox = test_support::find(window, &[], &"show-cursor".into()).unwrap();
        assert_eq!(checkbox.checked(), Some(false));
        assert_eq!(checkbox.focused(), Some(true));
    });

    // The root ancestor explicitly selects Preferences and restores its Sidebar focus.
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab tab tab tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.help.read(cx).is_open());
        assert!(demo.dialog.read(cx).is_open());
        let tabs = demo.tabs.read(cx);
        assert_eq!(tabs.selected_index(), Some(0));
        assert!(tabs.tab_focus_handle(0).is_focused(window));
        assert!(!demo.show_cursor);
        assert_eq!(
            test_support::find(window, &[], &"show-cursor".into())
                .unwrap()
                .checked(),
            Some(false)
        );
    });

    // Removing the parent surface must also finish the retained popup lifecycle.
    cx.simulate_keystrokes("tab");
    draw(cx);
    activate_key(cx, "space");
    cx.simulate_keystrokes("tab tab tab tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert!(demo.read(cx).help.read(cx).is_open());
        let dialog = demo.read(cx).dialog.clone();
        dialog.update(cx, |state, cx| state.close(window, cx));
    });
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.help.read(cx).is_open());
        assert!(!demo.dialog.read(cx).is_open());
        assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
    });
    open_dialog(cx, &demo);
    cx.update(|_, cx| {
        let demo = demo.read(cx);
        assert_eq!(demo.tabs.read(cx).selected_index(), Some(2));
        assert!(!demo.help.read(cx).is_open());
        assert!(!demo.show_cursor);
    });
}

#[gpui::test]
fn help_typography_wraps_in_both_appearances_without_adding_focus(cx: &mut TestAppContext) {
    const OWNERSHIP: &str = "In this example, DialogState owns panel visibility, while InputState retains the session name. Closing Help does not create another copy of either state.";
    const SOURCE: &str = "crates/base/examples/two_styles.rs";

    for appearance in [Appearance::Compact, Appearance::Settings] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(760.), px(780.)));
        draw(cx);
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 2);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            assert_eq!(demo.read(cx).tabs.read(cx).selected_index(), Some(2));
            let sections = demo.read(cx).accordion.clone();
            sections.update(cx, |state, cx| state.toggle(2, window, cx));
        });
        draw(cx);

        let mut wide_height = None;
        // Revisit the wide window to catch stale wrapping after a narrow layout.
        for (width, height) in [(760., 780.), (320., 480.), (280., 480.), (760., 780.)] {
            cx.simulate_resize(gpui::size(px(width), px(height)));
            draw(cx);
            draw(cx);
            let (body, text_height) = cx.update(|window, cx| {
                let content_id = gpui::ElementId::from("help-content");
                let sections_id = gpui::ElementId::from("help-sections");
                let content = test_support::find(window, &[], &content_id)
                    .unwrap()
                    .bounds();
                let body_id = (gpui::ElementId::from("session-settings"), "body").into();
                let body = test_support::find(window, &[], &body_id).unwrap().bounds();
                assert!(content.left() >= body.left());
                assert!(content.right() <= body.right() + px(1.));

                for (id, role, label) in [
                    ("help-title", Role::Heading, "Help"),
                    ("help-state-ownership", Role::Paragraph, OWNERSHIP),
                    (
                        "help-source-description",
                        Role::Paragraph,
                        "The consumer example lives in:",
                    ),
                    ("help-source-path", Role::Code, SOURCE),
                ] {
                    let id = gpui::ElementId::from(id);
                    let element = test_support::find(window, &[], &id).unwrap();
                    assert_eq!(element.role(), Some(role));
                    assert_eq!(element.label(), Some(label));
                    assert_eq!(element.focused(), None);
                    assert!(element.path().contains(&content_id));
                    if role != Role::Heading {
                        assert!(element.path().contains(&sections_id));
                    }
                    assert!(element.bounds().left() >= content.left());
                    assert!(element.bounds().right() <= content.right() + px(1.));
                }

                let text = test_support::texts(window);
                let ownership_id = gpui::ElementId::from("help-state-ownership");
                let ownership = text
                    .iter()
                    .find(|text| text.path().contains(&ownership_id))
                    .unwrap();
                assert_eq!(ownership.text(), OWNERSHIP);
                assert!(ownership.bounds().size.height > px(0.));
                assert!(ownership.bounds().left() >= content.left());
                assert!(ownership.bounds().right() <= content.right() + px(1.));
                let source_id = gpui::ElementId::from("help-source-path");
                let source = text
                    .iter()
                    .find(|text| text.path().contains(&source_id))
                    .unwrap();
                assert_eq!(source.text(), SOURCE);
                assert!(source.bounds().right() <= content.right() + px(1.));

                let demo = demo.read(cx);
                assert!(demo.tabs.read(cx).tab_focus_handle(2).is_focused(window));
                assert!(demo.accordion.read(cx).is_open(2));
                assert_eq!(demo.fields_scroll.offset().x, px(0.));
                (body, ownership.bounds().size.height)
            });
            match wide_height {
                None => wide_height = Some(text_height),
                Some(wide) if width == 760. => assert_eq!(text_height, wide),
                Some(wide) => assert!(text_height >= wide),
            }

            cx.simulate_event(ScrollEvent {
                position: point(body.left() + px(2.), body.center().y),
                delta: ScrollDelta::Pixels(point(px(0.), px(-10_000.))),
                ..Default::default()
            });
            draw(cx);
            cx.update(|window, cx| {
                let source = test_support::find(window, &[], &"help-source-path".into()).unwrap();
                assert!(source.visible());
                assert!(source.bounds().top() >= body.top());
                assert!(source.bounds().bottom() <= body.bottom() + px(1.));
                let demo = demo.read(cx);
                let scroll = &demo.fields_scroll;
                assert_eq!(scroll.offset().x, px(0.));
                assert!((scroll.offset().y + scroll.max_offset().y).abs() < px(1.));
                if width < 760. {
                    assert!(scroll.max_offset().y > px(0.));
                }
                assert!(demo.tabs.read(cx).tab_focus_handle(2).is_focused(window));
            });
        }

        // The added title must also fit the narrow popup with its existing
        // Breadcrumb actions and keycaps, including a fully reachable close button.
        for width in [320., 280.] {
            cx.simulate_resize(gpui::size(px(width), px(480.)));
            cx.update(|window, cx| {
                let sections = demo.read(cx).accordion.clone();
                sections.update(cx, |state, cx| {
                    if !state.is_open(0) {
                        state.toggle(0, window, cx);
                    }
                });
            });
            draw(cx);
            cx.update(|window, cx| {
                let help = demo.read(cx).help.read(cx).focus_handle().clone();
                help.focus(window, cx);
            });
            draw(cx);
            cx.run_until_parked();
            draw(cx);
            activate_key(cx, "enter");
            cx.update(|window, cx| {
                assert!(demo.read(cx).help.read(cx).is_open());
                let popup = test_support::snapshots(window)
                    .into_iter()
                    .find(|item| {
                        item.role() == Some(Role::Dialog) && item.label() == Some("Keyboard help")
                    })
                    .unwrap()
                    .bounds();
                let viewport = window.viewport_size();
                assert!(popup.left() >= px(0.) && popup.top() >= px(0.));
                assert!(popup.right() <= viewport.width + px(1.));
                assert!(popup.bottom() <= viewport.height + px(1.));
                for id in ["keyboard-guide-title", "close-help"] {
                    let item = test_support::find(window, &[], &id.into()).unwrap();
                    assert!(item.visible());
                    assert!(item.bounds().top() >= popup.top());
                    assert!(item.bounds().bottom() <= popup.bottom());
                }
            });
            cx.simulate_keystrokes("escape");
            draw(cx);
        }
        cx.simulate_keystrokes("escape");
        draw(cx);
    }
}

#[gpui::test]
fn configuration_code_copies_current_values_and_contains_nested_scrolling(cx: &mut TestAppContext) {
    let code_id = (gpui::ElementId::from("session-configuration"), "code").into();
    let copy_id = (gpui::ElementId::from("session-configuration"), "copy").into();
    let scroll_id = (gpui::ElementId::from("session-configuration"), "scroll").into();
    let body_id = (gpui::ElementId::from("session-settings"), "body").into();
    let name = "긴프로젝트_export_identifier_0123456789".repeat(4);
    let initial_notes = "첫 번째 줄\nA quoted \"setting\" stays literal.\n\n마지막 줄\n";
    let changed_notes = format!("{initial_notes}{}\n", "설정_configuration_".repeat(12));

    // The two palettes give this nested viewport different available widths.
    for appearance in [Appearance::Compact, Appearance::Settings] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(320.), px(480.)));
        draw(cx);
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let input = demo.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value(name.clone(), window, cx));
            let notes = demo.read(cx).notes.clone();
            notes.update(cx, |state, cx| state.set_value(initial_notes, window, cx));
            focus_section(window, cx, &demo, 2);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            let sections = demo.read(cx).accordion.clone();
            sections.update(cx, |state, cx| state.toggle(2, window, cx));
        });
        draw(cx);
        cx.update(|window, cx| {
            let panel = demo
                .read(cx)
                .accordion
                .read(cx)
                .panel_focus_handle(2)
                .clone();
            panel.focus(window, cx);
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("test sentinel".into()));
        });
        draw(cx);
        // The attachment's Inspect action now precedes the existing Copy action.
        cx.simulate_keystrokes("tab tab");
        draw(cx);
        let first_code = cx.update(|window, cx| {
            let copy = test_support::find(window, &[], &copy_id).unwrap();
            assert_eq!(copy.role(), Some(Role::Button));
            assert_eq!(copy.label(), Some("Copy Session configuration"));
            assert_eq!(copy.focused(), Some(true));
            assert!(copy.visible());
            let code = demo.read(cx).configuration_code(cx);
            let config: serde_json::Value = serde_json::from_str(code.as_ref()).unwrap();
            assert_eq!(config["session_name"], name);
            assert_eq!(config["notes"], initial_notes);
            code
        });
        activate_key(cx, "enter");
        cx.update(|window, cx| {
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some(first_code.to_string())
            );
            assert_eq!(
                test_support::find(window, &[], &copy_id).unwrap().focused(),
                Some(true)
            );
            let notes = demo.read(cx).notes.clone();
            notes.update(cx, |state, cx| {
                state.set_value(changed_notes.clone(), window, cx)
            });
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        let current_code = cx.update(|window, cx| {
            let code = demo.read(cx).configuration_code(cx);
            assert_ne!(code, first_code);
            let config: serde_json::Value = serde_json::from_str(code.as_ref()).unwrap();
            assert_eq!(config["session_name"], name);
            assert_eq!(config["notes"], changed_notes);
            assert_eq!(
                test_support::find(window, &[], &code_id).unwrap().label(),
                Some(code.as_ref())
            );
            code
        });
        activate_key(cx, "space");
        cx.update(|window, cx| {
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some(current_code.to_string()),
                "the copy callback must follow the newly rendered retained values"
            );
            assert_eq!(
                test_support::find(window, &[], &copy_id).unwrap().focused(),
                Some(true)
            );
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.run_until_parked();
        draw(cx);

        let mut text_height = None;
        for width in [320., 280.] {
            cx.simulate_resize(gpui::size(px(width), px(480.)));
            draw(cx);
            // Re-enter through the Copy button so native focus reveal is also
            // exercised after the narrower prose above changes its height.
            cx.simulate_keystrokes("shift-tab");
            draw(cx);
            cx.simulate_keystrokes("tab");
            draw(cx);
            cx.run_until_parked();
            draw(cx);
            let (body, code_bounds, parent_offset, maximum) = cx.update(|window, cx| {
                let body = test_support::find(window, &[], &body_id).unwrap().bounds();
                let code = test_support::find(window, &[], &code_id).unwrap();
                assert_eq!(code.role(), Some(Role::Code));
                assert_eq!(code.label(), Some(current_code.as_ref()));
                assert_eq!(code.focused(), Some(true));
                assert!(code.visible());
                assert!(code.bounds().left() >= body.left());
                assert!(code.bounds().right() <= body.right() + px(1.));
                assert!(code.bounds().top() >= body.top() - px(1.));
                assert!(code.bounds().bottom() <= body.bottom() + px(1.));
                let scroll = test_support::find(window, &[], &scroll_id).unwrap();
                assert_eq!(scroll.role(), Some(Role::ScrollView));
                let text = test_support::texts(window)
                    .into_iter()
                    .find(|text| text.path().contains(&code_id))
                    .unwrap();
                assert_eq!(text.text(), current_code.as_ref());
                assert!(text.bounds().size.width > code.bounds().size.width);
                if let Some(height) = text_height {
                    assert_eq!(text.bounds().size.height, height, "code must not soft-wrap");
                } else {
                    text_height = Some(text.bounds().size.height);
                }
                let demo = demo.read(cx);
                assert!(demo.configuration_focus.is_focused(window));
                assert_eq!(demo.tabs.read(cx).selected_index(), Some(2));
                assert!(demo.accordion.read(cx).is_open(2));
                assert_eq!(demo.fields_scroll.offset().x, px(0.));
                let maximum = demo.configuration_scroll.max_offset();
                assert!(maximum.x > px(0.) && maximum.y > px(0.));
                (body, code.bounds(), demo.fields_scroll.offset(), maximum)
            });
            cx.simulate_keystrokes("end down right pagedown");
            draw(cx);
            cx.update(|window, cx| {
                let demo = demo.read(cx);
                let offset = demo.configuration_scroll.offset();
                assert!((offset.x + maximum.x).abs() < px(1.));
                assert!((offset.y + maximum.y).abs() < px(1.));
                assert_eq!(demo.fields_scroll.offset(), parent_offset);
                assert!(demo.configuration_focus.is_focused(window));
                assert!(demo.accordion.read(cx).is_open(2));
            });
            cx.simulate_event(ScrollEvent {
                position: code_bounds.center(),
                delta: ScrollDelta::Pixels(point(px(-10_000.), px(-10_000.))),
                ..Default::default()
            });
            draw(cx);
            cx.update(|_, cx| {
                let demo = demo.read(cx);
                assert_eq!(demo.fields_scroll.offset(), parent_offset);
                assert!((demo.configuration_scroll.offset().x + maximum.x).abs() < px(1.));
                assert!((demo.configuration_scroll.offset().y + maximum.y).abs() < px(1.));
            });
            // The Sheet's exposed gutter remains independently scrollable.
            for delta in [-10_000., 10_000.] {
                cx.simulate_event(ScrollEvent {
                    position: point(body.left() + px(2.), body.center().y),
                    delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
                    ..Default::default()
                });
                draw(cx);
                cx.update(|_, cx| {
                    let demo = demo.read(cx);
                    let expected = if delta < 0. {
                        -demo.fields_scroll.max_offset().y
                    } else {
                        px(0.)
                    };
                    assert!((demo.fields_scroll.offset().y - expected).abs() < px(1.));
                    assert_eq!(demo.fields_scroll.offset().x, px(0.));
                    assert!((demo.configuration_scroll.offset().x + maximum.x).abs() < px(1.));
                    assert!((demo.configuration_scroll.offset().y + maximum.y).abs() < px(1.));
                });
            }
        }
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            let trace = demo.read(cx).settings_trace.read(cx);
            assert!(!trace.is_open());
            assert!(trace.trigger_focus_handle().is_focused(window));
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, _| {
            assert!(
                test_support::snapshots(window)
                    .iter()
                    .any(|item| { item.label() == Some("Done") && item.focused() == Some(true) })
            );
        });
        cx.simulate_keystrokes("shift-tab shift-tab");
        draw(cx);
        cx.update(|window, cx| assert!(demo.read(cx).configuration_focus.is_focused(window)));
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(!demo.dialog.read(cx).is_open());
            assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
        });
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert_eq!(demo.input.read(cx).value().as_ref(), name.as_str());
            assert_eq!(demo.notes.read(cx).value().as_ref(), changed_notes.as_str());
            assert_eq!(demo.configuration_code(cx), current_code);
            assert!(demo.input.focus_handle(cx).is_focused(window));
            assert_eq!(demo.tabs.read(cx).selected_index(), Some(2));
            assert!(demo.accordion.read(cx).is_open(2));
        });
    }
}

#[gpui::test]
fn configuration_attachment_tracks_utf8_content_and_keyboard_inspection(cx: &mut TestAppContext) {
    let attachment_id = gpui::ElementId::from("configuration-attachment");
    let code_id = (gpui::ElementId::from("session-configuration"), "code").into();
    let copy_id = (gpui::ElementId::from("session-configuration"), "copy").into();
    let viewport_id = (gpui::ElementId::from("session-settings"), "body").into();
    let assert_metadata = |window: &gpui::Window, code: &str| {
        let attachment = test_support::find(window, &[], &attachment_id).unwrap();
        let description = format!("application/json · {} bytes (UTF-8)", code.len());
        assert_eq!(attachment.role(), Some(Role::Group));
        assert_eq!(attachment.label(), Some("session-configuration.json"));
        assert_eq!(attachment.description(), Some(description.as_str()));
        assert_eq!(attachment.focused(), None);
        assert_eq!(
            test_support::find(window, &[], &code_id).unwrap().label(),
            Some(code)
        );
        assert!(
            code.len() > code.chars().count(),
            "metadata counts UTF-8 bytes, not characters"
        );
        assert!(
            code.contains('\n'),
            "the size includes pretty-printed JSON line breaks"
        );
        let descendants = test_support::snapshots(window);
        assert_eq!(
            descendants
                .iter()
                .filter(|part| {
                    part.path().contains(&attachment_id)
                        && part.label() == Some("session-configuration.json")
                })
                .count(),
            1,
            "the filename is named once"
        );
        for part in descendants.iter().filter(|part| {
            part.path().contains(&attachment_id) && part.role() != Some(Role::Button)
        }) {
            assert_eq!(
                part.focused(),
                None,
                "static attachment metadata has no focus stop"
            );
            if part.path().last() != Some(&attachment_id) {
                assert_eq!(part.role(), None);
                assert_eq!(part.label(), None);
                assert_eq!(part.description(), None);
            }
        }
        attachment.bounds()
    };

    // Different palette padding must not clip the filename or its metadata.
    for appearance in [Appearance::Compact, Appearance::Settings] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(320.), px(480.)));
        draw(cx);
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let input = demo.read(cx).input.clone();
            input.update(cx, |state, cx| state.set_value("Draft 초안 🔍", window, cx));
            let notes = demo.read(cx).notes.clone();
            notes.update(cx, |state, cx| {
                state.set_value("원본 노트📝\nBefore editing.\n", window, cx)
            });
            focus_section(window, cx, &demo, 2);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            let sections = demo.read(cx).accordion.clone();
            sections.update(cx, |state, cx| state.toggle(2, window, cx));
        });
        draw(cx);
        let original = cx.update(|_, cx| demo.read(cx).configuration_code(cx));
        for width in [320., 280.] {
            cx.simulate_resize(gpui::size(px(width), px(480.)));
            draw(cx);
            draw(cx);
            let (position, delta) = cx.update(|window, _| {
                let viewport = test_support::find(window, &[], &viewport_id)
                    .unwrap()
                    .bounds();
                let attachment = assert_metadata(window, original.as_ref());
                (
                    point(viewport.left() + px(2.), viewport.center().y),
                    viewport.center().y - attachment.center().y,
                )
            });
            cx.simulate_event(ScrollEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), delta)),
                ..Default::default()
            });
            draw(cx);
            cx.update(|window, cx| {
                let viewport = test_support::find(window, &[], &viewport_id)
                    .unwrap()
                    .bounds();
                let attachment = assert_metadata(window, original.as_ref());
                assert!(attachment.left() >= viewport.left());
                assert!(attachment.right() <= viewport.right() + px(1.));
                let texts = test_support::texts(window);
                let texts: Vec<_> = texts
                    .iter()
                    .filter(|text| text.path().contains(&attachment_id))
                    .collect();
                assert!(
                    texts
                        .iter()
                        .any(|text| text.text() == "session-configuration.json")
                );
                assert!(texts.iter().any(|text| text.text()
                    == format!("application/json · {} bytes (UTF-8)", original.len())));
                for text in texts {
                    assert!(text.visible());
                    assert!(text.bounds().left() >= attachment.left());
                    assert!(text.bounds().right() <= attachment.right() + px(1.));
                }
                assert_eq!(demo.read(cx).fields_scroll.offset().x, px(0.));
                assert!(
                    demo.read(cx)
                        .tabs
                        .read(cx)
                        .tab_focus_handle(2)
                        .is_focused(window)
                );
            });
        }

        if matches!(appearance, Appearance::Compact) {
            cx.simulate_keystrokes("escape");
            draw(cx);
            continue;
        }

        // One real edit flow proves the attachment follows current form values;
        // its Inspect action uses the existing CodeBlock without creating a file.
        cx.update(|window, cx| demo.read(cx).input.focus_handle(cx).focus(window, cx));
        draw(cx);
        let select_all = if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        };
        cx.simulate_keystrokes(select_all);
        let name = "다음 세션 🎯 Updated configuration";
        cx.simulate_input(name);
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 0);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| demo.read(cx).notes.focus_handle(cx).focus(window, cx));
        draw(cx);
        cx.simulate_keystrokes(select_all);
        let notes = "첫 줄 🚀\nEnglish body with a quoted \"value\".\n\n마지막 줄\n";
        cx.simulate_input(notes);
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 2);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        let current = cx.update(|window, cx| {
            let code = demo.read(cx).configuration_code(cx);
            assert_ne!(code, original);
            let value: serde_json::Value = serde_json::from_str(code.as_ref()).unwrap();
            assert_eq!(value["session_name"], name);
            assert_eq!(value["notes"], notes);
            assert_metadata(window, code.as_ref());
            let panel = demo
                .read(cx)
                .accordion
                .read(cx)
                .panel_focus_handle(2)
                .clone();
            panel.focus(window, cx);
            code
        });
        draw(cx);
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, _| {
            let inspect = test_support::find(window, &[], &"inspect-configuration".into()).unwrap();
            assert_eq!(inspect.role(), Some(Role::Button));
            assert_eq!(inspect.label(), Some("Inspect session configuration JSON"));
            assert_eq!(inspect.focused(), Some(true));
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(
                test_support::find(window, &[], &copy_id).unwrap().focused(),
                Some(true)
            )
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(demo.read(cx).configuration_focus.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(
                test_support::find(window, &[], &copy_id).unwrap().focused(),
                Some(true)
            )
        });
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(
                test_support::find(window, &[], &"inspect-configuration".into())
                    .unwrap()
                    .focused(),
                Some(true)
            )
        });
        for key in ["enter", "space"] {
            activate_key(cx, key);
            draw(cx);
            cx.update(|window, cx| {
                let viewport = test_support::find(window, &[], &viewport_id)
                    .unwrap()
                    .bounds();
                let code = test_support::find(window, &[], &code_id).unwrap();
                assert!(demo.read(cx).configuration_focus.is_focused(window));
                assert_eq!(code.role(), Some(Role::Code));
                assert_eq!(code.label(), Some(current.as_ref()));
                assert!(code.visible());
                assert!(code.bounds().top() >= viewport.top() - px(1.));
                assert!(code.bounds().bottom() <= viewport.bottom() + px(1.));
                assert!(code.bounds().left() >= viewport.left());
                assert!(code.bounds().right() <= viewport.right() + px(1.));
                assert_metadata(window, current.as_ref());
            });
            if key == "enter" {
                cx.simulate_keystrokes("shift-tab shift-tab");
                draw(cx);
            }
        }
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.update(|window, cx| {
            assert!(!demo.read(cx).dialog.read(cx).is_open());
            assert!(
                demo.read(cx)
                    .edit_hint
                    .read(cx)
                    .focus_handle()
                    .is_focused(window)
            );
        });
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert_eq!(demo.input.read(cx).value(), name);
            assert_eq!(demo.notes.read(cx).value(), notes);
            assert_eq!(demo.configuration_code(cx), current);
            assert_eq!(demo.tabs.read(cx).selected_index(), Some(2));
            assert!(demo.accordion.read(cx).is_open(2));
            assert_metadata(window, current.as_ref());
        });
    }
}

#[gpui::test]
fn settings_trace_discloses_current_values_with_retained_focus_and_parent_scroll(
    cx: &mut TestAppContext,
) {
    let trace_id = gpui::ElementId::from("settings-trace");
    let body_id = (trace_id.clone(), "body").into();
    let summary_id = (trace_id.clone(), "summary").into();
    let viewport_id = (gpui::ElementId::from("session-settings"), "body").into();
    let long_name = "긴 세션_LongEnglish_".repeat(6);
    let long_notes = format!(
        "첫 줄\n{}\n\n{}\n마지막 줄\n",
        "한국어와 English details. ".repeat(12),
        "unbroken설정".repeat(20)
    );
    let details = |session: &str, date: &str, notes: &str| {
        format!(
            "Current form values\nSession: {session}\nRecording date: {date}\nPreset: Balanced\nDrawing tool: Pen\nCursor: Visible\nToolbar: Automatic\nNotes:\n{notes}"
        )
    };

    for appearance in [Appearance::Compact, Appearance::Settings] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(320.), px(480.)));
        draw(cx);
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let input = demo.read(cx).input.clone();
            input.update(cx, |state, cx| {
                state.set_value(long_name.clone(), window, cx)
            });
            let notes = demo.read(cx).notes.clone();
            notes.update(cx, |state, cx| {
                state.set_value(long_notes.clone(), window, cx)
            });
            focus_section(window, cx, &demo, 2);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            let accordion = demo.read(cx).accordion.clone();
            accordion.update(cx, |state, cx| state.toggle(2, window, cx));
        });
        draw(cx);
        let (trigger_id, content_id) = cx.update(|_, cx| {
            let id = demo.read(cx).settings_trace.entity_id();
            (
                gpui::ElementId::from(("collapsible-trigger", id)),
                gpui::ElementId::from(("collapsible-content", id)),
            )
        });
        let expected = details(&long_name, "2026-11-09", &long_notes);

        // Layout is checked with both palettes; the action flow below runs once.
        for width in [320., 280.] {
            cx.simulate_resize(gpui::size(px(width), px(480.)));
            draw(cx);
            let closed_height = cx.update(|window, cx| {
                assert!(!demo.read(cx).settings_trace.read(cx).is_open());
                let trace = test_support::find(window, &[], &trace_id).unwrap();
                assert_eq!(trace.role(), Some(Role::Group));
                assert_eq!(trace.label(), None);
                let trigger = test_support::find(window, &[], &trigger_id).unwrap();
                assert_eq!(trigger.role(), Some(Role::Button));
                assert_eq!(
                    trigger.label(),
                    Some(format!("Current settings · {long_name}").as_str())
                );
                assert_eq!(trigger.expanded(), Some(false));
                assert!(test_support::find(window, &[], &content_id).is_none());
                assert!(test_support::find(window, &[], &body_id).is_none());
                assert!(test_support::find(window, &[], &"close-settings-trace".into()).is_none());
                let state = demo.read(cx).settings_trace.clone();
                state.update(cx, |state, cx| state.set_open(true, window, cx));
                trace.bounds().size.height
            });
            draw(cx);
            draw(cx);
            let viewport = cx.update(|window, _| {
                test_support::find(window, &[], &viewport_id)
                    .unwrap()
                    .bounds()
            });
            cx.simulate_event(ScrollEvent {
                position: point(viewport.left() + px(2.), viewport.center().y),
                delta: ScrollDelta::Pixels(point(px(0.), px(-10_000.))),
                ..Default::default()
            });
            draw(cx);
            let (initial_body_bounds, code_offset) = cx.update(|window, cx| {
                let trace = test_support::find(window, &[], &trace_id).unwrap();
                assert!(trace.bounds().size.height > closed_height);
                assert!(trace.bounds().left() >= viewport.left());
                assert!(trace.bounds().right() <= viewport.right() + px(1.));
                let region = test_support::find(window, &[], &content_id).unwrap();
                assert_eq!(region.role(), Some(Role::Region));
                assert_eq!(region.label(), Some("Current settings details"));
                let body = test_support::find(window, &[], &body_id).unwrap();
                assert_eq!(body.role(), Some(Role::Paragraph));
                assert_eq!(body.label(), Some(expected.as_str()));
                assert_eq!(body.focused(), None);
                for part in [&summary_id, &body_id] {
                    let text = test_support::texts(window)
                        .into_iter()
                        .find(|text| text.path().contains(part))
                        .unwrap();
                    assert!(text.bounds().left() >= trace.bounds().left());
                    assert!(text.bounds().right() <= trace.bounds().right() + px(1.));
                    if part == &body_id {
                        assert_eq!(text.text(), expected);
                    }
                }
                assert!(test_support::snapshots(window).iter().all(|part| {
                    !part.path().contains(&trace_id) || part.role() != Some(Role::ScrollView)
                }));
                let scroll = &demo.read(cx).fields_scroll;
                assert_eq!(scroll.offset().x, px(0.));
                assert_eq!(scroll.max_offset().x, px(0.));
                assert!(scroll.max_offset().y > px(0.));
                assert!((scroll.offset().y + scroll.max_offset().y).abs() < px(1.));
                (body.bounds(), demo.read(cx).configuration_scroll.offset())
            });
            // At narrow heights, the action and source text after the paragraph
            // can fill the viewport at its end. Reveal the paragraph with a
            // real wheel over the sheet gutter before testing a wheel on it.
            let reveal_delta = (viewport.center().y - initial_body_bounds.bottom()).max(px(0.));
            if reveal_delta > px(0.) {
                cx.simulate_event(ScrollEvent {
                    position: point(viewport.left() + px(2.), viewport.center().y),
                    delta: ScrollDelta::Pixels(point(px(0.), reveal_delta)),
                    ..Default::default()
                });
                draw(cx);
            }
            let (over_body, visible_body, body_bounds, before_body_wheel) = cx.update(|window, cx| {
                let body = test_support::find(window, &[], &body_id).unwrap();
                let visible = body.bounds().intersect(&viewport);
                assert!(
                    body.visible() && visible.size.width > px(2.) && visible.size.height > px(2.),
                    "width {width}: body {:?} must intersect viewport {viewport:?}; visible {visible:?}",
                    body.bounds(),
                );
                let position = visible.center();
                assert!(body.bounds().contains(&position) && viewport.contains(&position));
                assert_eq!(demo.read(cx).configuration_scroll.offset(), code_offset);
                (
                    position,
                    visible,
                    body.bounds(),
                    demo.read(cx).fields_scroll.offset().y,
                )
            });
            cx.simulate_event(ScrollEvent {
                position: over_body,
                delta: ScrollDelta::Pixels(point(px(0.), px(80.))),
                ..Default::default()
            });
            draw(cx);
            cx.update(|_, cx| {
                assert!(
                    demo.read(cx).fields_scroll.offset().y > before_body_wheel,
                    "width {width}: wheel at {over_body:?} inside visible body {visible_body:?} (full {body_bounds:?}, viewport {viewport:?}) must scroll parent from {before_body_wheel:?}; actual {:?}",
                    demo.read(cx).fields_scroll.offset(),
                );
                assert_eq!(demo.read(cx).configuration_scroll.offset(), code_offset);
            });
            cx.simulate_event(ScrollEvent {
                position: point(viewport.left() + px(2.), viewport.center().y),
                delta: ScrollDelta::Pixels(point(px(0.), px(10_000.))),
                ..Default::default()
            });
            draw(cx);
            cx.update(|window, cx| {
                assert_eq!(demo.read(cx).fields_scroll.offset().y, px(0.));
                let state = demo.read(cx).settings_trace.clone();
                state.update(cx, |state, cx| state.set_open(false, window, cx));
            });
            draw(cx);
            cx.update(|window, _| {
                assert!(
                    (test_support::find(window, &[], &trace_id)
                        .unwrap()
                        .bounds()
                        .size
                        .height
                        - closed_height)
                        .abs()
                        < px(1.)
                );
                assert!(test_support::find(window, &[], &body_id).is_none());
                assert!(test_support::find(window, &[], &content_id).is_none());
            });
        }
        if matches!(appearance, Appearance::Compact) {
            cx.simulate_keystrokes("escape");
            draw(cx);
            continue;
        }

        cx.update(|window, cx| demo.read(cx).input.focus_handle(cx).focus(window, cx));
        draw(cx);
        let select_all = if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        };
        cx.simulate_keystrokes(select_all);
        let name = "현재 작업 🎯";
        cx.simulate_input(name);
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 0);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| demo.read(cx).notes.focus_handle(cx).focus(window, cx));
        draw(cx);
        cx.simulate_keystrokes(select_all);
        let notes = "첫 줄\nCurrent notes 🚀\n";
        cx.simulate_input(notes);
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 2);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            let trigger = demo
                .read(cx)
                .settings_trace
                .read(cx)
                .trigger_focus_handle()
                .clone();
            trigger.focus(window, cx);
        });
        draw(cx);
        activate_key(cx, "enter");
        cx.update(|window, cx| {
            let state = demo.read(cx).settings_trace.read(cx);
            assert!(state.is_open());
            assert!(state.trigger_focus_handle().is_focused(window));
            let trigger = test_support::find(window, &[], &trigger_id).unwrap();
            assert_eq!(trigger.expanded(), Some(true));
            assert_eq!(
                trigger.label(),
                Some(format!("Current settings · {name}").as_str())
            );
            assert_eq!(
                test_support::find(window, &[], &body_id).unwrap().label(),
                Some(details(name, "2026-11-09", notes).as_str())
            );
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(
                test_support::find(window, &[], &"close-settings-trace".into())
                    .unwrap()
                    .focused(),
                Some(true)
            )
        });

        // Replacing current application values while the disclosure is open
        // must not remount its state or steal focus from its existing action.
        let replacement_name = "Refreshed session 교체";
        let replacement_notes = "Current replacement\n다시 읽는 본문";
        cx.update(|window, cx| {
            let input = demo.read(cx).input.clone();
            input.update(cx, |state, cx| {
                state.set_value(replacement_name, window, cx)
            });
            let notes = demo.read(cx).notes.clone();
            notes.update(cx, |state, cx| {
                state.set_value(replacement_notes, window, cx)
            });
            let calendar = demo.read(cx).calendar.clone();
            calendar.update(cx, |state, cx| {
                assert!(state.set_selection(CalendarSelection::Single(None), window, cx))
            });
        });
        draw(cx);
        let replacement = details(replacement_name, "Not selected", replacement_notes);
        cx.update(|window, cx| {
            assert!(demo.read(cx).settings_trace.read(cx).is_open());
            assert_eq!(
                test_support::find(window, &[], &"close-settings-trace".into())
                    .unwrap()
                    .focused(),
                Some(true)
            );
            assert_eq!(
                test_support::find(window, &[], &body_id).unwrap().label(),
                Some(replacement.as_str())
            );
            assert_eq!(
                test_support::find(window, &[], &trigger_id)
                    .unwrap()
                    .label(),
                Some(format!("Current settings · {replacement_name}").as_str())
            );
        });
        activate_key(cx, "enter");
        draw(cx);
        cx.update(|window, cx| {
            let state = demo.read(cx).settings_trace.read(cx);
            assert!(!state.is_open());
            assert!(state.trigger_focus_handle().is_focused(window));
            assert!(
                test_support::find(window, &[], &trigger_id)
                    .unwrap()
                    .visible()
            );
            assert!(test_support::find(window, &[], &content_id).is_none());
            assert!(test_support::find(window, &[], &"close-settings-trace".into()).is_none());
        });
        activate_key(cx, "space");
        cx.update(|window, cx| {
            let state = demo.read(cx).settings_trace.read(cx);
            assert!(state.is_open());
            assert!(state.trigger_focus_handle().is_focused(window));
        });
        cx.simulate_keystrokes("tab escape");
        draw(cx);
        cx.update(|window, cx| {
            assert!(!demo.read(cx).dialog.read(cx).is_open());
            assert!(demo.read(cx).settings_trace.read(cx).is_open());
            assert!(
                demo.read(cx)
                    .edit_hint
                    .read(cx)
                    .focus_handle()
                    .is_focused(window)
            );
        });
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(demo.settings_trace.read(cx).is_open());
            assert_eq!(demo.input.read(cx).value(), replacement_name);
            assert_eq!(demo.notes.read(cx).value(), replacement_notes);
            assert_eq!(demo.calendar.read(cx).selection().first_date(), None);
            assert_eq!(demo.tabs.read(cx).selected_index(), Some(2));
            assert!(demo.accordion.read(cx).is_open(2));
            assert_eq!(
                test_support::find(window, &[], &body_id).unwrap().label(),
                Some(replacement.as_str())
            );
        });
    }
}

#[gpui::test]
fn compact_commands_and_sections_preserve_nested_focus(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Compact);
    let commands = Rc::new(RefCell::new(Vec::new()));
    let assert_menu_checks = |cx: &mut VisualTestContext, cursor: bool, toolbar: bool| {
        cx.update(|window, _| {
            let items = test_support::snapshots(window);
            for (label, checked) in [("Show cursor", cursor), ("Keep toolbar visible", toolbar)] {
                let item = items
                    .iter()
                    .find(|item| {
                        item.role() == Some(Role::MenuItemCheckBox) && item.label() == Some(label)
                    })
                    .unwrap();
                assert_eq!(item.checked(), Some(checked));
            }
        });
    };
    cx.update(|_, cx| {
        let menu = demo.read(cx).actions.clone();
        let commands = commands.clone();
        cx.subscribe(&menu, move |menu, event: &MenuEvent, cx| {
            assert!(!menu.read(cx).is_open(cx), "commands observe a closed menu");
            let MenuEvent::Action(value) = event;
            commands.borrow_mut().push(value.clone());
        })
        .detach();
    });

    cx.simulate_keystrokes("tab down");
    draw(cx);
    cx.update(|window, cx| {
        let menu = demo.read(cx).actions.read(cx);
        assert!(menu.is_open(cx));
        assert_eq!(menu.highlighted_index(), Some(0));
        assert!(menu.content_focus_handle().is_focused(window));
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::Menu) && item.label() == Some("Session actions")
        }));
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::MenuItem)
                && item.label() == Some("Share (unavailable)")
                && item.disabled() == Some(true)
        }));
    });
    assert_menu_checks(cx, true, false);
    cx.simulate_keystrokes("down");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(demo.read(cx).actions.read(cx).highlighted_index(), Some(2));
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.show_cursor);
        assert!(!demo.actions.read(cx).is_open(cx));
        assert!(demo.actions.read(cx).focus_handle().is_focused(window));
    });

    cx.simulate_keystrokes("down k");
    draw(cx);
    assert_menu_checks(cx, false, false);
    cx.update(|_, cx| {
        assert_eq!(demo.read(cx).actions.read(cx).highlighted_index(), Some(3));
    });
    activate_key(cx, "space");
    cx.update(|_, cx| assert!(demo.read(cx).keep_toolbar));

    cx.simulate_keystrokes("down e");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.actions.read(cx).is_open(cx));
        assert!(demo.dialog.read(cx).is_open());
        assert!(demo.input.focus_handle(cx).is_focused(window));
        assert_eq!(
            test_support::find(window, &[], &"show-cursor".into())
                .unwrap()
                .checked(),
            Some(false)
        );
        assert_eq!(
            test_support::find(window, &[], &"keep-toolbar".into())
                .unwrap()
                .checked(),
            Some(true)
        );
    });
    assert_eq!(
        commands.borrow().as_slice(),
        &["cursor", "toolbar", "edit"].map(gpui::SharedString::from),
        "native key-up must not execute a command twice"
    );

    cx.simulate_keystrokes("tab tab right tab");
    draw(cx);
    cx.update(|window, cx| {
        let accordion = demo.read(cx).accordion.read(cx);
        assert!(accordion.is_open(0));
        assert!(accordion.trigger_focus_handle(0).is_focused(window));
    });
    cx.simulate_keystrokes("down");
    draw(cx);
    cx.update(|window, cx| {
        let accordion = demo.read(cx).accordion.read(cx);
        assert!(accordion.trigger_focus_handle(2).is_focused(window));
        assert!(accordion.is_open(0), "arrows only move header focus");
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert!(demo.read(cx).accordion.read(cx).is_open(2));
        assert!(test_support::find(window, &[], &"keyboard-help".into()).is_none());
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .accordion
                .read(cx)
                .panel_focus_handle(2)
                .is_focused(window),
            "the static text panel has an explicit Tab stop"
        );
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    activate_key(cx, "space");
    cx.update(|_, cx| assert!(demo.read(cx).accordion.read(cx).open_values().is_empty()));

    cx.simulate_keystrokes("home");
    draw(cx);
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.help.read(cx).is_open());
        assert!(demo.help.read(cx).content_focus_handle().is_focused(window));
        let accordion = demo.accordion.clone();
        accordion.update(cx, |state, cx| state.close(window, cx));
    });
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.accordion.read(cx).open_values().is_empty());
        assert!(!demo.help.read(cx).is_open());
        assert!(
            demo.accordion
                .read(cx)
                .trigger_focus_handle(0)
                .is_focused(window)
        );
        assert!(test_support::find(window, &[], &"keyboard-help".into()).is_none());
        assert!(demo.dialog.read(cx).is_open());
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.accordion.read(cx).is_open(0));
        assert!(!demo.help.read(cx).is_open());
        assert!(test_support::find(window, &[], &"keyboard-help".into()).is_some());
        assert!(!demo.show_cursor);
        assert!(demo.keep_toolbar);
    });

    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(
            demo.actions.read(cx).focus_handle().is_focused(window),
            "the dialog returns to the command menu's surviving trigger"
        );
    });

    // Pointer activation follows the same application callback as keyboard activation.
    activate_key(cx, "enter");
    let cursor_point = cx.update(|window, _| {
        let cursor = test_support::snapshots(window)
            .into_iter()
            .find(|item| {
                item.role() == Some(Role::MenuItemCheckBox) && item.label() == Some("Show cursor")
            })
            .unwrap();
        assert_eq!(cursor.checked(), Some(false));
        cursor.bounds().center()
    });
    cx.simulate_click(cursor_point, Modifiers::default());
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.show_cursor);
        assert!(!demo.actions.read(cx).is_open(cx));
        assert!(demo.actions.read(cx).focus_handle().is_focused(window));
    });
    assert_eq!(commands.borrow().len(), 4);

    // The preview controls mount only after their disclosure is opened.
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let options = demo.read(cx).preview_options.read(cx);
        assert!(!options.is_open());
        assert!(options.trigger_focus_handle().is_focused(window));
        assert!(test_support::find(window, &[], &"preview-progress".into()).is_none());
    });
    activate_key(cx, "space");

    let label_point = cx.update(|window, _| {
        let label = test_support::find(window, &[], &"preview-percentage-label".into()).unwrap();
        assert_eq!(label.label(), Some("Preview percentage"));
        assert_eq!(label.focused(), None);
        label.bounds().center()
    });
    cx.simulate_click(label_point, Modifiers::default());
    draw(cx);
    cx.update(|window, cx| {
        let preview = demo.read(cx).preview.read(cx);
        assert!(preview.thumb_focus_handle(0).is_focused(window));
        assert_eq!(
            preview.values()[0],
            40.0,
            "a label does not adjust the Slider"
        );
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .preview_options
                .read(cx)
                .trigger_focus_handle()
                .is_focused(window),
            "the label adds no Tab stop"
        );
    });

    // The same retained Slider value drives the application's progress preview.
    cx.simulate_keystrokes("tab right");
    draw(cx);
    cx.update(|window, cx| {
        let preview = demo.read(cx).preview.read(cx);
        assert!(preview.thumb_focus_handle(0).is_focused(window));
        assert_eq!(preview.values()[0], 45.0);
        let progress = test_support::find(window, &[], &"preview-progress".into()).unwrap();
        assert_eq!(progress.role(), Some(Role::ProgressIndicator));
        assert_eq!(progress.label(), Some("Preview progress"));
        assert!(test_support::find(window, &[], &"preview-progress-placeholder".into()).is_none());
    });
    cx.simulate_keystrokes("end");
    draw(cx);
    let progress_width = cx.update(|window, cx| {
        assert_eq!(demo.read(cx).preview.read(cx).values()[0], 100.0);
        test_support::find(window, &[], &"preview-progress".into())
            .unwrap()
            .bounds()
            .size
            .width
    });
    assert_eq!(
        cx.debug_bounds("preview-progress-fill").unwrap().size.width,
        progress_width,
    );
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let toggle = test_support::find(window, &[], &"unknown-progress".into()).unwrap();
        assert_eq!(toggle.role(), Some(Role::Button));
        assert_eq!(toggle.focused(), Some(true), "Progress has no Tab stop");
        assert_eq!(toggle.checked(), Some(false));
    });
    activate_key(cx, "space");
    cx.update(|window, cx| {
        assert!(demo.read(cx).preview_indeterminate);
        assert_eq!(demo.read(cx).preview.read(cx).values()[0], 100.0);
        let progress = test_support::find(window, &[], &"preview-progress".into()).unwrap();
        assert_eq!(progress.role(), Some(Role::ProgressIndicator));
        assert_eq!(progress.label(), Some("Preview progress"));
        assert_eq!(progress.focused(), None);
        let placeholder =
            test_support::find(window, &[], &"preview-progress-placeholder".into()).unwrap();
        assert!(placeholder.visible());
        assert_eq!(placeholder.role(), None);
        assert_eq!(placeholder.label(), None);
        assert_eq!(placeholder.focused(), None);
        assert_eq!(
            test_support::find(window, &[], &"unknown-progress".into())
                .unwrap()
                .checked(),
            Some(true),
        );
    });
    assert!(cx.debug_bounds("preview-progress-fill").is_none());

    // The native reduced-motion path keeps a visible, named loading indicator.
    cx.update(|_, cx| cx.set_reduce_motion(true));
    draw(cx);
    cx.update(|window, _| {
        let progress = test_support::find(window, &[], &"preview-progress".into()).unwrap();
        assert!(progress.visible());
        assert_eq!(progress.label(), Some("Preview progress"));
    });
    cx.update(|_, cx| cx.set_reduce_motion(false));
    draw(cx);
    cx.simulate_keystrokes("tab right");
    draw(cx);
    cx.update(|window, cx| {
        let formatting = demo.read(cx).formatting.read(cx);
        assert!(formatting.option_focus_handle(2).is_focused(window));
        assert_eq!(
            formatting.selected_values(),
            [gpui::SharedString::from("bold")]
        );
    });
    activate_key(cx, "space");
    cx.update(|_, cx| {
        assert_eq!(
            demo.read(cx).formatting.read(cx).selected_values(),
            ["bold", "italic"].map(gpui::SharedString::from),
        );
    });
    cx.simulate_keystrokes("tab tab tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .edit_hint
                .read(cx)
                .focus_handle()
                .is_focused(window),
            "ToggleGroup has one Tab stop before the summary, preview, and Edit controls"
        );
    });

    // Reset is an explicit confirmation, with cancellation as the initial focus.
    cx.simulate_keystrokes("tab down r");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.reset_dialog.read(cx).is_open());
        assert!(demo.reset_cancel_focus.is_focused(window));
        assert_eq!(demo.preview.read(cx).values()[0], 100.0);
        assert!(demo.preview_indeterminate);
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::AlertDialog) && item.label() == Some("Reset preview?")
        }));
    });
    cx.simulate_click(
        gpui::point(gpui::px(2.), gpui::px(2.)),
        Modifiers::default(),
    );
    draw(cx);
    cx.update(|window, cx| {
        assert!(demo.read(cx).reset_dialog.read(cx).is_open());
        assert!(demo.read(cx).reset_cancel_focus.is_focused(window));
    });
    activate_key(cx, "space");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.reset_dialog.read(cx).is_open());
        assert_eq!(demo.preview.read(cx).values()[0], 100.0);
        assert!(demo.preview_indeterminate);
        assert!(demo.actions.read(cx).focus_handle().is_focused(window));
        assert!(test_support::find(window, &[], &"preview-progress-placeholder".into()).is_some());
    });
    cx.simulate_keystrokes("down r");
    draw(cx);
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.reset_dialog.read(cx).is_open());
        assert_eq!(demo.preview.read(cx).values()[0], 40.0);
        assert!(!demo.preview_indeterminate);
        assert!(demo.actions.read(cx).focus_handle().is_focused(window));
        assert!(test_support::find(window, &[], &"preview-progress-placeholder".into()).is_none());
    });
    let restored_fill = cx.debug_bounds("preview-progress-fill").unwrap().size.width;
    assert!((restored_fill - progress_width * 0.4).abs() < px(1.));

    // Closing the disclosure removes its Tab stops without losing application data.
    cx.simulate_keystrokes("tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.preview_options.read(cx).is_open());
        assert!(test_support::find(window, &[], &"preview-progress".into()).is_none());
        assert!(test_support::find(window, &[], &"preview-progress-placeholder".into()).is_none());
        assert!(
            demo.context_actions
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
        assert_eq!(demo.preview.read(cx).values()[0], 40.0);
        assert_eq!(
            demo.formatting.read(cx).selected_values(),
            ["bold", "italic"].map(gpui::SharedString::from),
        );
    });

    // The same application actions are available from the summary's context menu.
    cx.simulate_keystrokes("shift-f10");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.context_actions.read(cx).is_open(cx));
        assert!(!demo.actions.read(cx).is_open(cx));
        assert!(
            demo.context_actions
                .read(cx)
                .content_focus_handle()
                .is_focused(window)
        );
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::Menu) && item.label() == Some("Session summary actions")
        }));
    });
    assert_menu_checks(cx, true, true);
    cx.simulate_keystrokes("down");
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.show_cursor);
        assert!(!demo.context_actions.read(cx).is_open(cx));
        assert!(
            demo.context_actions
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    let summary = cx.update(|window, _| {
        test_support::find(window, &[], &"session-summary".into())
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_mouse_down(summary, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(summary, MouseButton::Right, Modifiers::default());
    draw(cx);
    assert_menu_checks(cx, false, true);
    cx.simulate_keystrokes("e");
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.context_actions.read(cx).is_open(cx));
        assert!(demo.dialog.read(cx).is_open());
        assert!(demo.input.focus_handle(cx).is_focused(window));
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(
            demo.context_actions
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
}

#[gpui::test]
fn command_search_opens_a_retained_multiline_form(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Settings);
    let actions = Rc::new(RefCell::new(Vec::new()));
    cx.update(|_, cx| {
        let commands = demo.read(cx).commands.clone();
        let actions = actions.clone();
        cx.subscribe(&commands, move |_, event: &CommandEvent, _| {
            let CommandEvent::Action(value) = event;
            actions.borrow_mut().push(value.clone());
        })
        .detach();
    });
    cx.simulate_keystrokes("tab down s s");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.command_dialog.read(cx).is_open());
        assert!(demo.commands.read(cx).focus_handle().is_focused(window));
        let input = test_support::find(window, &[], &"command-query".into()).unwrap();
        assert_eq!(input.role(), Some(Role::TextInput));
        assert_eq!(input.label(), Some("Search commands"));
        assert_eq!(input.value(), Some(""));
    });
    cx.simulate_input("no matching command");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|_, cx| {
        assert!(demo.read(cx).commands.read(cx).visible_indices().is_empty());
        assert!(demo.read(cx).command_dialog.read(cx).is_open());
    });
    assert!(actions.borrow().is_empty());
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let clear = test_support::find(window, &[], &"clear-command-search".into()).unwrap();
        assert_eq!(clear.role(), Some(Role::Button));
        assert_eq!(clear.label(), Some("Clear search"));
        assert_eq!(clear.focused(), Some(true));
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        let commands = demo.commands.read(cx);
        assert!(commands.input().read(cx).value().is_empty());
        assert_eq!(commands.visible_indices(), &[0, 1, 2, 3, 4]);
        assert!(commands.focus_handle().is_focused(window));
        assert!(demo.command_dialog.read(cx).is_open());
    });
    assert!(actions.borrow().is_empty());
    cx.simulate_input("설정");
    draw(cx);
    cx.update(|_, cx| {
        let commands = demo.read(cx).commands.read(cx);
        assert_eq!(commands.visible_indices(), &[0]);
        assert_eq!(commands.highlighted_index(), Some(0));
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.command_dialog.read(cx).is_open());
        assert!(demo.dialog.read(cx).is_open());
        assert!(demo.input.focus_handle(cx).is_focused(window));
    });
    assert_eq!(
        actions.borrow().as_slice(),
        &[gpui::SharedString::from("edit")]
    );

    cx.simulate_keystrokes("tab tab tab tab tab tab tab tab tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(demo.read(cx).notes.focus_handle(cx).is_focused(window));
        let notes = test_support::find(window, &[], &"session-notes".into()).unwrap();
        assert_eq!(notes.role(), Some(Role::MultilineTextInput));
        assert_eq!(notes.label(), Some("Session notes"));
        assert_eq!(
            test_support::find(window, &[], &"notes-separator".into())
                .unwrap()
                .role(),
            Some(Role::Splitter),
        );
    });
    cx.simulate_input("first line");
    activate_key(cx, "enter");
    cx.simulate_input("두 번째 줄");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            demo.read(cx).notes.read(cx).value(),
            "first line\n두 번째 줄"
        );
        assert!(
            demo.read(cx).dialog.read(cx).is_open(),
            "Enter edits multiline text"
        );
        demo.read(cx)
            .notes
            .clone()
            .update(cx, |state, cx| state.set_disabled(true, cx));
    });
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .preset
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|item| { item.label() == Some("Done") && item.focused() == Some(true) })
        );
    });
    cx.simulate_keystrokes("shift-tab shift-tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx)
                .stroke
                .read(cx)
                .option_focus_handle(0)
                .is_focused(window)
        );
        demo.read(cx)
            .notes
            .clone()
            .update(cx, |state, cx| state.set_disabled(false, cx));
    });
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| assert!(demo.read(cx).notes.focus_handle(cx).is_focused(window)));
    cx.simulate_keystrokes("tab tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(demo.actions.read(cx).focus_handle().is_focused(window));
        assert_eq!(demo.notes.read(cx).value(), "first line\n두 번째 줄");
    });
    cx.simulate_keystrokes("down s s");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|_, cx| {
        let demo = demo.read(cx);
        assert!(demo.command_dialog.read(cx).is_open());
        assert_eq!(demo.commands.read(cx).input().read(cx).value(), "");
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).command_dialog.read(cx).is_open());
        assert!(
            demo.read(cx)
                .actions
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    assert_eq!(actions.borrow().len(), 1);
}

#[gpui::test]
fn field_feedback_and_search_selection_work_inside_the_dialog(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Settings);
    let changes = Rc::new(RefCell::new(Vec::new()));
    cx.update(|window, cx| {
        assert!(demo.read(cx).edit_hint.read(cx).is_open());
        let trigger = test_support::find(window, &[], &"edit-session".into()).unwrap();
        assert_eq!(
            trigger.description(),
            Some("Change the session name, recording preset, and notes.")
        );
        let preset = demo.read(cx).preset.clone();
        let changes = changes.clone();
        cx.subscribe(&preset, move |_, event: &ComboboxEvent, _| {
            let ComboboxEvent::Change(value) = event;
            changes.borrow_mut().push(value.clone());
        })
        .detach();
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).edit_hint.read(cx).is_open());
        assert!(
            demo.read(cx)
                .edit_hint
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.preset_preview.read(cx).is_open());
        assert!(
            demo.preset_preview
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
        assert!(!demo.dialog.read(cx).is_open());
        let avatar = test_support::find(window, &[], &"preset-avatar".into()).unwrap();
        assert_eq!(avatar.role(), Some(Role::Image));
        assert_eq!(avatar.label(), Some("Balanced preset artwork"));
        assert_eq!(avatar.focused(), None);
        let frame = test_support::find(window, &[], &"preset-frame".into()).unwrap();
        let size = frame.bounds().size;
        assert!((size.width / size.height - 16. / 9.).abs() < 0.01);
        assert_eq!(frame.role(), None);
        assert_eq!(frame.focused(), None);
    });
    // The preview keeps the trigger's normal activation and the dialog's return focus.
    open_dialog(cx, &demo);
    cx.update(|_, cx| assert!(!demo.read(cx).preset_preview.read(cx).is_open()));
    let select_all = if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    };
    cx.simulate_keystrokes(select_all);
    cx.simulate_keystrokes("backspace");
    draw(cx);
    cx.update(|window, cx| {
        let input = demo.read(cx).input.read(cx);
        let semantics = input.accessibility().unwrap();
        assert!(semantics.is_required());
        assert!(semantics.is_invalid());
        let name = test_support::find(window, &[], &"session-name".into()).unwrap();
        assert_eq!(
            name.description(),
            Some("A session name is required. Choose a name to recognize this session.")
        );
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|item| { item.label() == Some("Done") && item.disabled() == Some(true) })
        );
        let error_id = (gpui::ElementId::from("session-name-field"), "error").into();
        assert!(
            test_support::find(window, &[], &error_id)
                .unwrap()
                .visible()
        );
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).dialog.read(cx).is_open());
        let alert = test_support::find(window, &[], &"session-name-alert".into()).unwrap();
        assert_eq!(alert.role(), Some(Role::Alert));
        assert_eq!(alert.label(), Some("Session name is missing"));
        assert!(
            demo.read(cx)
                .preset_preview
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    open_dialog(cx, &demo);
    cx.update(|window, cx| {
        assert!(test_support::find(window, &[], &"session-name-alert".into()).is_none());
        demo.read(cx)
            .input
            .clone()
            .update(cx, |state, cx| state.set_readonly(true, cx));
    });
    draw(cx);
    cx.update(|window, cx| {
        let action = test_support::find(window, &[], &"use-default-session-name".into()).unwrap();
        assert_eq!(action.disabled(), Some(true));
        assert!(demo.read(cx).input.read(cx).value().is_empty());
        demo.read(cx)
            .input
            .clone()
            .update(cx, |state, cx| state.set_readonly(false, cx));
    });
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        let action = test_support::find(window, &[], &"use-default-session-name".into()).unwrap();
        assert_eq!(action.label(), Some("Use default name"));
        assert_eq!(action.focused(), Some(true));
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let input = demo.read(cx).input.read(cx);
        assert_eq!(input.value(), "MiniCast");
        assert!(!input.accessibility().unwrap().is_invalid());
        assert!(demo.read(cx).input.focus_handle(cx).is_focused(window));
        assert!(test_support::find(window, &[], &"use-default-session-name".into()).is_none());
    });
    cx.simulate_keystrokes(select_all);
    cx.simulate_input("Workshop");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            !demo
                .read(cx)
                .input
                .read(cx)
                .accessibility()
                .unwrap()
                .is_invalid()
        );
        let error_id = (gpui::ElementId::from("session-name-field"), "error").into();
        assert!(test_support::find(window, &[], &error_id).is_none());
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    let label = cx.update(|window, _| {
        let id = (gpui::ElementId::from("session-name-field"), "label").into();
        test_support::find(window, &[], &id)
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(label, Modifiers::default());
    draw(cx);
    cx.update(|window, cx| assert!(demo.read(cx).input.focus_handle(cx).is_focused(window)));

    cx.simulate_keystrokes("tab tab tab tab tab tab tab tab tab tab");
    draw(cx);
    draw(cx); // The focused input's reveal request updates the form's scroll offset.
    cx.update(|window, cx| {
        let preset = demo.read(cx).preset.read(cx);
        assert!(preset.focus_handle().is_focused(window));
        let input = test_support::find(window, &[], &"recording-preset".into()).unwrap();
        assert!(input.visible());
        assert_eq!(input.role(), Some(Role::ComboBox));
        assert_eq!(input.label(), Some("Recording preset"));
        assert_eq!(input.value(), Some("Balanced"));
        assert_eq!(
            input.description(),
            Some("Search a preset and press Enter to select it.")
        );
        assert_eq!(input.expanded(), Some(false));
    });
    cx.simulate_keystrokes(select_all);
    cx.simulate_input("발표");
    draw(cx);
    cx.update(|_, cx| {
        let preset = demo.read(cx).preset.read(cx);
        assert!(preset.is_open());
        assert_eq!(preset.visible_indices(cx), &[2]);
        assert_eq!(
            preset
                .selected_items(cx)
                .next()
                .map(|item| item.value())
                .map(|value| value.as_ref()),
            Some("balanced")
        );
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        let preset = demo.preset.read(cx);
        assert!(!preset.is_open());
        assert!(
            demo.dialog.read(cx).is_open(),
            "first Escape only cancels suggestions"
        );
        assert!(preset.focus_handle().is_focused(window));
        assert_eq!(preset.input().read(cx).value(), "Balanced");
    });
    assert!(changes.borrow().is_empty());
    cx.simulate_keystrokes(select_all);
    cx.simulate_input("발표");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let preset = demo.read(cx).preset.read(cx);
        assert!(!preset.is_open());
        assert!(preset.focus_handle().is_focused(window));
        assert_eq!(
            preset
                .selected_items(cx)
                .next()
                .map(|item| item.value())
                .map(|value| value.as_ref()),
            Some("presentation")
        );
        assert_eq!(preset.input().read(cx).value(), "Presentation");
    });
    assert_eq!(
        changes.borrow().as_slice(),
        &[vec![gpui::SharedString::from("presentation")]]
    );
    cx.simulate_keystrokes(select_all);
    cx.simulate_input("no matching preset");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|_, cx| assert!(demo.read(cx).preset.read(cx).is_open()));
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).preset.read(cx).is_open());
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|item| { item.label() == Some("Done") && item.focused() == Some(true) })
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(
            demo.preset_preview
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
        assert_eq!(
            demo.preset.read(cx).input().read(cx).value(),
            "Presentation"
        );
        assert!(test_support::find(window, &[], &"session-name-alert".into()).is_none());
    });
    // The newly committed preset changes the media presentation without owning
    // another selected value or taking focus from the existing preview trigger.
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, cx| {
        assert!(demo.read(cx).preset_preview.read(cx).is_open());
        let avatar = test_support::find(window, &[], &"preset-avatar".into()).unwrap();
        assert_eq!(avatar.label(), Some("Presentation preset artwork"));
        let fallback = (gpui::ElementId::from("preset-avatar"), "fallback").into();
        assert!(test_support::find(window, &[], &fallback).is_some());
        let image = (gpui::ElementId::from("preset-avatar"), "image").into();
        assert!(test_support::find(window, &[], &image).is_none());
        let frame = test_support::find(window, &[], &"preset-frame".into()).unwrap();
        let size = frame.bounds().size;
        assert!((size.width / size.height - 4. / 3.).abs() < 0.01);
        let texts = test_support::texts(window);
        for (id, value) in [
            ("preset-frame-session-name", "Workshop"),
            ("preset-frame-format", "4:3 frame preview"),
        ] {
            assert!(texts.iter().any(|text| {
                text.path().contains(&id.into()) && text.visible() && text.text() == value
            }));
        }
        assert!(texts.iter().any(|text| {
            text.path().contains(&fallback) && text.visible() && text.text() == "PR"
        }));
        assert!(
            demo.read(cx)
                .preset_preview
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
    });
    open_dialog(cx, &demo);
    cx.update(|_, cx| {
        assert_eq!(demo.read(cx).input.read(cx).value(), "Workshop");
        assert_eq!(
            demo.read(cx)
                .preset
                .read(cx)
                .selected_items(cx)
                .next()
                .map(|item| item.value())
                .map(|value| value.as_ref()),
            Some("presentation")
        );
    });
    assert_eq!(changes.borrow().len(), 1);
}

#[gpui::test]
fn date_picker_and_split_panes_preserve_nested_form_state(cx: &mut TestAppContext) {
    let marker_id = gpui::ElementId::from("schedule-marker");
    let marker_label_id = (marker_id.clone(), "label").into();
    let (cx, demo) = harness(cx, Appearance::Compact);
    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab tab end left tab");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert_eq!(demo.tabs.read(cx).selected_index(), Some(3));
        assert!(demo.date_picker.read(cx).focus_handle().is_focused(window));
        assert!(!demo.date_picker.read(cx).is_open(cx));
        assert!(test_support::find(window, &[], &"recording-calendar".into()).is_none());
        let notice = test_support::find(window, &[], &"schedule-notice".into()).unwrap();
        assert_eq!(notice.role(), Some(Role::Note));
        assert_eq!(notice.label(), Some("Recording preferences"));
        let trigger = test_support::find(window, &[], &"recording-date".into()).unwrap();
        assert_eq!(trigger.role(), Some(Role::Button));
        assert_eq!(trigger.expanded(), Some(false));
        assert_eq!(trigger.label(), Some("Recording date, 2026-11-09"));
        let marker = test_support::find(window, &[], &marker_id).unwrap();
        assert_eq!(marker.role(), None);
        assert_eq!(marker.label(), None);
        assert_eq!(marker.focused(), None);
        let label = test_support::find(window, &[], &marker_label_id).unwrap();
        assert_eq!(label.role(), Some(Role::Label));
        assert_eq!(label.label(), Some("Selected date · 2026-11-09"));
        assert_eq!(label.focused(), None);
        for part in ["before", "after"] {
            let rule = test_support::find(window, &[], &(marker_id.clone(), part).into()).unwrap();
            assert_eq!(rule.role(), None);
            assert_eq!(rule.label(), None);
            assert_eq!(rule.focused(), None);
        }
        assert_eq!(
            test_support::snapshots(window)
                .iter()
                .filter(|item| item.path().contains(&marker_id) && item.label().is_some())
                .count(),
            1,
            "the date boundary supplies one semantic label"
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.date_picker.read(cx).is_open(cx));
        let calendar = demo.calendar.read(cx);
        assert!(calendar.focus_handle().is_focused(window));
        assert_eq!(
            calendar.selection().first_date(),
            NaiveDate::from_ymd_opt(2026, 11, 9)
        );
        let day = test_support::snapshots(window)
            .into_iter()
            .find(|item| item.role() == Some(Role::GridCell) && item.focused() == Some(true))
            .unwrap();
        assert!(
            day.visible(),
            "opening the popup focuses its visible selected day"
        );
    });

    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.update(|window, _| {
        let next = test_support::find(window, &[], &"next-month".into()).unwrap();
        assert_eq!(next.focused(), Some(true));
    });
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    cx.update(|window, _| {
        let previous = test_support::find(window, &[], &"previous-month".into()).unwrap();
        assert_eq!(previous.focused(), Some(true));
    });
    activate_key(cx, "enter");
    cx.update(|_, cx| {
        assert_eq!(
            demo.read(cx).calendar.read(cx).month(),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
        );
    });
    cx.simulate_keystrokes("tab");
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let calendar = demo.read(cx).calendar.read(cx);
        assert_eq!(
            calendar.month(),
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
        );
        assert!(calendar.focus_handle().is_focused(window));
    });

    cx.simulate_keystrokes("right pagedown");
    draw(cx);
    draw(cx);
    cx.update(|window, cx| {
        let calendar = demo.read(cx).calendar.read(cx);
        assert_eq!(
            calendar.focused_date(),
            NaiveDate::from_ymd_opt(2026, 12, 10).unwrap()
        );
        assert_eq!(
            calendar.selection().first_date(),
            NaiveDate::from_ymd_opt(2026, 11, 9)
        );
        let grid = test_support::find(window, &[], &"recording-calendar".into()).unwrap();
        assert_eq!(grid.role(), Some(Role::Grid));
        assert_eq!(grid.label(), Some("Recording date, December 2026"));
        assert_eq!(
            test_support::find(window, &[], &marker_label_id)
                .unwrap()
                .label(),
            Some("Selected date · 2026-11-09"),
            "moving the calendar cursor does not commit a new boundary date"
        );
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.date_picker.read(cx).is_open(cx));
        assert!(
            demo.dialog.read(cx).is_open(),
            "Escape closes the picker before the dialog"
        );
        assert!(demo.date_picker.read(cx).focus_handle().is_focused(window));
        assert_eq!(
            demo.calendar.read(cx).selection().first_date(),
            NaiveDate::from_ymd_opt(2026, 11, 9)
        );
        assert_eq!(
            test_support::find(window, &[], &marker_label_id)
                .unwrap()
                .label(),
            Some("Selected date · 2026-11-09")
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let calendar = demo.read(cx).calendar.read(cx);
        assert_eq!(
            calendar.focused_date(),
            NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
        );
        assert!(calendar.focus_handle().is_focused(window));
    });
    cx.simulate_keystrokes("right pagedown");
    draw(cx);
    activate_key(cx, "enter");
    draw(cx);
    cx.update(|window, cx| {
        assert_eq!(
            demo.read(cx).calendar.read(cx).selection().first_date(),
            NaiveDate::from_ymd_opt(2026, 12, 10)
        );
        let demo = demo.read(cx);
        assert!(!demo.date_picker.read(cx).is_open(cx));
        assert!(demo.date_picker.read(cx).focus_handle().is_focused(window));
        assert_eq!(
            test_support::find(window, &[], &"recording-date".into())
                .unwrap()
                .label(),
            Some("Recording date, 2026-12-10")
        );
        assert_eq!(
            test_support::find(window, &[], &marker_label_id)
                .unwrap()
                .label(),
            Some("Selected date · 2026-12-10")
        );
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.simulate_input("Prepare the workshop demo.");
    cx.simulate_keystrokes("home down");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.notes.focus_handle(cx).is_focused(window));
        assert_eq!(
            demo.schedule_split.read(cx).ratio(),
            0.5,
            "editing keys do not resize panes"
        );
    });
    let old_table_height = cx.update(|_, cx| demo.read(cx).schedule_scroll.bounds().size.height);
    cx.simulate_keystrokes("tab down");
    draw(cx);
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        let split = demo.schedule_split.read(cx);
        assert!(split.focus_handle().is_focused(window));
        assert!((split.ratio() - 0.52).abs() < 0.001);
        assert!(demo.schedule_scroll.bounds().size.height < old_table_height);
        let id = (gpui::ElementId::from("schedule-panes"), "handle").into();
        let handle = test_support::find(window, &[], &id).unwrap();
        assert_eq!(handle.role(), Some(Role::Splitter));
        assert_eq!(handle.label(), Some("Space for session notes"));
        assert!(
            handle.visible(),
            "the keyboard-focused divider is visible in the form"
        );
        assert_eq!(demo.notes.read(cx).value(), "Prepare the workshop demo.");
    });
    let fields = cx.update(|window, _| {
        test_support::find(
            window,
            &[],
            &(gpui::ElementId::from("session-settings"), "body").into(),
        )
        .unwrap()
        .bounds()
        .center()
    });
    cx.simulate_event(ScrollEvent {
        position: fields,
        delta: ScrollDelta::Pixels(point(px(0.), px(-800.))),
        ..Default::default()
    });
    draw(cx);
    let table_viewport = cx.update(|window, cx| {
        let table = test_support::find(window, &[], &"recording-plan".into()).unwrap();
        assert_eq!(table.role(), Some(Role::Table));
        assert_eq!(table.label(), Some("Recording plan"));
        assert_eq!(
            test_support::find(window, &[], &"plan-date-heading".into())
                .unwrap()
                .role(),
            Some(Role::ColumnHeader)
        );
        assert_eq!(
            test_support::find(window, &[], &"plan-session".into())
                .unwrap()
                .role(),
            Some(Role::RowHeader)
        );
        assert!(test_support::texts(window).iter().any(|text| {
            text.path().contains(&"plan-date".into()) && text.text() == "2026-12-10"
        }));
        let viewport = test_support::find(window, &[], &"recording-plan-scroll".into()).unwrap();
        assert_eq!(viewport.role(), Some(Role::ScrollView));
        assert!(viewport.visible());
        assert!(demo.read(cx).schedule_scroll.max_offset().x > px(0.));
        viewport.bounds().center()
    });
    cx.simulate_event(ScrollEvent {
        position: table_viewport,
        delta: ScrollDelta::Pixels(point(px(-120.), px(0.))),
        ..Default::default()
    });
    draw(cx);
    cx.update(|_, cx| {
        assert!(demo.read(cx).schedule_scroll.offset().x < px(0.));
    });

    // With the picker closed, Escape closes the parent. Date, note text, and
    // panel sizes survive mounting the form again when the dialog reopens.
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.dialog.read(cx).is_open());
        assert!(demo.edit_hint.read(cx).focus_handle().is_focused(window));
    });
    open_dialog(cx, &demo);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert_eq!(demo.tabs.read(cx).selected_index(), Some(3));
        assert_eq!(
            demo.calendar.read(cx).selection().first_date(),
            NaiveDate::from_ymd_opt(2026, 12, 10)
        );
        assert!(!demo.date_picker.read(cx).is_open(cx));
        assert!((demo.schedule_split.read(cx).ratio() - 0.52).abs() < 0.001);
        assert_eq!(demo.notes.read(cx).value(), "Prepare the workshop demo.");
        assert!(test_support::texts(window).iter().any(|text| {
            text.path().contains(&"plan-date".into()) && text.text() == "2026-12-10"
        }));
        assert_eq!(
            test_support::find(window, &[], &marker_label_id)
                .unwrap()
                .label(),
            Some("Selected date · 2026-12-10")
        );
    });
}

#[gpui::test]
fn schedule_marker_tracks_selection_at_narrow_widths_without_adding_focus(cx: &mut TestAppContext) {
    let marker_id = gpui::ElementId::from("schedule-marker");
    let label_id = (marker_id.clone(), "label").into();
    let body_id = (gpui::ElementId::from("session-settings"), "body").into();
    for appearance in [Appearance::Compact, Appearance::Settings] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(320.), px(480.)));
        draw(cx);
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 3);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            let picker = demo.read(cx).date_picker.read(cx).focus_handle().clone();
            picker.focus(window, cx);
        });
        draw(cx);
        for (width, date, expected) in [
            (320., None, "No date selected"),
            (
                280.,
                NaiveDate::from_ymd_opt(2026, 12, 31),
                "Selected date · 2026-12-31",
            ),
        ] {
            cx.simulate_resize(gpui::size(px(width), px(480.)));
            cx.update(|window, cx| {
                let calendar = demo.read(cx).calendar.clone();
                calendar.update(cx, |state, cx| {
                    assert!(state.set_selection(CalendarSelection::Single(date), window, cx));
                });
            });
            draw(cx);
            cx.run_until_parked();
            draw(cx);
            let (position, delta) = cx.update(|window, _| {
                let body = test_support::find(window, &[], &body_id).unwrap().bounds();
                let marker = test_support::find(window, &[], &marker_id)
                    .unwrap()
                    .bounds();
                (
                    point(body.left() + px(2.), body.center().y),
                    body.center().y - marker.center().y,
                )
            });
            cx.simulate_event(ScrollEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), delta)),
                ..Default::default()
            });
            draw(cx);
            cx.update(|window, cx| {
                let body = test_support::find(window, &[], &body_id).unwrap().bounds();
                let marker = test_support::find(window, &[], &marker_id).unwrap();
                let label = test_support::find(window, &[], &label_id).unwrap();
                assert!(marker.visible() && label.visible());
                assert!(marker.bounds().left() >= body.left());
                assert!(marker.bounds().right() <= body.right() + px(1.));
                assert!(label.bounds().top() >= body.top());
                assert!(label.bounds().bottom() <= body.bottom() + px(1.));
                assert_eq!(label.role(), Some(Role::Label));
                assert_eq!(label.label(), Some(expected));
                assert_eq!(label.focused(), None);
                let texts = test_support::texts(window);
                let text = texts
                    .iter()
                    .find(|text| text.path().contains(&label_id))
                    .unwrap();
                assert_eq!(text.text(), expected);
                assert!(text.visible());
                assert!(text.bounds().left() >= marker.bounds().left());
                assert!(text.bounds().right() <= marker.bounds().right() + px(1.));
                let demo = demo.read(cx);
                assert_eq!(demo.calendar.read(cx).selection().first_date(), date);
                assert!(demo.date_picker.read(cx).focus_handle().is_focused(window));
                assert_eq!(demo.fields_scroll.offset().x, px(0.));
            });
        }
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| assert!(demo.read(cx).notes.focus_handle(cx).is_focused(window)));
    }
}

#[gpui::test]
fn schedule_message_reuses_notes_and_preserves_metadata_focus_and_parent_scroll(
    cx: &mut TestAppContext,
) {
    let message_id = gpui::ElementId::from("session-note-message");
    let author_id = (message_id.clone(), "author").into();
    let time_id = (message_id.clone(), "time").into();
    let text_id = (message_id.clone(), "body").into();
    let viewport_id = (gpui::ElementId::from("session-settings"), "body").into();
    let restored = format!(
        "첫 번째 줄\n{}\n\n{}\n마지막 줄\n",
        "긴 한국어 본문과 English message text. ".repeat(8),
        "unbroken메시지".repeat(24),
    );

    // The two palettes have different padding; both must keep a long message
    // inside the existing sheet viewport at narrow widths.
    for appearance in [Appearance::Compact, Appearance::Settings] {
        let (cx, demo) = harness(cx, appearance);
        cx.simulate_resize(gpui::size(px(320.), px(480.)));
        draw(cx);
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let notes = demo.read(cx).notes.clone();
            notes.update(cx, |state, cx| {
                state.set_value(restored.clone(), window, cx)
            });
            focus_section(window, cx, &demo, 3);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        for width in [320., 280.] {
            cx.simulate_resize(gpui::size(px(width), px(480.)));
            draw(cx);
            draw(cx);
            let viewport = cx.update(|window, _| {
                test_support::find(window, &[], &viewport_id)
                    .unwrap()
                    .bounds()
            });
            cx.simulate_event(ScrollEvent {
                position: point(viewport.left() + px(2.), viewport.center().y),
                delta: ScrollDelta::Pixels(point(px(-10_000.), px(-10_000.))),
                ..Default::default()
            });
            draw(cx);
            let message_position = cx.update(|window, cx| {
                let message = test_support::find(window, &[], &message_id).unwrap();
                assert_eq!(message.role(), Some(Role::Comment));
                assert_eq!(
                    message.label(),
                    None,
                    "the frame does not repeat its content"
                );
                assert_eq!(message.focused(), None);
                assert!(message.bounds().left() >= viewport.left());
                assert!(message.bounds().right() <= viewport.right() + px(1.));
                for (id, role, expected) in [
                    (&author_id, Role::Label, "You"),
                    (&time_id, Role::Label, "Edit time not recorded"),
                    (&text_id, Role::Paragraph, restored.as_str()),
                ] {
                    let part = test_support::find(window, &[], id).unwrap();
                    assert_eq!(part.role(), Some(role));
                    assert_eq!(part.label(), Some(expected));
                    assert_eq!(part.focused(), None);
                    assert!(part.bounds().left() >= message.bounds().left());
                    assert!(part.bounds().right() <= message.bounds().right() + px(1.));
                }
                assert!(test_support::snapshots(window).iter().all(|part| {
                    !part.path().contains(&message_id) || part.role() != Some(Role::ScrollView)
                }));
                let text = test_support::texts(window)
                    .into_iter()
                    .find(|text| text.path().contains(&text_id))
                    .unwrap();
                assert_eq!(text.text(), restored);
                assert!(text.bounds().right() <= message.bounds().right() + px(1.));
                let action = test_support::find(window, &[], &"edit-session-note".into()).unwrap();
                assert_eq!(action.role(), Some(Role::Button));
                assert_eq!(action.label(), Some("Edit session note"));
                assert!(action.visible());
                let demo = demo.read(cx);
                assert_eq!(demo.notes_edited_at, None, "restoration is not a user edit");
                assert_eq!(demo.fields_scroll.max_offset().x, px(0.));
                assert_eq!(demo.fields_scroll.offset().x, px(0.));
                assert!(
                    (demo.fields_scroll.offset().y + demo.fields_scroll.max_offset().y).abs()
                        < px(1.)
                );
                let text = test_support::find(window, &[], &text_id).unwrap().bounds();
                point(
                    text.center().x,
                    (text.bottom().min(viewport.bottom())) - px(8.),
                )
            });
            let end = cx.update(|_, cx| demo.read(cx).fields_scroll.offset().y);
            // A wheel over the message belongs to the parent ScrollArea; the
            // read-only body adds neither a scroll boundary nor a focus stop.
            cx.simulate_event(ScrollEvent {
                position: message_position,
                delta: ScrollDelta::Pixels(point(px(0.), px(80.))),
                ..Default::default()
            });
            draw(cx);
            cx.update(|_, cx| assert!(demo.read(cx).fields_scroll.offset().y > end));
        }

        if matches!(appearance, Appearance::Compact) {
            cx.simulate_keystrokes("escape");
            draw(cx);
            continue;
        }

        // Exercise the editing and action flow once after both layout checks.
        // An edit in Preferences is the same retained note shown in Schedule.
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 0);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| demo.read(cx).notes.focus_handle(cx).focus(window, cx));
        draw(cx);
        let select_all = if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        };
        cx.simulate_keystrokes(select_all);
        let edited = "Preferences에서 수정한 메모\nThe same body appears in Schedule.\n";
        cx.simulate_input(edited);
        draw(cx);
        let edited_at = cx.update(|_, cx| demo.read(cx).notes_edited_at.unwrap());
        cx.update(|window, cx| {
            focus_section(window, cx, &demo, 3);
        });
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, cx| {
            assert_eq!(demo.read(cx).notes.read(cx).value(), edited);
            assert_eq!(
                test_support::find(window, &[], &text_id).unwrap().label(),
                Some(edited)
            );
            let expected = format!("Edited {}", edited_at.format("%Y-%m-%d %H:%M:%S UTC"));
            assert_eq!(
                test_support::find(window, &[], &time_id).unwrap().label(),
                Some(expected.as_str())
            );
            demo.read(cx).notes.focus_handle(cx).focus(window, cx);
        });
        draw(cx);
        cx.simulate_keystrokes("home right end");
        cx.update(|window, cx| {
            let calendar = demo.read(cx).calendar.clone();
            calendar.update(cx, |state, cx| {
                assert!(state.set_selection(
                    CalendarSelection::Single(NaiveDate::from_ymd_opt(2026, 12, 24)),
                    window,
                    cx
                ));
            });
        });
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(
                demo.read(cx).notes_edited_at,
                Some(edited_at),
                "caret, focus, and date changes are not note edits"
            );
        });

        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, cx| {
            assert!(
                demo.read(cx)
                    .schedule_split
                    .read(cx)
                    .focus_handle()
                    .is_focused(window)
            )
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        draw(cx);
        cx.update(|window, _| {
            let action = test_support::find(window, &[], &"edit-session-note".into()).unwrap();
            assert_eq!(action.focused(), Some(true));
            assert!(action.visible());
        });
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|window, _| {
            assert!(
                test_support::snapshots(window)
                    .iter()
                    .any(|part| { part.label() == Some("Done") && part.focused() == Some(true) })
            );
        });
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        for key in ["enter", "space"] {
            activate_key(cx, key);
            draw(cx);
            cx.update(|window, cx| {
                assert!(demo.read(cx).notes.focus_handle(cx).is_focused(window));
                assert!(
                    test_support::find(window, &[], &"schedule-notes".into())
                        .unwrap()
                        .visible()
                );
                assert_eq!(demo.read(cx).notes_edited_at, Some(edited_at));
            });
            if key == "enter" {
                cx.simulate_keystrokes("tab tab");
                draw(cx);
            }
        }

        cx.simulate_keystrokes(select_all);
        cx.simulate_keystrokes("backspace");
        draw(cx);
        cx.update(|window, cx| {
            assert!(demo.read(cx).notes.read(cx).value().is_empty());
            assert!(demo.read(cx).notes.focus_handle(cx).is_focused(window));
            assert!(test_support::find(window, &[], &message_id).is_none());
            assert!(test_support::find(window, &[], &"edit-session-note".into()).is_none());
        });
        cx.simulate_input("다시 작성한 메모\nRetained when the sheet reopens.");
        draw(cx);
        let (body, time) = cx.update(|_, cx| {
            let demo = demo.read(cx);
            (demo.notes.read(cx).value(), demo.notes_edited_at)
        });
        assert!(time.is_some());
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.update(|window, cx| {
            assert!(!demo.read(cx).dialog.read(cx).is_open());
            assert!(
                demo.read(cx)
                    .edit_hint
                    .read(cx)
                    .focus_handle()
                    .is_focused(window)
            );
        });
        open_dialog(cx, &demo);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert_eq!(demo.tabs.read(cx).selected_index(), Some(3));
            assert_eq!(demo.notes.read(cx).value(), body);
            assert_eq!(demo.notes_edited_at, time);
            assert_eq!(
                demo.calendar.read(cx).selection().first_date(),
                NaiveDate::from_ymd_opt(2026, 12, 24)
            );
            assert_eq!(
                test_support::find(window, &[], &text_id).unwrap().label(),
                Some(body.as_ref())
            );
        });
    }
}

#[gpui::test]
fn preset_table_filters_sorts_pages_and_applies_stable_rows(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Settings);
    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab tab tab tab tab");
    draw(cx);
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    let id = gpui::ElementId::from("preset-catalogue");
    let pagination = gpui::ElementId::from((id.clone(), "pagination"));
    let previous = gpui::ElementId::from((pagination.clone(), "previous"));
    let next = gpui::ElementId::from((pagination.clone(), "next"));
    let sort = gpui::ElementId::from((id.clone(), "sort-0"));
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        let state = demo.preset_table.read(cx);
        assert_eq!(demo.tabs.read(cx).selected_index(), Some(4));
        assert!(state.query_input().focus_handle(cx).is_focused(window));
        assert_eq!(state.page_count(), 2);
        assert_eq!(
            state
                .visible_rows()
                .map(|row| row.id().as_ref())
                .collect::<Vec<_>>(),
            ["balanced", "broadcast"]
        );
        let table = test_support::find(window, &[], &(id.clone(), "table").into()).unwrap();
        assert_eq!(table.role(), Some(Role::Table));
        assert_eq!(table.label(), Some("Recording presets"));
        assert_eq!(
            test_support::find(window, &[], &pagination).unwrap().role(),
            Some(Role::Navigation)
        );
        assert_eq!(
            test_support::find(window, &[], &previous)
                .unwrap()
                .disabled(),
            Some(true)
        );
        let unavailable = gpui::ElementId::from((id.clone(), "row-broadcast"));
        assert_eq!(
            test_support::find(window, &[], &(unavailable, "action").into())
                .unwrap()
                .disabled(),
            Some(true)
        );
    });

    // Column controls use native menu checks and restore the trigger on close.
    cx.simulate_keystrokes("tab");
    activate_key(cx, "enter");
    cx.simulate_keystrokes("down");
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert!(!demo.read(cx).preset_table.read(cx).is_column_visible(1));
        assert!(test_support::find(window, &[], &(id.clone(), "heading-1").into()).is_none());
        assert_eq!(
            test_support::find(window, &[], &(id.clone(), "columns").into())
                .unwrap()
                .focused(),
            Some(true)
        );
    });
    activate_key(cx, "enter");
    cx.simulate_keystrokes("down");
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    activate_key(cx, "space");
    cx.update(|window, cx| {
        let state = demo.read(cx).preset_table.read(cx);
        assert!(state.is_column_visible(1));
        assert_eq!(
            state.filtered_selected_count(),
            1,
            "unavailable Broadcast cannot be selected"
        );
        assert!(state.is_row_selected(&"balanced".into()));
        let row = gpui::ElementId::from((id.clone(), "row-balanced"));
        assert_eq!(
            test_support::find(window, &[], &row).unwrap().selected(),
            Some(true)
        );
        assert_eq!(
            test_support::find(window, &[], &(id.clone(), "select-page").into())
                .unwrap()
                .checked(),
            Some(true)
        );
    });
    // The current preset can be independently selected and unselected.
    cx.simulate_keystrokes("tab tab tab");
    activate_key(cx, "space");
    cx.update(|window, cx| {
        assert_eq!(
            demo.read(cx)
                .preset_table
                .read(cx)
                .filtered_selected_count(),
            0
        );
        assert_eq!(
            test_support::find(window, &[], &(id.clone(), "select-page").into())
                .unwrap()
                .checked(),
            Some(false)
        );
    });
    activate_key(cx, "space");
    // Disabled Broadcast is skipped; the next page is the next Tab stop.
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        assert_eq!(
            test_support::find(window, &[], &next).unwrap().focused(),
            Some(true)
        )
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let state = demo.read(cx).preset_table.read(cx);
        assert_eq!(state.page_index(), 1);
        assert_eq!(
            state
                .visible_rows()
                .map(|row| row.id().as_ref())
                .collect::<Vec<_>>(),
            ["presentation", "quality"]
        );
        assert_eq!(
            test_support::find(window, &[], &next).unwrap().disabled(),
            Some(true)
        );
    });
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab shift-tab shift-tab shift-tab shift-tab");
    draw(cx);
    cx.update(|window, cx| {
        assert_eq!(
            test_support::find(window, &[], &sort).unwrap().focused(),
            Some(true)
        );
        assert_eq!(
            demo.read(cx).preset_table.read(cx).page_index(),
            1,
            "focus changes preserve the page"
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert_eq!(demo.read(cx).preset_table.read(cx).page_index(), 0);
        assert_eq!(
            test_support::find(window, &[], &sort).unwrap().label(),
            Some("Preset, sorted ascending; sort descending")
        );
        assert_eq!(
            test_support::find(window, &[], &(id.clone(), "heading-0").into())
                .unwrap()
                .sort_direction(),
            Some(gpui::accesskit::SortDirection::Ascending)
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let state = demo.read(cx).preset_table.read(cx);
        assert_eq!(
            state
                .visible_rows()
                .map(|row| row.id().as_ref())
                .collect::<Vec<_>>(),
            ["presentation", "quality"]
        );
        assert_eq!(
            test_support::find(window, &[], &sort).unwrap().label(),
            Some("Preset, sorted descending; sort ascending")
        );
        assert_eq!(
            state.filtered_selected_count(),
            1,
            "selection survives paging and sorting"
        );
        assert_eq!(
            test_support::find(window, &[], &(id.clone(), "heading-0").into())
                .unwrap()
                .sort_direction(),
            Some(gpui::accesskit::SortDirection::Descending)
        );
    });

    cx.simulate_keystrokes("shift-tab shift-tab shift-tab");
    cx.simulate_input("no matching preset");
    draw(cx);
    cx.update(|window, cx| {
        let state = demo.read(cx).preset_table.read(cx);
        assert_eq!(state.filtered_count(), 0);
        assert_eq!(state.page_index(), 0);
        assert_eq!(state.page_count(), 0);
        assert!(test_support::find(window, &[], &(id.clone(), "table").into()).is_none());
        assert_eq!(
            test_support::find(window, &[], &previous)
                .unwrap()
                .disabled(),
            Some(true)
        );
        assert_eq!(
            test_support::find(window, &[], &next).unwrap().disabled(),
            Some(true)
        );
        assert_eq!(
            demo.read(cx)
                .preset
                .read(cx)
                .selected_items(cx)
                .next()
                .map(|item| item.value())
                .map(|value| value.as_ref()),
            Some("balanced")
        );
    });
    cx.simulate_keystrokes("tab tab");
    draw(cx);
    cx.update(|window, _| {
        assert_eq!(
            test_support::find(window, &[], &(id.clone(), "clear-filter").into())
                .unwrap()
                .focused(),
            Some(true)
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let state = demo.read(cx).preset_table.read(cx);
        assert_eq!(state.filtered_count(), 4);
        assert!(state.query_input().read(cx).value().is_empty());
        assert!(state.query_input().focus_handle(cx).is_focused(window));
    });
    cx.simulate_input("pReSeNtAtIoN");
    draw(cx);
    cx.update(|_, cx| {
        let state = demo.read(cx).preset_table.read(cx);
        assert_eq!(state.filtered_count(), 1);
        assert_eq!(state.page_count(), 1);
        assert_eq!(
            state.visible_rows().next().unwrap().id().as_ref(),
            "presentation"
        );
    });
    cx.simulate_keystrokes("tab tab tab tab tab tab");
    draw(cx);
    let row = gpui::ElementId::from((id.clone(), "row-presentation"));
    let action = gpui::ElementId::from((row, "action"));
    cx.update(|window, _| {
        let action = test_support::find(window, &[], &action).unwrap();
        assert_eq!(action.focused(), Some(true));
        assert!(
            action.visible(),
            "keyboard traversal reveals the offscreen action column"
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let preset = demo.read(cx).preset.read(cx);
        assert_eq!(
            preset
                .selected_items(cx)
                .next()
                .map(|item| item.value())
                .map(|value| value.as_ref()),
            Some("presentation")
        );
        assert_eq!(preset.input().read(cx).value(), "Presentation");
        let action = test_support::find(window, &[], &action).unwrap();
        assert_eq!(action.disabled(), Some(true));
        assert_eq!(action.focused(), Some(true));
    });
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, _| {
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|item| item.label() == Some("Done") && item.focused() == Some(true))
        );
    });
    activate_key(cx, "enter");
    open_dialog(cx, &demo);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        let state = demo.preset_table.read(cx);
        assert_eq!(demo.tabs.read(cx).selected_index(), Some(4));
        assert_eq!(state.query_input().read(cx).value(), "pReSeNtAtIoN");
        assert_eq!(state.filtered_count(), 1);
        assert_eq!(state.page_index(), 0);
        assert_eq!(
            test_support::find(window, &[], &sort).unwrap().label(),
            Some("Preset, sorted descending; sort ascending")
        );
        assert_eq!(
            demo.preset
                .read(cx)
                .selected_items(cx)
                .next()
                .map(|item| item.value())
                .map(|value| value.as_ref()),
            Some("presentation")
        );
    });
}

#[gpui::test]
fn disabled_select_and_composition_commit_do_not_leak_actions(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Settings);
    open_dialog(cx, &demo);
    // Exercise the public GPUI composition handler, not a native IME.
    cx.update(|window, cx| {
        let input = demo.read(cx).input.clone();
        input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.replace_and_mark_text_in_range(None, "ㅎ", Some(1..1), window, cx);
            state.replace_and_mark_text_in_range(None, "한", Some(1..1), window, cx);
            state.replace_text_in_range(None, "한글", window, cx);
            assert_eq!(state.value(), "한글");
        });
        let select = demo.read(cx).select.clone();
        select.update(cx, |state, cx| state.set_disabled(true, window, cx));
    });
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(!demo.select.read(cx).is_open());
        assert!(!demo.select.read(cx).focus_handle().is_focused(window));
        assert!(
            test_support::snapshots(window).iter().any(|item| {
                item.label() == Some("Preferences") && item.focused() == Some(true)
            })
        );
        assert_eq!(
            test_support::find(window, &[], &"drawing-tool".into())
                .unwrap()
                .disabled(),
            Some(true)
        );
    });
    for id in ["drawing-tool", "drawing-tool-label"] {
        let point = cx.update(|window, _| {
            let target = test_support::find(window, &[], &id.into()).unwrap();
            assert_eq!(target.disabled(), Some(true));
            target.bounds().center()
        });
        cx.simulate_click(point, Modifiers::default());
        draw(cx);
        cx.update(|window, cx| {
            let demo = demo.read(cx);
            assert!(!demo.select.read(cx).is_open());
            assert!(demo.dialog.read(cx).is_open());
            assert!(demo.tabs.read(cx).tab_focus_handle(0).is_focused(window));
        });
    }
}

#[gpui::test]
fn catalogue_sheets_keep_each_edge_and_scroll_body_within_the_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_design::init);
    let (root, cx) = cx.add_window_view(|window, cx| {
        let catalogue = cx.new(|cx| catalogue::Catalogue::new(window, cx));
        Root::new(catalogue, window, cx)
    });
    let view = root
        .read_with(cx, |root, _| root.view().clone())
        .downcast::<catalogue::Catalogue>()
        .unwrap();
    let forms = cx.update(|window, cx| {
        window.activate_window();
        let tabs = view.read(cx).tabs.clone();
        tabs.update(cx, |tabs, cx| {
            tabs.select_value("forms", window, cx);
        });
        view.read(cx).forms.clone()
    });
    for (width, height) in [(320., 520.), (980., 840.)] {
        cx.simulate_resize(gpui::size(px(width), px(height)));
        for edge in 0..4 {
            cx.update(|window, cx| {
                let trigger = forms.read(cx).sheet_triggers[edge].clone();
                trigger.focus(window, cx);
            });
            draw(cx);
            activate_key(cx, "enter");
            let (header, body, footer, overflow) = cx.update(|window, cx| {
                assert!(forms.read(cx).sheet.read(cx).is_open());
                let id = gpui::ElementId::from("session-help");
                let bounds = |part| {
                    test_support::find(window, &[], &(id.clone(), part).into())
                        .unwrap()
                        .bounds()
                };
                assert!(
                    test_support::snapshots(window)
                        .iter()
                        .any(|node| node.label() == Some("Close help")
                            && node.focused() == Some(true))
                );
                (
                    bounds("header"),
                    bounds("body"),
                    bounds("footer"),
                    forms.read(cx).sheet_scroll.max_offset().y,
                )
            });
            for bounds in [header, body, footer] {
                assert!(bounds.left() >= px(-1.) && bounds.right() <= px(width) + px(1.));
                assert!(bounds.top() >= px(-1.) && bounds.bottom() <= px(height) + px(1.));
            }
            assert!(header.bottom() <= body.top() && body.bottom() <= footer.top());
            match edge {
                0 => {
                    assert!(header.top().abs() <= px(1.));
                    assert!((header.size.width - px(width)).abs() <= px(1.));
                }
                1 => {
                    assert!((header.right() - px(width)).abs() <= px(1.));
                    assert!(header.top().abs() <= px(1.));
                    assert!((footer.bottom() - px(height)).abs() <= px(1.));
                }
                2 => {
                    assert!((footer.bottom() - px(height)).abs() <= px(1.));
                    assert!((header.size.width - px(width)).abs() <= px(1.));
                }
                _ => {
                    assert!(header.left().abs() <= px(1.));
                    assert!(header.top().abs() <= px(1.));
                    assert!((footer.bottom() - px(height)).abs() <= px(1.));
                }
            }
            if height == 520. || edge == 0 || edge == 2 {
                assert!(
                    overflow > px(0.),
                    "the actual help text overflows the bounded body"
                );
            }
            if overflow > px(0.) {
                cx.simulate_event(ScrollEvent {
                    position: body.center(),
                    delta: ScrollDelta::Pixels(point(px(0.), px(-100.))),
                    ..Default::default()
                });
                draw(cx);
                cx.update(|window, cx| {
                    assert!(forms.read(cx).sheet_scroll.offset().y < px(0.));
                    let id = gpui::ElementId::from("session-help");
                    assert_eq!(
                        test_support::find(window, &[], &(id.clone(), "header").into())
                            .unwrap()
                            .bounds(),
                        header
                    );
                    assert_eq!(
                        test_support::find(window, &[], &(id, "footer").into())
                            .unwrap()
                            .bounds(),
                        footer
                    );
                });
            }
            activate_key(cx, "escape");
            cx.update(|window, cx| {
                assert!(!forms.read(cx).sheet.read(cx).is_open());
                assert!(forms.read(cx).sheet_triggers[edge].is_focused(window));
            });
        }
    }
}

#[gpui::test]
fn catalogue_connects_confirmation_data_and_conversation(cx: &mut TestAppContext) {
    let assert_named_buttons = |window: &gpui::Window| {
        for node in test_support::snapshots(window)
            .into_iter()
            .filter(|node| node.role() == Some(Role::Button))
        {
            assert!(
                node.label().is_some_and(|label| !label.trim().is_empty()),
                "button needs a native accessible name: {:?}",
                node.path()
            );
        }
    };
    cx.update(gpui_design::init);
    let (root, cx) = cx.add_window_view(|window, cx| {
        let catalogue = cx.new(|cx| catalogue::Catalogue::new(window, cx));
        Root::new(catalogue, window, cx)
    });
    let view = root
        .read_with(cx, |root, _| root.view().clone())
        .downcast::<catalogue::Catalogue>()
        .unwrap();
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    cx.update(|window, _| {
        assert_named_buttons(window);
        let nodes = test_support::snapshots(window);
        assert!(
            nodes.iter().any(|node| node.role() == Some(Role::MenuBar)
                && node.label() == Some("Document commands"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Some(Role::Navigation)
                    && node.label() == Some("Documentation pages"))
        );
    });

    // Exercise the actual nested Sidebar consumer once. Collapsing navigation
    // unmounts its entries while the application's selected page stays mounted.
    let (project, sidebar, tools) = cx.update(|window, cx| {
        let navigation = view.read(cx).navigation.read(cx);
        let states = (
            navigation.sections.clone(),
            navigation.sidebar.clone(),
            navigation.tools.clone(),
        );
        let focus = states.1.read(cx).trigger_focus_handle().clone();
        focus.focus(window, cx);
        states
    });
    draw(cx);
    cx.simulate_keystrokes("tab tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            tools.read(cx).trigger_focus_handle().is_focused(window),
            "Tab skips unavailable Activity"
        );
        assert_eq!(project.read(cx).selected_index(), Some(0));
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|node| node.role() == Some(Role::Group) && node.label() == Some("Project"))
        );
    });
    cx.simulate_keystrokes("tab");
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert_eq!(project.read(cx).selected_index(), Some(2));
        let page = test_support::find(window, &[], &"project-section".into()).unwrap();
        assert_eq!(page.role(), Some(Role::Region));
        assert_eq!(page.label(), Some("Members"));
        tools.update(cx, |tools, cx| tools.set_open(false, window, cx));
    });
    draw(cx);
    cx.update(|window, cx| {
        assert!(tools.read(cx).trigger_focus_handle().is_focused(window));
        let member = (gpui::ElementId::from("project-sidebar"), "2".to_string()).into();
        assert!(test_support::find(window, &[], &member).is_none());
        assert_eq!(
            test_support::find(window, &[], &"project-section".into())
                .unwrap()
                .label(),
            Some("Members")
        );
    });
    activate_key(cx, "enter");
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(project.read(cx).tab_focus_handle(2).is_focused(window));
        sidebar.update(cx, |sidebar, cx| sidebar.set_open(false, window, cx));
    });
    draw(cx);
    cx.update(|window, cx| {
        assert!(sidebar.read(cx).trigger_focus_handle().is_focused(window));
        let nodes = test_support::snapshots(window);
        assert!(
            !nodes.iter().any(|node| node.role() == Some(Role::Button)
                && node.label() == Some("Project overview"))
        );
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Button)
            && node.label() == Some("Project sections")
            && node.expanded() == Some(false)));
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert_eq!(project.read(cx).selected_index(), Some(2));
        assert_eq!(
            test_support::find(window, &[], &"project-section".into())
                .unwrap()
                .label(),
            Some("Members")
        );
    });

    for width in [320., 980.] {
        cx.simulate_resize(gpui::size(px(width), px(840.)));
        draw(cx);
        draw(cx);
        cx.update(|window, _| {
            let navigation = test_support::find(window, &[], &"project-sidebar".into())
                .unwrap()
                .bounds();
            let content = test_support::find(window, &[], &"project-section".into())
                .unwrap()
                .bounds();
            assert!(navigation.left() >= px(0.) && navigation.right() <= px(width));
            assert!(content.left() >= px(0.) && content.right() <= px(width));
            if width < 640. {
                assert!(navigation.bottom() <= content.top());
            } else {
                assert!(navigation.right() <= content.left());
            }
        });
    }

    let section = |cx: &mut VisualTestContext, index: usize| {
        cx.update(|window, cx| {
            let tabs = view.read(cx).tabs.clone();
            tabs.update(cx, |tabs, cx| {
                let value = tabs.items()[index].value().clone();
                tabs.select_value(value, window, cx);
                tabs.tab_focus_handle(index).focus(window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        draw(cx);
        cx.update(|window, _| assert_named_buttons(window));
    };
    section(cx, 1);
    cx.simulate_keystrokes("tab");
    cx.simulate_input("742915");
    draw(cx);
    cx.update(|window, _| {
        let input = test_support::find(window, &[], &"session-code".into()).unwrap();
        assert_eq!(input.value(), Some("742915"));
    });
    cx.simulate_keystrokes("tab");
    activate_key(cx, "enter");
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Dialog)
            && node.label() == Some("Confirm this local session")));
        assert_eq!(
            test_support::find(window, &[], &"confirm-session".into())
                .unwrap()
                .focused(),
            Some(true)
        );
    });
    activate_key(cx, "enter");
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        assert!(!nodes.iter().any(|node| node.role() == Some(Role::Dialog)));
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Status)
            && node.label() == Some("Session confirmed in this local example.")));
        assert_eq!(
            test_support::find(window, &[], &"review-session".into())
                .unwrap()
                .focused(),
            Some(true)
        );
    });

    section(cx, 2);
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Region)
            && node.label() == Some("Local media review previews")));
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Group)
            && node.label() == Some("Transfer samples in megabytes")));
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Table)
            && node.label() == Some("All transfer values in megabytes, including hidden series")));
    });
    section(cx, 3);
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Some(Role::ScrollView)
                    && node.label() == Some("Local conversation history"))
        );
        assert!(nodes.iter().any(
            |node| node.role() == Some(Role::Group) && node.label() == Some("Question 1 of 2")
        ));
    });
    section(cx, 1);
    cx.update(|window, _| {
        assert_eq!(
            test_support::find(window, &[], &"session-code".into())
                .unwrap()
                .value(),
            Some("742915")
        );
        assert!(
            !test_support::snapshots(window)
                .iter()
                .any(|node| node.role() == Some(Role::Dialog))
        );
    });

    section(cx, 4);
    cx.simulate_keystrokes("tab");
    draw(cx);
    activate_key(cx, "enter");
    activate_key(cx, "enter");
    cx.update(|window, _| {
        let picker = test_support::find(window, &[], &"recording-date-range".into()).unwrap();
        assert_eq!(picker.expanded(), Some(true), "a partial range stays open");
    });
    cx.simulate_keystrokes("right right");
    activate_key(cx, "enter");
    cx.run_until_parked();
    draw(cx);
    cx.update(|window, _| {
        let picker = test_support::find(window, &[], &"recording-date-range".into()).unwrap();
        assert_eq!(picker.expanded(), Some(false));
        assert_eq!(picker.focused(), Some(true));
        assert_eq!(
            picker.label(),
            Some("Recording date range: Nov 11 – Nov 13, 2026")
        );
    });
    // Previous month is disabled at the availability calendar's lower bound.
    cx.simulate_keystrokes("tab tab tab tab");
    draw(cx);
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        let focused = nodes
            .iter()
            .filter(|node| node.focused() == Some(true))
            .map(|node| (node.role(), node.label()))
            .collect::<Vec<_>>();
        assert!(
            nodes.iter().any(|node| {
                node.role() == Some(Role::ComboBox)
                    && node.label() == Some("Search recording targets")
                    && node.focused() == Some(true)
            }),
            "focused controls: {focused:?}"
        );
    });
    cx.simulate_input("archive");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Some(Role::Group) && node.label() == Some("Online"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Some(Role::ComboBox) && node.expanded() == Some(true))
        );
        assert!(nodes.iter().any(|node| node.role() == Some(Role::Button)
            && node.label() == Some("Remove Cloud archive")));
    });
    cx.simulate_keystrokes("escape tab tab");
    draw(cx);
    activate_key(cx, "enter");
    draw(cx);
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        assert!(!nodes.iter().any(|node| node.role() == Some(Role::Button)
            && node.label() == Some("Remove Cloud archive")));
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == Some(Role::ComboBox) && node.focused() == Some(true))
        );
    });
    cx.simulate_keystrokes("tab tab tab");
    activate_key(cx, "enter");
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab");
    cx.simulate_input("Backup");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, _| {
        assert!(
            test_support::snapshots(window).iter().any(|node| {
                node.role() == Some(Role::Button) && node.label() == Some("Remove Backup archive")
            }),
            "the replaced item collection is selectable without recreating the editor"
        );
    });
    cx.simulate_keystrokes("escape tab tab tab tab tab");
    draw(cx);
    cx.update(|window, _| {
        assert_eq!(
            test_support::find(window, &[], &"calendar-reading-direction".into())
                .unwrap()
                .focused(),
            Some(true)
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        let sunday = nodes
            .iter()
            .find(|node| node.role() == Some(Role::ColumnHeader) && node.label() == Some("Sun"))
            .unwrap();
        let monday = nodes
            .iter()
            .find(|node| node.role() == Some(Role::ColumnHeader) && node.label() == Some("Mon"))
            .unwrap();
        assert!(
            sunday.bounds().left() > monday.bounds().left(),
            "Sunday-first columns are visually reversed for RTL"
        );
    });
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab shift-tab shift-tab shift-tab");
    draw(cx);
    cx.update(|window, _| {
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|node| node.role() == Some(Role::GridCell)
                    && node.label() == Some("Wednesday, November 11, 2026")
                    && node.focused() == Some(true))
        );
    });
    cx.simulate_keystrokes("left left left");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        let saturday = nodes
            .iter()
            .find(|node| {
                node.role() == Some(Role::GridCell)
                    && node.label() == Some("Saturday, November 14, 2026")
            })
            .unwrap();
        assert_eq!(saturday.focused(), Some(true));
        assert_eq!(saturday.disabled(), Some(true));
        assert_eq!(
            saturday.selected(),
            Some(false),
            "unavailable dates are discoverable but cannot be selected"
        );
    });
    cx.simulate_keystrokes("left left");
    activate_key(cx, "enter");
    cx.update(|window, _| {
        let nodes = test_support::snapshots(window);
        let monday = nodes
            .iter()
            .find(|node| {
                node.role() == Some(Role::GridCell)
                    && node.label() == Some("Monday, November 16, 2026")
            })
            .unwrap();
        assert_eq!(monday.focused(), Some(true));
        assert_eq!(monday.selected(), Some(true));
    });
}
