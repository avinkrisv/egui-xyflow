use crate::types::viewport::Viewport;

/// Animates viewport transitions with easing.
#[derive(Clone)]
pub struct ViewportAnimation {
    pub from: Viewport,
    pub to: Viewport,
    pub duration: f32,
    pub start_time: f64,
    pub ease: fn(f32) -> f32,
    pub active: bool,
}

impl ViewportAnimation {
    pub fn new(
        from: Viewport,
        to: Viewport,
        duration: f32,
        start_time: f64,
        ease: fn(f32) -> f32,
    ) -> Self {
        Self {
            from,
            to,
            duration,
            start_time,
            ease,
            active: true,
        }
    }

    /// Tick the animation, returns interpolated viewport.
    pub fn tick(&mut self, current_time: f64) -> Viewport {
        if !self.active {
            return self.to;
        }
        let elapsed = (current_time - self.start_time) as f32;
        let t = (elapsed / self.duration).clamp(0.0, 1.0);
        let eased = (self.ease)(t);
        if t >= 1.0 {
            self.active = false;
            return self.to;
        }
        Viewport {
            x: self.from.x + (self.to.x - self.from.x) * eased,
            y: self.from.y + (self.to.y - self.from.y) * eased,
            zoom: self.from.zoom + (self.to.zoom - self.from.zoom) * eased,
        }
    }
}
