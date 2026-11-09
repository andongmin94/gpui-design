//! Copyable application presentations built from GPUI layout and shared controls.
//!
//! The consumer owns these files and their palette imports or Styled values.

mod alert;
mod aspect_ratio;
mod avatar;
mod badge;
mod button_group;
mod card;
mod data_table;
mod empty;
mod input_group;
mod item;
mod kbd;
mod pagination;
mod skeleton;
mod spinner;

pub(crate) use alert::Alert;
pub(crate) use aspect_ratio::AspectRatio;
pub(crate) use avatar::Avatar;
pub(crate) use badge::Badge;
pub(crate) use button_group::ButtonGroup;
pub(crate) use card::Card;
pub(crate) use data_table::{DataTable, DataTableColumn, DataTableRow, DataTableState};
pub(crate) use empty::EmptyState;
pub(crate) use input_group::InputGroup;
pub(crate) use item::Item;
pub(crate) use kbd::{Kbd, KbdGroup};
pub(crate) use pagination::Pagination;
pub(crate) use skeleton::Skeleton;
pub(crate) use spinner::Spinner;
