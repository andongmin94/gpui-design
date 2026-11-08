# Navigation and overlays

The [two-style example](../crates/base/examples/two_styles.rs) combines the same
settings behavior with compact dark and spacious light presentation. Its
Session actions menu can open Dialog, toggle application values, or request a
preview reset through AlertDialog. A Collapsible holds the preview controls.
Preferences and Help tabs demonstrate active panels; Help contains an Accordion, and its
keyboard section opens a guide in a non-modal Popover. All styles live in the
example.

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

## Accordion

`AccordionState` owns a fixed, ordered collection of `AccordionItem::new(value,
label)` entries. Values must be unique. The initial state is fully collapsed;
`.default_open(value)` opens one enabled entry. An unknown or disabled default is
a configuration error and panics. Mark unavailable entries with `.disabled(true)`.

Render `Accordion::new(id, &state)` around the headers and panels, then pair each
`AccordionHeader::new(&state, index)` with its `AccordionPanel::new(&state, index)`
in collection order. Plain GPUI containers can arrange each pair. The root supplies
a focus scope and keyboard handling, without choosing a layout or appearance.

The header contains a native heading and a button. Its children and styles apply
to the button; use `.heading_level(level)` to change the heading level from 3.
Place only non-interactive content inside the header. The application draws the
indicator and may use `open_index()` to distinguish open and closed headers.

| Input on a header | Behavior |
| --- | --- |
| Click / Enter / Space | Open this panel, closing the previous one; collapse if already open |
| Up / Down | Focus the previous / next enabled header, wrapping at either end |
| Home / End | Focus the first / last enabled header |
| Tab / Shift-Tab | Follow normal document order through enabled headers and open content |

Arrow navigation does not expand panels. Every enabled header is a normal Tab
stop. Navigation actions only apply when a header itself has focus, so an Input
inside a panel retains its own Home/End and arrow behavior. Disabled headers do
not activate or participate in keyboard focus traversal.

State methods include `items`, `open_index`, `open_item`, `trigger_focus_handle`,
`panel_focus_handle`, `toggle(index, window, cx)`, and `close(window, cx)`. Subscribe
to `AccordionEvent::Change(Some(value))` when a section opens and `Change(None)`
when all sections close. A no-op does not emit another event. Observe the state
to redraw the consumer.

Closing a panel removes its layout, focus stops, and accessibility nodes. Keep
form values and child entities in the application view if they must survive
remounting. Panels are named regions without their own Tab stop by default; use
`.tab_stop(true)` for static content that should receive focus itself.

If a programmatic change closes the panel that owns keyboard focus, focus moves
to its surviving header, or the newly opened header when switching sections. A
change does not steal focus from an unrelated control. Moving focus before panel
removal also lets a nested Popover complete its dismissal lifecycle. If the entire
Accordion host is removed, the application must focus a surviving control.

The library emits heading level, button role, expanded/disabled states, and a named
Region. Explicit native button-to-panel controls/labelled-by relationships are not
emitted: the pinned GPUI API does not expose the required sibling-node IDs.

## Collapsible

Create `CollapsibleState::new(cx)` once for a single disclosure. It starts closed;
`.default_open(true)` opens it initially without emitting an event. Observe the
state for rendering, and keep child entities in the application view so their
values survive the content being removed and mounted again.

Render `Collapsible::new(id, &state)` around one
`CollapsibleTrigger::new(&state)` and one `CollapsibleContent::new(&state)`.
Keep the root and trigger mounted while closed. Supply an `accessibility_label`
on both the trigger and content, and application-owned styling and children.
The trigger composes Button and exposes expanded/disabled state. The open
content is a named Region; closed content has no layout, focus stops, or
accessibility nodes. Explicit native trigger-to-content relationships are not
emitted by the pinned GPUI API.

Click, Enter, and Space toggle the disclosure. Opening leaves focus on the
trigger, and normal Tab traversal enters its children. The component introduces
no heading or arrow/Home/End key handling. Use content `.tab_stop(true)` when
static content should itself receive focus.

State methods include `is_open`, `is_disabled`, `trigger_focus_handle`,
`content_focus_handle`, `set_open(open, window, cx)`, `toggle(window, cx)`, and
`set_disabled(disabled, window, cx)`. `CollapsibleEvent::Change(bool)` reports an
actual change; repeated requests emit no event. Disabling prevents user toggling
but preserves the current visibility and does not disable the content.
Programmatic `set_open` remains available while disabled.

Closing content that owns focus moves focus to its still-mounted trigger. If
that trigger is disabled, the existing non-Tab root scope receives focus so Tab
can leave. Focus outside the disclosure is preserved. As with Accordion, removing
the whole host requires the application to focus a surviving control first.

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

## DropdownMenu

The command menu reuses Popover for anchoring, open/close state, dismissal, and
focus restoration. It adds a command cursor and activation events. Create
`DropdownMenuState::new(items, cx)` with fixed, ordered
`DropdownMenuItem::new(value, label)` entries. Values must be unique; use
`.disabled(true)` for unavailable commands. Empty or entirely disabled collections
can open but cannot execute a command.

Render `DropdownMenu::new(id, &state)` with:

- `.trigger(|button| ...)` for the native trigger's accessible name and appearance.
- `.accessibility_label(...)` for the menu's accessible name.
- `.content(...)` containing one `DropdownMenuOption::new(&state, index)` per item,
  in collection order. Supply its text, spacing, colors, and other visual content.
- `.anchor(...)` and `.offset(...)` to configure the existing popup positioning.

Observe the state for rendering. Read `highlighted_index()` to style the active
command and `items()` for labels and disabled state. The menu retains no selected
value: commands represent actions and may execute repeatedly. Interactive form
controls belong outside the menu's command content.

| Input | Behavior |
| --- | --- |
| Trigger click / Enter / Space / Down | Open at the first enabled command |
| Trigger Up | Open at the last enabled command |
| Menu Up / Down | Move the cursor, wrapping and skipping disabled commands |
| Menu Home / End | Move to the first / last enabled command |
| One printable character | Cycle through enabled labels starting with that character, ignoring case |
| Option click / Menu Enter / Space | Execute the command once and close |
| Escape | Close and restore trigger focus |
| Tab / Shift-Tab | Close and leave through Popover's normal focus traversal |

Typeahead matches one character at a time; it has no buffered search or timer.
Ctrl, Alt, platform, and Fn shortcuts do not feed this search. Pointer hover moves
the same cursor used by keyboard navigation. Disabled commands remain visible
and carry disabled metadata, but are skipped by navigation and cannot execute.

Subscribe to `DropdownMenuEvent::Action(value)` to handle a command in the
application. The menu closes and restores its trigger before delivering the
event. A handler can therefore open Dialog, whose close operation later returns
to the menu trigger. The example uses `cx.subscribe_in` to receive the window
while handling commands.

State methods include `open`, `close`, `set_disabled`, `focus_handle`, and
`content_focus_handle`. `is_open(cx)` and `is_disabled(cx)` take an `&App` because
they read the composed Popover state. There is one owner for the popup lifecycle.
Options are not separate Tab stops; keyboard focus stays on the menu scope and
the highlighted option supplies its active descendant. A containing Dialog still
bounds Tab traversal, and external pointer presses retain their normal action.

Keep the menu host mounted while closed. The same host-removal focus contract as
Popover applies. This version provides plain commands; submenus, checked menu
items, and radio menu groups are not included.

Native metadata includes a Button trigger with expanded/disabled state, a Menu
scope, named MenuItem commands, and the active descendant. It does not emit a
native has-popup property or a trigger-to-menu controls relationship. These
contracts describe the implemented behavior, not a claim of full accessibility
pattern conformance or completed screen-reader validation.

## AlertDialog

`AlertDialog::new(&dialog_state)` composes the existing Dialog host for a short
decision. Use the same retained `DialogState`, `DialogBackdrop`, `DialogPopup`,
`DialogTitle`, `DialogDescription`, and `DialogClose` parts; there is no separate
alert state or duplicated modal lifecycle. Keep the host mounted while closed.

Give the host an `accessibility_label` and `accessibility_description`, and
provide its backdrop and popup. The wrapper fixes the native role to AlertDialog
and prevents backdrop presses from dismissing it. Escape still cancels through
Dialog's existing close path. Tab containment and return focus also belong to
Dialog and Root.

Create a stable focus handle for the cancellation button and pass it to
`DialogState::new(cx).with_initial_focus(&cancel_focus)`. Apply that same handle
to the Button supplied by `DialogClose::trigger`. The confirmation is an ordinary
Button whose application callback performs the action and closes the state.
Closing the dialog by itself never implies confirmation; do not attach the
confirmation action to a generic `DialogEvent::Closed` notification.

The example's Reset preview menu command opens this host. Cancel initially owns
focus and preserves the current preview. Only the explicit Reset preview button
restores 40% known progress, then returns focus to the menu trigger.

## Validation

The existing GPUI test framework checks selected and disabled metadata, orientation,
focus traversal, inactive panels, popup dismissal, command dispatch, and the
representative composed flows across both appearances. Composition tests cover a command
opening Dialog, header navigation without expansion, collapse of a panel with
an open Popover, disclosure mounting, and explicit preview reset confirmation.
The full suite runs once on Linux; composition checks and native
example builds also run on Windows and macOS. They do not replace
native IME, screen reader, DPI, and visual checks on a desktop.
