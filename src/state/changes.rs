use crate::types::changes::{EdgeChange, NodeChange};
use crate::types::edge::Edge;
use crate::types::node::Node;

/// Apply a list of node changes to the node vector.
pub fn apply_node_changes<D: Clone>(changes: &[NodeChange<D>], nodes: &mut Vec<Node<D>>) {
    for change in changes {
        match change {
            NodeChange::Position {
                id,
                position,
                dragging,
            } => {
                if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                    if let Some(pos) = position {
                        node.position = *pos;
                    }
                    if let Some(d) = dragging {
                        node.dragging = *d;
                    }
                }
            }
            NodeChange::Dimensions { id, dimensions } => {
                if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                    node.measured = *dimensions;
                    // Also update explicit width/height so that resize is
                    // reflected even when the node was created with an
                    // explicit size (width/height take priority over measured
                    // in InternalNode::width()/height()).
                    if let Some(d) = dimensions {
                        node.width = Some(d.width);
                        node.height = Some(d.height);
                    }
                }
            }
            NodeChange::Select { id, selected } => {
                if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                    node.selected = *selected;
                }
            }
            NodeChange::Remove { id } => {
                nodes.retain(|n| n.id != *id);
            }
            NodeChange::Add { node, index } => {
                if let Some(idx) = index {
                    let idx = (*idx).min(nodes.len());
                    nodes.insert(idx, node.clone());
                } else {
                    nodes.push(node.clone());
                }
            }
            NodeChange::Replace { id, node } => {
                if let Some(existing) = nodes.iter_mut().find(|n| n.id == *id) {
                    *existing = node.clone();
                }
            }
        }
    }
}

/// Apply a list of edge changes to the edge vector.
pub fn apply_edge_changes<D: Clone>(changes: &[EdgeChange<D>], edges: &mut Vec<Edge<D>>) {
    for change in changes {
        match change {
            EdgeChange::Select { id, selected } => {
                if let Some(edge) = edges.iter_mut().find(|e| e.id == *id) {
                    edge.selected = *selected;
                }
            }
            EdgeChange::Remove { id } => {
                edges.retain(|e| e.id != *id);
            }
            EdgeChange::Add { edge, index } => {
                if let Some(idx) = index {
                    let idx = (*idx).min(edges.len());
                    edges.insert(idx, edge.clone());
                } else {
                    edges.push(edge.clone());
                }
            }
            EdgeChange::Replace { id, edge } => {
                if let Some(existing) = edges.iter_mut().find(|e| e.id == *id) {
                    *existing = edge.clone();
                }
            }
            EdgeChange::Anchor {
                id,
                source_anchor,
                target_anchor,
            } => {
                if let Some(edge) = edges.iter_mut().find(|e| e.id == *id) {
                    if let Some(sa) = source_anchor {
                        edge.source_anchor = sa.clone();
                    }
                    if let Some(ta) = target_anchor {
                        edge.target_anchor = ta.clone();
                    }
                }
            }
        }
    }
}
