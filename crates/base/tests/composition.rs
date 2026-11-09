//! Exercises the same consumer code as the native two_styles example.
//! Each distinct flow runs once, across representative compact and settings appearances.
//! GPUI's input simulation does not run a platform IME or screen reader.

#[path = "../examples/two_styles.rs"]
mod two_styles;

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
    ComboboxEvent, CommandEvent, MenuEvent, NaiveDate, RadioGroupEvent, Root, SelectEvent,
    TabsEvent, test_support,
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
    for (appearance, preferred_rems) in [(Appearance::Compact, 22.), (Appearance::Settings, 28.)] {
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
fn settings_tabs_and_popover_preserve_nested_focus(cx: &mut TestAppContext) {
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
    cx.simulate_keystrokes("tab tab right");
    draw(cx);
    cx.update(|window, cx| {
        let tabs = demo.read(cx).tabs.read(cx);
        assert_eq!(tabs.selected_index(), Some(2), "skip the disabled Sync tab");
        assert!(tabs.tab_focus_handle(2).is_focused(window));
        assert!(test_support::find(window, &[], &"show-cursor".into()).is_none());
        assert!(
            test_support::snapshots(window).iter().any(|item| {
                item.role() == Some(Role::TabPanel) && item.label() == Some("Help")
            })
        );
        assert!(test_support::snapshots(window).iter().any(|item| {
            item.role() == Some(Role::Tab)
                && item.label() == Some("Sync (unavailable)")
                && item.disabled() == Some(true)
        }));
    });
    assert_eq!(
        changes.borrow().as_slice(),
        &[gpui::SharedString::from("help")]
    );

    cx.simulate_keystrokes("tab tab");
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
    // Only ancestor actions and Close help join Tab order; current text and keycaps do not.
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

    // A hidden panel has no Tab stops; its application-owned values survive remounting.
    cx.simulate_keystrokes("tab tab home tab");
    draw(cx);
    activate_key(cx, "space");
    cx.update(|_, cx| assert!(!demo.read(cx).show_cursor));
    cx.simulate_keystrokes("shift-tab right");
    draw(cx);
    cx.update(|window, _| {
        assert!(test_support::find(window, &[], &"show-cursor".into()).is_none());
    });
    cx.simulate_keystrokes("left tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!demo.read(cx).show_cursor);
        let checkbox = test_support::find(window, &[], &"show-cursor".into()).unwrap();
        assert_eq!(checkbox.checked(), Some(false));
        assert_eq!(checkbox.focused(), Some(true));
    });

    // The root ancestor returns to Preferences through existing tab focus activation.
    cx.simulate_keystrokes("shift-tab right tab tab");
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
    cx.simulate_keystrokes("right tab tab");
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
fn compact_commands_and_sections_preserve_nested_focus(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Compact);
    let commands = Rc::new(RefCell::new(Vec::new()));
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

    cx.simulate_keystrokes("down t t");
    draw(cx);
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
        assert_eq!(accordion.open_index(), Some(0));
        assert!(accordion.trigger_focus_handle(0).is_focused(window));
    });
    cx.simulate_keystrokes("down");
    draw(cx);
    cx.update(|window, cx| {
        let accordion = demo.read(cx).accordion.read(cx);
        assert!(accordion.trigger_focus_handle(2).is_focused(window));
        assert_eq!(
            accordion.open_index(),
            Some(0),
            "arrows only move header focus"
        );
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        assert_eq!(demo.read(cx).accordion.read(cx).open_index(), Some(2));
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
    cx.update(|_, cx| assert_eq!(demo.read(cx).accordion.read(cx).open_index(), None));

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
        assert_eq!(demo.accordion.read(cx).open_index(), None);
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
        assert_eq!(demo.accordion.read(cx).open_index(), Some(0));
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
        test_support::snapshots(window)
            .into_iter()
            .find(|item| {
                item.role() == Some(Role::MenuItem) && item.label() == Some("Toggle cursor")
            })
            .unwrap()
            .bounds()
            .center()
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

    // The same retained Slider value drives the application's progress preview.
    cx.simulate_keystrokes("tab right");
    draw(cx);
    cx.update(|window, cx| {
        let preview = demo.read(cx).preview.read(cx);
        assert!(preview.focus_handle().is_focused(window));
        assert_eq!(preview.value(), 45.0);
        let progress = test_support::find(window, &[], &"preview-progress".into()).unwrap();
        assert_eq!(progress.role(), Some(Role::ProgressIndicator));
        assert_eq!(progress.label(), Some("Preview progress"));
        assert!(test_support::find(window, &[], &"preview-progress-placeholder".into()).is_none());
    });
    cx.simulate_keystrokes("end");
    draw(cx);
    let progress_width = cx.update(|window, cx| {
        assert_eq!(demo.read(cx).preview.read(cx).value(), 100.0);
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
        assert_eq!(demo.read(cx).preview.read(cx).value(), 100.0);
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
        assert_eq!(demo.preview.read(cx).value(), 100.0);
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
        assert_eq!(demo.preview.read(cx).value(), 100.0);
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
        assert_eq!(demo.preview.read(cx).value(), 40.0);
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
        assert_eq!(demo.preview.read(cx).value(), 40.0);
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
    cx.simulate_keystrokes("tab down s");
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

    cx.simulate_keystrokes("tab tab tab tab tab tab");
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
    cx.simulate_keystrokes("down s");
    draw(cx);
    activate_key(cx, "enter");
    cx.update(|_, cx| {
        assert_eq!(demo.read(cx).commands.read(cx).input().read(cx).value(), "");
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

    cx.simulate_keystrokes("tab tab tab tab tab tab tab");
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
            preset.selected_value(cx).map(|value| value.as_ref()),
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
            preset.selected_value(cx).map(|value| value.as_ref()),
            Some("presentation")
        );
        assert_eq!(preset.input().read(cx).value(), "Presentation");
    });
    assert_eq!(
        changes.borrow().as_slice(),
        &[gpui::SharedString::from("presentation")]
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
                .selected_value(cx)
                .map(|value| value.as_ref()),
            Some("presentation")
        );
    });
    assert_eq!(changes.borrow().len(), 1);
}

#[gpui::test]
fn date_picker_and_split_panes_preserve_nested_form_state(cx: &mut TestAppContext) {
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
    });
    activate_key(cx, "enter");
    cx.update(|window, cx| {
        let demo = demo.read(cx);
        assert!(demo.date_picker.read(cx).is_open(cx));
        let calendar = demo.calendar.read(cx);
        assert!(calendar.focus_handle().is_focused(window));
        assert_eq!(
            calendar.selected_date(),
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
            calendar.selected_date(),
            NaiveDate::from_ymd_opt(2026, 11, 9)
        );
        let grid = test_support::find(window, &[], &"recording-calendar".into()).unwrap();
        assert_eq!(grid.role(), Some(Role::Grid));
        assert_eq!(grid.label(), Some("Recording date, December 2026"));
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
            demo.calendar.read(cx).selected_date(),
            NaiveDate::from_ymd_opt(2026, 11, 9)
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
            demo.read(cx).calendar.read(cx).selected_date(),
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
            demo.calendar.read(cx).selected_date(),
            NaiveDate::from_ymd_opt(2026, 12, 10)
        );
        assert!(!demo.date_picker.read(cx).is_open(cx));
        assert!((demo.schedule_split.read(cx).ratio() - 0.52).abs() < 0.001);
        assert_eq!(demo.notes.read(cx).value(), "Prepare the workshop demo.");
        assert!(test_support::texts(window).iter().any(|text| {
            text.path().contains(&"plan-date".into()) && text.text() == "2026-12-10"
        }));
    });
}

#[gpui::test]
fn preset_table_filters_sorts_pages_and_applies_stable_rows(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Settings);
    open_dialog(cx, &demo);
    cx.simulate_keystrokes("tab tab end tab");
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

    // Search, two sort buttons, then the only enabled page action.
    cx.simulate_keystrokes("tab tab tab");
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
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab shift-tab shift-tab");
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
    });

    cx.simulate_keystrokes("shift-tab");
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
                .selected_value(cx)
                .map(|value| value.as_ref()),
            Some("balanced")
        );
    });
    cx.simulate_keystrokes("tab");
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
    cx.simulate_keystrokes("tab tab tab");
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
            preset.selected_value(cx).map(|value| value.as_ref()),
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
                .selected_value(cx)
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
    let trigger_point = cx.update(|window, _| {
        test_support::find(window, &[], &"drawing-tool".into())
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(trigger_point, Modifiers::default());
    draw(cx);
    cx.update(|_, cx| {
        assert!(!demo.read(cx).select.read(cx).is_open());
        assert!(demo.read(cx).dialog.read(cx).is_open());
    });
}
