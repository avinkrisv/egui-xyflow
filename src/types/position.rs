//! Positioning primitives: [`Position`], [`Dimensions`], [`Transform`], and related types.

/// Handle/edge positioning relative to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Position {
    /// Top edge of the node.
    Top,
    /// Right edge of the node.
    Right,
    /// Bottom edge of the node.
    Bottom,
    /// Left edge of the node.
    Left,
    /// Edges connect to the center of the node.
    ///
    /// Use this for graph visualisations (force-directed, radial, etc.) where
    /// nodes are small shapes and edges should radiate in any direction rather
    /// than snapping to a particular side.
    Center,
    /// Edge endpoint automatically tracks the closest border side to the other
    /// connected node. Resolved dynamically each frame based on relative node
    /// positions.
    Closest,
}

impl Position {
    /// Return the opposite side (Top<->Bottom, Left<->Right).
    pub fn opposite(self) -> Self {
        match self {
            Position::Top => Position::Bottom,
            Position::Bottom => Position::Top,
            Position::Left => Position::Right,
            Position::Right => Position::Left,
            Position::Center => Position::Center,
            Position::Closest => Position::Closest,
        }
    }

    /// Return `true` for `Left` or `Right`.
    pub fn is_horizontal(self) -> bool {
        matches!(self, Position::Left | Position::Right)
    }

    /// Return `true` for `Top` or `Bottom`.
    pub fn is_vertical(self) -> bool {
        matches!(self, Position::Top | Position::Bottom)
    }

    /// Resolve `Closest` into a concrete side based on relative node centers.
    /// Returns `self` unchanged for all other variants.
    pub fn resolve_closest(self, from_center: egui::Pos2, to_center: egui::Pos2) -> Position {
        match self {
            Position::Closest => {
                let dx = to_center.x - from_center.x;
                let dy = to_center.y - from_center.y;
                if dx.abs() > dy.abs() {
                    if dx > 0.0 { Position::Right } else { Position::Left }
                } else if dy > 0.0 {
                    Position::Bottom
                } else {
                    Position::Top
                }
            }
            other => other,
        }
    }
}

/// Dimensions of a node or element.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dimensions {
    /// Width in flow-space units (pixels at scale 1.0).
    pub width: f32,
    /// Height in flow-space units (pixels at scale 1.0).
    pub height: f32,
}

impl Dimensions {
    /// Create new dimensions.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Coordinate extent defining min/max bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CoordinateExtent {
    /// Top-left corner of the extent, in flow space.
    pub min: egui::Pos2,
    /// Bottom-right corner of the extent, in flow space.
    pub max: egui::Pos2,
}

impl CoordinateExtent {
    /// An unbounded extent spanning `±∞` on both axes.
    pub const INFINITE: Self = Self {
        min: egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY),
        max: egui::pos2(f32::INFINITY, f32::INFINITY),
    };

    /// Create a new extent from the top-left and bottom-right corners.
    pub fn new(min: egui::Pos2, max: egui::Pos2) -> Self {
        Self { min, max }
    }
}

/// Viewport transform: translation (x, y) and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform {
    /// Horizontal translation, in screen pixels, applied to flow-space coordinates.
    pub x: f32,
    /// Vertical translation, in screen pixels, applied to flow-space coordinates.
    pub y: f32,
    /// Zoom factor. `1.0` is identity; `>1.0` zooms in, `<1.0` zooms out.
    pub scale: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
        }
    }
}

/// Snap grid spacing [x, y].
pub type SnapGrid = [f32; 2];

/// Node origin offset [x, y], normalized 0.0-1.0.
pub type NodeOrigin = [f32; 2];
