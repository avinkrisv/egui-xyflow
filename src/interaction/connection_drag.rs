use crate::edges::positions::get_handle_absolute_position;
use crate::types::connection::ConnectionMode;
use crate::types::handle::{Handle, HandleType};
use crate::types::node::{InternalNode, NodeId};
use std::collections::HashMap;

/// Find the closest handle to a position within a given radius.
///
/// In `Strict` mode (the default) only opposite-type handles are considered
/// valid targets: source handles may only connect to target handles and vice
/// versa.
///
/// In `Loose` mode any handle on any node (except the originating handle
/// itself) is a valid target, regardless of type.
pub fn get_closest_handle<D>(
    position: egui::Pos2,
    connection_radius: f32,
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    from_handle: &Handle,
    mode: ConnectionMode,
) -> Option<Handle> {
    let mut best: Option<Handle> = None;
    let mut best_dist = f32::INFINITY;

    // Expand the coarse search box a bit beyond the connection radius so that
    // handles slightly outside the per-node bounding box are still reachable.
    let search_radius = connection_radius + 250.0;

    // The "preferred" handle type is the opposite of the from-handle.  In
    // Strict mode only this type is accepted; in Loose mode any type is
    // accepted but the opposite type is still preferred when equidistant.
    let preferred_type = match from_handle.handle_type {
        HandleType::Source => HandleType::Target,
        HandleType::Target => HandleType::Source,
    };

    for node in node_lookup.values() {
        if node.node.hidden {
            continue;
        }

        // Quick bounding-box pre-filter to skip distant nodes cheaply.
        let node_rect = node.rect();
        let expanded = node_rect.expand(search_radius);
        if !expanded.contains(position) {
            continue;
        }

        let all_handles = node
            .internals
            .handle_bounds
            .source
            .iter()
            .chain(node.internals.handle_bounds.target.iter());

        for handle in all_handles {
            // Never connect a handle back to itself.
            if handle.node_id == from_handle.node_id
                && handle.handle_type == from_handle.handle_type
                && handle.id == from_handle.id
            {
                continue;
            }

            // In Strict mode, skip handles of the same type as the source.
            if mode == ConnectionMode::Strict && handle.handle_type == from_handle.handle_type {
                continue;
            }

            let handle_pos = get_handle_absolute_position(node, handle);
            let dist = handle_pos.distance(position);

            if dist > connection_radius {
                continue;
            }

            // Accept if closer, or if equidistant but this handle is the
            // preferred (opposite) type.
            let is_preferred_type = handle.handle_type == preferred_type;
            let better = dist < best_dist
                || ((dist - best_dist).abs() < f32::EPSILON
                    && is_preferred_type
                    && best
                        .as_ref()
                        .map(|b| b.handle_type != preferred_type)
                        .unwrap_or(false));

            if better {
                best = Some(handle.clone());
                best_dist = dist;
            }
        }
    }

    best
}
