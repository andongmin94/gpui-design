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
    Keystroke, Modifiers, Role, TestAppContext, VisualTestContext,
};
use gpui_design::{DropdownMenuEvent, RadioGroupEvent, Root, SelectEvent, TabsEvent, test_support};
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

    // Removing the parent surface must also finish the retained popup lifecycle.
    cx.simulate_keystrokes("shift-tab end tab tab");
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
fn compact_commands_and_sections_preserve_nested_focus(cx: &mut TestAppContext) {
    let (cx, demo) = harness(cx, Appearance::Compact);
    let commands = Rc::new(RefCell::new(Vec::new()));
    cx.update(|_, cx| {
        let menu = demo.read(cx).actions.clone();
        let commands = commands.clone();
        cx.subscribe(&menu, move |menu, event: &DropdownMenuEvent, cx| {
            assert!(!menu.read(cx).is_open(cx), "commands observe a closed menu");
            let DropdownMenuEvent::Action(value) = event;
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

    cx.simulate_keystrokes("up up");
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

    cx.simulate_keystrokes("tab tab end tab");
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
        assert_eq!(
            test_support::find(window, &[], &"unknown-progress".into())
                .unwrap()
                .checked(),
            Some(true),
        );
    });
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
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            demo.read(cx).trigger_focus.is_focused(window),
            "ToggleGroup has one Tab stop"
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
    });

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
        assert!(demo.trigger_focus.is_focused(window));
        assert_eq!(demo.preview.read(cx).value(), 40.0);
        assert_eq!(
            demo.formatting.read(cx).selected_values(),
            ["bold", "italic"].map(gpui::SharedString::from),
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
