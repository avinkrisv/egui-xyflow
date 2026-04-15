//! Handle types for node connection points.
//!
//! [`NodeHandle`] is the user-facing type placed on nodes; [`Handle`] is the
//! resolved version with absolute coordinates used internally.

use std::sync::Arc;

use super::position::Position;

/// Whether a handle is a source (output) or target (input) for connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HandleType {
    /// Output handle — connections start here.
    Source,
    /// Input handle — connections end here.
    Target,
}

/// A resolved handle with absolute position within a node.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Handle {
    /// Optional handle identifier; distinguishes multiple handles of the same
    /// [`HandleType`] on a node. `None` when the node has a single unnamed handle.
    pub id: Option<String>,
    /// Id of the node that owns this handle.
    pub node_id: Arc<str>,
    /// X offset of the handle's top-left corner relative to the node origin, in flow space.
    pub x: f32,
    /// Y offset of the handle's top-left corner relative to the node origin, in flow space.
    pub y: f32,
    /// Which side of the node the handle sits on.
    pub position: Position,
    /// Whether this handle is a source or target.
    pub handle_type: HandleType,
    /// Handle hit-area width in flow-space pixels.
    pub width: f32,
    /// Handle hit-area height in flow-space pixels.
    pub height: f32,
}

impl Handle {
    /// Return the center point of this handle relative to its node.
    pub fn center(&self) -> egui::Pos2 {
        egui::pos2(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// User-specified handle on a node (before measurement).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeHandle {
    /// Optional handle identifier; required when a node has multiple handles
    /// of the same [`HandleType`] so edges can target a specific one.
    pub id: Option<String>,
    /// Whether this handle accepts outgoing (`Source`) or incoming (`Target`) connections.
    pub handle_type: HandleType,
    /// Which side of the node this handle is placed on.
    pub position: Position,
    /// Optional manual X offset relative to the node origin. Defaults to `0.0`,
    /// in which case [`Position`] drives placement along its side.
    #[cfg_attr(feature = "serde", serde(default))]
    pub x: f32,
    /// Optional manual Y offset relative to the node origin. Defaults to `0.0`,
    /// in which case [`Position`] drives placement along its side.
    #[cfg_attr(feature = "serde", serde(default))]
    pub y: f32,
    /// Override for the handle's hit-area width; falls back to `FlowConfig`.
    pub width: Option<f32>,
    /// Override for the handle's hit-area height; falls back to `FlowConfig`.
    pub height: Option<f32>,
}

impl NodeHandle {
    /// Create a source (output) handle at the given position.
    pub fn source(position: Position) -> Self {
        Self {
            id: None,
            handle_type: HandleType::Source,
            position,
            x: 0.0,
            y: 0.0,
            width: None,
            height: None,
        }
    }

    /// Create a target (input) handle at the given position.
    pub fn target(position: Position) -> Self {
        Self {
            id: None,
            handle_type: HandleType::Target,
            position,
            x: 0.0,
            y: 0.0,
            width: None,
            height: None,
        }
    }

    /// Assign an identifier to this handle for multi-handle nodes.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}
