use super::edge::{Edge, EdgeAnchor, EdgeId, EdgeStyle};
use super::node::{Node, NodeId};
use super::position::Dimensions;

/// A mutation to apply to a node.
///
/// Pass a `Vec<NodeChange>` to [`crate::state::flow_state::FlowState::apply_node_changes`].
#[derive(Debug, Clone)]
pub enum NodeChange<D = ()> {
    Position {
        id: NodeId,
        position: Option<egui::Pos2>,
        dragging: Option<bool>,
    },
    Dimensions {
        id: NodeId,
        dimensions: Option<Dimensions>,
    },
    Select {
        id: NodeId,
        selected: bool,
    },
    Remove {
        id: NodeId,
    },
    Add {
        node: Node<D>,
        index: Option<usize>,
    },
    Replace {
        id: NodeId,
        node: Node<D>,
    },
}

/// A mutation to apply to an edge.
///
/// Pass a `Vec<EdgeChange>` to [`crate::state::flow_state::FlowState::apply_edge_changes`].
#[derive(Debug, Clone)]
pub enum EdgeChange<D = ()> {
    Select {
        id: EdgeId,
        selected: bool,
    },
    Remove {
        id: EdgeId,
    },
    Add {
        edge: Edge<D>,
        index: Option<usize>,
    },
    Replace {
        id: EdgeId,
        edge: Edge<D>,
    },
    /// Update edge endpoint anchors. `Some(Some(..))` sets an anchor,
    /// `Some(None)` clears it, `None` leaves it unchanged.
    Anchor {
        id: EdgeId,
        source_anchor: Option<Option<EdgeAnchor>>,
        target_anchor: Option<Option<EdgeAnchor>>,
    },
    /// Update per-edge visual style. `Some(Some(..))` sets a style,
    /// `Some(None)` clears it back to config defaults.
    Style {
        id: EdgeId,
        style: Option<EdgeStyle>,
    },
}
