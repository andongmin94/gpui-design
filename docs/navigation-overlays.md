# Navigation and overlays

The [two-style example](../crates/base/examples/two_styles.rs) combines the same
settings behavior with compact dark and spacious light presentation. Its
Session actions menu and the summary's ContextMenu can open Dialog, toggle
application values, or request a preview reset through AlertDialog. HoverCard
previews the current preset, date, and notes status while the preset button keeps
its ordinary settings action. A Collapsible holds the preview controls.
Search commands opens a Command query and result list inside Dialog; both menus
dispatch the same application actions. Separator provides semantic division
between content regions, and the settings form includes retained multiline notes.
Compact uses Preferences, Help, Schedule, and Presets tabs with active panels.
Settings uses the [Sidebar recipe](presentation-recipes.md#sidebar) and a named
Region for the current section. Help contains an Accordion, and its keyboard
section opens a guide in a non-modal Popover. Breadcrumb shows that guide's
location, while the application-owned
[Kbd recipe](presentation-recipes.md#kbd) displays the existing keyboard controls.
All styles live in the example.

Call `gpui_design::init(cx)` and host the window in `gpui_design::Root`. Keep the
state entities and their observation subscriptions in your view. The example
shows the complete setup and application-owned styling.

## Breadcrumb

The family supplies three structural elements, each with ordinary `Styled` and
`ParentElement` support:

| Part | Native contract | Application responsibility |
| --- | --- | --- |
| `Breadcrumb::new(id)` | Navigation landmark, named `Breadcrumb` by default | Use `.accessibility_label(...)` to describe or localize the path |
| `BreadcrumbList::new(id)` | List in supplied child order | Render items in path order and choose direction, wrapping, and spacing |
| `BreadcrumbItem::new(id)` | ListItem; `.current(true)` adds native `aria_current(Page)` | Mark the current item and supply each location's text or action |

Compose ordinary Button controls for ancestors and native text for the current
location. The structure creates no focus stop, selected value, navigation event,
or router. The supplied Buttons retain their usual pointer, Enter/Space, and Tab
behavior. Each application callback chooses the destination and any focus change.

```rust,ignore
Breadcrumb::new("keyboard-guide-breadcrumb")
    .accessibility_label("Keyboard guide location")
    .child(
        BreadcrumbList::new("keyboard-guide-path")
            .flex()
            .items_center()
            .gap_1()
            .child(BreadcrumbItem::new("settings-location").child(settings_button))
            .child(BreadcrumbItem::new("help-location").child(help_button))
            .child(
                BreadcrumbItem::new("guide-location")
                    .current(true)
                    .child(gpui::text!(id = "keyboard-guide-current-text", "Keyboard guide")),
            ),
    )
```

`gpui::text!` provides native text semantics for the noninteractive current
location. Plain `&str` or `SharedString` children draw text without their own
native role; the path uses raw text for its decorative `›` separators.
The ancestor Buttons already provide their accessible names. Separators, path
contents, colors, typography, and layout remain application-owned.

The example renders `Session settings > Help > Keyboard guide` inside the
existing Help Popover. Session settings first closes the Popover, calls
`TabsState::select_value("preferences", window, cx)`, then focuses the existing
navigation handle at index zero. Explicit selection works for both Compact's
TabList and Settings' Sidebar, whose focus alone does not select a section.
Help closes the Popover and returns focus to its trigger while
keeping Help selected. Keyboard guide is the current, noninteractive final item.
These callbacks use the existing Popover and Tabs state instead of a second
navigation model.

Set `.current(true)` only on the current path item. The default false value
omits current-page metadata; it does not disable or change the behavior of
supplied children. The native property is AccessKit's `AriaCurrent::Page` on the
ListItem itself. That contract does not establish how a particular desktop
screen reader announces the path.

## Direction

Wrap a subtree with `DirectionProvider::new(Direction::Rtl)` or `Direction::Ltr`.
Descendant controls resolve the nearest direction while rendering. Nested
providers restore the enclosing direction for following siblings, and a control's
explicit `.direction(...)` overrides inheritance. `Direction::current(cx)` is
available while constructing descendants inside the scope.

Direction controls logical horizontal navigation for Tabs, RadioGroup,
ToggleGroup, Slider, Carousel, Chart, Menubar and NavigationMenu. Slider also
mirrors horizontal pointer coordinates. Up/Down and logical Home/End retain
their normal meaning. Supply matching application layout with
`direction.row(div())` or `direction.row(TabList::new(...))`; the provider does
not rewrite arbitrary GPUI flex containers. It aligns text to logical start,
while GPUI still owns shaping, bidirectional editing and platform IME behavior.

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

`select_value(value, window, cx)` selects an enabled item without moving keyboard
focus. It returns `true` and emits one `TabsEvent::Change` only when selection
changes; unknown, disabled, and already-selected values return `false` without
an event. Observe the state for the associated redraw. If selecting another
item removes the currently focused content, the caller must move focus to a
retained navigation entry or another valid target. This method does not choose
that target or turn an ordinary navigation Button into a Tab.

TabList activation is automatic: moving focus to an enabled tab also selects it.
This includes pointer, keyboard, and programmatic focus. Each list has one Tab
stop, at its selected tab.

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

The Sidebar recipe reuses the fixed items, selected value, and stable focus
handles from a dedicated `TabsState`, with explicit Button activation instead
of TabList's automatic selection. Create a fresh state for that use: TabList
installs retained focus subscriptions, so a state previously bound to TabList
must not be shared with or retargeted to Sidebar. Resizing the Settings example
only rearranges its Sidebar and content; it never swaps them for TabList.

`Button::current(true)` marks the selected navigation Button with native
`AriaCurrent::Page`. Its default false value omits that property. The method
leaves styling and focus unchanged; `Button::selected(...)` and the application's
destination handler control those concerns separately. Sidebar keeps native
Navigation/Button/Region semantics rather than the TabList/Tab/TabPanel roles.

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

DropdownMenu and ContextMenu share `MenuState`, `MenuItem`, `MenuEvent`, and
`MenuOption`. Their shared command behavior reuses Popover for open/close state,
dismissal, traversal, and focus restoration. Create
`MenuState::new(items, cx)` with fixed, ordered
`MenuItem::new(value, label)` entries. Values must be unique; use
`.disabled(true)` for unavailable commands. Empty or entirely disabled collections
can open but cannot execute a command.

Render `DropdownMenu::new(id, &state)` with:

- `.trigger(|button| ...)` for the native trigger's accessible name and appearance.
- `.accessibility_label(...)` for the menu's accessible name.
- `.content(...)` containing one `MenuOption::new(&state, index)` per item,
  in collection order. Supply its text, spacing, colors, and other visual content.
- `.anchor(...)` and `.offset(...)` to configure the existing popup positioning.

Observe the state for rendering. Read `highlighted_index()` to style the active
command and `items()` for labels and disabled state. The menu retains no selected
value: commands represent actions and may execute repeatedly. Interactive form
controls belong outside the menu's command content.

### Checked commands

Use `MenuOption::checked(bool)` to render an application-controlled checkbox
command. Omitting the builder keeps a plain `MenuItem`; `.checked(false)` exposes
an unchecked `MenuItemCheckBox`, while `.checked(true)` exposes a checked one.
The application supplies the visible indicator and checked value on each render.

```rust
MenuOption::new(&menu, index)
    .checked(show_cursor)
    .child("Show cursor")
```

Checked commands use the same navigation, disabled behavior, and
`MenuEvent::Action(value)` as plain commands. Activation closes the menu and
restores focus before delivering the event. The application handles that action
and renders its next checked value; MenuState neither stores nor automatically
toggles the checked value. Changing the supplied value does not dispatch an action.
The example's Show cursor and Keep toolbar visible commands share the retained
application booleans used by its Checkbox and Switch. Both DropdownMenu and
ContextMenu render those values through the same menu-content composition.

### Shared interaction and lifecycle

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

Subscribe to `MenuEvent::Action(value)` to handle a command in the
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
Popover applies. `check_state(CheckboxState::Indeterminate)` supplies a mixed
checkbox command; `radio_checked(bool)` supplies MenuItemRadio semantics. The
application owns mutually exclusive radio values, grouping, and their rendering.

Native metadata includes a Button trigger with expanded/disabled state, a Menu
scope, named plain/checkbox/radio commands with their supplied checked state,
submenu expanded state, and the active descendant. It does not emit a
native has-popup property or a trigger-to-menu controls relationship. These
contracts describe the implemented behavior, not a claim of full accessibility
pattern conformance or completed screen-reader validation.

### Submenus

Create child `MenuState` entities before their parent. A parent item declares
`.submenu(&child)`; render that item using `MenuSubmenu::new(&parent, index)` with
`.trigger(|option| ...)` and `.content(...)` containing the child's options.
The submenu host owns popup positioning and reuses the child's existing Popover
lifecycle. Menu trees must be acyclic, and a child belongs to one parent.

Native focus on a submenu row synchronizes the parent menu's highlighted item.
The expansion arrow opens the highlighted submenu; the opposite arrow or Escape
closes one submenu and restores the parent menu scope. Pointer hover opens the
corresponding child and closes sibling branches. Plain leaf activation closes
the whole tree and emits exactly one `MenuEvent::Action` on the root state.
Tab/Shift-Tab leave the complete menu tree through the root Popover traversal.
The application subscribes to the root and owns leaf actions and checked/radio
values. Keep all hosts in the composition while closed.

## Menubar

Build `MenubarState` from ordered `MenubarItem` values referring to existing
`MenuState` entities. Render `Menubar` with one `MenubarMenu` per item. Each menu
entry supplies its trigger and content while retaining the shared Menu command
contract, including nested submenus and controlled check/radio values.

Only one top-level item is a Tab stop. Left/Right move in logical order and
Home/End reach the ends, skipping disabled entries. Down/Up open the first/last
enabled command. While a menu is open, arrows and pointer travel can switch to
another top-level menu; only one branch remains open. Escape returns to the
corresponding entry. The catalogue Navigation view demonstrates these transitions
and routes leaf commands to retained application state.

## NavigationMenu

`NavigationMenuState` retains an ordered set of `NavigationMenuItem` values.
A plain item is a destination; `.with_popup()` declares a disclosure. Render a
named `NavigationMenu` with one `NavigationMenuEntry` per item. Popup contents
are ordinary groups, not command menus or dialogs. Supply existing Links or
Buttons and optional initial focus. `NavigationMenuLink` emits a destination
through `NavigationMenuEvent::Navigate`; the application owns routing.

The top row uses roving focus and logical Left/Right/Home/End keys. Activation
or Down opens a disclosure. When one is already open, pointer travel switches
between popup entries; mere hover over an otherwise closed bar does not open
unrequested content. Popup controls retain normal Tab traversal and Popover's
Escape/outside-dismissal/return-focus behavior. Destination links can mark the
current page independently of the open disclosure.

## ContextMenu

Use the same `MenuState::new(items, cx)`, `MenuItem`, `MenuOption`, and
`MenuEvent::Action(value)` contract for commands associated with a visible region.
Create a distinct state entity for each mounted menu host. The example's
DropdownMenu and ContextMenu share item definitions and application callbacks,
but keep their open state, cursor, and return-focus target separate.

```rust
ContextMenu::new("session-summary", &menu)
    .target_label("Session summary")
    .accessibility_label("Session actions")
    .target(summary_content)
    .content(menu_surface)
    .p_2()
    .border_1()
    .focus_visible(|style| style.border_color(accent))
```

The target is a focusable, named region, not an action button. Supply
noninteractive target content with `.target(...)` and the same ordered
`MenuOption::new(&menu, index)` elements used by DropdownMenu inside `.content`.
`target_label` names the region; `accessibility_label` names the menu. Styles and
interactive style builders apply to the target, while the application styles its
menu surface separately.

| Input on the target | Behavior |
| --- | --- |
| Left click | Focus the region without opening the menu |
| Right click | Open at the pointer's window coordinate; another request repositions the open menu |
| Shift-F10 / Menu key | Open a closed menu below the target |
| Native ShowContextMenu accessibility action | Open below the target |
| Menu navigation and activation | Use the same enabled-item cursor, typeahead, and action-after-close behavior as DropdownMenu |
| Escape | Close and return focus to the target |
| Tab / Shift-Tab | Close through Popover's ordinary target-relative traversal |

The Menu key is available when the platform delivers it. Imperative
`MenuState::open(window, cx)` also uses target-relative placement. Pointer
placement uses the existing Popup/Positioner measurement and viewport clamping;
it does not replace their geometry engine. Outside clicks and parent Dialog
containment use the same Popover lifecycle. A new context request closes any
existing submenu branch and focuses the root menu at its first enabled item.

`set_disabled(true, window, cx)` closes the menu and blocks target entry and
activation. An already focused disabled target retains its binding so Tab can
leave. If removing the whole target, move focus to a surviving control as part of
that application update. Retained popup state is dismissed on detachment.

Native metadata includes a named target, expanded/disabled state, the Shift-F10
shortcut and ShowContextMenu action, plus the shared Menu, MenuItem, and
MenuItemCheckBox/MenuItemRadio and submenu semantics. The same command tree can
be supplied to DropdownMenu or ContextMenu. Interactive descendant targets and
touch long-press are not included.

## Command

Create `CommandState::new(items, window, cx)` from fixed, ordered
`SearchItem::new(value, label)` entries with unique values. Optional `.keywords`
add search aliases; `.disabled(true)` makes an action unavailable. Observe the
state for result rendering and subscribe to `CommandEvent::Action(value)` to
handle execution in the application.

`Command::new(id, &state)` hosts `CommandInput::new(id, &state)` and
`CommandList::new(id, &state)`. Give the query and list accessible labels, and
render `CommandOption::new(&state, index)` as direct list children for
`visible_indices()`, in that order. Read `items()` for labels and
`highlighted_index()` for application-owned active-row styling. The list uses
the existing GPUI ScrollHandle; give it a height or maximum height so the active
result can scroll into view. Render an application-owned empty message when no
items match.

The existing InputState is the query's source of truth, exposed through `input()`.
`set_query(text, window, cx)` updates it programmatically. Each whitespace-separated
query term must appear, ignoring case, in the item's value, label, or keywords.
Matching retains item order and includes disabled matches for display. A changed
query highlights its first enabled result; empty or all-disabled results have no
active command. There is no asynchronous source, fuzzy ranking, or grouped list.

| Input | Behavior |
| --- | --- |
| Up / Down | Move among enabled matching results, wrapping at either end |
| Enter | Execute the highlighted enabled result once |
| Pointer hover / click | Highlight an enabled result / execute that result |
| Space, Home, End, Left, Right | Remain ordinary query editing |
| Tab / Shift-Tab | Follow normal container focus traversal; result rows are not Tab stops |
| Escape | Remains available to the surrounding surface |

Native focus stays in the query. While an IME has marked composition text,
command navigation and execution leave that interaction to the input. Automated
marked-text checks do not replace desktop Korean/CJK IME validation.

The native tree exposes a named text input, listbox, and selected/disabled options.
The pinned GPUI active-descendant API requires a focused ancestor; a sibling
query/list cannot truthfully emit that relationship, so this implementation does
not claim one or completed screen-reader validation.

Command itself owns no popup lifecycle. To make a command palette, place it
inside the existing Dialog and set the DialogState's initial focus to
`focus_handle()`. The example closes that dialog before executing an action that
may open another Dialog, preserving the menu trigger as the return target.
Opening the palette clears its query in application code.

SearchItem and the search implementation are also used by the editable
[Combobox](form-controls.md#combobox). Combobox supplies its own committed value
and popup lifecycle while Command remains an action list.

## Tooltip

Retain `TooltipState::new(&trigger_focus, window, cx)` alongside the existing
Button focus handle. Render `Tooltip::new(id, &state, description)` and configure
the Button through `.trigger(|button| ...)`. This preserves the application's
activation handler and focus identity. Ordinary Styled methods on Tooltip
configure the noninteractive text surface; the trigger builder owns button
appearance. No additional init call or keybinding is needed.

Keyboard focus shows the text immediately; pointer hover waits 500 ms. Leaving
cancels pending hover. Blur, activation, window deactivation, and Escape dismiss
it without moving focus. Escape suppresses reopening until another focus or
pointer entry. Cancelling a pending, undisplayed hint leaves Escape available to
an enclosing surface. Enter and Space still activate the Button once. Unmounting
the trigger cancels pending hover work and resets retained visibility.

The trigger always exposes the description on its native node. Displayed text
has Tooltip role and no Tab stop. GPUI's native tooltip layer owns placement,
window-edge clamping, and drawing; this component adds the focus/hover lifecycle.
Its mounted Escape subscription belongs to that window and does not issue a
global clear operation that could dismiss another tooltip.

The current API supports plain text attached to a Button. Rich interactive
content, touch long-press behavior, and a configurable provider are not part of
this contract. The example wraps its existing Edit session button, so the same
button opens the settings Dialog and remains its return-focus target.

## HoverCard

HoverCard supplies a rich, noninteractive preview of a Button's ordinary action.
Retain a stable trigger focus handle and create
`HoverCardState::new(&trigger_focus, window, cx)`. Observe the state to redraw the
consumer. Optional `.with_delays(open_duration, close_duration)` configures pointer
entry and exit delays before mounting; defaults are 500 ms and 200 ms.

```rust
HoverCard::new("preset-preview", &preview)
    .accessibility_label("Recording preset preview")
    .offset(px(6.))
    .trigger(|button| {
        button
            .accessibility_label("Recording preset. Edit settings")
            .on_click(open_settings)
            .child("Preset: Balanced")
    })
    .content(preview_surface)
```

The trigger builder preserves the Button's action and supplies its appearance.
The state owns its focus handle and disabled state; use `set_disabled` to
coordinate the preview and Button. `Styled` methods on HoverCard apply to the
host frame. Style the application-supplied content separately, and use
`.anchor(...)` and `.offset(...)` for the existing Popup geometry.

| Input | Behavior |
| --- | --- |
| Pointer enters the trigger | Open after the entry delay without moving focus |
| Pointer leaves trigger and card | Close after the exit delay, unless the trigger retains keyboard focus |
| Pointer enters the card or returns to the trigger | Cancel delayed closing |
| Keyboard focus enters the trigger | Open immediately; focus remains on the Button |
| Escape | Dismiss a visible preview and suppress reopening during the current entry |
| Click / Enter / Space | Dismiss the preview; the Button or clicked control still performs its ordinary action |
| Focus changes / window deactivates | Dismiss without moving focus |

The close delay lets a pointer cross the trigger-to-card gap. Keyboard-opened
previews remain until blur or explicit dismissal, even if the pointer moves away.
Escape leaves focus in place and does not reopen the card merely because another
frame is drawn or the pointer moves within the same trigger. A fresh trigger
entry or keyboard focus can show it again. Cancelling a pending, undisplayed
preview leaves Escape available to an enclosing surface.

State methods are `is_open`, `is_disabled`, `focus_handle`,
`close(window, cx)`, and `set_disabled(disabled, window, cx)`. Explicit close
suppresses the current entry. Re-enabling waits for another entry or focus.
Unmounting cancels delayed work and clears retained visibility; a mounted
Escape subscription belongs only to that window.

Popup owns placement, viewport clamping, deferred painting, and occlusion. The
visible preview is a supplementary Tooltip surface with no Tab stop; it never
opens a focus trap. `accessibility_label` names that surface. Keep essential
information available in the Button's accessible description or its ordinary
destination, rather than only in a pointer preview. Interactive content belongs
in Popover. Touch long-press and a global hover provider are not supplied.

The example previews the current recording preset, recording date, and notes
status. Clicking or pressing Enter on that same Button opens the settings Dialog,
which returns focus to the Button when closed.

## Separator

`Separator::new(id)` supplies a semantic horizontal separator.
`.axis(Axis::Vertical)` changes its native orientation; `.decorative(true)` omits
its accessibility node for purely visual division. The native semantic role is
AccessKit's Splitter without focus or activation actions. This component is a
static separator; Resizable pane-divider interaction is not implied.

Use ordinary GPUI styles for its dimensions and color, such as
`.h(px(1.)).w_full().bg(border)` for a horizontal line. Axis does not impose a
visual layout. Separators have no children or Tab stop.

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

## Drawer

`DrawerState` composes one `DialogState`. Construct it with `new(window, cx)` or
adopt a configured Dialog through `with_dialog`; mount that state in one Drawer.
`open`/`close`, initial focus, containment, Escape, backdrop dismissal and return
focus remain owned by Dialog. The state forwards `DialogEvent` and exposes
`dialog()` for ordinary DialogClose controls.

`Drawer::new(id, &state)` places a viewport-limited modal surface at the bottom.
Supply its `.handle(...)`, optional `.backdrop(...)` and styled body/actions.
Only a press on the dedicated handle starts a downward drag; scrolling or editing
in the body does not. Releasing after the displacement threshold closes through
Dialog; a shorter or cancelled drag returns the surface to its resting position.
Window deactivation cancels an active drag. This contract has one resting point,
with no inertial physics or snap-point configuration.

## Toast

`ToastState::new(window, cx)` owns a five-second notification lifetime; set a
nonzero `.with_duration(...)` before mounting if needed. Keep the matching
`Toast::new(id, &state)` in the view even while closed. Set its accessible message,
body and ordinary action Buttons; `ToastClose` provides the dismiss action.
Applications own stacking and content, rather than a second queue service.

`show` starts or replaces the message without taking focus. The mounted countdown
pauses on hover, contained focus and window inactivity, then resumes from its
remaining duration. `dismiss`, Escape and the close button are explicit dismissals;
timeout is reported separately by `ToastEvent`. If an explicit dismissal removes
the focused notification, its captured prior focus is restored. Unfocused
dismissal preserves whatever control the user is using. Removing the entire host
closes it without restoring captured focus; the application must move focus to a
surviving control when removing focused content. Native live metadata is
polite by default and assertive for `.urgent(true)`; actual screen-reader delivery
must still be checked on each platform.

## Validation

The existing GPUI test framework checks selected and disabled metadata, orientation,
focus traversal, inactive panels, popup dismissal, command dispatch, and the
representative composed flows across both appearances and the catalogue. Composition tests cover a command
opening Dialog, header navigation without expansion, collapse of a panel with
an open Popover, disclosure mounting, and explicit preview reset confirmation.
The consumer flows cover command search into the multiline settings
form, normal and disabled-field Tab traversal, and retained notes.
They also cover the tooltip trigger, Field label/feedback, and searchable preset
selection inside Dialog, including query cancellation and retained selection.
The existing flows also exercise keyboard and pointer context-menu commands
opening that Dialog, and a HoverCard trigger retaining its ordinary action and
return-focus target. Checked commands use those same flows to verify current
application values across both menu hosts and unchanged action/focus behavior.
Primitive tests cover point placement and pointer
preview timing at their owning layer.
Breadcrumb's validation scope is its native navigation/list/item roles,
current-page metadata, and the composed ancestor actions in the existing Help
flow. That flow also checks that static key hints leave focus with the existing
controls. These contracts do not establish native screen-reader announcements
or the visual appearance of the path and keycaps.
The full suite runs once on Linux, including isolated recipe checks. Windows and
macOS run the real consumer composition flows with recipe-only tests filtered
out, and all three platforms build the native example. They do not replace
native IME, screen reader, DPI, and visual checks on a desktop.
