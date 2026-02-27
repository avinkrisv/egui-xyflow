//! Change enums for declarative graph mutations.
//!
//! [`NodeChange`] and [`EdgeChange`] represent atomic operations applied via
//! [`crate::state::flow_state::FlowState::apply_node_changes`] and
//! [`crate::state::flow_state::FlowState::apply_edge_changes`].

use super::edge::{Edge, EdgeAnchor, EdgeId, EdgeStyle};
use super::node::{Node, NodeId};
use super::position::Dimensions;

/// A mutation to apply to a node.
///
/// Pass a `Vec<NodeChange>` to [`crate::state::flow_state::FlowState::apply_node_changes`].
#[derive(Debug, Clone)]
pub enum NodeChange<D = ()> {
    /// Update position and/or dragging state.
    Position {
        id: NodeId,
        position: Option<egui::Pos2>,
        dragging: Option<bool>,
    },
    /// Update measured dimensions.
    Dimensions {
        id: NodeId,
        dimensions: Option<Dimensions>,
    },
    /// Change selection state.
    Select {
        id: NodeId,
        selected: bool,
    },
    /// Remove the node from the graph.
    Remove {
        id: NodeId,
    },
    /// Insert a new node, optionally at a specific index.
    Add {
        node: Node<D>,
        index: Option<usize>,
    },
    /// Replace a node entirely.
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
    /// Change selection state.
    Select {
        id: EdgeId,
        selected: bool,
    },
    /// Remove the edge from the graph.
    Remove {
        id: EdgeId,
    },
    /// Insert a new edge, optionally at a specific index.
    Add {
        edge: Edge<D>,
        index: Option<usize>,
    },
    /// Replace an edge entirely.
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
