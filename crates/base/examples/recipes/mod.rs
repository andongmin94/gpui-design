//! Copyable application presentations built from GPUI layout and shared controls.
//!
//! The consumer owns these files and the palette imported by each recipe.

mod badge;
mod button_group;
mod card;
mod empty;

pub(crate) use badge::Badge;
pub(crate) use button_group::ButtonGroup;
pub(crate) use card::Card;
pub(crate) use empty::EmptyState;
