//! Handle rendering (connection point circles on nodes).

use crate::config::FlowConfig;
use crate::edges::positions::get_handle_absolute_position;
use crate::graph::node_position::flow_to_screen;
use crate::types::node::InternalNode;
use crate::types::position::Transform;

/// Render handles for a node. Returns handle rects for hit-testing.
pub(crate) fn render_handles<D>(
    painter: &egui::Painter,
    node: &InternalNode<D>,
    transform: &Transform,
    config: &FlowConfig,
    hovered_node: bool,
    pointer_pos: Option<egui::Pos2>,
) -> Vec<HandleHitRect> {
    let source_count = node.internals.handle_bounds.source.len();
    let target_count = node.internals.handle_bounds.target.len();
    let mut hit_rects = Vec::with_capacity(source_count + target_count);
    let handle_radius = config.handle_size * 0.5 * transform.scale;

    let all_handles = node
        .internals
        .handle_bounds
        .source
        .iter()
        .chain(node.internals.handle_bounds.target.iter());

    for handle in all_handles {
        let abs_pos = get_handle_absolute_position(node, handle);
        let screen_pos = flow_to_screen(abs_pos, transform);

        // Closest handles are invisible — still register a hit rect for
        // connection dragging but skip the visual circle.
        if handle.position != crate::types::position::Position::Closest {
            let is_hovered = pointer_pos
                .map(|p| p.distance(screen_pos) < handle_radius * 2.0)
                .unwrap_or(false);

            let color = if is_hovered {
                config.handle_hover_color
            } else if hovered_node {
                config.handle_color
            } else {
                // Semi-transparent when node not hovered
                let c = config.handle_color;
                egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 128)
            };

            painter.circle_filled(screen_pos, handle_radius, color);
            painter.circle_stroke(
                screen_pos,
                handle_radius,
                egui::Stroke::new(1.0, egui::Color32::WHITE),
            );
        }

        hit_rects.push(HandleHitRect {
            screen_center: screen_pos,
            radius: handle_radius * 1.5, // Slightly larger hit area
            handle: handle.clone(),
        });
    }

    hit_rects
}

#[derive(Debug, Clone)]
pub(crate) struct HandleHitRect {
    pub(crate) screen_center: egui::Pos2,
    pub(crate) radius: f32,
    pub(crate) handle: crate::types::handle::Handle,
}

impl HandleHitRect {
    pub(crate) fn contains(&self, pos: egui::Pos2) -> bool {
        pos.distance(self.screen_center) <= self.radius
    }
}
