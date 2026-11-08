// Modified for GPUI Design: retain input touch and deferred-context state only.
use std::rc::{Rc, Weak};
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
#[cfg(target_family = "wasm")]
use web_time::Instant;

use gpui::{App, Global};

/// Holds the deferred interaction context open for as long as it is alive.
///
/// Handed out by [`GlobalState::register_deferred_popover`]; drop it to close
/// the context again. The registry only ever weighs the token's liveness, so
/// there is nothing inside to read.
pub struct DeferredPopover(#[allow(dead_code)] Rc<()>);

/// Application-wide state shared by Base behaviors.
pub struct GlobalState {
    deferred_popovers: Vec<Weak<()>>,
    /// When a finger last went down. A tap reaches controls as a mouse press;
    /// this is how they tell it from one.
    last_touch: Option<Instant>,
}

impl Global for GlobalState {}

impl GlobalState {
    fn new() -> Self {
        Self {
            deferred_popovers: Vec::new(),
            last_touch: None,
        }
    }

    /// Records that a finger went down. Called from the touch drag GPUI
    /// offers on every touch, before it becomes a tap, a long press or a pan.
    pub fn note_touch(cx: &mut App) {
        Self::init(cx);
        Self::global_mut(cx).last_touch = Some(Instant::now());
    }

    /// Whether the press being handled came from a finger: a touch went
    /// down recently enough that the mouse events of a tap are still
    /// arriving. A double tap takes up to twice the tap interval.
    pub fn is_touch_press(cx: &App) -> bool {
        cx.try_global::<Self>()
            .and_then(|state| state.last_touch)
            .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(1))
    }

    /// Ensures that the Base global exists.
    #[doc(hidden)]
    pub fn init(cx: &mut App) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::new());
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Returns whether any deferred popup currently owns an open interaction
    /// context.
    pub fn is_in_deferred_context(cx: &App) -> bool {
        Self::global(cx)
            .deferred_popovers
            .iter()
            .any(|popover| popover.strong_count() > 0)
    }

    /// Registers an open deferred popup, which stays registered for as long as
    /// the returned token is held.
    ///
    /// A token rather than an identifier, because popup state is routinely
    /// dropped without ever being closed: state that stops being rendered — a
    /// popover scrolled out of a virtual list, a panel closed while its menu is
    /// open — is collected at the end of the frame, with no chance to
    /// deregister. A registration that outlived its popup would leave the
    /// application believing a popup is open forever, and everything that
    /// steps aside for open popups (the native context menu of a text input,
    /// say) would stay disabled for the rest of the session.
    pub fn register_deferred_popover(cx: &mut App) -> DeferredPopover {
        let token = Rc::new(());
        let state = Self::global_mut(cx);
        state
            .deferred_popovers
            .retain(|popover| popover.strong_count() > 0);
        state.deferred_popovers.push(Rc::downgrade(&token));
        DeferredPopover(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn initialization_keeps_an_open_deferred_context(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            GlobalState::init(cx);
            let popover = GlobalState::register_deferred_popover(cx);
            GlobalState::init(cx);
            assert!(GlobalState::is_in_deferred_context(cx));
            drop(popover);
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }

    #[gpui::test]
    fn a_dropped_registration_closes_the_deferred_context(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            GlobalState::init(cx);
            let outer = GlobalState::register_deferred_popover(cx);
            {
                let _inner = GlobalState::register_deferred_popover(cx);
                assert!(GlobalState::is_in_deferred_context(cx));
            }
            assert!(GlobalState::is_in_deferred_context(cx));
            drop(outer);
            assert!(!GlobalState::is_in_deferred_context(cx));

            let popover = GlobalState::register_deferred_popover(cx);
            assert_eq!(GlobalState::global(cx).deferred_popovers.len(), 1);
            drop(popover);
            assert!(!GlobalState::is_in_deferred_context(cx));
        });
    }
}
