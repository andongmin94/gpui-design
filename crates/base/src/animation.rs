// Modified for GPUI Design: retain only scrollbar easing from GPUI Kit.
/// Cubic ease-out — fast start, slow end. Good for enter animations.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in — slow start, fast end. Good for exit animations.
pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}
