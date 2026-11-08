//! Headless form, navigation, and overlay primitives for GPUI.
//!
//! Applications own presentation. Call [`init`] once and host each window's
//! content in [`Root`] for keyboard navigation and modal focus containment.
//!
//! Derived from GPUI Kit v0.9.0 under Apache-2.0; the public surface and
//! initialization are scoped to the documented controls and their support APIs.

mod a11y_disabled;
mod accordion;
pub mod actions;
mod alert_dialog;
mod animation;
mod auto_scroll;
mod button;
mod checkbox;
mod collapsible;
mod component_traits;
mod dialog;
mod dropdown_menu;
mod element_ext;
mod event;
mod focus_trap;
mod geometry;
mod global_state;
pub mod input;
#[cfg(all(target_os = "macos", not(test)))]
mod macos_accessibility;
mod observe;
mod overscroll;
mod popover;
mod popup;
mod positioner;
mod progress;
mod radio_group;
mod reduce_motion;
mod root;
mod scroll_bounce;
mod scrollbar;
mod select;
mod slider;
mod state_style;
mod styled;
mod switch;
mod tabs;
mod text_boundary;
mod theme;
mod toggle;
mod toggle_group;
mod touch_selection;

pub use accordion::{
    Accordion, AccordionEvent, AccordionHeader, AccordionItem, AccordionPanel, AccordionState,
};
pub use alert_dialog::AlertDialog;
pub(crate) use auto_scroll::AutoScroll;
pub use button::{Button, ButtonStyles};
pub use checkbox::{Checkbox, CheckboxState, CheckboxStyles};
pub use collapsible::{
    Collapsible, CollapsibleContent, CollapsibleEvent, CollapsibleState, CollapsibleTrigger,
};
pub use component_traits::Selectable;
pub use dialog::{
    Dialog, DialogBackdrop, DialogChangeReason, DialogClose, DialogDescription, DialogEvent,
    DialogPopup, DialogState, DialogTitle,
};
pub use dropdown_menu::{
    DropdownMenu, DropdownMenuEvent, DropdownMenuItem, DropdownMenuOption, DropdownMenuState,
};
pub use element_ext::ElementExt;
pub use event::{InteractiveElementExt, OngoingScrollExt};
pub use focus_trap::FocusTrapElement;
pub(crate) use focus_trap::active_focus_trap;
pub use geometry::*;
pub(crate) use global_state::GlobalState;
pub use input::{Input, InputAccessibility, InputBase, InputEditorStyle, InputState, InputStyles};
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) use macos_accessibility::install_window_hit_test_forwarder;
pub use observe::{ObservedElement, ObservedTextElement, TestSupportExt, TestTextExt};
pub use popover::{Popover, PopoverState};
pub use popup::{POPUP_PRIORITY, Popup};
pub use positioner::{Align, Positioner, ResolvedPosition};
pub use progress::Progress;
pub use radio_group::{RadioGroup, RadioGroupEvent, RadioGroupState, RadioItem, RadioOption};
pub use reduce_motion::apply_system_reduce_motion;
pub use root::Root;
pub use scroll_bounce::{ScrollBounce, ScrollBounceMotion};
pub use scrollbar::{
    Scrollbar, ScrollbarAxis, ScrollbarEntrance, ScrollbarHandle, ScrollbarMode, ScrollbarMotion,
    ScrollbarStyles, ScrollbarThumbStyle, ScrollbarTrackStyle,
};
pub use select::{Select, SelectEvent, SelectItem, SelectOption, SelectState};
pub use slider::{Slider, SliderEvent, SliderState};
pub use state_style::StateStyle;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use styled::styled_ext_reflection_methods;
pub use styled::{RoleOverride, StyledExt, box_shadow, h_flex, v_flex};
pub use switch::{Switch, SwitchStyles};
pub use tabs::{Tab, TabItem, TabList, TabPanel, TabsEvent, TabsState};
pub use theme::{InputColors, ScrollbarTheme, Theme, ThemeAppearance};
pub use toggle::Toggle;
pub use toggle_group::{ToggleGroup, ToggleGroupEvent, ToggleGroupState, ToggleItem, ToggleOption};
pub use touch_selection::{SelectionEdge, TouchHandle, TouchSelectionSnapshot};

use gpui::App;

/// Whether the application is compiled for iOS or Android.
#[inline]
pub const fn is_mobile() -> bool {
    cfg!(any(target_os = "ios", target_os = "android"))
}

/// Register keyboard actions and shared input/focus state before creating views.
pub fn init(cx: &mut App) {
    let _ = Theme::global_mut(cx);
    GlobalState::init(cx);
    reduce_motion::init(cx);
    accordion::init(cx);
    dialog::init(cx);
    dropdown_menu::init(cx);
    focus_trap::init(cx);
    popover::init(cx);
    radio_group::init(cx);
    select::init(cx);
    slider::init(cx);
    tabs::init(cx);
    toggle_group::init(cx);
    input::init(cx);
    root::init(cx);
}

#[cfg(feature = "test-support")]
pub mod test_support;
