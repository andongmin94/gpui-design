# Form controls

The [native settings example](../crates/base/examples/two_styles.rs) is a complete
consumer of the public API. Run it in either appearance using the commands in the
[README](../README.md#run-the-example). It combines Button, Input, Select,
Checkbox, Switch, RadioGroup, Slider, and Progress with the navigation and overlay controls,
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
| Slider | Retained `Entity<SliderState>` | Observe for rendering; `SliderEvent::Change` reports a changed, normalized value |
| Progress | Application percentage and indeterminate flag | Pass the current value and state on each render; no interaction callback |

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

## Slider

Create `SliderState::new(min..=max, step, cx)` once and optionally set an initial
value with `.default_value(value)`. Bounds and values must be finite, `min < max`,
and `step` must be positive. Values are clamped to the range and snapped to a
step grid starting at the minimum. The range is inclusive at both ends.

Render `Slider::new(id, &state).accessibility_label(...)` and supply the track,
fill, and thumb as children. `percentage()` gives the current position from 0 to
1. The slider's root bounds define the thumb-center travel rail; place the thumb
center at that fraction of the root and offset by half the application's thumb
size. The example uses a comfortable-height hit area and a thin track centered
inside it. Neither colors nor control dimensions are imposed by the library.

The state exposes `value`, `min_value`, `max_value`, `step_value`, `percentage`,
`focus_handle`, and `is_disabled`. Use `set_value(value, window, cx)` for a
programmatic update and `set_disabled(disabled, window, cx)` to change whether the
user can interact. Programmatic value updates remain possible while disabled.
Both programmatic and user changes emit `SliderEvent::Change(value)` only when
the normalized value actually changes; observe the state to update the view.

Pointer presses choose a position, and dragging continues until release even
outside the rail. `axis(Axis::Vertical)` changes the geometric direction so the
minimum is at the bottom; horizontal sliders increase from left to right.
The control exposes a Slider role, accessible name, orientation, numeric bounds,
current value, and step through native GPUI accessibility metadata.

| Input | Behavior |
| --- | --- |
| Left / Down | Decrease by one step |
| Right / Up | Increase by one step |
| Home / End | Set the minimum / maximum |
| PageDown / PageUp | Decrease / increase by ten steps |
| Tab / Shift-Tab | Follow normal focus traversal |

Disabled sliders ignore user changes and are skipped by Tab. A slider that is
disabled while focused keeps its focus binding long enough for Tab to leave.
This version provides a single linear value and one thumb.

## Progress

`Progress::new(id).value(percentage).accessibility_label(...)` supplies a named,
non-interactive ProgressIndicator with a numeric range of 0 to 100. Finite values
outside that range are clamped; non-finite values are configuration errors. The
default is known progress of zero percent. Use `.indeterminate(true)` when the
amount of progress is unknown; this omits the current numeric value.

Supply the visual track and fill with ordinary GPUI children. The application
also owns labels and any animation for unknown progress. Progress adds no focus
handle or Tab stop. The example connects a Slider to Progress to demonstrate a
controlled fill updating from the application's retained numeric state.

## Verification boundary

The existing GPUI tests cover pointer and native key event routing, checked/mixed
metadata, disabled behavior, group navigation, numeric controls, and representative
settings flows across both appearances. Headless event simulation does not validate an operating system
IME, screen reader, mixed-DPI display, or the rendered appearance of a native window.
Those checks remain separate from automated behavior tests.
