//! Node lookup helpers — query functions that operate on the
//! `HashMap<NodeId, InternalNode<D>>` maintained by [`crate::state::flow_state::FlowState`].
//!
//! All functions here are pure (they take the lookup by reference and return
//! data); mutation lives in [`super::flow_state`].

use std::collections::HashMap;

use crate::types::edge::Edge;
use crate::types::handle::HandleType;
use crate::types::node::{InternalNode, NodeId};

// ─────────────────────────────────────────────────────────────────────────────
// Single-node queries
// ─────────────────────────────────────────────────────────────────────────────

/// Return the absolute (world-space) position of a node, or `None` if the id
/// is not found.
pub fn get_node_position<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    id: &NodeId,
) -> Option<egui::Pos2> {
    node_lookup.get(id).map(|n| n.internals.position_absolute)
}

/// Return the bounding rect of a single node in flow space, or `None`.
pub fn get_node_rect<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    id: &NodeId,
) -> Option<egui::Rect> {
    node_lookup.get(id).map(|n| n.rect())
}

/// Return `true` if a node with the given id exists and is not hidden.
pub fn is_node_visible<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>, id: &NodeId) -> bool {
    node_lookup.get(id).map(|n| !n.node.hidden).unwrap_or(false)
}

/// Return `true` if the node is currently selected.
pub fn is_node_selected<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>, id: &NodeId) -> bool {
    node_lookup
        .get(id)
        .map(|n| n.node.selected)
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Multi-node queries
// ─────────────────────────────────────────────────────────────────────────────

/// Return all node IDs, sorted by z-index ascending (lowest drawn first).
pub fn sorted_by_z<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>) -> Vec<NodeId> {
    let mut ids = Vec::with_capacity(node_lookup.len());
    ids.extend(node_lookup.keys().cloned());
    ids.sort_by_key(|id| node_lookup.get(id).map(|n| n.internals.z).unwrap_or(0));
    ids
}

/// Return the IDs of all currently selected nodes.
pub fn selected_node_ids<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>) -> Vec<NodeId> {
    node_lookup
        .iter()
        .filter(|(_, n)| n.node.selected)
        .map(|(id, _)| id.clone())
        .collect()
}

/// Return the IDs of all visible (non-hidden) nodes.
pub fn visible_node_ids<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>) -> Vec<NodeId> {
    node_lookup
        .iter()
        .filter(|(_, n)| !n.node.hidden)
        .map(|(id, _)| id.clone())
        .collect()
}

/// Return the combined bounding rect of all visible nodes, or
/// `egui::Rect::NOTHING` if there are none.
pub fn bounding_rect<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>) -> egui::Rect {
    node_lookup
        .values()
        .filter(|n| !n.node.hidden)
        .fold(egui::Rect::NOTHING, |acc, n| acc.union(n.rect()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph traversal
// ─────────────────────────────────────────────────────────────────────────────

/// Return the IDs of all nodes that have an edge **from** `node_id`.
pub fn outgoing_node_ids<'a, ND, ED>(
    node_id: &NodeId,
    node_lookup: &'a HashMap<NodeId, InternalNode<ND>>,
    edges: &'a [Edge<ED>],
) -> Vec<&'a NodeId> {
    edges
        .iter()
        .filter(|e| &e.source == node_id)
        .filter_map(|e| node_lookup.get_key_value(&e.target).map(|(k, _)| k))
        .collect()
}

/// Return the IDs of all nodes that have an edge **to** `node_id`.
pub fn incoming_node_ids<'a, ND, ED>(
    node_id: &NodeId,
    node_lookup: &'a HashMap<NodeId, InternalNode<ND>>,
    edges: &'a [Edge<ED>],
) -> Vec<&'a NodeId> {
    edges
        .iter()
        .filter(|e| &e.target == node_id)
        .filter_map(|e| node_lookup.get_key_value(&e.source).map(|(k, _)| k))
        .collect()
}

/// Return all edges that are connected to `node_id` (as source or target).
pub fn connected_edges<'a, ED>(node_id: &NodeId, edges: &'a [Edge<ED>]) -> Vec<&'a Edge<ED>> {
    edges
        .iter()
        .filter(|e| &e.source == node_id || &e.target == node_id)
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Handle queries
// ─────────────────────────────────────────────────────────────────────────────

/// Return the screen-space center of a named handle on a node, or `None` if
/// the node / handle is not found.
///
/// `handle_id` — `None` matches the first handle of the given type.
pub fn handle_screen_center<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    node_id: &NodeId,
    handle_type: HandleType,
    handle_id: Option<&str>,
    transform: &crate::types::position::Transform,
) -> Option<egui::Pos2> {
    use crate::graph::node_position::flow_to_screen;

    let node = node_lookup.get(node_id)?;
    let handles = match handle_type {
        HandleType::Source => &node.internals.handle_bounds.source,
        HandleType::Target => &node.internals.handle_bounds.target,
    };
    let handle = if let Some(hid) = handle_id {
        handles.iter().find(|h| h.id.as_deref() == Some(hid))?
    } else {
        handles.first()?
    };

    let abs = node.internals.position_absolute;
    let flow_center = egui::pos2(
        abs.x + handle.x + handle.width / 2.0,
        abs.y + handle.y + handle.height / 2.0,
    );
    Some(flow_to_screen(flow_center, transform))
}

/// Return `true` if `node_id` has at least one source handle.
pub fn has_source_handle<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    node_id: &NodeId,
) -> bool {
    node_lookup
        .get(node_id)
        .map(|n| !n.internals.handle_bounds.source.is_empty())
        .unwrap_or(false)
}

/// Return `true` if `node_id` has at least one target handle.
pub fn has_target_handle<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    node_id: &NodeId,
) -> bool {
    node_lookup
        .get(node_id)
        .map(|n| !n.internals.handle_bounds.target.is_empty())
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::node::{Node, NodeHandleBounds, NodeInternals};

    fn make_lookup() -> HashMap<NodeId, InternalNode<()>> {
        let mut m = HashMap::new();
        for (id_str, x, y, z, selected, hidden) in [
            ("a", 0.0_f32, 0.0_f32, 0_i32, false, false),
            ("b", 100.0, 0.0, 1, true, false),
            ("c", 200.0, 0.0, 0, false, true),
        ] {
            let id = NodeId::new(id_str);
            let mut node: Node<()> = Node::builder(id_str).build();
            node.selected = selected;
            node.hidden = hidden;
            node.width = Some(100.0);
            node.height = Some(40.0);
            m.insert(
                id.clone(),
                InternalNode {
                    node,
                    internals: NodeInternals {
                        position_absolute: egui::pos2(x, y),
                        z,
                        handle_bounds: NodeHandleBounds::default(),
                        shape: crate::types::position::NodeShape::Rect,
                    },
                },
            );
        }
        m
    }

    #[test]
    fn test_get_node_position() {
        let lk = make_lookup();
        assert_eq!(
            get_node_position(&lk, &NodeId::new("a")),
            Some(egui::pos2(0.0, 0.0))
        );
        assert_eq!(get_node_position(&lk, &NodeId::new("z")), None);
    }

    #[test]
    fn test_visible_node_ids() {
        let lk = make_lookup();
        let visible = visible_node_ids(&lk);
        assert!(visible.contains(&NodeId::new("a")));
        assert!(visible.contains(&NodeId::new("b")));
        assert!(
            !visible.contains(&NodeId::new("c")),
            "hidden node should be excluded"
        );
    }

    #[test]
    fn test_selected_node_ids() {
        let lk = make_lookup();
        let sel = selected_node_ids(&lk);
        assert_eq!(sel, vec![NodeId::new("b")]);
    }

    #[test]
    fn test_sorted_by_z() {
        let lk = make_lookup();
        let sorted = sorted_by_z(&lk);
        // "a" (z=0) and "c" (z=0) come before "b" (z=1)
        let pos_b = sorted.iter().position(|id| id.as_str() == "b").unwrap();
        let pos_a = sorted.iter().position(|id| id.as_str() == "a").unwrap();
        assert!(pos_a < pos_b);
    }

    #[test]
    fn test_bounding_rect() {
        let lk = make_lookup();
        let r = bounding_rect(&lk);
        // "c" is hidden, so only "a" (0,0 → 100,40) and "b" (100,0 → 200,40)
        assert_eq!(r.min, egui::pos2(0.0, 0.0));
        assert_eq!(r.max, egui::pos2(200.0, 40.0));
    }
}
