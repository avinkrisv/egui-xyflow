//! Connection types for in-progress and completed handle-to-handle connections.

use super::handle::Handle;
use super::node::NodeId;
use super::position::Position;

/// A completed connection between two node handles.
///
/// Returned in [`crate::events::FlowEvents::connections_made`] when a user
/// finishes dragging from one handle to another.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Connection {
    pub source: NodeId,
    pub target: NodeId,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
}

/// Type-erased summary of an existing edge, passed to
/// [`crate::render::canvas::ConnectionValidator`] so validators can inspect
/// the current graph without borrowing `FlowState`.
///
/// Uses borrowed references to avoid cloning; the lifetime is tied to the
/// `FlowState` that owns the edges.
#[derive(Debug, Clone)]
pub struct EdgeInfo<'a> {
    pub source: &'a NodeId,
    pub target: &'a NodeId,
    pub source_handle: Option<&'a str>,
    pub target_handle: Option<&'a str>,
}

/// Whether connections require an exact handle target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConnectionMode {
    /// Connections must land on a compatible handle.
    #[default]
    Strict,
    /// Connections can attach to any point on a node.
    Loose,
}

/// State of an in-progress connection drag.
#[derive(Debug, Clone, Default)]
pub enum ConnectionState {
    #[default]
    None,
    InProgress {
        is_valid: Option<bool>,
        from: egui::Pos2,
        from_handle: Handle,
        from_position: Position,
        from_node_id: NodeId,
        to: egui::Pos2,
        to_handle: Box<Option<Handle>>,
        to_position: Position,
        to_node_id: Option<NodeId>,
    },
}
