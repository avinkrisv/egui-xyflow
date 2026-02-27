use super::position::Transform;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, zoom: 1.0 }
    }
}

impl Viewport {
    pub fn new(x: f32, y: f32, zoom: f32) -> Self {
        Self { x, y, zoom }
    }

    pub fn to_transform(&self) -> Transform {
        Transform { x: self.x, y: self.y, scale: self.zoom }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PanOnScrollMode {
    Free,
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SelectionMode {
    Partial,
    Full,
}
