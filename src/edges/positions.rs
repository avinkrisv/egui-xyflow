//! Edge endpoint position resolution from handles, anchors, and node geometry.

use crate::types::edge::EdgePosition;
use crate::types::handle::{Handle, HandleType};
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::Position;
use std::collections::HashMap;

/// Resolve the edge positions from source and target nodes.
///
/// When `source_anchor` or `target_anchor` is provided it takes precedence
/// over handle lookup — the anchor's resolved point and side are used
/// directly.
#[allow(clippy::too_many_arguments)]
pub fn get_edge_position<D>(
    source_id: &NodeId,
    target_id: &NodeId,
    source_handle_id: Option<&str>,
    target_handle_id: Option<&str>,
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    default_source_pos: Position,
    default_target_pos: Position,
    source_anchor: Option<&crate::types::edge::EdgeAnchor>,
    target_anchor: Option<&crate::types::edge::EdgeAnchor>,
) -> Option<EdgePosition> {
    let source_node = node_lookup.get(source_id)?;
    let target_node = node_lookup.get(target_id)?;

    let (sx, sy, source_pos) = if let Some(anchor) = source_anchor {
        let pt = anchor.resolve(source_node.rect());
        (pt.x, pt.y, anchor.side)
    } else {
        get_handle_position_for_node(
            source_node,
            HandleType::Source,
            source_handle_id,
            default_source_pos,
        )
    };
    let (tx, ty, target_pos) = if let Some(anchor) = target_anchor {
        let pt = anchor.resolve(target_node.rect());
        (pt.x, pt.y, anchor.side)
    } else {
        get_handle_position_for_node(
            target_node,
            HandleType::Target,
            target_handle_id,
            default_target_pos,
        )
    };

    // Resolve Position::Closest into a concrete side
    let source_center = source_node.rect().center();
    let target_center = target_node.rect().center();
    let resolved_source = source_pos.resolve_closest(source_center, target_center);
    let resolved_target = target_pos.resolve_closest(target_center, source_center);

    // When the original position was Closest (and no anchor override),
    // reproject the connection point to the resolved side's center.
    let (sx, sy) = if source_pos == Position::Closest && source_anchor.is_none() {
        reproject_to_side(source_node, resolved_source)
    } else {
        (sx, sy)
    };
    let (tx, ty) = if target_pos == Position::Closest && target_anchor.is_none() {
        reproject_to_side(target_node, resolved_target)
    } else {
        (tx, ty)
    };

    Some(EdgePosition {
        source_x: sx,
        source_y: sy,
        target_x: tx,
        target_y: ty,
        source_pos: resolved_source,
        target_pos: resolved_target,
    })
}

/// Compute the center connection point for a resolved side of a node.
fn reproject_to_side<D>(node: &InternalNode<D>, pos: Position) -> (f32, f32) {
    let rect = node.rect();
    match pos {
        Position::Top => (rect.center().x, rect.min.y),
        Position::Bottom => (rect.center().x, rect.max.y),
        Position::Left => (rect.min.x, rect.center().y),
        Position::Right => (rect.max.x, rect.center().y),
        _ => (rect.center().x, rect.center().y),
    }
}

fn get_handle_position_for_node<D>(
    node: &InternalNode<D>,
    handle_type: HandleType,
    handle_id: Option<&str>,
    default_pos: Position,
) -> (f32, f32, Position) {
    let handles = match handle_type {
        HandleType::Source => &node.internals.handle_bounds.source,
        HandleType::Target => &node.internals.handle_bounds.target,
    };

    // Find matching handle
    let handle = if let Some(hid) = handle_id {
        handles.iter().find(|h| h.id.as_deref() == Some(hid))
    } else {
        handles.first()
    };

    if let Some(h) = handle {
        let pos = h.position;
        let abs = get_handle_absolute_position(node, h);
        (abs.x, abs.y, pos)
    } else {
        // Fallback: center of the node side (or node center for Position::Center/Closest)
        let rect = node.rect();
        let (x, y) = match default_pos {
            Position::Top => (rect.center().x, rect.min.y),
            Position::Bottom => (rect.center().x, rect.max.y),
            Position::Left => (rect.min.x, rect.center().y),
            Position::Right => (rect.max.x, rect.center().y),
            Position::Center | Position::Closest => (rect.center().x, rect.center().y),
        };
        (x, y, default_pos)
    }
}

/// Get absolute position of a handle center.
pub fn get_handle_absolute_position<D>(node: &InternalNode<D>, handle: &Handle) -> egui::Pos2 {
    let abs = node.internals.position_absolute;
    egui::pos2(
        abs.x + handle.x + handle.width / 2.0,
        abs.y + handle.y + handle.height / 2.0,
    )
}

/// Project a flow-space point onto the nearest border of `rect` and return
/// the corresponding [`crate::types::edge::EdgeAnchor`].
pub fn project_to_border(
    point: egui::Pos2,
    rect: egui::Rect,
) -> crate::types::edge::EdgeAnchor {
    use crate::types::edge::EdgeAnchor;

    // Compute closest point on each side and its distance
    let candidates = [
        // Top
        (
            Position::Top,
            egui::pos2(point.x.clamp(rect.min.x, rect.max.x), rect.min.y),
        ),
        // Bottom
        (
            Position::Bottom,
            egui::pos2(point.x.clamp(rect.min.x, rect.max.x), rect.max.y),
        ),
        // Left
        (
            Position::Left,
            egui::pos2(rect.min.x, point.y.clamp(rect.min.y, rect.max.y)),
        ),
        // Right
        (
            Position::Right,
            egui::pos2(rect.max.x, point.y.clamp(rect.min.y, rect.max.y)),
        ),
    ];

    let (side, closest) = candidates
        .iter()
        .min_by(|a, b| {
            let da = a.1.distance(point);
            let db = b.1.distance(point);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap();

    let t = match side {
        Position::Top | Position::Bottom => {
            if rect.width() > 0.0 {
                ((closest.x - rect.min.x) / rect.width()).clamp(0.0, 1.0)
            } else {
                0.5
            }
        }
        Position::Left | Position::Right => {
            if rect.height() > 0.0 {
                ((closest.y - rect.min.y) / rect.height()).clamp(0.0, 1.0)
            } else {
                0.5
            }
        }
        _ => 0.5,
    };

    EdgeAnchor { side, t }
}
