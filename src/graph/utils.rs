use crate::types::edge::Edge;
use crate::types::node::{InternalNode, NodeId};
use std::collections::HashMap;

/// Get the bounding rect of all visible nodes.
pub fn get_nodes_bounds<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>) -> egui::Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for node in node_lookup.values() {
        if node.node.hidden {
            continue;
        }
        let rect = node.rect();
        min_x = min_x.min(rect.min.x);
        min_y = min_y.min(rect.min.y);
        max_x = max_x.max(rect.max.x);
        max_y = max_y.max(rect.max.y);
    }

    if min_x == f32::INFINITY {
        return egui::Rect::NOTHING;
    }

    egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
}

/// Get nodes that are downstream of the given node.
pub fn get_outgoers<'a, ND, ED>(
    node_id: &NodeId,
    nodes: &'a HashMap<NodeId, InternalNode<ND>>,
    edges: &'a [Edge<ED>],
) -> Vec<&'a NodeId> {
    edges
        .iter()
        .filter(|e| e.source == *node_id)
        .filter_map(|e| {
            if nodes.contains_key(&e.target) {
                Some(&e.target)
            } else {
                None
            }
        })
        .collect()
}

/// Get nodes that are upstream of the given node.
pub fn get_incomers<'a, ND, ED>(
    node_id: &NodeId,
    nodes: &'a HashMap<NodeId, InternalNode<ND>>,
    edges: &'a [Edge<ED>],
) -> Vec<&'a NodeId> {
    edges
        .iter()
        .filter(|e| e.target == *node_id)
        .filter_map(|e| {
            if nodes.contains_key(&e.source) {
                Some(&e.source)
            } else {
                None
            }
        })
        .collect()
}

/// Get all edges connected to a given node.
pub fn get_connected_edges<'a, ED>(node_id: &NodeId, edges: &'a [Edge<ED>]) -> Vec<&'a Edge<ED>> {
    edges
        .iter()
        .filter(|e| e.source == *node_id || e.target == *node_id)
        .collect()
}

/// Compute viewport to fit the given bounds within a canvas rect.
pub fn get_viewport_for_bounds(
    bounds: egui::Rect,
    canvas_width: f32,
    canvas_height: f32,
    min_zoom: f32,
    max_zoom: f32,
    padding: f32,
) -> crate::types::viewport::Viewport {
    if bounds == egui::Rect::NOTHING || bounds.width() == 0.0 || bounds.height() == 0.0 {
        return crate::types::viewport::Viewport::default();
    }

    let padded_w = bounds.width() + padding * 2.0;
    let padded_h = bounds.height() + padding * 2.0;

    let zoom = (canvas_width / padded_w)
        .min(canvas_height / padded_h)
        .clamp(min_zoom, max_zoom);

    let x = (canvas_width - bounds.width() * zoom) / 2.0 - bounds.min.x * zoom;
    let y = (canvas_height - bounds.height() * zoom) / 2.0 - bounds.min.y * zoom;

    crate::types::viewport::Viewport { x, y, zoom }
}
