// Modified for GPUI Design: retain prepaint observation without rich-text scopes.
use gpui::{App, Bounds, ParentElement, Pixels, Styled as _, Window, canvas};

/// Extends a GPUI parent element with post-layout prepaint observation.
pub trait ElementExt: ParentElement + Sized {
    /// Invokes `callback` during prepaint with this element's resolved bounds.
    fn on_prepaint<F>(self, callback: F) -> Self
    where
        F: FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    {
        self.child(
            canvas(
                move |bounds, window, cx| callback(bounds, window, cx),
                |_, _, _, _| {},
            )
            // Without an inset, an absolute child starts after the padding but
            // is still sized to the padding box, overhanging the far edges.
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

impl<T: ParentElement> ElementExt for T {}
