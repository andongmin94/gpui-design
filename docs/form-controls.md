# Form controls

The [native settings example](../crates/base/examples/two_styles.rs) is a complete
consumer of the public API. Run it in either appearance using the commands in the
[README](../README.md#run-the-example). It combines Button, Input, Select,
Checkbox, Switch, and RadioGroup with the navigation and overlay controls,
without putting appearance in the library. The [navigation and overlay guide](navigation-overlays.md)
describes its Preferences/Help tabs and keyboard-help popover.

Call `gpui_design::init(cx)` once and host window content in `gpui_design::Root`.
Root supplies Tab traversal and contains it inside an open Dialog. Give each
control a stable element ID and an accessible label.

## State ownership

| Control | State owner | Updating the application |
| --- | --- | --- |
| Checkbox | Application field of type `CheckboxState` or `bool` | `on_change` reports the next state; store it and call `cx.notify()` |
| Switch | Application `bool` | `on_change` reports the next value; store it and call `cx.notify()` |
| RadioGroup | Retained `Entity<RadioGroupState>` | Observe the entity for rendering; subscribe to `RadioGroupEvent::Change` for user selection changes |
| Select | Retained `Entity<SelectState>` | Observe the entity for rendering; subscribe to `SelectEvent::Change` for committed user changes |

Checkbox and Switch are controlled. Rendering them never changes application
state. If the application leaves the supplied value unchanged after a callback,
the control keeps showing that value. A programmatic field change is applied on
the next render and does not call `on_change`.

Keep retained entities and their subscriptions in the view, as the native example
does. Recreating a RadioGroupState in every render would discard its selection
and stable option focus handles.

## Checkbox and Switch

Use `.checked(bool)` for a binary checkbox. For an aggregate selection, use
`.state(CheckboxState::Indeterminate)` to expose a mixed state. Activating a mixed
checkbox requests `CheckboxState::Checked`; the library does not infer or alter
any child selections. Switch always uses a boolean state.

Both controls accept `.disabled(bool)`, `.accessibility_label(...)`,
`.track_focus(&handle)`, `.tab_index(...)`, and `.tab_stop(...)`. The change
callback receives the next value, the native click event, the window, and the app
context. For example, inside a view that stores a boolean `show_cursor`:

```rust
let owner = cx.entity();
Checkbox::new("show-cursor")
    .checked(self.show_cursor)
    .accessibility_label("Show cursor")
    .on_change(move |next, _, _, cx| {
        owner.update(cx, |this, cx| {
            this.show_cursor = next == CheckboxState::Checked;
            cx.notify();
        });
    })
    .child("Show cursor")
```

This is the behavior wiring. The application adds GPUI styling and an indicator
as children. Checkbox supplies no checkmark, and Switch supplies no track or
thumb. Their root `.styles(...)` builders expose checked/disabled refinements;
Checkbox additionally exposes indeterminate styling. Keep a Switch's accessible
label stable when it changes state: its checked metadata already describes on/off.

## RadioGroup

Construct `RadioGroupState::new(items, cx)` once with a fixed ordered collection of
`RadioItem::new(value, label)`. Values must be unique. Mark unavailable options
with `.disabled(true)`, and optionally choose an enabled `.default_value(value)`.
Without a matching enabled default, the group starts unselected.

Render `RadioGroup::new(id, &state)` with an accessible label, and one
`RadioOption::new(&state, index)` for each item in the same order. Supply each
option's content and style in the application. The group sets its accessibility
role and orientation; `.axis(Axis::Horizontal)` changes orientation metadata.
Use GPUI layout methods such as `.flex()` separately to arrange the options.

The state exposes `items()`, `selected_index()`, `selected_item()`, and
`option_focus_handle(index)`. Use `set_disabled(disabled, window, cx)` to disable
the whole group. Disabling preserves the selected value and emits no selection
event. `RadioGroupEvent::Change(value)` is emitted once when a user selects a
different value. Selecting the current value does not emit another change.

## Keyboard and disabled behavior

| Input | Checkbox / Switch | RadioGroup |
| --- | --- | --- |
| Tab / Shift-Tab | Enter or leave an enabled control | Enter at the selected enabled option, or the first enabled option when unselected; one Tab stop for the whole group |
| Space / Enter | Request the next value through the native activation path | Select the focused option |
| Right / Down | No selection navigation | Select and focus the next enabled option, wrapping at the end |
| Left / Up | No selection navigation | Select and focus the previous enabled option, wrapping at the beginning |
| Escape | Propagates to the containing surface | Propagates to the containing surface |

RadioGroup arrow selection is immediate. Select keeps a pending keyboard cursor
while its popup is open and commits only on activation. Both skip disabled
options. An empty or entirely disabled RadioGroup has no Tab stop.

Disabled controls ignore activation and are skipped during Tab traversal. If a
focused control becomes disabled, it retains its focus binding until focus leaves
so Tab and Shift-Tab can still reach the neighboring control.

## Verification boundary

The existing GPUI tests cover pointer and native key event routing, checked/mixed
metadata, disabled behavior, group navigation, and the composed settings flow in
both appearances. Headless event simulation does not validate an operating system
IME, screen reader, mixed-DPI display, or the rendered appearance of a native window.
Those checks remain separate from automated behavior tests.
