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
    /// Node where the connection originated (the source handle's node).
    pub source: NodeId,
    /// Node where the connection ended (the target handle's node).
    pub target: NodeId,
    /// Id of the specific source handle, if the source node has multiple handles.
    pub source_handle: Option<String>,
    /// Id of the specific target handle, if the target node has multiple handles.
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
    /// Source node of the existing edge.
    pub source: &'a NodeId,
    /// Target node of the existing edge.
    pub target: &'a NodeId,
    /// Source handle id, if any.
    pub source_handle: Option<&'a str>,
    /// Target handle id, if any.
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
    /// No connection is being drawn.
    #[default]
    None,
    /// A connection drag is active.
    InProgress {
        /// Validation result from the [`ConnectionValidator`](crate::render::canvas::ConnectionValidator), if one is registered.
        /// `None` means no validator was consulted (treated as valid).
        is_valid: Option<bool>,
        /// Flow-space position the drag started from (center of the source handle).
        from: egui::Pos2,
        /// The source handle the drag originated from.
        from_handle: Handle,
        /// Which side of the source node the drag originates on.
        from_position: Position,
        /// Id of the source node.
        from_node_id: NodeId,
        /// Current flow-space position of the drag end (cursor or snapped handle).
        to: egui::Pos2,
        /// The handle currently being hovered, if any. Boxed to keep the enum compact.
        to_handle: Box<Option<Handle>>,
        /// Side of the hovered target node the connection would attach to.
        to_position: Position,
        /// Id of the hovered target node, if the cursor is over one.
        to_node_id: Option<NodeId>,
    },
}
