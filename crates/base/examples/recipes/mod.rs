//! Copyable application presentations built from GPUI layout and shared controls.
//!
//! The consumer owns these files and their palette imports or Styled values.

mod alert;
mod aspect_ratio;
mod attachment;
mod avatar;
mod badge;
mod button_group;
mod card;
mod code_block;
mod data_table;
mod empty;
mod input_group;
mod item;
mod kbd;
mod marker;
mod message;
mod pagination;
mod sheet;
mod skeleton;
mod spinner;
mod trace;
mod typography;

pub(crate) use alert::Alert;
pub(crate) use aspect_ratio::AspectRatio;
pub(crate) use attachment::Attachment;
pub(crate) use avatar::Avatar;
pub(crate) use badge::Badge;
pub(crate) use button_group::ButtonGroup;
pub(crate) use card::Card;
pub(crate) use code_block::CodeBlock;
pub(crate) use data_table::{DataTable, DataTableColumn, DataTableRow, DataTableState};
pub(crate) use empty::EmptyState;
pub(crate) use input_group::InputGroup;
pub(crate) use item::Item;
pub(crate) use kbd::{Kbd, KbdGroup};
pub(crate) use marker::Marker;
pub(crate) use message::Message;
pub(crate) use pagination::Pagination;
pub(crate) use sheet::Sheet;
pub(crate) use skeleton::Skeleton;
pub(crate) use spinner::Spinner;
pub(crate) use trace::Trace;
pub(crate) use typography::{Heading, InlineCode, Paragraph};

// Run the isolated Sidebar review in the existing three-platform composition
// target without changing the native showcase or its Tabs/TabPanel contract.
#[cfg(all(test, feature = "test-support"))]
#[path = "../../tests/sidebar_candidate/mod.rs"]
mod sidebar_candidate;
