use crate::graph::node_position::screen_to_flow;
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::Transform;
use crate::types::viewport::SelectionMode;
use std::collections::HashMap;

/// Get nodes inside a selection rectangle.
pub fn get_nodes_inside<D>(
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    selection_rect: egui::Rect,
    transform: &Transform,
    mode: SelectionMode,
) -> Vec<NodeId> {
    // Convert selection rect from screen to flow coords
    let flow_min = screen_to_flow(selection_rect.min, transform);
    let flow_max = screen_to_flow(selection_rect.max, transform);
    let flow_rect = egui::Rect::from_min_max(
        egui::pos2(flow_min.x.min(flow_max.x), flow_min.y.min(flow_max.y)),
        egui::pos2(flow_min.x.max(flow_max.x), flow_min.y.max(flow_max.y)),
    );

    node_lookup
        .iter()
        .filter(|(_, node)| {
            if node.node.hidden {
                return false;
            }
            if node.node.selectable == Some(false) {
                return false;
            }
            let node_rect = node.rect();
            let overlap = get_overlap_area(flow_rect, node_rect);
            match mode {
                SelectionMode::Partial => overlap > 0.0,
                SelectionMode::Full => {
                    let node_area = node_rect.width() * node_rect.height();
                    node_area > 0.0 && overlap >= node_area - 0.1
                }
            }
        })
        .map(|(id, _)| id.clone())
        .collect()
}

fn get_overlap_area(a: egui::Rect, b: egui::Rect) -> f32 {
    let x_overlap = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
    let y_overlap = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0);
    x_overlap * y_overlap
}
