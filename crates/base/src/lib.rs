//! Headless form, navigation, and overlay primitives for GPUI.
//!
//! Applications own presentation. Call [`init`] once and host each window's
//! content in [`Root`] for keyboard navigation and modal focus containment.
//!
//! Derived from GPUI Kit v0.9.0 under Apache-2.0; the public surface and
//! initialization are scoped to the documented controls and their support APIs.

mod a11y_current;
mod a11y_disabled;
mod accordion;
pub mod actions;
mod alert_dialog;
mod animation;
mod auto_scroll;
mod breadcrumb;
mod button;
mod calendar;
mod carousel;
mod chart;
mod checkbox;
mod collapsible;
mod combobox;
mod command;
mod component_traits;
mod context_menu;
mod date_picker;
mod dialog;
mod direction;
mod drawer;
mod dropdown_menu;
mod element_ext;
mod event;
mod field;
mod focus_trap;
mod geometry;
mod global_state;
mod hover_card;
pub mod input;
mod input_otp;
mod label;
#[cfg(all(target_os = "macos", not(test)))]
mod macos_accessibility;
mod menu;
mod menubar;
mod message_scroller;
mod navigation_menu;
mod observe;
mod overscroll;
mod popover;
mod popup;
mod positioner;
mod progress;
mod questionnaire;
mod radio_group;
mod reduce_motion;
mod resizable;
mod root;
mod scroll_area;
mod scroll_bounce;
mod scrollbar;
mod select;
mod separator;
mod slider;
mod state_style;
mod styled;
mod switch;
mod table;
mod tabs;
mod text_boundary;
mod theme;
mod toast;
mod toggle;
mod toggle_group;
mod tooltip;
mod touch_selection;

pub use accordion::{
    Accordion, AccordionEvent, AccordionHeader, AccordionItem, AccordionPanel, AccordionState,
};
pub use alert_dialog::AlertDialog;
pub(crate) use auto_scroll::AutoScroll;
pub use breadcrumb::{Breadcrumb, BreadcrumbItem, BreadcrumbList};
pub use button::{Button, ButtonStyles};
pub use calendar::{
    Calendar, CalendarDay, CalendarEvent, CalendarSelection, CalendarState, NaiveDate,
};
pub use carousel::{Carousel, CarouselControl, CarouselEvent, CarouselSlide, CarouselState};
pub use chart::{Chart, ChartEvent, ChartSeries, ChartState};
pub use checkbox::{Checkbox, CheckboxState, CheckboxStyles};
pub use collapsible::{
    Collapsible, CollapsibleContent, CollapsibleEvent, CollapsibleState, CollapsibleTrigger,
};
pub use combobox::{
    Combobox, ComboboxEvent, ComboboxGroup, ComboboxInput, ComboboxList, ComboboxOption,
    ComboboxState,
};
pub use command::{
    Command, CommandEvent, CommandGroup, CommandInput, CommandList, CommandOption, CommandState,
    SearchItem,
};
pub use component_traits::Selectable;
pub use context_menu::ContextMenu;
pub use date_picker::{DatePicker, DatePickerEvent, DatePickerState};
pub use dialog::{
    Dialog, DialogBackdrop, DialogChangeReason, DialogClose, DialogDescription, DialogEvent,
    DialogPopup, DialogState, DialogTitle,
};
pub use direction::{Direction, DirectionProvider};
pub use drawer::{Drawer, DrawerState};
pub use dropdown_menu::DropdownMenu;
pub use element_ext::ElementExt;
pub use event::{InteractiveElementExt, OngoingScrollExt};
pub use field::{Field, FieldStyles};
pub use focus_trap::FocusTrapElement;
pub(crate) use focus_trap::active_focus_trap;
pub use geometry::*;
pub(crate) use global_state::GlobalState;
pub use hover_card::{HoverCard, HoverCardState};
pub use input::{
    Input, InputAccessibility, InputBase, InputEditorStyle, InputState, InputStyles, Textarea,
    TextareaState,
};
pub use input_otp::{InputOtp, InputOtpEvent, InputOtpSlot, InputOtpState};
pub use label::Label;
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) use macos_accessibility::install_window_hit_test_forwarder;
pub use menu::{MenuEvent, MenuItem, MenuOption, MenuState, MenuSubmenu};
pub use menubar::{Menubar, MenubarItem, MenubarMenu, MenubarState};
pub use message_scroller::{MessageScroller, MessageScrollerState};
pub use navigation_menu::{
    NavigationMenu, NavigationMenuEntry, NavigationMenuEvent, NavigationMenuItem,
    NavigationMenuLink, NavigationMenuState,
};
pub use observe::{ObservedElement, ObservedTextElement, TestSupportExt, TestTextExt};
pub use popover::{Popover, PopoverState};
pub use popup::{POPUP_PRIORITY, Popup};
pub use positioner::{Align, Positioner, ResolvedPosition};
pub use progress::Progress;
pub use questionnaire::{Questionnaire, QuestionnaireStage, QuestionnaireState};
pub use radio_group::{RadioGroup, RadioGroupEvent, RadioGroupState, RadioItem, RadioOption};
pub use reduce_motion::apply_system_reduce_motion;
pub use resizable::{Resizable, ResizableEvent, ResizableHandle, ResizableState};
pub use root::Root;
pub use scroll_area::ScrollArea;
pub use scroll_bounce::{ScrollBounce, ScrollBounceMotion};
pub use scrollbar::{
    Scrollbar, ScrollbarAxis, ScrollbarEntrance, ScrollbarHandle, ScrollbarMode, ScrollbarMotion,
    ScrollbarStyles, ScrollbarThumbStyle, ScrollbarTrackStyle,
};
pub use select::{Select, SelectEvent, SelectItem, SelectOption, SelectState};
pub use separator::Separator;
pub use slider::{Slider, SliderEvent, SliderState, SliderThumb};
pub use state_style::StateStyle;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use styled::styled_ext_reflection_methods;
pub use styled::{RoleOverride, StyledExt, box_shadow, h_flex, v_flex};
pub use switch::{Switch, SwitchStyles};
pub use table::{Table, TableCaption, TableCell, TableColumn, TableRow};
pub use tabs::{Tab, TabItem, TabList, TabPanel, TabsEvent, TabsState};
pub use theme::{InputColors, ScrollbarTheme, Theme};
pub use toast::{Toast, ToastClose, ToastDismissReason, ToastEvent, ToastState};
pub use toggle::Toggle;
pub use toggle_group::{ToggleGroup, ToggleGroupEvent, ToggleGroupState, ToggleItem, ToggleOption};
pub use tooltip::{Tooltip, TooltipState};
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
    calendar::init(cx);
    carousel::init(cx);
    chart::init(cx);
    context_menu::init(cx);
    dialog::init(cx);
    dropdown_menu::init(cx);
    focus_trap::init(cx);
    menu::init(cx);
    menubar::init(cx);
    navigation_menu::init(cx);
    popover::init(cx);
    radio_group::init(cx);
    resizable::init(cx);
    select::init(cx);
    slider::init(cx);
    tabs::init(cx);
    toast::init(cx);
    toggle_group::init(cx);
    input::init(cx);
    root::init(cx);
}

#[cfg(feature = "test-support")]
pub mod test_support;
