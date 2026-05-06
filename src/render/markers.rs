//! Arrow marker rendering for edge endpoints.

use crate::types::edge::{EdgeMarker, MarkerType};
use crate::types::position::Position;

/// Render an arrow marker at the given screen position.
///
/// `from` is the screen-space point at the other end of the edge segment
/// leading into `tip`. It is used to derive direction when `direction`
/// resolves to [`Position::Center`] or [`Position::Closest`]; cardinal
/// directions ignore it.
pub(crate) fn render_marker(
    painter: &egui::Painter,
    tip: egui::Pos2,
    direction: Position,
    edge_color: egui::Color32,
    edge_stroke_width: f32,
    marker: &EdgeMarker,
    from: Option<egui::Pos2>,
) {
    let depth = marker.width.unwrap_or((edge_stroke_width * 4.0).max(6.0));
    let half_w = marker.height.unwrap_or(depth) * 0.5;
    let head_color = marker.color.unwrap_or(edge_color);

    let (dx, dy) = match direction {
        Position::Left => (depth, 0.0),
        Position::Right => (-depth, 0.0),
        Position::Top => (0.0, depth),
        Position::Bottom => (0.0, -depth),
        Position::Center | Position::Closest => {
            if let Some(src) = from {
                let ddx = tip.x - src.x;
                let ddy = tip.y - src.y;
                let dist = (ddx * ddx + ddy * ddy).sqrt().max(1.0);
                (-depth * ddx / dist, -depth * ddy / dist)
            } else {
                (depth, 0.0)
            }
        }
    };

    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let nx = -dy / len * half_w;
    let ny = dx / len * half_w;

    let p1 = egui::pos2(tip.x + dx + nx, tip.y + dy + ny);
    let p2 = egui::pos2(tip.x + dx - nx, tip.y + dy - ny);

    match marker.marker_type {
        MarkerType::ArrowClosed => {
            painter.add(egui::Shape::convex_polygon(
                vec![tip, p1, p2],
                head_color,
                egui::Stroke::NONE,
            ));
        }
        MarkerType::Arrow => {
            let stroke_w = marker.stroke_width.unwrap_or(edge_stroke_width.max(1.0));
            let stroke = egui::Stroke::new(stroke_w, head_color);
            painter.line_segment([p1, tip], stroke);
            painter.line_segment([p2, tip], stroke);
        }
    }
}
