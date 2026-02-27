use super::handle::Handle;
use super::node::NodeId;
use super::position::Position;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Connection {
    pub source: NodeId,
    pub target: NodeId,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
}

/// Type-erased summary of an existing edge, passed to [`ConnectionValidator`]
/// so validators can inspect the current graph without borrowing `FlowState`.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConnectionMode {
    Strict,
    Loose,
}

impl Default for ConnectionMode {
    fn default() -> Self {
        ConnectionMode::Strict
    }
}

/// State of an in-progress connection drag.
#[derive(Debug, Clone)]
pub enum ConnectionState {
    None,
    InProgress {
        is_valid: Option<bool>,
        from: egui::Pos2,
        from_handle: Handle,
        from_position: Position,
        from_node_id: NodeId,
        to: egui::Pos2,
        to_handle: Option<Handle>,
        to_position: Position,
        to_node_id: Option<NodeId>,
    },
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::None
    }
}
