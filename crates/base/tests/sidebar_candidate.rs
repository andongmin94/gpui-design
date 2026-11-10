//! Isolated review of Sidebar; this does not import or alter two_styles.
//! Uses the focus-neutral TabsState::select_value method in this review branch.
//! These checks have not been compiled or run by the patch author.

#![cfg(feature = "test-support")]

use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Context, Element as _, ElementId, Entity, FocusHandle, Hsla,
    InteractiveElement as _, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
    ParentElement as _, Pixels, Rems, Render, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, TestAppContext,
    VisualTestContext, Window, accesskit, div, px, rems, rgb, size,
};
use gpui_design::{Button, Root, TabItem, TabsEvent, TabsState, TestSupportExt as _, test_support};

#[path = "sidebar_candidate/recipe.rs"]
mod candidate;

use candidate::sidebar::Sidebar;

// Only the five consumer-owned tokens read by the unmodified Sidebar candidate.
// The values match the corresponding tokens in two_styles at dd5a899.
#[derive(Clone, Copy)]
pub(crate) struct Palette {
    foreground: Hsla,
    highlight: Hsla,
    accent: Hsla,
    font_size: Rems,
    radius: Pixels,
}

fn palettes() -> [Palette; 2] {
    [
        Palette {
            foreground: rgb(0xf1f5f9).into(),
            highlight: rgb(0x2b4160).into(),
            accent: rgb(0x93c5fd).into(),
            font_size: rems(0.8125),
            radius: px(4.),
        },
        Palette {
            foreground: rgb(0x243328).into(),
            highlight: rgb(0xe1eee3).into(),
            accent: rgb(0x286447).into(),
            font_size: rems(0.9375),
            radius: px(10.),
        },
    ]
}

type Changes = Rc<RefCell<Vec<SharedString>>>;

struct Sample {
    state: Entity<TabsState>,
    palette: Palette,
    width: Pixels,
    before: FocusHandle,
    content_action: FocusHandle,
    after: FocusHandle,
    _subscription: Subscription,
}

fn entry_id(index: usize) -> ElementId {
    (ElementId::from("candidate-sidebar"), index.to_string()).into()
}

fn content_id(value: &str) -> ElementId {
    (ElementId::from("candidate-content"), value.to_string()).into()
}

impl Render for Sample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.state.read(cx).selected_item().unwrap();
        let value = selected.value().clone();
        let label = selected.label().clone();
        div()
            .id("candidate-frame")
            .test_support()
            .w(self.width)
            .flex()
            .flex_col()
            .child(
                Button::new("candidate-before")
                    .accessibility_label("Before navigation")
                    .track_focus(&self.before)
                    .h(px(32.))
                    .child("Before"),
            )
            .child(
                Sidebar::new(
                    "candidate-sidebar",
                    &self.state,
                    "Candidate sections",
                    self.palette,
                )
                .w_full(),
            )
            .child(
                div()
                    .id(content_id(value.as_ref()))
                    .test_support()
                    .role(Role::Region)
                    .aria_label(label)
                    .w_full()
                    .min_w_0()
                    .child(
                        Button::new("candidate-content-action")
                            .accessibility_label("Content action")
                            .track_focus(&self.content_action)
                            .h(px(32.))
                            .child("Content action"),
                    ),
            )
            .child(
                Button::new("candidate-after")
                    .accessibility_label("After content")
                    .track_focus(&self.after)
                    .h(px(32.))
                    .child("After"),
            )
    }
}

fn harness(
    cx: &mut TestAppContext,
    palette: Palette,
    items: Vec<TabItem>,
) -> (&mut VisualTestContext, Entity<Sample>, Changes) {
    cx.update(gpui_design::init);
    let (root, cx) = cx.add_window_view(move |window, cx| {
        let sample = cx.new(|cx| {
            let state = cx.new(|cx| TabsState::new(items, cx));
            let subscription = cx.observe(&state, |_, _, cx| cx.notify());
            Sample {
                state,
                palette,
                width: px(320.),
                before: cx.focus_handle().tab_stop(true),
                content_action: cx.focus_handle().tab_stop(true),
                after: cx.focus_handle().tab_stop(true),
                _subscription: subscription,
            }
        });
        Root::new(sample, window, cx)
    });
    let sample = root
        .read_with(cx, |root, _| root.view().clone())
        .downcast::<Sample>()
        .unwrap();
    let changes = Rc::new(RefCell::new(Vec::new()));
    cx.update(|window, cx| {
        let state = sample.read(cx).state.clone();
        let events = changes.clone();
        cx.subscribe(&state, move |_, event: &TabsEvent, _| {
            let TabsEvent::Change(value) = event;
            events.borrow_mut().push(value.clone());
        })
        .detach();
        window.activate_window();
        let before = sample.read(cx).before.clone();
        before.focus(window, cx);
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    (cx, sample, changes)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn activate_key(cx: &mut VisualTestContext, key: &str) {
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

fn assert_entry_focus(cx: &mut VisualTestContext, index: usize) {
    cx.update(|window, _| {
        let entry = test_support::find(window, &[], &entry_id(index)).unwrap();
        assert_eq!(entry.role(), Some(Role::Button));
        assert_eq!(entry.focused(), Some(true));
    });
}

#[gpui::test]
fn sidebar_candidate_uses_button_focus_and_explicit_activation(cx: &mut TestAppContext) {
    let (cx, sample, changes) = harness(
        cx,
        palettes()[0],
        vec![
            TabItem::new("preferences", "Preferences"),
            TabItem::new("sync", "Sync unavailable").disabled(true),
            TabItem::new("help", "Help"),
        ],
    );
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert_entry_focus(cx, 0);
    cx.simulate_keystrokes("right down home end");
    draw(cx);
    assert_entry_focus(cx, 0);
    cx.update(|_, cx| assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(0)));
    assert!(changes.borrow().is_empty());

    cx.simulate_keystrokes("tab");
    draw(cx);
    assert_entry_focus(cx, 2);
    cx.update(|_, cx| assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(0)));
    activate_key(cx, "enter");
    assert_entry_focus(cx, 2);
    cx.update(|window, cx| {
        assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(2));
        assert!(test_support::find(window, &[], &content_id("preferences")).is_none());
        let content = test_support::find(window, &[], &content_id("help")).unwrap();
        assert_eq!(content.role(), Some(Role::Region));
        assert_eq!(content.label(), Some("Help"));
        assert_eq!(content.focused(), None);
        let nodes = test_support::snapshots(window);
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.role() == Some(Role::Region))
                .count(),
            1
        );
        assert!(nodes.iter().all(|node| !matches!(
            node.role(),
            Some(Role::TabList) | Some(Role::Tab) | Some(Role::TabPanel)
        )));
    });
    activate_key(cx, "space");
    assert_entry_focus(cx, 2);
    assert_eq!(*changes.borrow(), [SharedString::from("help")]);

    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| assert!(sample.read(cx).content_action.is_focused(window)));
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| assert!(sample.read(cx).after.is_focused(window)));
    cx.simulate_keystrokes("shift-tab shift-tab");
    draw(cx);
    assert_entry_focus(cx, 2);
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    assert_entry_focus(cx, 0);
    activate_key(cx, "space");
    assert_entry_focus(cx, 0);
    cx.update(|_, cx| assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(0)));

    let disabled_position = cx.update(|window, _| {
        let entry = test_support::find(window, &[], &entry_id(1)).unwrap();
        assert_eq!(entry.disabled(), Some(true));
        entry.bounds().center()
    });
    cx.simulate_click(disabled_position, Modifiers::default());
    draw(cx);
    cx.update(|_, cx| assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(0)));
    assert_eq!(
        *changes.borrow(),
        [SharedString::from("help"), SharedString::from("preferences")]
    );
    let help_position = cx.update(|window, _| {
        test_support::find(window, &[], &entry_id(2))
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(help_position, Modifiers::default());
    draw(cx);
    assert_entry_focus(cx, 2);
    cx.update(|_, cx| assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(2)));
    assert_eq!(
        *changes.borrow(),
        [
            SharedString::from("help"),
            SharedString::from("preferences"),
            SharedString::from("help"),
        ]
    );
}

#[gpui::test]
fn sidebar_candidate_has_named_navigation_and_narrow_geometry(cx: &mut TestAppContext) {
    let labels = [
        format!("설정 {}", "공백없는아주긴한글이름".repeat(6)),
        "Sync unavailable".to_string(),
        format!("Help {}", "long_unbroken_english_section_name".repeat(5)),
    ];
    for palette in palettes() {
        let (cx, sample, changes) = harness(
            cx,
            palette,
            vec![
                TabItem::new("preferences", labels[0].clone()),
                TabItem::new("sync", labels[1].clone()).disabled(true),
                TabItem::new("help", labels[2].clone()),
            ],
        );
        cx.simulate_resize(size(px(480.), px(1400.)));
        cx.update(|window, cx| {
            let state = sample.read(cx).state.clone();
            let native = Sidebar::new("native-probe", &state, "Candidate sections", palette)
                .render(window, cx)
                .into_element();
            assert_eq!(native.a11y_role(), Some(Role::Navigation));
            let mut node = accesskit::Node::new(Role::Navigation);
            native.write_a11y_info(&mut node);
            assert_eq!(node.label(), Some("Candidate sections"));
            assert!(!node.supports_action(accesskit::Action::Focus));
            assert!(!node.supports_action(accesskit::Action::Click));
        });
        let mut wide_height = None;
        for width in [320., 160., 320.] {
            sample.update(cx, |sample, cx| {
                sample.width = px(width);
                cx.notify();
            });
            draw(cx);
            cx.run_until_parked();
            draw(cx);
            cx.update(|window, cx| {
                let frame = test_support::find(window, &[], &"candidate-frame".into()).unwrap();
                assert!(frame.bounds().size.width <= px(width) + px(1.));
                for (index, label) in labels.iter().enumerate() {
                    let entry = test_support::find(window, &[], &entry_id(index)).unwrap();
                    assert_eq!(entry.role(), Some(Role::Button));
                    assert_eq!(entry.label(), Some(label.as_str()));
                    assert_eq!(entry.disabled(), (index == 1).then_some(true));
                    assert!(entry.bounds().left() >= frame.bounds().left());
                    assert!(entry.bounds().right() <= frame.bounds().right() + px(1.));
                }
                let height = test_support::find(window, &[], &entry_id(0))
                    .unwrap()
                    .bounds()
                    .size
                    .height;
                match wide_height {
                    None => wide_height = Some(height),
                    Some(wide) if width == 320. => assert!((height - wide).abs() < px(1.)),
                    Some(wide) => assert!(height > wide),
                }
                assert!(sample.read(cx).before.is_focused(window));
                assert_eq!(sample.read(cx).state.read(cx).selected_index(), Some(0));
            });
        }
        assert!(changes.borrow().is_empty());
    }
}
