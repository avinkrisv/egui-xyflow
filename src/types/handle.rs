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
    pub id: Option<String>,
    pub node_id: Arc<str>,
    pub x: f32,
    pub y: f32,
    pub position: Position,
    pub handle_type: HandleType,
    pub width: f32,
    pub height: f32,
}

impl Handle {
    pub fn center(&self) -> egui::Pos2 {
        egui::pos2(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// User-specified handle on a node (before measurement).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeHandle {
    pub id: Option<String>,
    pub handle_type: HandleType,
    pub position: Position,
    #[cfg_attr(feature = "serde", serde(default))]
    pub x: f32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub y: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl NodeHandle {
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

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}
