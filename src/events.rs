//! Per-frame event reporting for `FlowCanvas`.
//!
//! [`crate::render::canvas::FlowCanvas::show`] returns a [`FlowEvents`] value that accumulates every
//! notable thing that happened during that frame.  Because `show` is called
//! once per egui frame you can simply inspect the struct immediately after the
//! call — no callbacks or channels needed.
//!
//! # Example
//!
//! ```rust,ignore
//! let events = FlowCanvas::new(&mut state, &DefaultNodeWidget).show(ui);
//!
//! for conn in &events.connections_made {
//!     println!("new edge: {:?} → {:?}", conn.source, conn.target);
//! }
//! if events.selection_changed {
//!     println!("selection changed");
//! }
//! ```

use crate::types::connection::Connection;
use crate::types::edge::{EdgeAnchor, EdgeId};
use crate::types::node::NodeId;

// ─────────────────────────────────────────────────────────────────────────────
// FlowEvents
// ─────────────────────────────────────────────────────────────────────────────

/// All events that occurred during a single `FlowCanvas::show` call.
///
/// Every `Vec` field is empty (and every `bool` is `false`) when nothing
/// relevant happened, so you can cheaply check for activity with `.is_empty()`
/// or the boolean flags.
#[derive(Debug, Default, Clone)]
pub struct FlowEvents {
    // ── Connection events ────────────────────────────────────────────────────
    /// Connections that were successfully validated **and** created this frame.
    /// Each entry corresponds to a new edge that was added to the graph.
    pub connections_made: Vec<Connection>,

    /// A connection drag was initiated from a handle this frame.
    /// The [`NodeId`] is the source node of the in-progress connection.
    pub connection_started: Option<NodeId>,

    /// A connection drag ended this frame (either completed or cancelled).
    pub connection_ended: bool,

    // ── Node drag events ─────────────────────────────────────────────────────
    /// Nodes whose drag was **started** this frame.
    /// A node appears here on the very first frame it starts moving.
    pub nodes_drag_started: Vec<NodeId>,

    /// Nodes that were dragged this frame together with their **new** flow-space
    /// positions (after the delta has been applied and snapping resolved).
    pub nodes_dragged: Vec<(NodeId, egui::Pos2)>,

    /// Nodes whose drag was **stopped** this frame.
    pub nodes_drag_stopped: Vec<NodeId>,

    // ── Node resize events ───────────────────────────────────────────────────
    /// Nodes that were resized this frame, together with their new
    /// `(width, height)` in flow space.
    pub nodes_resized: Vec<(NodeId, f32, f32)>,

    // ── Click events ─────────────────────────────────────────────────────────
    /// Nodes that received a click (primary button, short press) this frame.
    pub nodes_clicked: Vec<NodeId>,

    /// Edges that received a click this frame.
    pub edges_clicked: Vec<EdgeId>,

    // ── Selection events ─────────────────────────────────────────────────────
    /// `true` if any node or edge selection changed this frame.
    pub selection_changed: bool,

    /// The full set of node IDs that are selected **after** this frame.
    /// Only populated when [`selection_changed`](Self::selection_changed) is
    /// `true`.
    pub selected_nodes: Vec<NodeId>,

    /// The full set of edge IDs that are selected **after** this frame.
    /// Only populated when [`selection_changed`](Self::selection_changed) is
    /// `true`.
    pub selected_edges: Vec<EdgeId>,

    // ── Delete events ────────────────────────────────────────────────────────
    /// Nodes removed from the graph this frame (Delete / Backspace key).
    pub nodes_deleted: Vec<NodeId>,

    /// Edges removed from the graph this frame (Delete / Backspace key or
    /// because a connected node was deleted).
    pub edges_deleted: Vec<EdgeId>,

    // ── Hover events ─────────────────────────────────────────────────────────
    /// The node currently under the pointer this frame, if any.
    pub node_hovered: Option<NodeId>,

    /// The edge currently under the pointer this frame, if any.
    ///
    /// Uses the same accurate bezier / smooth-step / straight hit-test as
    /// edge click selection, sampled each frame. Opt out by setting
    /// [`FlowConfig::track_edge_hover`](crate::config::FlowConfig::track_edge_hover)
    /// to `false` if the per-frame cost is a concern on very large graphs.
    pub edge_hovered: Option<EdgeId>,

    // ── Anchor events ────────────────────────────────────────────────────────
    /// Edge endpoints that were repositioned by the user this frame.
    /// Each entry is `(edge_id, new_source_anchor, new_target_anchor)`.
    pub edge_anchors_changed: Vec<(EdgeId, Option<EdgeAnchor>, Option<EdgeAnchor>)>,

    // ── Viewport events ──────────────────────────────────────────────────────
    /// `true` if the viewport was panned or zoomed this frame (instant or
    /// animated tick).
    pub viewport_changed: bool,
}

impl FlowEvents {
    /// Returns `true` if no events occurred this frame — i.e. every field is
    /// at its default empty/false value.  Useful for skipping downstream
    /// processing when the graph is quiescent.
    pub fn is_empty(&self) -> bool {
        self.connections_made.is_empty()
            && self.connection_started.is_none()
            && !self.connection_ended
            && self.nodes_drag_started.is_empty()
            && self.nodes_dragged.is_empty()
            && self.nodes_drag_stopped.is_empty()
            && self.nodes_resized.is_empty()
            && self.nodes_clicked.is_empty()
            && self.edges_clicked.is_empty()
            && !self.selection_changed
            && self.nodes_deleted.is_empty()
            && self.edges_deleted.is_empty()
            && self.node_hovered.is_none()
            && self.edge_hovered.is_none()
            && self.edge_anchors_changed.is_empty()
            && !self.viewport_changed
    }

    // ── Convenience constructors used internally ──────────────────────────────

    /// Record a newly created connection.
    pub(crate) fn push_connection(&mut self, conn: Connection) {
        self.connections_made.push(conn);
    }

    /// Record that a connection drag started from `node_id`.
    pub(crate) fn set_connection_started(&mut self, node_id: NodeId) {
        self.connection_started = Some(node_id);
    }

    /// Record that a connection drag ended.
    pub(crate) fn set_connection_ended(&mut self) {
        self.connection_ended = true;
    }

    /// Record a node drag-start event.
    pub(crate) fn push_drag_start(&mut self, id: NodeId) {
        if !self.nodes_drag_started.contains(&id) {
            self.nodes_drag_started.push(id);
        }
    }

    /// Record a node drag-move event.
    pub(crate) fn push_dragged(&mut self, id: NodeId, new_pos: egui::Pos2) {
        // Update-in-place if already recorded for this frame (last-writer wins).
        if let Some(entry) = self.nodes_dragged.iter_mut().find(|(eid, _)| *eid == id) {
            entry.1 = new_pos;
        } else {
            self.nodes_dragged.push((id, new_pos));
        }
    }

    /// Record a node drag-stop event.
    pub(crate) fn push_drag_stop(&mut self, id: NodeId) {
        if !self.nodes_drag_stopped.contains(&id) {
            self.nodes_drag_stopped.push(id);
        }
    }

    /// Record a node resize event.
    pub(crate) fn push_resized(&mut self, id: NodeId, w: f32, h: f32) {
        if let Some(entry) = self.nodes_resized.iter_mut().find(|(eid, ..)| *eid == id) {
            entry.1 = w;
            entry.2 = h;
        } else {
            self.nodes_resized.push((id, w, h));
        }
    }

    /// Record a node click.
    pub(crate) fn push_node_click(&mut self, id: NodeId) {
        if !self.nodes_clicked.contains(&id) {
            self.nodes_clicked.push(id);
        }
    }

    /// Record an edge click.
    pub(crate) fn push_edge_click(&mut self, id: EdgeId) {
        if !self.edges_clicked.contains(&id) {
            self.edges_clicked.push(id);
        }
    }

    /// Mark selection as changed and snapshot the current selected sets.
    pub(crate) fn set_selection_changed(
        &mut self,
        selected_nodes: Vec<NodeId>,
        selected_edges: Vec<EdgeId>,
    ) {
        self.selection_changed = true;
        self.selected_nodes = selected_nodes;
        self.selected_edges = selected_edges;
    }

    /// Record deleted nodes.
    pub(crate) fn push_nodes_deleted(&mut self, ids: impl IntoIterator<Item = NodeId>) {
        self.nodes_deleted.extend(ids);
    }

    /// Record deleted edges.
    pub(crate) fn push_edges_deleted(&mut self, ids: impl IntoIterator<Item = EdgeId>) {
        self.edges_deleted.extend(ids);
    }

    /// Record the currently hovered node.
    pub(crate) fn set_node_hovered(&mut self, id: NodeId) {
        self.node_hovered = Some(id);
    }

    /// Record the currently hovered edge.
    pub(crate) fn set_edge_hovered(&mut self, id: EdgeId) {
        self.edge_hovered = Some(id);
    }

    /// Record an edge anchor change.
    pub(crate) fn push_anchor_changed(
        &mut self,
        id: EdgeId,
        source: Option<EdgeAnchor>,
        target: Option<EdgeAnchor>,
    ) {
        self.edge_anchors_changed.push((id, source, target));
    }

    /// Mark the viewport as having changed.
    pub(crate) fn set_viewport_changed(&mut self) {
        self.viewport_changed = true;
    }
}
