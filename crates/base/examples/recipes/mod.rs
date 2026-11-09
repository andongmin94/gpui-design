//! Copyable application presentations built from GPUI layout and shared controls.
//!
//! The consumer owns these files and the palette imported by each recipe.

mod alert;
mod badge;
mod button_group;
mod card;
mod empty;
mod input_group;
mod item;

pub(crate) use alert::Alert;
pub(crate) use badge::Badge;
pub(crate) use button_group::ButtonGroup;
pub(crate) use card::Card;
pub(crate) use empty::EmptyState;
pub(crate) use input_group::InputGroup;
pub(crate) use item::Item;
