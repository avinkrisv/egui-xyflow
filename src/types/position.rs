/// Handle/edge positioning relative to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Position {
    Top,
    Right,
    Bottom,
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

    pub fn is_horizontal(self) -> bool {
        matches!(self, Position::Left | Position::Right)
    }

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
    pub width: f32,
    pub height: f32,
}

impl Dimensions {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Coordinate extent defining min/max bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CoordinateExtent {
    pub min: egui::Pos2,
    pub max: egui::Pos2,
}

impl CoordinateExtent {
    pub const INFINITE: Self = Self {
        min: egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY),
        max: egui::pos2(f32::INFINITY, f32::INFINITY),
    };

    pub fn new(min: egui::Pos2, max: egui::Pos2) -> Self {
        Self { min, max }
    }
}

/// Viewport transform: translation (x, y) and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform {
    pub x: f32,
    pub y: f32,
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
