//! Edge endpoint position resolution from handles, anchors, and node geometry.

use crate::types::edge::EdgePosition;
use crate::types::handle::{Handle, HandleType};
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::{NodeShape, Position};
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

    let (sx, sy, source_pos, source_fallback) = if let Some(anchor) = source_anchor {
        let pt = anchor.resolve(source_node.rect());
        (pt.x, pt.y, anchor.side, false)
    } else {
        get_handle_position_for_node(
            source_node,
            HandleType::Source,
            source_handle_id,
            default_source_pos,
        )
    };
    let (tx, ty, target_pos, target_fallback) = if let Some(anchor) = target_anchor {
        let pt = anchor.resolve(target_node.rect());
        (pt.x, pt.y, anchor.side, false)
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

    // Shape-aware perimeter intersection: when the endpoint came from the
    // no-explicit-handle fallback and the node declares a non-Rect shape,
    // anchor on the real silhouette along the centre-to-centre line.
    let (sx, sy) = if source_fallback {
        shape_perimeter_point(source_node, target_center).unwrap_or((sx, sy))
    } else {
        (sx, sy)
    };
    let (tx, ty) = if target_fallback {
        shape_perimeter_point(target_node, source_center).unwrap_or((tx, ty))
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
) -> (f32, f32, Position, bool) {
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
        (abs.x, abs.y, pos, false)
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
        (x, y, default_pos, true)
    }
}

/// Intersect the line from `node`'s centre to `other_center` with the node's
/// declared shape silhouette, returning the flow-space point on the perimeter.
/// Returns `None` for [`NodeShape::Rect`] (caller keeps the side-centre anchor).
fn shape_perimeter_point<D>(
    node: &InternalNode<D>,
    other_center: egui::Pos2,
) -> Option<(f32, f32)> {
    let rect = node.rect();
    let center = rect.center();
    let dir = other_center - center;
    let len = dir.length();
    if len < f32::EPSILON {
        return None;
    }
    let unit = dir / len;
    match node.internals.shape {
        NodeShape::Rect => None,
        NodeShape::Circle { radius } => {
            let p = center + unit * radius.max(0.0);
            Some((p.x, p.y))
        }
        NodeShape::RoundedRect { rounding } => {
            rounded_rect_intersect(rect, other_center, rounding).map(|p| (p.x, p.y))
        }
    }
}

/// Intersection of the ray from `rect.center()` toward `other_center` with a
/// rounded-rectangle perimeter (flat sides + quarter-circle corners).
fn rounded_rect_intersect(
    rect: egui::Rect,
    other_center: egui::Pos2,
    rounding: f32,
) -> Option<egui::Pos2> {
    let center = rect.center();
    let dir = other_center - center;
    let len = dir.length();
    if len < f32::EPSILON {
        return None;
    }
    let half_w = rect.width() * 0.5;
    let half_h = rect.height() * 0.5;
    let r = rounding.max(0.0).min(half_w.min(half_h));

    // Ray-rect hit point (as if corners were sharp).
    let tx = if dir.x.abs() > f32::EPSILON {
        half_w / dir.x.abs()
    } else {
        f32::INFINITY
    };
    let ty = if dir.y.abs() > f32::EPSILON {
        half_h / dir.y.abs()
    } else {
        f32::INFINITY
    };
    let t = tx.min(ty);
    let hit = center + dir * t;

    if r <= 0.0 {
        return Some(hit);
    }

    // Is the flat-side hit inside the rounded corner's exclusion zone?
    let corner_cx = if hit.x >= center.x {
        rect.max.x - r
    } else {
        rect.min.x + r
    };
    let corner_cy = if hit.y >= center.y {
        rect.max.y - r
    } else {
        rect.min.y + r
    };
    let in_corner_x = (hit.x - center.x).abs() > (corner_cx - center.x).abs();
    let in_corner_y = (hit.y - center.y).abs() > (corner_cy - center.y).abs();
    if !(in_corner_x && in_corner_y) {
        return Some(hit);
    }

    // Solve for intersection with the quarter-circle corner centred at
    // (corner_cx, corner_cy) with radius r. Ray: center + u * dir, u > 0.
    let ox = center.x - corner_cx;
    let oy = center.y - corner_cy;
    let a = dir.x * dir.x + dir.y * dir.y;
    let b = 2.0 * (ox * dir.x + oy * dir.y);
    let c = ox * ox + oy * oy - r * r;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 || a < f32::EPSILON {
        return Some(hit);
    }
    let sqrt_disc = disc.sqrt();
    let u1 = (-b - sqrt_disc) / (2.0 * a);
    let u2 = (-b + sqrt_disc) / (2.0 * a);
    let u = [u1, u2]
        .into_iter()
        .filter(|v| *v > 0.0)
        .fold(f32::INFINITY, f32::min);
    if !u.is_finite() {
        return Some(hit);
    }
    Some(center + dir * u)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::node::{InternalNode, Node, NodeHandleBounds, NodeInternals};

    fn make_node(x: f32, y: f32, w: f32, h: f32, shape: NodeShape) -> InternalNode<()> {
        let mut node: Node<()> = Node::builder("n").build();
        node.position = egui::pos2(x, y);
        node.width = Some(w);
        node.height = Some(h);
        InternalNode {
            node,
            internals: NodeInternals {
                position_absolute: egui::pos2(x, y),
                z: 0,
                handle_bounds: NodeHandleBounds::default(),
                shape,
            },
        }
    }

    #[test]
    fn circle_shape_anchors_on_perimeter() {
        let n = make_node(0.0, 0.0, 100.0, 100.0, NodeShape::Circle { radius: 50.0 });
        // Target to the right: intersection is at center + (50, 0)
        let p = shape_perimeter_point(&n, egui::pos2(200.0, 50.0)).unwrap();
        assert!((p.0 - 100.0).abs() < 1e-3, "x={}", p.0);
        assert!((p.1 - 50.0).abs() < 1e-3, "y={}", p.1);
    }

    #[test]
    fn rect_shape_returns_none_so_caller_keeps_side_center() {
        let n = make_node(0.0, 0.0, 100.0, 100.0, NodeShape::Rect);
        assert!(shape_perimeter_point(&n, egui::pos2(200.0, 50.0)).is_none());
    }

    #[test]
    fn rounded_rect_flat_side_hits_rect_edge() {
        let n = make_node(
            0.0,
            0.0,
            100.0,
            100.0,
            NodeShape::RoundedRect { rounding: 10.0 },
        );
        // Straight right along centreline → hits right edge at (100, 50), not the corner arc.
        let p = shape_perimeter_point(&n, egui::pos2(200.0, 50.0)).unwrap();
        assert!((p.0 - 100.0).abs() < 1e-3);
        assert!((p.1 - 50.0).abs() < 1e-3);
    }

    #[test]
    fn rounded_rect_diagonal_hits_corner_arc() {
        let n = make_node(
            0.0,
            0.0,
            100.0,
            100.0,
            NodeShape::RoundedRect { rounding: 10.0 },
        );
        // Toward the top-right corner. The arc centre is (90, 10), radius 10.
        // The intersection must satisfy (x-90)^2 + (y-10)^2 = 100.
        let p = shape_perimeter_point(&n, egui::pos2(200.0, -100.0)).unwrap();
        let dx = p.0 - 90.0;
        let dy = p.1 - 10.0;
        let d2 = dx * dx + dy * dy;
        assert!((d2 - 100.0).abs() < 1e-2, "d^2 = {}", d2);
    }
}
