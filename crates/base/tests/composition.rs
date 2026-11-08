//! Exercises the same consumer code as the native two_styles example.
//! GPUI's input simulation does not run a platform IME or screen reader.

#[path = "../examples/two_styles.rs"]
mod two_styles;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    AppContext as _, Entity, EntityInputHandler as _, Focusable as _, KeyDownEvent, KeyUpEvent,
    Keystroke, Modifiers, Role, TestAppContext, VisualTestContext,
};
use gpui_design::{RadioGroupEvent, Root, SelectEvent, TabsEvent, test_support};
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
        let trigger = demo.read(cx).trigger_focus.clone();
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

fn exercise_keyboard_flow(cx: &mut TestAppContext, appearance: Appearance) {
    let (cx, demo) = harness(cx, appearance);
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
    cx.update(|window, _| {
        assert!(
            test_support::snapshots(window)
                .iter()
                .any(|item| { item.label() == Some("Done") && item.focused() == Some(true) }),
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
        assert!(demo.read(cx).trigger_focus.is_focused(window));
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
fn compact_controls_complete_the_keyboard_flow(cx: &mut TestAppContext) {
    exercise_keyboard_flow(cx, Appearance::Compact);
}

#[gpui::test]
fn settings_controls_complete_the_same_keyboard_flow(cx: &mut TestAppContext) {
    exercise_keyboard_flow(cx, Appearance::Settings);
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
        assert!(demo.trigger_focus.is_focused(window));
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
        assert!(demo.trigger_focus.is_focused(window));
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
        assert!(demo.trigger_focus.is_focused(window));
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

fn exercise_navigation_and_help(cx: &mut TestAppContext, appearance: Appearance) {
    let (cx, demo) = harness(cx, appearance);
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

    cx.simulate_keystrokes("tab");
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
    });

    cx.simulate_keystrokes("tab");
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
            test_support::snapshots(window)
                .iter()
                .any(|item| { item.label() == Some("Done") && item.focused() == Some(true) }),
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

    // Removing the parent surface must also finish the retained popup lifecycle.
    cx.simulate_keystrokes("shift-tab end tab");
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
        assert!(demo.trigger_focus.is_focused(window));
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
fn compact_tabs_and_popover_preserve_nested_focus(cx: &mut TestAppContext) {
    exercise_navigation_and_help(cx, Appearance::Compact);
}

#[gpui::test]
fn settings_tabs_and_popover_preserve_nested_focus(cx: &mut TestAppContext) {
    exercise_navigation_and_help(cx, Appearance::Settings);
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
