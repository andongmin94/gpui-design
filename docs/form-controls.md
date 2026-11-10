# Form controls

The [native settings example](../crates/base/examples/two_styles.rs) is a complete
consumer of the public API. Run it in either appearance using the commands in the
[README](../README.md#run-the-example). It combines Button, Input, Select,
Textarea, Field, Label, Combobox, Calendar, DatePicker, Checkbox, Switch, RadioGroup, Slider, Progress, Toggle, and ToggleGroup
with the navigation and overlay controls,
without putting appearance in the library. The [navigation and overlay guide](navigation-overlays.md)
describes its tabs and keyboard-help popover. The [layout and data guide](layout-and-data.md)
covers the Schedule table and the form's ScrollArea composition.

Call `gpui_design::init(cx)` once and host window content in `gpui_design::Root`.
Root supplies Tab traversal and contains it inside an open Dialog. Give each
control a stable element ID and an accessible label.

## State ownership

| Control | State owner | Updating the application |
| --- | --- | --- |
| Textarea | Retained `Entity<TextareaState>` | Observe for surrounding presentation; the existing Input events report edits |
| Field | Application label, help, requirement, and validation error | Render current metadata around an existing Input, Textarea, or Combobox query; no new input state |
| Label | Application text, target FocusHandle, and disabled flag | Focus the existing control on click; keep its accessible name and disabled state in sync |
| Combobox | Retained `Entity<ComboboxState>` | Observe for rendering; `ComboboxEvent::Change` reports a different committed user choice |
| Calendar | Retained `Entity<CalendarState>` | Observe for rendering; `CalendarEvent::Change` reports a different user-selected date |
| DatePicker | Retained `Entity<DatePickerState>` composing existing Calendar/Popover state | Observe for date/popup rendering; `DatePickerEvent::Change` runs after a different date commits and the popup closes |
| Checkbox | Application field of type `CheckboxState` or `bool` | `on_change` reports the next state; store it and call `cx.notify()` |
| Toggle | Application `bool` | `on_change` requests the next pressed value; render it back with `pressed` |
| ToggleGroup | Retained `Entity<ToggleGroupState>` | Observe for rendering; `ToggleGroupEvent::Change` reports selected values in item order |
| Switch | Application `bool` | `on_change` reports the next value; store it and call `cx.notify()` |
| RadioGroup | Retained `Entity<RadioGroupState>` | Observe the entity for rendering; subscribe to `RadioGroupEvent::Change` for user selection changes |
| Select | Retained `Entity<SelectState>` | Observe the entity for rendering; subscribe to `SelectEvent::Change` for committed user changes |
| Slider | Retained `Entity<SliderState>` | Observe for rendering; `SliderEvent::Change` reports the changed ordered values |
| Progress | Application percentage and indeterminate flag | Pass the current value and state on each render; no interaction callback |

Checkbox and Switch are controlled. Rendering them never changes application
state. If the application leaves the supplied value unchanged after a callback,
the control keeps showing that value. A programmatic field change is applied on
the next render and does not call `on_change`.

Keep retained entities and their subscriptions in the view, as the native example
does. Recreating a RadioGroupState in every render would discard its selection
and stable option focus handles.

## Textarea

`Textarea::new(&state)` renders the existing multiline editing engine. Construct
`TextareaState::new(window, cx)` once, using `.rows(3)` for fixed rows or
`.auto_grow(min, max)` for bounded automatic growth. The state also supports
placeholder/default text and soft wrapping; selection, undo, scrolling, clipboard,
and the native text-input interface stay in that same retained entity.

Give it an `InputAccessibility` with `Role::MultilineTextInput` and an accessible
label. Wrap the renderer in `InputBase` for the application-owned frame, setting
the frame's `.role(None::<Role>)` and `.semantic_owner::<Textarea>(state.entity_id())` so the
text entity is the sole native semantic/focus owner. The example uses the same
application palette for both its single-line field and multiline notes.

Enter inserts a newline by default. Tab and Shift-Tab follow ordinary form
navigation for both row-based and auto-growing textareas; code-editor indentation
remains separate. `set_disabled(bool, cx)` prevents edits and ordinary Tab entry.
A field disabled while focused keeps its binding so Tab can leave. Re-enabling
restores entry. `set_readonly(bool, cx)` prevents edits while retaining focus and
selection. Programmatic `set_value` remains available for application updates.

Keep the state outside a collapsible section, tab panel, or Dialog's render path
so closing that surface preserves the text. The example retains Session notes
when the settings dialog is closed and reopened.

## Field

`Field::new(id, &input_state, label)` associates a visible label, optional help,
and validation feedback with one retained input. The same constructor accepts
InputState and TextareaState; a Combobox exposes its query through `input()`.
Keep that input's renderer among the Field's children. The label is clickable,
focuses the live enabled input, and does not add another Tab stop.

```rust
Field::new("session-name-field", &self.input, "Session name")
    .description("Choose a name to recognize this session.")
    .required(true)
    .error(name_invalid.then(|| "A session name is required.".into()))
    .styles(|styles| {
        styles
            .description(|style| style.text_color(muted))
            .error(|style| style.text_color(error_color))
    })
    .flex()
    .flex_col()
    .gap_1()
    .child(input_frame)
```

The application owns validation and submission. In the native example, an empty
name supplies the error and disables Done; typing a name clears both. Field
renders label, control children, help, and error in order, with styles supplied
through `FieldStyles`. It supplies no color palette, validation schema, or form
controller. Its current control association covers the retained text-input
family, not Checkbox, Switch, or arbitrary controls.

The actual native input receives the label, an error-before-help description,
and required/invalid flags. Existing role, placeholder, accessibility identifier,
value-exposure policy, and popup expanded state are preserved. Passing
`error(None)` clears invalid state. The supporting `InputAccessibility` builders
are `with_description`, `with_required`, and `with_invalid`; the input remains
the only editing and focus owner.

## Label

`Label::new(id, &focus_handle, text)` renders plain text with native Label
semantics and an accessible name matching that text. Clicking it focuses the
supplied existing control. The Label has no focus handle or Tab stop of its own,
so ordinary keyboard traversal remains with the control. Its `Styled` interface
supplies application-owned text style and layout; it accepts text rather than
interactive child content.

```rust
let preview = self.preview.read(cx);
let label = Label::new(
    "preview-percentage-label",
    preview.thumb_focus_handle(0),
    "Preview percentage",
)
.disabled(preview.is_disabled())
.text_color(muted);
let slider = Slider::new("preview-percentage", &self.preview)
    .accessibility_label("Preview percentage")
    .child(SliderThumb::new(&self.preview, 0).accessibility_label("Preview percentage"));

div().flex().flex_col().gap_1().child(label).child(slider)
```

Keep the referenced target mounted in the same window, and mirror its disabled
state with `.disabled(bool)` on every render. A disabled Label does not move
focus. The supplied FocusHandle identifies the target but does not tell Label
whether the control is enabled; the application owns that policy and redraws
when it changes. Label retains no second control state.

The focus action does not activate the target: it does not open Select, adjust
Slider, or toggle Checkbox/Switch. The native example uses it above Drawing tool
and Preview percentage; clicking either label leaves the selection or value
unchanged before the control receives ordinary keyboard input.

Set the target's `accessibility_label(...)` independently. The pinned GPUI API
does not expose a public labelled-by relationship for these sibling elements,
so Label does not automatically name the target. Its native Label role and text
name describe the label itself. For Input/Textarea label, help, and validation
association, use Field, which projects metadata onto its retained input and
checks the input's live disabled state when its label is clicked.

## Combobox

Construct `ComboboxState::new(items, window, cx)` with fixed ordered
`SearchItem::new(value, label)` entries. Values must be unique; `.keywords(...)`
adds aliases and `.disabled(true)` marks an unavailable choice. This descriptor
is shared with Command, as are input editing, filtering, highlight, and list
scrolling. Combobox adds one committed selection and its popup lifecycle.

Use `set_selected_value(Some(value), window, cx)` to initialize or update the
selection, or `None` to clear it. The method rejects unknown/disabled values and
returns whether the request was accepted. Programmatic changes emit no Change
event. User Enter or pointer commits emit `ComboboxEvent::Change(value)` only
when the selected value actually changes.

Render `Combobox::new(id, &state).input(|input| ...).content(surface)`. The input
builder configures its frame, name, placeholder, and `InputStyles`. Put a named
`ComboboxList::new(id, &state)` in the surface and render direct
`ComboboxOption::new(&state, index)` children in `visible_indices(cx)` order.
Use `items(cx)` for labels, `highlighted_index(cx)` for the current search cursor,
and `selected_index()` for the committed choice. Keep empty text outside the
list. Supply a bounded list height and all visual styles in the application.

| Interaction | Result |
| --- | --- |
| Pointer or arrow opens a closed input | Shows all options with the committed choice highlighted; the query clears |
| Typing | Filters suggestions while keeping the previous committed value |
| Up / Down | Moves through enabled results without changing selection |
| Enter / result click | Commits the highlighted/clicked enabled choice and restores its label |
| Escape | Discards the draft and restores the label; an enclosing Dialog stays open |
| Tab / Shift-Tab, blur, outside press | Discards the draft while preserving normal focus movement |

Only the input is a Tab stop. `set_disabled(bool, window, cx)` closes the popup
and disables ordinary entry. `set_query(text, window, cx)` starts a focused draft.
While the native input has marked text, Enter cannot commit an option; Escape
unmarks that composition first and leaves the suggestions open. These are
automated native-handler contracts, not desktop IME validation.

The input exposes ComboBox role, name, value, expanded state, and any Field
metadata. Listbox options expose the committed selection independently of the
pending highlight. The pinned GPUI API cannot express an input-to-sibling-list
active-descendant relationship. This implementation covers fixed single
selection; asynchronous sources, grouping, multiple selection, and free-form
committed values remain outside its scope.

The example's recording preset uses this control inside the settings Dialog.
The form responds to Input's existing reveal requests so keyboard focus can
reach fields below its scrolling viewport. Closing and reopening the Dialog
preserves the committed preset.

## Calendar

`CalendarState` retains a single optional `NaiveDate`, the focused date, and the
visible month. `NaiveDate` is re-exported from Chrono; no second date model or
timezone conversion is introduced. Create the state once:

```rust
let calendar = cx.new(|cx| {
    let initial = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
    CalendarState::new(initial, cx)
        .bounds(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2027, 12, 31).unwrap(),
        )
        .default_value(initial)
});
```

Without `default_value`, selection starts empty. Default bounds cover years
1–9999; `bounds` narrows the inclusive selectable range before mounting. The
grid exposes six complete Monday-first weeks through `days()`. Adjacent-month
dates remain visible; dates outside the configured bounds are disabled.

Render `Calendar::new(id, &calendar)` with an accessible month/year label. Use
ordinary identified `div().id(...).role(Role::Row)` containers for one seven-cell weekday header
and six date rows. Weekday headings use `Role::ColumnHeader` with zero-based
column indices. Each `CalendarDay::new(&calendar, date)` composes Button's
activation/focus behavior and supplies its native GridCell role, selected state,
disabled state, and row/column indices. The grid declares seven rows including
the weekday header. Render each date returned by `days()` exactly once, in order.
Keep each day's children non-interactive.

The application owns grid geometry, weekday names, localized full-date labels,
selected-day styles, and month navigation buttons. `CalendarDay` defaults to an
ISO date accessibility label, supports `.accessibility_label(...)`, and accepts
ordinary GPUI styles plus Button's `.styles(...)` for selected/disabled states.
See `Demo::calendar_content` in the native example for the complete composition.

| Input | Result |
| --- | --- |
| Tab / Shift+Tab | Enter the grid at its one enabled day stop, or leave it |
| Left / Right | Move focus by one day |
| Up / Down | Move focus by one week |
| Home / End | Move to Monday / Sunday of the current week |
| Page Up / Page Down | Move by one month, clamping month-end dates |
| Shift+Page Up / Shift+Page Down | Move by one year |
| Enter / Space / click | Commit the enabled date; emit Change only if it changed |

Keyboard movement crosses months and clamps at selectable bounds without
changing the committed value. A newly focused day requests native reveal, which
a containing ScrollArea can satisfy. Month buttons call `previous_month` or
`next_month`, and use `can_previous_month`/`can_next_month` for disabled state.
Those methods retain an external navigation button's focus. Read `focus_handle()`
when explicitly moving focus into the calendar; its grid slot can change as the
visible month changes, so do not cache that handle as a permanent date identity.

`set_selected_date(Some(date), window, cx)` updates selection and visible month
without a user event, and rejects out-of-range values. Passing `None` clears
selection while retaining navigation. `set_disabled` blocks user navigation and
selection and removes normal Tab entry while preserving the selected date.

This contract covers a single Gregorian date and bounded selection. Range or
multiple selection, arbitrary disabled-day predicates, locale/week-start
configuration, clocks, and timezone handling are outside the current family.
DatePicker adds a popup composition over this same single-date contract. The
example demonstrates a retained plan, not a recording scheduler.

## DatePicker

DatePicker connects an existing Calendar to Popover's trigger, placement,
dismissal, and focus lifecycle. Calendar remains the only owner of the selected
date. Construct the calendar with its bounds and optional initial value, then
create the picker once in the same window:

```rust
let picker = cx.new(|cx| DatePickerState::new(&calendar, window, cx));
```

Observe the picker for rendering: it observes both its Calendar and Popover, so
a separate Calendar observation is unnecessary for the same view. Access the
date with `selected_date(cx)` and the nested date state with `calendar()`.
Render the Calendar only inside this picker's popup, not simultaneously in a
second standalone grid.

```rust
DatePicker::new("recording-date", &picker)
    .accessibility_label("Choose recording date")
    .offset(px(6.))
    .trigger(|button| {
        button
            .accessibility_label(format!("Recording date, {formatted_date}"))
            .child(formatted_date)
    })
    .content(calendar_surface)
```

The application supplies formatted trigger text, the calendar/month controls,
and the popup's dimensions and appearance. `accessibility_label` names the
popup; name the Button trigger separately. Ordinary CalendarDay activation is
enough to finish selection; the application should not duplicate a click-to-close
handler on every day. Root remains responsible for normal Tab traversal and an
enclosing Dialog's focus containment.

| Action | Result |
| --- | --- |
| Open with a selected date | Reset navigation to that date and focus its current grid slot |
| Open without selection | Retain and focus the Calendar's last navigation date |
| Navigate days/months | Change navigation without changing the committed date |
| Click / Enter / Space on an enabled day | Select it, close, and restore the trigger; reselecting the same date also closes |
| Escape | Close the picker before an enclosing Dialog, retaining selection |
| Tab / Shift+Tab past popup contents | Close and continue from the trigger's position |
| Click outside | Close while preserving the outside control's new focus |

The day focus is resolved immediately before opening because month changes can
move the selected date to a different focus slot. `DatePickerEvent::Change` is
emitted only for a different user-selected value and only after the popup closes.
Subscribers can therefore focus another control or open a new surface without
the finishing picker taking focus back. Public Calendar Change behavior remains
deduplicated; the composition also observes successful same-date activation.

`open` and `close` provide imperative lifecycle control. `set_disabled` closes
an open popup and disables both its trigger and Calendar. Use this method to
coordinate disabled state instead of modifying the nested calendar independently.
`set_selected_date(Some(date), window, cx)` validates the bounds, closes the
popup, and changes the value without emitting Change. Invalid dates leave the
picker unchanged; `None` clears selection and retains Calendar navigation.

This picker supports one date and an application-formatted Button trigger. It
does not parse text, choose date ranges/multiple dates, supply presets, or add
locale/timezone rules beyond the Calendar contract.

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

Create `SliderState::new(min..=max, step, [values...], cx)` once. A single value
creates a single-thumb slider; two values describe a range, and more values add
ordered stops. The nonempty collection fixes the thumb count. Bounds and values
must be finite, `min < max`, and `step` must be positive. Values are clamped and
snapped to a step grid starting at the minimum. The range includes both ends.

Render `Slider::new(id, &state).accessibility_label(...)` as the named group and
rail. Supply the track and fill, and one
`SliderThumb::new(&state, index).accessibility_label(...)` per value, in order.
Each thumb owns its native Slider role, focus, keys, and value actions. Give range
ends distinct names, such as "Trim start" and "Trim end". `percentage(index)`
gives that thumb's position from 0 to 1. The root bounds define its travel rail; place the thumb
center at that fraction of the root and offset by half the application's thumb
size. The example uses a comfortable-height hit area and a thin track centered
inside it. Neither colors nor control dimensions are imposed by the library.

The state exposes `values`, `min_value`, `max_value`, `step_value`, `percentage`,
`thumb_focus_handle(index)`, and `is_disabled`. Use
`set_thumb_value(index, value, window, cx)` or `set_values(values, window, cx)`
for programmatic updates and `set_disabled(disabled, window, cx)` to control user
interaction. Programmatic updates remain possible while disabled. Individual
thumbs stop at their neighbors without exchanging identities; equal values are
allowed. `SliderEvent::Change(values)` reports the complete ordered values once
when a normalized value changes. Observe the state to update application visuals.

Rail presses choose the nearest thumb, and dragging keeps that thumb until
release, including outside the rail. `axis(Axis::Vertical)` places the minimum
at the bottom; horizontal values follow the logical direction. Each thumb
exposes its accessible name, orientation, current value, step and numeric bounds
limited by neighboring thumbs. Applications supply thumb focus presentation.

| Input | Behavior |
| --- | --- |
| Left / Down | Decrease by one step |
| Right / Up | Increase by one step |
| Home / End | Set the focused thumb's allowed minimum / maximum |
| PageDown / PageUp | Decrease / increase by ten steps |
| Tab / Shift-Tab | Visit each enabled thumb in value order, then leave the group |

Disabled sliders ignore user changes and are skipped by Tab. A slider that is
disabled while focused keeps its focus binding long enough for Tab to leave.
The session preview uses one thumb; the catalogue's media trim control uses two
independent ends and switches between horizontal/vertical geometry and RTL.

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

The copyable [Spinner recipe](presentation-recipes.md#spinner) composes this same
indeterminate Progress with GPUI animation. The example's Unknown progress toggle
replaces the numeric bar with Spinner and a decorative Skeleton; switching back
uses the retained Slider value. Reset preview explicitly restores known progress
at 40%.

## Toggle and ToggleGroup

`Toggle::new(id).pressed(bool)` is a controlled toggle button. It composes the
existing Button, reusing native click/Enter/Space activation, disabled behavior,
and focus. Its added contract is the pressed accessibility state and
`on_change(next_pressed, event, window, cx)`. The application stores that value,
notifies its view, and supplies content and pressed-state styling. A stable label
names the action while the pressed metadata describes its current state.

Toggle accepts `accessibility_label`, `disabled`, `track_focus`, `tab_index`,
and `tab_stop`, along with normal GPUI styling and children.

For independent options that can be pressed together, create
`ToggleGroupState::new(items, cx)` from fixed, ordered
`ToggleItem::new(value, label)` entries with unique values. Mark unavailable items
with `.disabled(true)` and optionally set `.default_values([values])`. Defaults
must identify enabled items. Selection starts empty when no defaults are supplied.
Use `.single_selection()` for optional exclusive choice. Activating another option
replaces the previous value; activating the pressed option clears selection.
Single-selection defaults may contain at most one distinct value, regardless of
the order of `.single_selection()` and `.default_values(...)`.

Render `ToggleGroup::new(id, &state).accessibility_label(...)` and one
`ToggleOption::new(&state, index)` for each item. Each option composes Toggle.
The group supplies a Group role and orientation; application code supplies layout
separately. Read `items()`, `is_pressed(index)`, and `selected_values()` for the
current presentation. Observe the entity and subscribe to
`ToggleGroupEvent::Change(values)` for changes to the selected set; values follow
the original item order.

The group has one Tab stop, at its last focused enabled option or initially its
first enabled option. Horizontal Left/Right or vertical Up/Down move between
enabled options and wrap. Home/End reach the first/last enabled option. The other
axis propagates. Moving focus never changes the selected set; click, Enter, and
Space toggle the focused option according to the selection mode. Tab leaves the group.

`set_disabled(disabled, window, cx)` disables the whole group while preserving
selection. `option_focus_handle(index)` exposes stable option focus. RadioGroup
provides exclusive choice with immediate selection during arrow navigation;
ToggleGroup keeps focus movement independent of selection in either mode.
The session example uses independent bold/italic summary-text styling, and the
catalogue uses single selection for optional Center/End document alignment.

## InputOtp

`InputOtpState::new(length, window, cx)` creates one retained `InputState` with a
fixed nonzero slot count and an ASCII-digit/length validator. `InputOtp` paints
application-supplied slots through `.slot(|slot, window, cx| ...)`; each slot
receives its index, character, active/selected state and disabled/read-only flags.
Use those facts for borders, grouping and selection presentation. A slot click
selects a position in the same editor, and there is only one input Tab stop and
native accessibility owner.

Native paste, selection, arrow navigation, Backspace/Delete and undo remain with
the existing editor. The entire resulting pasted value must be numeric and fit;
text is not silently truncated or distributed among separate editors.
`InputOtpEvent` reports the edited `value` and whether all slots are filled.
`set_value` validates programmatic replacement; `input()` exposes the retained
editor for existing Enter/focus integration without changing its validator.
Completion is a length condition, and verification remains application-owned.

The catalogue Forms and feedback view uses the local code `742915`: completing
entry enables Review session, which opens a real confirmation Drawer; confirming
closes the modal before showing a timed Toast. Editing the code clears the local
confirmation state. No remote verification or transport is involved.

## Questionnaire

`QuestionnaireState<A>` stores the application's answer type and a fixed linear
flow. `submit_answer` receives an answer plus the application's validation result,
retains the answer even when rejected, and advances only after successful
validation. Validation errors keep the current question available for correction. `back()` returns to
a previous question/review without discarding answers; `complete()` transitions
from Review to Complete. An empty questionnaire begins at Review.

`Questionnaire::new(id, label)` supplies a named semantic group for the current
stage. Compose existing editors and ordinary navigation Buttons, notify the
owning view after state changes, and move focus before unmounting an editor. The
catalogue demonstrates two retained Input entities, validation, Back, review and
completion into the local conversation. There is no schema language, branching
engine, submission backend or duplicate field registry.

## Verification boundary

The existing GPUI tests cover pointer and native key event routing, checked/mixed
metadata, disabled behavior, group navigation, numeric controls, and representative
settings flows across both appearances. Headless event simulation does not validate an operating system
IME, screen reader, mixed-DPI display, or the rendered appearance of a native window.
Those checks remain separate from automated behavior tests.
