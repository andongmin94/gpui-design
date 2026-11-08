# Navigation and overlays

The [two-style example](../crates/base/examples/two_styles.rs) combines the same
settings behavior with compact dark and spacious light presentation. Its
Preferences and Help tabs demonstrate active panels, and Help opens a keyboard
guide in a non-modal Popover inside Dialog. All styles live in the example.

Call `gpui_design::init(cx)` and host the window in `gpui_design::Root`. Keep the
state entities and their observation subscriptions in your view. The example
shows the complete setup and application-owned styling.

## Tabs

The tab family consists of `TabsState`, `TabList`, `Tab`, and `TabPanel`:

1. Create `TabsState::new(items, cx)` with fixed ordered `TabItem::new(value, label)`
   entries. Values must be unique. Mark unavailable entries with `.disabled(true)`.
2. The first enabled tab starts selected. Use `.default_value(value)` to select
   another enabled item; an unknown or disabled default is a configuration error
   and panics. An empty or entirely disabled collection has no active panel.
3. Render `TabList::new(id, &state)` with an accessible label and one
   `Tab::new(&state, index)` for each item, in order.
4. Render matching `TabPanel::new(&state, index)` elements beside the list. A plain
   GPUI `div` can arrange the list and panels. There is no additional required root
   wrapper or imposed layout.

Each tab supplies its role, label, selected/disabled metadata, stable focus handle,
and activation. The application supplies its visual content. The state exposes
`items()`, `selected_index()`, `selected_item()`, `tab_focus_handle(index)`, and
`panel_focus_handle(index)`. Observe the state to redraw the consuming view and
subscribe to `TabsEvent::Change(value)` for changed selection. Selecting the
current tab does not emit a duplicate event.

Activation is automatic: moving focus to an enabled tab also selects it. This
includes pointer, keyboard, and programmatic focus. Each list has one Tab stop,
at its selected tab.

| Input | Horizontal list | Vertical list |
| --- | --- | --- |
| Left / Right | Previous / next enabled tab | Propagates without changing tabs |
| Up / Down | Propagates without changing tabs | Previous / next enabled tab |
| Home / End | First / last enabled tab | First / last enabled tab |
| Enter / Space | Activate the focused tab | Activate the focused tab |
| Tab / Shift-Tab | Leave the tab list | Leave the tab list |

Arrow movement wraps at either end and skips disabled entries. Set the list's
orientation with `.axis(Axis::Horizontal)` or `.axis(Axis::Vertical)` and arrange
it with normal GPUI layout methods. Orientation does not choose a visual layout.

Only the active panel is mounted. Inactive panels contribute no layout, focus
stops, or accessibility nodes. Keep form data and retained input entities in the
application view so values survive a panel being removed and rendered again.

By default a panel is a focus scope without its own Tab stop; Tab can enter its
interactive children. Use `.tab_stop(true)` for a panel that needs to receive
focus itself, such as one containing only static text. The panel's accessible
label comes from its tab. The pinned GPUI API exposes roles, labels, and selected
state but not a public sibling-node relationship API, so explicit native
tab-to-panel controls/labelled-by relationships are not emitted. Native screen
reader behavior still needs desktop validation.

## Popover

`PopoverState` owns the open state, trigger focus, popup focus scope, and disabled
state. Create it once with `cx.new(PopoverState::new)` and observe it for rendering.
Optionally use `.with_initial_focus(&handle)` for a control inside the popup.
Otherwise opening focuses the content scope and Tab enters its first control.

Build `Popover::new(id, &state)` with:

- `.trigger(|button| ...)` to supply the trigger's content, accessible name, and
  appearance using the existing Button API.
- `.accessibility_label(...)` to name the popup's non-modal dialog surface.
- `.content(...)` for application-owned popup contents and surface styling.
- `.anchor(...)` and `.offset(...)` when the existing Popup geometry needs a
  different attachment point or gap.

The library owns the trigger's activation, focus, disabled state, and expanded
metadata. The supplied `id` identifies that trigger. Its button builder should
provide appearance and an accessible name, while application buttons inside the
popup can call `PopoverState::close(window, cx)`.

State methods include `open`, `close`, `is_open`, `is_disabled`,
`set_disabled(disabled, window, cx)`, `focus_handle`, and `content_focus_handle`.
Disabling closes an open popup and prevents reopening. Keep the Popover mounted
while closed so its trigger retains identity and its anchoring geometry can be
measured.

Popover is non-modal: it does not trap focus. Tab visits its contents and then
moves to the next ordinary control after its trigger, closing the popup. Shift-Tab
before its first content control closes it and returns to the trigger. A containing
Dialog continues to bound that traversal.

Escape closes the focused popup and restores trigger focus. If Popover is inside
Dialog, the first Escape leaves the parent dialog open. A pointer press outside
closes the popup while allowing the clicked control to receive focus. Focus moved
elsewhere, parent-surface closure, and window deactivation also dismiss it; the
popup does not pull focus back after another surface has taken it.

When an application removes the entire focused Popover host, including its
trigger, it must move focus to a surviving control as part of that update. The
popup cleans up its open state without choosing an arbitrary replacement target.
The example's tab switches and Dialog closure already move focus before removing
the popup surface.

## Validation

The existing GPUI test framework checks selected and disabled metadata, orientation,
focus traversal, inactive panels, popup dismissal, and the complete composed
example in both appearances. These checks run in Windows and Ubuntu CI. They do
not replace native IME, screen reader, DPI, and visual checks on a desktop.
