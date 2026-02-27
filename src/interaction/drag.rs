use crate::graph::node_position::snap_position;
use crate::types::changes::NodeChange;
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::Transform;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Single-node drag
// ─────────────────────────────────────────────────────────────────────────────

/// Process node drag for a single node. Returns position change if dragged.
pub fn handle_node_drag<D>(
    node_id: &NodeId,
    drag_delta: egui::Vec2,
    transform: &Transform,
    snap_to_grid: bool,
    snap_grid: &[f32; 2],
    current_pos: egui::Pos2,
) -> Option<NodeChange<D>> {
    if drag_delta == egui::Vec2::ZERO {
        return None;
    }

    // Convert screen delta to flow delta
    let flow_delta = egui::vec2(
        drag_delta.x / transform.scale,
        drag_delta.y / transform.scale,
    );

    let mut new_pos = egui::pos2(current_pos.x + flow_delta.x, current_pos.y + flow_delta.y);

    if snap_to_grid {
        new_pos = snap_position(new_pos, snap_grid);
    }

    Some(NodeChange::Position {
        id: node_id.clone(),
        position: Some(new_pos),
        dragging: Some(true),
    })
}

/// Generate drag-end change for a single node.
pub fn handle_node_drag_end<D>(node_id: &NodeId) -> NodeChange<D> {
    NodeChange::Position {
        id: node_id.clone(),
        position: None,
        dragging: Some(false),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Multi-node drag
// ─────────────────────────────────────────────────────────────────────────────

/// Compute position changes for **all** selected nodes when one selected node
/// is being dragged.
///
/// `primary_id` is the node the user is physically dragging.  If it is
/// selected we also move every other selected node by the same flow-space
/// delta, producing a unified multi-selection drag.
///
/// If `primary_id` is NOT selected (or there is only one selected node) the
/// function returns an empty `Vec` so the caller can fall back to the normal
/// single-node drag path.
pub fn handle_multi_node_drag<D>(
    primary_id: &NodeId,
    drag_delta: egui::Vec2,
    transform: &Transform,
    snap_to_grid: bool,
    snap_grid: &[f32; 2],
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
) -> Vec<NodeChange<D>> {
    if drag_delta == egui::Vec2::ZERO {
        return Vec::new();
    }

    // Only trigger multi-drag when the dragged node is itself selected.
    let primary_selected = node_lookup
        .get(primary_id)
        .map(|n| n.node.selected)
        .unwrap_or(false);

    if !primary_selected {
        return Vec::new();
    }

    // Collect all selected nodes (including the primary).
    let selected: Vec<(&NodeId, egui::Pos2)> = node_lookup
        .iter()
        .filter(|(_, n)| n.node.selected && !n.node.hidden)
        .map(|(id, n)| (id, n.internals.position_absolute))
        .collect();

    // A multi-drag is only meaningful when more than one node is selected.
    // With a single selection the normal single-node code path is fine, but
    // returning changes here is harmless (and slightly redundant — we do it
    // anyway so the caller can always use this result when multi-drag fires).
    if selected.is_empty() {
        return Vec::new();
    }

    let flow_delta = egui::vec2(
        drag_delta.x / transform.scale,
        drag_delta.y / transform.scale,
    );

    selected
        .into_iter()
        .filter_map(|(id, flow_pos)| {
            let can_drag = node_lookup
                .get(id)
                .and_then(|n| n.node.draggable)
                .unwrap_or(true);

            if !can_drag {
                return None;
            }

            let mut new_pos = egui::pos2(flow_pos.x + flow_delta.x, flow_pos.y + flow_delta.y);

            if snap_to_grid {
                new_pos = snap_position(new_pos, snap_grid);
            }

            Some(NodeChange::Position {
                id: id.clone(),
                position: Some(new_pos),
                dragging: Some(true),
            })
        })
        .collect()
}

/// Generate drag-end changes for all currently-dragging nodes.
///
/// Call this when a drag gesture finishes so that every dragging node has its
/// `dragging` flag cleared.
pub fn handle_multi_node_drag_end<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
) -> Vec<NodeChange<D>> {
    node_lookup
        .iter()
        .filter(|(_, n)| n.node.dragging)
        .map(|(id, _)| NodeChange::Position {
            id: id.clone(),
            position: None,
            dragging: Some(false),
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Coordinate helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a screen-space drag delta to a flow-space delta given the current
/// viewport transform (zoom).
#[inline]
pub fn screen_delta_to_flow(delta: egui::Vec2, transform: &Transform) -> egui::Vec2 {
    egui::vec2(delta.x / transform.scale, delta.y / transform.scale)
}
