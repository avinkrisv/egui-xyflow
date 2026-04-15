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
        /// Node to update.
        id: NodeId,
        /// New flow-space position; `None` leaves it unchanged.
        position: Option<egui::Pos2>,
        /// New dragging flag; `None` leaves it unchanged.
        dragging: Option<bool>,
    },
    /// Update measured dimensions.
    Dimensions {
        /// Node to update.
        id: NodeId,
        /// Dimensions reported by the renderer; `None` clears the measurement.
        dimensions: Option<Dimensions>,
    },
    /// Change selection state.
    Select {
        /// Node to update.
        id: NodeId,
        /// New selection state.
        selected: bool,
    },
    /// Remove the node from the graph.
    Remove {
        /// Node to remove.
        id: NodeId,
    },
    /// Insert a new node, optionally at a specific index.
    Add {
        /// Node to insert.
        node: Node<D>,
        /// Index in the node list; `None` appends.
        index: Option<usize>,
    },
    /// Replace a node entirely.
    Replace {
        /// Id of the node to replace.
        id: NodeId,
        /// Replacement node; its own id need not match `id`.
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
        /// Edge to update.
        id: EdgeId,
        /// New selection state.
        selected: bool,
    },
    /// Remove the edge from the graph.
    Remove {
        /// Edge to remove.
        id: EdgeId,
    },
    /// Insert a new edge, optionally at a specific index.
    Add {
        /// Edge to insert.
        edge: Edge<D>,
        /// Index in the edge list; `None` appends.
        index: Option<usize>,
    },
    /// Replace an edge entirely.
    Replace {
        /// Id of the edge to replace.
        id: EdgeId,
        /// Replacement edge; its own id need not match `id`.
        edge: Edge<D>,
    },
    /// Update edge endpoint anchors. `Some(Some(..))` sets an anchor,
    /// `Some(None)` clears it, `None` leaves it unchanged.
    Anchor {
        /// Edge to update.
        id: EdgeId,
        /// Source anchor patch (outer `None` = leave alone, inner `None` = clear).
        source_anchor: Option<Option<EdgeAnchor>>,
        /// Target anchor patch (outer `None` = leave alone, inner `None` = clear).
        target_anchor: Option<Option<EdgeAnchor>>,
    },
    /// Update per-edge visual style. `Some(Some(..))` sets a style,
    /// `Some(None)` clears it back to config defaults.
    Style {
        /// Edge to update.
        id: EdgeId,
        /// Replacement style; `None` clears the per-edge override.
        style: Option<EdgeStyle>,
    },
}
