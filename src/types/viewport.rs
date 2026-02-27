//! Viewport state and related enums for pan/zoom control.

use super::position::Transform;

/// The current viewport state: pan offset and zoom level.
///
/// Modify this to programmatically pan or zoom the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Viewport {
    /// Horizontal pan offset in screen pixels.
    pub x: f32,
    /// Vertical pan offset in screen pixels.
    pub y: f32,
    /// Zoom factor (1.0 = 100%).
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, zoom: 1.0 }
    }
}

impl Viewport {
    /// Create a new viewport with explicit pan and zoom values.
    pub fn new(x: f32, y: f32, zoom: f32) -> Self {
        Self { x, y, zoom }
    }

    /// Convert to the internal [`Transform`] used by the rendering pipeline.
    pub fn to_transform(&self) -> Transform {
        Transform { x: self.x, y: self.y, scale: self.zoom }
    }
}

/// Controls which axis panning responds to when `pan_on_scroll` is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PanOnScrollMode {
    /// Pan freely in both axes.
    Free,
    /// Pan only vertically.
    Vertical,
    /// Pan only horizontally.
    Horizontal,
}

/// How much of a node must be inside the selection rectangle to be selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SelectionMode {
    /// Select if any part of the node overlaps the rectangle.
    Partial,
    /// Select only if the entire node is inside the rectangle.
    Full,
}
