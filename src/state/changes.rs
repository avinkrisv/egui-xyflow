//! Change application logic.
//!
//! Pure functions that apply [`NodeChange`] and [`EdgeChange`] vectors to
//! the node/edge `Vec`s using O(1) indexed lookups.

use std::collections::HashMap;

use crate::types::changes::{EdgeChange, NodeChange};
use crate::types::edge::{Edge, EdgeId};
use crate::types::node::{Node, NodeId};

/// Apply a list of node changes to the node vector.
///
/// Uses a temporary HashMap index for O(1) lookups instead of O(n) scans.
pub fn apply_node_changes<D: Clone>(changes: &[NodeChange<D>], nodes: &mut Vec<Node<D>>) {
    if changes.is_empty() {
        return;
    }

    // Build a temporary index: NodeId → position in the vec.
    // Arc<str>-backed NodeId makes clone O(1).
    let mut index: HashMap<NodeId, usize> = HashMap::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        index.insert(node.id.clone(), i);
    }

    for change in changes {
        match change {
            NodeChange::Position {
                id,
                position,
                dragging,
            } => {
                if let Some(&idx) = index.get(id) {
                    if let Some(pos) = position {
                        nodes[idx].position = *pos;
                    }
                    if let Some(d) = dragging {
                        nodes[idx].dragging = *d;
                    }
                }
            }
            NodeChange::Dimensions { id, dimensions } => {
                if let Some(&idx) = index.get(id) {
                    nodes[idx].measured = *dimensions;
                    if let Some(d) = dimensions {
                        nodes[idx].width = Some(d.width);
                        nodes[idx].height = Some(d.height);
                    }
                }
            }
            NodeChange::Select { id, selected } => {
                if let Some(&idx) = index.get(id) {
                    nodes[idx].selected = *selected;
                }
            }
            NodeChange::Remove { id } => {
                nodes.retain(|n| n.id != *id);
                // Rebuild index — removal shifts indices.
                rebuild_node_index(&mut index, nodes);
            }
            NodeChange::Add { node, index: insert_idx } => {
                if let Some(idx) = insert_idx {
                    let idx = (*idx).min(nodes.len());
                    nodes.insert(idx, node.clone());
                } else {
                    nodes.push(node.clone());
                }
                // Rebuild index — insertion shifts indices.
                rebuild_node_index(&mut index, nodes);
            }
            NodeChange::Replace { id, node } => {
                if let Some(&idx) = index.get(id) {
                    nodes[idx] = node.clone();
                }
            }
        }
    }
}

fn rebuild_node_index<D>(index: &mut HashMap<NodeId, usize>, nodes: &[Node<D>]) {
    index.clear();
    for (i, node) in nodes.iter().enumerate() {
        index.insert(node.id.clone(), i);
    }
}

/// Apply a list of edge changes to the edge vector.
///
/// Uses a temporary HashMap index for O(1) lookups instead of O(n) scans.
pub fn apply_edge_changes<D: Clone>(changes: &[EdgeChange<D>], edges: &mut Vec<Edge<D>>) {
    if changes.is_empty() {
        return;
    }

    let mut index: HashMap<EdgeId, usize> = HashMap::with_capacity(edges.len());
    for (i, edge) in edges.iter().enumerate() {
        index.insert(edge.id.clone(), i);
    }

    for change in changes {
        match change {
            EdgeChange::Select { id, selected } => {
                if let Some(&idx) = index.get(id) {
                    edges[idx].selected = *selected;
                }
            }
            EdgeChange::Remove { id } => {
                edges.retain(|e| e.id != *id);
                rebuild_edge_index(&mut index, edges);
            }
            EdgeChange::Add { edge, index: insert_idx } => {
                if let Some(idx) = insert_idx {
                    let idx = (*idx).min(edges.len());
                    edges.insert(idx, edge.clone());
                } else {
                    edges.push(edge.clone());
                }
                rebuild_edge_index(&mut index, edges);
            }
            EdgeChange::Replace { id, edge } => {
                if let Some(&idx) = index.get(id) {
                    edges[idx] = edge.clone();
                }
            }
            EdgeChange::Anchor {
                id,
                source_anchor,
                target_anchor,
            } => {
                if let Some(&idx) = index.get(id) {
                    if let Some(sa) = source_anchor {
                        edges[idx].source_anchor = *sa;
                    }
                    if let Some(ta) = target_anchor {
                        edges[idx].target_anchor = *ta;
                    }
                }
            }
            EdgeChange::Style { id, style } => {
                if let Some(&idx) = index.get(id) {
                    edges[idx].style = *style;
                }
            }
        }
    }
}

fn rebuild_edge_index<D>(index: &mut HashMap<EdgeId, usize>, edges: &[Edge<D>]) {
    index.clear();
    for (i, edge) in edges.iter().enumerate() {
        index.insert(edge.id.clone(), i);
    }
}
