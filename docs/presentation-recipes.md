# Presentation recipes

These 23 catalogue recipes are reusable application source, used by the native
[two-style example](../crates/base/examples/two_styles.rs) and
[catalogue example](../crates/base/examples/catalogue.rs). Chart also has a
supporting presentation recipe for its public behavior family. They compose GPUI
layout with the existing controls and keep colors, spacing, and sizing in the
consumer's `Palette` or application-supplied `Styled` values.

## Copy into an application

1. Copy the desired files from
   [`crates/base/examples/recipes`](../crates/base/examples/recipes) into an
   application module, preserving the repository's license terms.
2. Declare the copied modules and re-export the types, following the small
   [`mod.rs`](../crates/base/examples/recipes/mod.rs). Copy only the modules needed
   by the application and list only those modules.
3. Copy [`presentation.rs`](../crates/base/examples/presentation.rs) beside the
   application's `recipes` directory. It contains the actual `Palette` and both
   appearances used by the tested example, without the Demo or its behavior.
   Declare it and expose its tokens in the recipes' parent module:

   ```rust,ignore
   mod presentation;
   pub(crate) use presentation::{Appearance, Palette};
   mod recipes;
   ```

   Choose `Appearance::Settings.palette()` or `Appearance::Compact.palette()` in
   the application view. Recipe imports such as `use super::super::Palette`
   work directly. Change the copied token values to fit the application, or replace Palette with an
   existing token type. AspectRatio takes no palette; its consumer supplies the
   visual style. Bubble and Chart currently keep their own copyable styling.
4. Import the types from that application module. They use `pub(crate)` visibility
   and are not exported from `gpui_design`. No extra dependency is required.

The source in this repository is the distribution mechanism. The palette module
and recipes are application source, outside the library exports. There is no
recipe registry, installer, or global theme provider for these visual tokens.
The existing crate continues to own interactive behavior.

Some recipes compose other recipe modules. Copy those modules and their re-exports
alongside the desired recipe:

| Recipe | Other recipe modules required |
| --- | --- |
| Pagination | ButtonGroup |
| DataTable | InputGroup, EmptyState, Pagination, and ButtonGroup |
| Message | Typography (`Paragraph`) |
| Trace | Typography (`Paragraph`) |

## Card

[`card.rs`](../crates/base/examples/recipes/card.rs) supplies a named region with a
level-two heading. The description accepts text; the header action, content,
and footer each accept one element, so the application can compose several
children in a `div()`.

```rust,ignore
Card::new("session-card", "Session", palette)
    .description("Recording preferences")
    .header_action(Badge::new("toolbar-status", "Toolbar pinned", palette).accent(true))
    .content(session_summary)
    .footer(session_controls)
    .w_full()
```

The card adds no focus stop. Controls placed in its slots retain their own
keyboard and pointer behavior. Its `Styled` implementation allows frame overrides;
change the copied source to fit different heading levels or header/footer layouts.

## Badge

[`badge.rs`](../crates/base/examples/recipes/badge.rs) is a static named label.
The default appearance is secondary; `.accent(true)` uses the palette's accent
colors. The example displays actual cursor and toolbar settings with these two
appearances.

```rust,ignore
Badge::new("cursor-status", "Cursor visible", palette).accent(true)
```

Badge adds no Tab stop, button action, pressed state, or live announcement. Its
text must explain the state independently of color. Interactive chips should use
an existing Button or Toggle with application styling.

## Kbd

[`kbd.rs`](../crates/base/examples/recipes/kbd.rs) supplies static individual
keycaps and combinations of keys. Each Kbd uses its visible text as the default
native Label name. Give symbolic keys a readable name explicitly:

```rust,ignore
Kbd::new("guide-up-key", "↑", palette)
    .accessibility_label("Arrow up");

KbdGroup::new("context-menu-shortcut", palette)
    .key(Kbd::new("context-menu-shift", "Shift", palette))
    .key(Kbd::new("context-menu-f10", "F10", palette));
```

`KbdGroup` preserves the supplied order and inserts `+` between keycaps. Its
native Group has no repeated name; each key retains its own readable Label.
Use a group for keys pressed together, such as Shift+Tab. Arrange independent
Kbd elements in ordinary application layout when showing alternatives, such as
Enter or Space, or the arrow keys.

Both types implement `Styled`. The palette supplies the keycap's field color,
border, corner radius, foreground, and font size, plus muted text for the group
separator. Applications own key labels and any platform-specific glyph choice.
The recipe has no focus stop, action, keybinding registration, or shortcut
dispatch; it displays the bindings already provided by the associated control.

The example's Help → Keyboard guide shows Tab, Shift+Tab, arrows, Enter/Space,
and Escape alongside descriptions of their existing behavior. The summary
displays Shift+F10 for its existing ContextMenu. Replacing plain hint text with
these keycaps does not add keyboard commands or another focus target.

## Empty

[`empty.rs`](../crates/base/examples/recipes/empty.rs) exports `EmptyState` to
avoid confusion with GPUI's `Empty` element. It supplies a named group, level-two
heading, description, and optional media/action slots.

```rust,ignore
EmptyState::new("empty-results", "No results", "Try another search.", palette)
    .media(Badge::new("result-count", "0 results", palette))
    .action(clear_search_button)
```

The recipe does not own search, loading, notification, or focus state. Supplied
media owns its accessible description when it conveys information. The example
places this recipe beside `CommandList`, inside `Command`, so its action is an
ordinary Button outside the listbox. Clear search calls `CommandState::set_query`
and returns focus to the existing query input. The dialog remains open and no
command is dispatched. `Styled` can override the outer frame.

## Button Group

[`button_group.rs`](../crates/base/examples/recipes/button_group.rs) gives related
Buttons one frame and decorative separators. It accepts existing `Button`
instances, applies connected styling, and preserves their identity, actions,
disabled state, and individual Tab stops.

```rust,ignore
ButtonGroup::new("calendar-month", "Calendar month", palette)
    .axis(Axis::Horizontal)
    .button(previous_month_button)
    .text("November 2026")
    .button(next_month_button)
    .w_full()
```

Horizontal is the default; `Axis::Vertical` stacks the same parts. `text` is a
static label with no focus stop. `Styled` allows frame and size overrides.
Focus uses an accent fill because the group owns the outer border. The group
adds no arrow-key navigation or pressed-value state; `ToggleGroup` remains the
control for multiple pressed values with roving focus.

## Input Group

[`input_group.rs`](../crates/base/examples/recipes/input_group.rs) supplies one
frame around an existing control and optional leading/trailing elements. The
constructor takes the control itself, so its native input renderer, value,
selection, and input interface stay with the existing editor.

```rust,ignore
let control = InputBase::new("session-name")
    .role(None::<Role>)
    .semantic_owner::<Input>(input.entity_id())
    .h(palette.control_height)
    .w_full()
    .child(Input::new(&input));

InputGroup::new("session-name-group", control, palette)
    .focused(input.focus_handle(cx).is_focused(window))
    .invalid(name_invalid)
    .trailing(default_name_button)
```

Place the group inside an existing `Field` to preserve the input's label, help,
required state, and error metadata. Keep the native editor directly below its
`InputBase` semantic owner; the group surrounds that complete control. Remove the
control's old outer border and padding when the group supplies them. `CommandInput`
can likewise be supplied directly, preserving its search and input semantics.

Horizontal is the default. `.leading(element)` and `.trailing(element)` appear
before and after the control. `.axis(Axis::Vertical)` places them above and below
it; the example uses a trailing keyboard hint below the existing Textarea.
Each slot accepts one element, which can be an application-owned container.
Traversal follows the supplied leading/control/trailing order; put action buttons
in the trailing slot when the input should precede them in the Tab sequence.

The frame adds no Tab stop or click-to-focus handler. Supplied buttons retain
their focus and callbacks. `focused` and `invalid` style the border only, with
invalid taking precedence; native invalid metadata remains the Field's job.
The application chooses which addons are enabled. In the example, Use default
appears only for a missing session name and is disabled when the input is not
editable. Its existing input state receives `MiniCast` and regains focus when the
action runs. The recipe does not create a second text value or infer mutation
permissions from its child elements. `Styled` can override the frame.

## Item

[`item.rs`](../crates/base/examples/recipes/item.rs) groups a title, optional
description, and optional trailing action in a content row.

```rust,ignore
Item::new("recording-preset-item", "Recording preset", palette)
    .description(selected_preset_label)
    .action(preset_preview_and_edit)
```

Its native Group is named by the title. The text column can shrink while the
action keeps its required width. The example supplies the existing HoverCard
trigger, which previews the preset on hover/focus and opens settings on ordinary
activation. Its focus handle, button name, and return-focus behavior are retained.

The row has no whole-row click, Tab stop, selected value, or list navigation.
Supplied controls own their semantics and actions. Use Field for associated input
labels and validation. Media, header/footer slots, and a separate ItemGroup are
outside the current recipe's scope. `Styled` can change the frame and spacing.

## Alert

[`alert.rs`](../crates/base/examples/recipes/alert.rs) presents an inline title and
description. Ordinary content uses the native `Note` role; `.urgent(true)` selects
the native `Alert` role. `.destructive(true)` changes colors independently of that
urgency decision.

```rust,ignore
Alert::new(
    "session-name-alert",
    "Session name is missing",
    "Open Edit session and enter a name or use the default name.",
    palette,
)
.destructive(true)
.urgent(true)
```

The Schedule panel uses the default informational form. Closing settings with a
missing name shows the urgent form on the summary. Reopening settings removes it
while Field supplies the input's validation details, avoiding two visible copies
of the same error. Once the name is restored, the summary warning is absent.

Alert does not move focus, open a modal, own dismissal state, or schedule repeated
announcements. The native accessibility adapter determines how its role is
presented; exposing that role is not evidence of native screen-reader delivery.
Use AlertDialog for a modal decision that requires explicit confirmation. The
current inline recipe has no action or media slot; `Styled` controls its frame.

## Pagination

[`pagination.rs`](../crates/base/examples/recipes/pagination.rs) composes existing
Button and ButtonGroup controls into named native navigation. The application
supplies the zero-based page index, page count, and callback.

```rust,ignore
Pagination::new("preset-pages", "Recording preset pages", page, page_count, palette)
    .on_change(move |page, _, cx| {
        table_state.update(cx, |state, cx| state.set_page(page, cx));
    })
```

For a nonempty collection, the page index must be less than the count. Empty
results use index zero and count zero, display `No pages`, and disable both
buttons. Otherwise the text is `Page N of M`, Previous is disabled on the first
page, and Next is disabled on the last. The buttons preserve ordinary Tab and
activation behavior, including focus on a button that becomes disabled after its
action. The recipe requests a page change; it owns no retained page or data.
`Styled` controls the frame. Numbered links, ellipsis, routing, and fetching are
outside this recipe's scope.

## Data Table

[`data_table.rs`](../crates/base/examples/recipes/data_table.rs) provides a working
client-side text-table composition. It uses the existing native Input/Field,
Table, ScrollArea, and Button, plus the InputGroup, EmptyState, and Pagination
recipes. Columns and immutable rows belong to the consuming application.

```rust,ignore
let state = cx.new(|cx| {
    DataTableState::new(
        [
            DataTableColumn::new("Preset", px(180.)),
            DataTableColumn::new("Description", px(280.)),
        ],
        rows, // DataTableRow::new(stable_id, [name, description]).disabled(false)
        2,    // Rows per page; must be positive.
        window,
        cx,
    )
});

DataTable::new("preset-catalogue", &state, palette)
    .accessibility_label("Recording presets")
    .action_label("Use preset")
    .selected_value(current_preset)
    .on_action(move |id, window, cx| {
        preset.update(cx, |state, cx| {
            state.set_selected_value(Some(id.as_ref()), window, cx);
        });
    })
```

Keep the state entity in the application and observe it to redraw the parent.
Apply the application's `InputEditorStyle` to `query_input()`, as the two-style
example does. A row has a unique stable ID and one text value per column. The
initial display order is the supplied row order. Header Buttons toggle one column
between ascending and descending case-insensitive text order; equal keys retain
their original order.

The native input owns the query. Every whitespace-separated term must appear in
the combined cell text, ignoring case. Matching/sorted indices are derived when
the query or sort changes. Both changes return to the first page; moving focus
does not reset it. `visible_rows()`, `filtered_count()`, `page_index()`, and
`page_count()` expose the current view. `set_page` clamps to the available range,
and page/filter/sort changes return the row viewport to its vertical start.

The example derives its four rows from the same preset definitions used by the
Combobox. `.selected_value` supplies that Combobox's current value each render;
the table keeps no second selected value. A row action passes the stable ID to
the application even after filtering, sorting, or paging. Current and unavailable
rows have disabled action Buttons. The selected value remains in the application
when its row is filtered out or appears on another page.

An empty result replaces the row viewport with EmptyState and Clear filter.
Clearing restores the rows and query focus without applying a row action. With
results, focus follows query → sort Buttons → enabled row actions → enabled page
Buttons. Horizontal and vertical ScrollArea scrolling reveals focused controls;
there is no grid-style arrow navigation. Query, sort, and page survive unmounting
the view while its state entity is retained.

Native Table indices describe the displayed page. Header Button names announce
the current sorting description and next action; native sort-direction metadata
is not supplied by the pinned Table/GPUI API. The columns are fixed-width text,
with one application action column. Numeric/date ordering, custom cell renderers,
multi-column sorting, multiple row selection, column hiding/reordering, mutable
row sources, remote data, and virtualization are outside the current scope.
Adapt the copied source when an application needs different data operations.

## Spinner

[`spinner.rs`](../crates/base/examples/recipes/spinner.rs) adds a rotating arc to
the existing indeterminate Progress. Supply the operation's accessible name:

```rust,ignore
Spinner::new("preview-progress", "Preview progress", palette)
    .size(rems(1.25))
    .text_color(palette.accent)
```

The default size is one rem and the default color is the consumer's accent.
`Styled` controls the root; the arc follows its actual bounds and text color.
Progress supplies the sole native ProgressIndicator role, label, and numeric
range, with no current numeric value. The visual arc adds no separate role,
accessible name, or Tab stop. This is an indeterminate progress indicator; it
does not provide an automatic live announcement.

GPUI's `AnimationExt` owns the one-second repeating animation. It renders a
static arc without requesting animation frames when `App::reduce_motion()` is
enabled. Applications can set that existing preference with
`App::set_reduce_motion(bool)`; automatic synchronization with an operating
system preference is outside this recipe. There is no recipe-specific animation
controller, retained loading state, or timer.

Mount Spinner while the application considers an operation pending and remove
it when that operation finishes. In the example, the existing Unknown progress
toggle replaces the determinate bar with Spinner and Skeleton in the same
bounded row. Both progress presentations keep the `preview-progress` ID and
accessible name. The Slider retains its known value while the unknown state is
shown, and Reset preview returns to known progress at 40%. This remains a
controlled preview; the recipe does not start a recording or a background task.

## Skeleton

[`skeleton.rs`](../crates/base/examples/recipes/skeleton.rs) is a static decorative
placeholder. The application supplies its dimensions and decides when to replace
it with ready content:

```rust,ignore
Skeleton::new("preview-placeholder", palette)
    .w(px(180.))
    .h(px(16.))
    .rounded(px(4.))
```

The default is a full-width line as high as the consumer's font size, using its
border color, corner radius, and 0.35 opacity. Ordinary `Styled` refinements can
make text lines, blocks, or circular placeholders. The surface owns no timer or
animation, so it remains still under every motion preference.

Skeleton adds no native role, accessible name, children, or focus stop. Name the
loading operation separately, as the example does with Spinner. It owns no
loading state or content measurement; a collection of placeholders is ordinary
application layout. The example shows one where the percentage fill is not yet
available and removes it when known progress is restored.

## Avatar

[`avatar.rs`](../crates/base/examples/recipes/avatar.rs) presents a circular image
with explicit alternative text and application-supplied fallback text:

```rust,ignore
Avatar::new("preset-avatar", "Balanced preset artwork", "BL", palette)
    .image(preset_artwork)
    .size(rems(2.5))
```

`image` accepts GPUI's existing `ImageSource` conversions. Omitting it shows the
fallback immediately; an image error also shows the fallback. While an image is
pending, GPUI's native 200 ms loading delay applies before showing the fallback.
GPUI owns decoding, loading, caching, and redraws. The recipe adds no separate
loading task, timer, cache, or retry policy.

The root supplies one native Image role and the given accessible name in every
state. The image and fallback add no separate name or focus stop. Supply a name
that describes the image's meaning, rather than relying on the visible initials.
The application chooses fallback text and any localization; this recipe does not
derive initials from names. The default frame is 2.5 rem square, with palette
colors and circular clipping. `Styled` can change its size and colors; keep width
and height equal for a circle. Images use GPUI's cover fit. Status badges, avatar
groups, and interactive actions remain application compositions.

The example embeds [`preset-avatar.svg`](../crates/base/examples/assets/preset-avatar.svg)
for the Balanced preset. Other presets use explicit text such as `PR` and `HQ`,
and an empty selection uses `?`, with an accessible name derived from the current
preset label. The source is selected while rendering the existing HoverCard from
the retained Combobox state; there is no second selected-preset value. This
bundled artwork requires no network request. Applications supply their own assets
when copying the recipe.

## Aspect Ratio

[`aspect_ratio.rs`](../crates/base/examples/recipes/aspect_ratio.rs) provides a
clipped media frame using GPUI's existing aspect-ratio layout:

```rust,ignore
AspectRatio::new("recording-frame", 16. / 9.)
    .w(px(320.))
    .rounded(palette.radius)
    .bg(palette.field)
    .child(media)
```

The ratio is width divided by height and must be finite and positive. The default
width fills its parent; give that parent a definite width or set `.w(...)` on the
frame. Leave height automatic so GPUI can derive it from the width. Explicit
height or min/max constraints follow GPUI's ordinary layout rules and can
override its preferred ratio.

Children live in an absolute container that fills the frame, so their intrinsic
sizes do not determine its height. Oversized content is clipped, including
pointer interaction outside the frame. Supply `.size_full()` on an image or other
child that should fill the container, and choose image object fit on that child.
The frame adds no accessible role, name, focus stop, or retained state. Media
children supply their own accessible descriptions and interaction. Background,
rounding, and other presentation belong to the application through `Styled`.

The preset HoverCard shows an illustrative recording frame containing the current
session name. It uses 4:3 for Presentation and 16:9 for other presets or an empty
selection. Changing the retained preset changes the ratio on the next render;
editing the session name changes the displayed text. The supplied child has the
native Image name describing the format. The preview creates no recording and
does not add controls to HoverCard's noninteractive content.

## Sheet

[`sheet.rs`](../crates/base/examples/recipes/sheet.rs) presents existing Dialog
content in a full-height panel at the right edge of the window:

```rust,ignore
Sheet::new("session-settings", &dialog, &fields_scroll, "Session settings", palette)
    .description("Choose the session's drawing and toolbar settings.")
    .child(form_fields)
    .footer(
        DialogClose::new(&dialog)
            .trigger(|button| button.child("Done")),
    )
```

Keep the Sheet mounted under `gpui_design::Root` while closed. The application
owns and retains both the supplied `Entity<DialogState>` and `ScrollHandle`.
Configure initial focus on that existing DialogState and use its `open` and
`close` methods. Dialog continues to own modal focus containment, Escape and
backdrop dismissal, and return focus. The Sheet adds no open state or selection
model. Its title and optional description provide both the visible header and
the underlying Dialog's accessible name and description.

The preferred width is `palette.dialog_width`, limited to the viewport width.
The default popup fills the viewport height, has square corners, and uses one
border on its left edge. The header and optional footer stay outside a single
vertical ScrollArea body. Each `.child(...)` goes into that body, whose padding,
spacing, and scrollbar colors come from the consumer palette. The body takes
the remaining height and uses the application's retained scroll handle. On
reopening, native reveal of the initial focus target can adjust its offset.
The consumer must keep the header and footer's combined content within the
viewport height. Long form content belongs in the scrolling body.

`Styled` customizes the popup surface. Use `.w(...)` to change its preferred width
and retain the default maximum width, height, and flex layout to preserve the
Sheet's sizing contract. This recipe covers right-edge presentation; other edges,
drag gestures, and transition animation are outside its current scope. It does
not implement Drawer interaction.

The application supplies footer controls, including any `DialogClose` and its
disabled state. Keyboard interaction stays with the supplied controls and
Dialog's existing Tab containment. ScrollArea reveals focused descendants on
its enabled axis and provides native pointer scrolling; it adds no independent
scroll focus target or scroll-key bindings.

Both appearances now use Sheet for the existing Session settings form. The
example reuses its DialogState, initial name-input focus, body scroll handle,
Preferences/Help/Schedule/Presets sections, and Done validation rule. Compact
keeps TabList/Tab/TabPanel in a 22 rem Sheet. Settings supplies a 46 rem preferred
width and uses Sidebar with the selected named Region. Both widths remain capped
by the viewport; the wider presentation gives navigation and form content room
to sit beside one another. Closing and reopening keeps the same input, selection,
and table state. The existing Edit
triggers and menu/command actions open that state; the recipe adds no separate
selection. Avatar and AspectRatio still render the retained preset in its
HoverCard preview. Search commands and Reset preview keep their centered Dialog
and AlertDialog presentations.
The example caps its deferred Select, Combobox, Calendar, and Help popup widths
against the window as well; those surfaces do not inherit the Sheet's width cap.

## Sidebar

[`sidebar.rs`](../crates/base/examples/recipes/sidebar.rs) composes named
navigation from ordinary Buttons and the fixed items and selection of a retained
`TabsState`:

```rust,ignore
// Create and retain a state dedicated to this Sidebar, and observe its changes.
let sections = cx.new(|cx| TabsState::new(items, cx));

Sidebar::new("settings-sections", &sections, "Settings sections", palette);
```

The root exposes native Navigation semantics and the supplied label. Each entry
is a Button named by its item's label and uses that item's stable
`tab_focus_handle(index)`. Disabled items retain their disabled presentation and
are skipped by ordinary Tab traversal. The selected entry sets
`Button::current(true)`, providing native `AriaCurrent::Page`, independently of
`Button::selected(...)` and its visual highlight. Entries keep Button roles;
the navigation does not expose tab-selected metadata.

| Input | Result |
| --- | --- |
| Tab / Shift-Tab | Move among enabled entries and surrounding controls without changing the selected section |
| Enter / Space / click | Select the activated entry through `TabsState::select_value` |
| Arrow keys | Do not change the selected section |

The application owns content rendering and destination focus. Render only the
selected section and give it an appropriate named Region; inactive sections
must not leave controls or accessibility nodes mounted. Keep input entities and
form values in the application so switching sections preserves them. Selection
through `select_value` does not move focus; explicit navigation from another
surface, such as Breadcrumb, must also choose a retained focus target.

Use a fresh `TabsState` for Sidebar and keep it separate from any state used by
TabList, Tab, or TabPanel. TabList installs automatic focus-selection
subscriptions that remain with its state after it is unmounted. Reusing that
state for Sidebar would make focus change selection and break the explicit
activation contract. This restriction applies to switching between the two
presentations as well as mounting them together. The example creates a separate
Demo and state for each fixed appearance.

The default Sidebar is a 10 rem vertical column. `Styled` can change its width,
spacing, and presentation; the consumer places it beside the current content or
stacks the two. Settings uses it inside the existing Sheet. At viewport widths
below 640 px the Sidebar becomes full-width above the current Region. This only
changes layout: it retains the same Buttons, selected item, keyboard behavior,
and focus handles. Compact continues to use its own horizontal TabList and
TabPanel composition. Breadcrumb's Session settings action explicitly selects
Preferences and focuses its navigation entry in either appearance.

The recipe adds no routing, navigation history, disclosure state, nested groups,
or separate selection model. Its application-owned layout uses the existing
Sheet's scrolling body and modal focus containment.

## Typography

[`typography.rs`](../crates/base/examples/recipes/typography.rs) supplies `Heading`,
`Paragraph`, and `InlineCode` as one presentation recipe. GPUI owns text shaping,
layout, and painting. The application supplies the content, outline levels, font
choice, available width, and surrounding spacing.

```rust,ignore
Heading::new("help-title", "Help", 2, palette);

Paragraph::new("state-ownership", palette)
    .text("The application retains ")
    .code("DialogState")
    .text(" and ")
    .code("InputState")
    .text(" while panels are closed.");

InlineCode::new(
    "help-source",
    "crates/base/examples/two_styles.rs",
    palette,
);
```

`Heading::new` requires a positive native heading level. Its visible text also
provides its complete accessible name. Choose the level from the content outline:
changing it changes the semantic hierarchy independently of the default visual
size. Every level starts at `1.25 * palette.font_size` with semibold weight.
Use ordinary `Styled` refinements for a different visual scale, color, or spacing.

`Paragraph::text` and `Paragraph::code` append their supplied text in order,
without inserting spaces or interpreting markup. Include punctuation and spaces
in the text segments. One native `StyledText` shapes the complete paragraph, so
inline code remains part of the same text flow and line wrapping. The paragraph
defaults to normal wrapping, the palette's body size and foreground, and a line
height of 1.5 times its font size. Give its parent an available width and leave
height automatic for long text. The recipe adds no truncation, fixed height, or
independent scrolling; surrounding Sheet/ScrollArea content owns overflow.

The paragraph exposes native Paragraph semantics and its full concatenated text
as one accessible name. Its code ranges use the consumer's code font and
background as native text-run styling. GPUI does not expose an accessibility role
for each styled range, so these ranges do not create separate Code nodes.
`InlineCode` provides a separately named native Code element when the code needs
its own element. Use `Paragraph::code` when code belongs inside a prose sentence.
All three types are static text and add no focus stop or keyboard action.

The consumer's `palette.code_font_family` selects the requested code font. The
example requests DejaVu Sans Mono on Linux, Consolas on Windows, and Menlo on
macOS; applications should supply a font appropriate for their own environment.
`Styled` controls each element's layout and text presentation. Paragraph code
ranges retain their explicit code-font and background styling within the body
text. Changing visible size does not change a Heading's native level or the
text used for accessible names.

Both appearances use the recipe in the Help tab: a level-two Help heading sits
above the existing level-three Accordion headings, and the panels contain prose
about retained application state and an independently named source identifier.
The keyboard-help popup gains a concise heading while retaining its existing
Breadcrumb actions and Kbd hints. Longer explanations remain inside the Sheet's
scrolling body. Consumer builders stay in small helper functions to limit stack
use during debug rendering.

This recipe supplies text presentation and metadata. Markdown parsing, syntax
highlighting, code copying, text editing, and a multiline Code Block composition
remain outside its scope. It introduces no retained state or second text renderer.

## Code Block

[`code_block.rs`](../crates/base/examples/recipes/code_block.rs) composes read-only
multiline code with the existing Button, GPUI clipboard, and ScrollArea:

```rust,ignore
CodeBlock::new(
    "session-configuration",
    configuration_json,
    "Session configuration",
    &configuration_scroll,
    &configuration_focus,
    palette,
);
```

Retain the `ScrollHandle` and `FocusHandle` in the application. The supplied label
is shown beside the Copy button, whose accessible name is `Copy {label}`.
The code exposes native Code semantics and its complete original text as its
accessible name. The viewport is a keyboard focus target; Copy remains an
ordinary Button with its existing pointer, Enter, Space, and focus behavior.
The recipe adds no text-input node, editor state, or selection model.

The default frame is 12 rem high and fills the available width. Its header stays
outside the bounded code viewport. `Styled` customizes the frame; keep a bounded
height and width so the ScrollArea can contain long bodies and lines. Code uses
the palette's code font and foreground. Native GPUI text layout displays the
supplied lines without soft wrapping; horizontal and vertical overflow remain
inside the same ScrollArea. The recipe neither parses code nor adds syntax
highlighting, line numbers, or virtualized rendering.

Copy sends the entire supplied string to `App::write_to_clipboard`, including
offscreen content and an empty string. It does not trim, normalize, or replace
spaces, line endings, or a trailing newline. LF, CRLF, and Unicode content are
preserved in the copied payload. The visible text is shaped by GPUI; the recipe
does not define an additional tab-stop policy or visual CRLF normalization.
There is no separate copied-state timer or automatic announcement.

When the code viewport itself has focus, unmodified arrow keys scroll on their
respective axes, Page Up/Down scroll vertically, Home returns to the top-left,
and End moves to the bottom-right. Offsets stay inside the content's scroll
range. Tab and Shift-Tab retain normal traversal, Escape remains available to
the enclosing modal, and modified keys are left to the application. Copy-button
key events keep the Button's contract. Wheel gestures over code stay within
that viewport even at its edges; scroll outside it to move the parent Sheet.

Both appearances use CodeBlock in Help's State ownership panel, before the
source-path identifier. The consumer serializes current retained settings as
JSON with the existing `serde_json` dependency. This is a display and copy
snapshot; copying does not parse or apply configuration. The example keeps its
consumer builder separate from the main render function and reuses the existing
Sheet, focus containment, and parent scrolling. Typography continues to handle
prose and short inline code; CodeBlock owns the bounded multiline presentation.

## Marker

[`marker.rs`](../crates/base/examples/recipes/marker.rs) places a static label
between two existing decorative Separators. Supply the text appropriate to the
application's date or status boundary:

```rust,ignore
Marker::new("schedule-marker", "Selected date · 2026-11-09", palette);
```

The label exposes native Label semantics and uses its complete visible text as
its accessible name. The containing layout has no role or name, and both
Separators use `decorative(true)`, omitting their native role and orientation.
The marker therefore contributes one meaningful label, with no duplicated
separator semantics. It adds no focus handle, Tab stop, action, or live-region
announcement.

The frame fills the available width. The centered label uses the palette's body
size and muted color, a line height of 1.5 times its font size, normal wrapping,
and at most 75% of the frame width. The two horizontal rules are one pixel thick,
use the palette's border color, and share the remaining space. Long text can grow
vertically without truncation. `Styled` controls the frame and inherited text
presentation; supply a bounded width and allow the label's height to grow. The
recipe uses GPUI's existing text layout and adds no text renderer or timeline state.

Both appearances place Marker between Schedule's DatePicker and its notes/plan
panes. The consumer reads `CalendarState::selected_date()` and formats
`Selected date · YYYY-MM-DD`, or `No date selected` when the selection is empty.
Moving the calendar cursor without committing a selection does not change this
boundary. Date formatting remains application-owned, and the marker does not
start a recording or maintain a second selected date.

## Message

[`message.rs`](../crates/base/examples/recipes/message.rs) presents one read-only
message with application-supplied author, time metadata, and body text. It composes
existing GPUI layout and Typography's Paragraph; optional actions are supplied
controls with their own behavior:

```rust,ignore
Message::new(
    "session-note-message",
    "You",
    "Edited 2026-11-10 09:30:00 UTC",
    current_note,
    palette,
)
.action(edit_note_button)
```

The root has native Comment semantics and no repeated accessible name. Author
and time are separately named Labels in reading order, followed by the Paragraph
whose full text is its accessible name. Time is a caller-formatted string: the
recipe does not parse dates, choose a timezone, infer delivery status, or add a
live announcement. The metadata and body add no focus stop. The application owns
stable IDs and the meaning of the supplied author, time, and text.

The frame fills its bounded parent and grows vertically. Stacked author/time
metadata and body wrap using GPUI's text layout, including long Korean or English
content; explicit body line breaks are retained. There is no body truncation,
fixed content height, or internal scroll controller. Put long messages in the
application's existing ScrollArea. `Styled` changes the frame and inherited
metadata presentation; Paragraph retains its palette style. Allow height to grow
and give supplied actions a layout that fits the available width. Actions sit
below the body and do not compete with its width. `.action(...)` accepts one
element, so the application can supply an existing Button or a container of
controls. Buttons retain their own
accessible names, enabled state, pointer/Enter/Space activation, and ordinary
Tab/Shift+Tab traversal. The message has no whole-row activation or selection
state, and does not manage focus when the application removes an action.

Message identifies authored content and its provenance. Marker labels a boundary
between content, and Item groups a generic title, description, and trailing
control. Button supplies the action behavior in all these compositions; Message
adds no alternate button, input, or focus model. Avatar is independent and is not
required to repeat an author already named in the metadata.

Both appearances use Message below Schedule's existing notes/plan panes. The body
reads the same retained Textarea state, and the application records the latest
local `InputEvent::Change` time in UTC. Edit note focuses that existing input;
editing it updates the read-only presentation. Programmatic `set_value` does not
emit that edit event; content without recorded edit metadata displays
`Edit time not recorded`. Focusing the input or changing the recording date does
not update the edit timestamp. When the note is empty, the
consumer shows the existing EmptyState instead of inventing a message. The local
edit timestamp is not a send or delivery receipt. Closing settings retains the
note and its metadata; there is no second note value, transcript, transport,
streaming renderer, or timeline state.

## Attachment

[`attachment.rs`](../crates/base/examples/recipes/attachment.rs) presents a file's
name, media type, and size as application-supplied text, with an optional existing
action. It describes metadata; the content itself remains with the application.

```rust,ignore
Attachment::new(
    "configuration-attachment",
    "session-configuration.json",
    "application/json",
    format!("{} bytes (UTF-8)", configuration.len()),
    palette,
)
.action(inspect_json_button)
```

The native Group's accessible name is the complete filename. Its description is
the same `media_type · size_label` text shown below the filename. The visible
filename and metadata children have no separate native role or name, so each
value is exposed once through the group. Supplied action Buttons retain their
own names and roles. The attachment frame adds no Tab stop, whole-row click,
selection state, link destination, or live announcement.

All three metadata values are supplied strings. The application chooses units,
localization, and how to describe unavailable metadata; the recipe does not
parse filenames, infer types, inspect files, format byte quantities, or validate
the claimed size. Zero bytes can be displayed literally. An ordinary Group is
used because metadata does not itself supply a document viewer or a file link.
Unlike Item's generic title/description row, Attachment has explicit file
metadata. Unlike Message's authored Comment and body, it identifies a separate
content resource without claiming authorship or delivery.

The frame fills a bounded parent. Filename and metadata wrap without truncation,
and the frame grows vertically; an action sits below the text so it does not
compete with the filename's width. `Styled` customizes the frame and inherited
text presentation. Long collections belong in the application's existing
ScrollArea. `.action(...)` accepts one element, including an existing Button or
an application-owned container whose controls fit the available width. Button
continues to own enabled state, pointer/Enter/Space activation, focus, and
Tab/Shift+Tab traversal. The application owns focus policy if it removes a
focused action; Attachment has no new focus controller.

Both appearances use Attachment in Help's State ownership panel above the
existing CodeBlock. The consumer serializes the current settings once, uses the
same string for the code and metadata, and displays `session-configuration.json`,
`application/json`, and its UTF-8 byte count. Editing a name or note changes that
metadata with the displayed JSON. Inspect JSON focuses the existing code viewport
and uses its existing scrolling and keyboard behavior. The explanatory text
identifies this content as in memory and not saved. No file is created or opened,
and no upload or download occurs. The existing CodeBlock Copy action remains
independent; Attachment adds no clipboard operation, preview renderer, progress
state, transport, or storage mechanism.

## Trace

[`trace.rs`](../crates/base/examples/recipes/trace.rs) presents a summary that
expands into named read-only details and an optional application action. It
composes existing Collapsible parts, Button behavior, and Typography's Paragraph.

```rust,ignore
let disclosure = cx.new(CollapsibleState::new);
// Retain this entity and observe it so the application view redraws.

Trace::new(
    "settings-trace",
    &disclosure,
    "Current settings · MiniCast",
    "Current settings details",
    detail_text,
    palette,
)
.action(close_details_button)
```

The application owns and observes the supplied `Entity<CollapsibleState>`.
Configure its initial state with `default_open` and use the existing `set_open`
and `set_disabled` methods for later changes. Keep the root and trigger mounted
while closed. Trace supplies no additional disclosure state, event type, data
collection, or execution controller.

The existing Collapsible root is an unnamed native Group with a non-Tab focus
scope. Its trigger is a Button named by the complete summary, with the native
expanded and disabled metadata supplied by CollapsibleTrigger. The visible
expansion cue adds no role or name. Opening through the trigger leaves focus
there; ordinary Tab traversal enters any supplied action. The details Region
uses the explicit `details_label`, contributes no Tab stop, and contains one
Paragraph named by its complete text. The pinned GPUI API does not expose native
trigger/content relationship IDs. Closed details contribute no layout, action
focus stops, or accessibility nodes.

Pointer, Enter, and Space toggle through the existing trigger. Closing while an
action in the details owns focus returns focus to the live trigger. If the
trigger is disabled, the existing Collapsible lifecycle instead uses its non-Tab
root so traversal can leave the disclosure. Disabling the trigger does not close
or disable the open details or supplied actions; their enabled state belongs to
the application and their existing Buttons. Changing summary or detail strings
does not reset the retained open state or focus. The application must move focus
to a retained target before removing the entire host or a focused action.

Give the frame a bounded width. Summary and body wrap using GPUI text layout and
grow vertically without truncation. The optional `.action(...)` element sits
inside the details after the body; a container can supply multiple controls with
its own fitting layout. `Styled` customizes the outer frame; the supplied palette
styles the trigger and Paragraph. Long details use the parent's existing
ScrollArea, with no inner viewport or new scroll-key handling.

Both appearances place Trace after the configuration preview in Help's State
ownership panel. The summary uses the current session name, and the details read
the current session, Calendar selection, preset, drawing tool, cursor and toolbar
settings, and note values. Close details calls the existing
`CollapsibleState::set_open(false, ...)`, which restores summary focus. Editing
the underlying form updates the visible text; closing and reopening settings
preserves the retained disclosure and form values.
This consumer describes current settings and does not record operations, jobs,
or model reasoning. Trace has no log source, parser, streaming output, transport,
or timeline model. Item remains a static generic row, Message presents authored
content, and Collapsible continues to own the shared disclosure behavior.

## Verification scope

The existing composition suite runs the same recipe modules as the native
example. Its command-search flow covers empty-state recovery, and its date-picker
flow covers traversal and activation of the grouped month buttons. Static shapes
and palette values have no separate test suite. The existing field/preset flow
covers missing-name recovery, the disabled addon for a read-only input, urgent
inline semantics, and the Item's retained preview/edit behavior. The date-picker
flow also checks that the ordinary schedule notice remains a Note. The existing
three-platform CI continues to check real consumer composition and native
compilation. Linux additionally runs isolated recipe tests. Windows and macOS
filter the `two_styles::recipes::` tests from the same composition target, keeping
the real application flows while avoiding repeated recipe-only checks. One
additional composed flow covers the preset table's sorting, filter/empty recovery,
page boundaries, stable-ID action, and retained choice. There is no separate data
test framework or duplicated platform-specific flow. These results do not
establish native screen-reader, IME, visual, or performance validation.

The existing preview flow also covers named Spinner rendering, a decorative
non-focusable Skeleton, GPUI reduced-motion rendering, reset cancellation,
restoring the percentage fill, and removing the indicators when their disclosure
closes. The loading recipes add no test function or CI job; animation scheduling
remains GPUI's responsibility rather than a second test controller here.

The existing Help flow checks Kbd's readable symbolic key name and absence of
new focus stops, alongside the composed Breadcrumb ancestor actions. The
context-menu flow exercises the existing shortcut displayed by its keycaps.
There is no separate keybinding test controller,
palette suite, or CI job for these static key hints. Native screen-reader speech
and desktop visual rendering remain separate validation work.

The media recipes add two focused regressions within the same composition target.
Avatar exercises absent, pending, failed, loaded, and cleared image sources while
checking its native name, focus neutrality, and fixed geometry; the loaded path
decodes the bundled SVG. AspectRatio exercises two container widths, oversized
content, and pointer clipping beyond the frame. The existing preset flow checks
that the retained choice drives the preview's artwork, frame ratio, and session
name while preserving the existing trigger and dismissal behavior. These checks
do not establish remote image transport, native screen-reader speech, desktop
visual rendering, or performance measurements.

Sheet coverage exercises edge placement, normal and narrow viewport widths,
fixed header/footer geometry, body scrolling, descendant focus reveal, and
closing and reopening with retained form state. The checks also exercise the
shared Dialog's backdrop wheel handling to keep underlying content from scrolling,
and opens Select in a narrow window to check the child popup's horizontal bounds.
Nested-modal checks cover focus containment and wheel isolation in the top dialog,
closing it while the Sheet remains open, and
returning through the Sheet to its original trigger. These are layout and
interaction checks in the existing GPUI test contexts; their scope does not
establish native screen-reader, platform IME, or desktop visual validation.

Typography checks cover native heading levels independently of visual size,
complete accessible names, Paragraph and Code roles, and the absence of text
focus stops. Layout coverage uses long text and code with both appearances at
normal and narrow widths. The Help consumer preserves its existing hierarchy,
ancestor actions, and keyboard traversal. The composition target is also run
with a 1.5 MiB test-thread stack to check the example's render-stack regression.
These are native metadata, layout, and simulated-input checks; screen-reader
speech, platform IME behavior, and desktop visual results require separate
validation.

Sidebar is exercised in the Settings consumer's existing composition flows;
there is no separate candidate fixture. The flows check explicit activation,
disabled entries, current-section semantics, retained values, ordinary focus
traversal, and the stacked narrow-window layout. Compact's TabList behavior
remains covered by its real consumer flows. Button's native metadata contract
owns current-page checks, and TabsState's core regression owns focus-neutral
programmatic selection and change events.

Code Block coverage checks empty and multiline content, Unicode, complete code
names, preserved line endings, and exact full-text copying through GPUI's test
clipboard. Layout checks exercise both appearances, narrow widths, long lines,
and long bodies. The consumer flow verifies current settings in the copied JSON,
keyboard entry and traversal, code-scroll ends, and isolation from the parent
Sheet's scroll range. Clipboard assertions run only in the test platform and do
not exercise the user's desktop clipboard service. The composition target also
retains the 1.5 MiB stack check; native clipboard integration, screen-reader
speech, IME, and desktop visual observations remain separate validation work.

Marker coverage checks a single meaningful label, decorative separator omission,
and normal focus traversal past the marker. Layout checks exercise long Korean and
English labels, narrow widths, and both appearances. Schedule checks distinguish
the current selected date from a moved or cancelled calendar cursor, preserve
the existing keyboard flow, and update the boundary when the selection changes
or clears. These are metadata, layout, and simulated-input checks; native
screen-reader speech, IME, DPI, and desktop visual observations remain separate
validation work.

Message's focused recipe check covers the Comment/Label/Paragraph hierarchy,
each supplied text's accessible name appearing once, explicit line breaks, long
Korean and English content, and narrow-width wrapping in both appearances.
Button's own tests cover activation and disabled behavior. The Schedule consumer
checks the shared note value, local edit metadata, empty-state transitions,
Edit note focus, and existing Sheet scrolling and dismissal. These are headless
metadata, layout, and input
checks; native screen-reader speech and desktop visual validation remain
separate.

Attachment's focused recipe check covers the full filename name and combined
metadata description without duplicate child semantics, long Korean and English filenames,
narrow-width wrapping, and both appearances. Supplied actions retain Button's
independently tested activation and disabled behavior. The Help consumer checks
that the byte count follows the current UTF-8 JSON, Inspect JSON reaches the
existing code focus target, and
CodeBlock keeps its complete copy and scrolling behavior. Clipboard assertions
remain limited to GPUI's test clipboard; native file access, screen-reader speech,
and desktop visual observations are not established by these checks.

Trace's focused recipe check covers summary naming, expanded metadata, the named
Region and Paragraph hierarchy, closed-content removal, and long summaries and
details wrapping at narrow widths in both appearances. Collapsible's core tests
own activation, disabled-trigger behavior, and focus return on closure; supplied
actions use the existing Button contract. The Help consumer checks current form
values,
disclosure retention, the Close details action, and surrounding Sheet scrolling
and focus. The composition target retains its 1.5 MiB stack check. These tests
exercise native metadata, layout, and simulated input; desktop rendering,
screen-reader speech, IME, and performance still require separate validation.


## Bubble and native chart presentation

The catalogue adds two independent source-owned presentations under
`crates/base/examples/recipes`. `bubble.rs` is the Bubble catalogue entry:
incoming/outgoing alignment, grouped author/time metadata, a wrapping message
body and supplied reaction actions. Existing Buttons own activation and focus;
message grouping and reaction values belong to the conversation view. It adds no
transport, persistence or alternate message-state engine.

`chart.rs` is the presentation for the public Chart family, and is not counted
as a second catalogue entry. It supplies native canvas line/bar/area paths, axes,
series Toggles, an inspected-value readout and a same-data Table. Its color array,
layout and formatting are application-owned. See the
[layout and data guide](layout-and-data.md) for the shared Chart contract.

Run `cargo run -p gpui-design --example catalogue --locked` to use these alongside
MessageScroller/Questionnaire and Carousel/Direction respectively. The existing
`two_styles` example continues to host the session-oriented recipes.
