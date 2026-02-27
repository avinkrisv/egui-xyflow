//! Easing functions for smooth viewport transitions.

/// Linear easing (no easing).
pub fn ease_linear(t: f32) -> f32 {
    t
}

/// Cubic ease-in-out (matches xyflow's d3-ease default).
pub fn ease_cubic(t: f32) -> f32 {
    let t2 = t * 2.0;
    if t2 <= 1.0 {
        t2 * t2 * t2 / 2.0
    } else {
        let t3 = t2 - 2.0;
        (t3 * t3 * t3 + 2.0) / 2.0
    }
}

/// Quadratic ease-in.
pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}

/// Quadratic ease-out.
pub fn ease_out_quad(t: f32) -> f32 {
    t * (2.0 - t)
}

/// Quadratic ease-in-out.
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}
