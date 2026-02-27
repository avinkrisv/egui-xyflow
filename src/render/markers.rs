//! Arrow marker rendering for edge endpoints.

use crate::types::position::Position;

/// Render an arrow marker at the given position.
///
/// For `Position::Center`, an optional `from` point can be provided
/// so the arrow points along the edge direction instead of a fixed axis.
pub(crate) fn render_arrow(
    painter: &egui::Painter,
    tip: egui::Pos2,
    direction: Position,
    color: egui::Color32,
    stroke_width: f32,
) {
    render_arrow_from(painter, tip, direction, color, stroke_width, None);
}

/// Render an arrow marker, optionally using a `from` point to derive direction
/// when `direction` is `Position::Center`.
pub(crate) fn render_arrow_from(
    painter: &egui::Painter,
    tip: egui::Pos2,
    direction: Position,
    color: egui::Color32,
    stroke_width: f32,
    from: Option<egui::Pos2>,
) {
    let size = (stroke_width * 4.0).max(6.0);
    let (dx, dy) = match direction {
        Position::Left => (size, 0.0),
        Position::Right => (-size, 0.0),
        Position::Top => (0.0, size),
        Position::Bottom => (0.0, -size),
        Position::Center | Position::Closest => {
            // Derive direction from the source point toward the tip
            if let Some(src) = from {
                let ddx = tip.x - src.x;
                let ddy = tip.y - src.y;
                let dist = (ddx * ddx + ddy * ddy).sqrt().max(1.0);
                // Arrow points back along the edge (away from tip toward source)
                (-size * ddx / dist, -size * ddy / dist)
            } else {
                // Fallback: point leftward
                (size, 0.0)
            }
        }
    };

    let p1 = egui::pos2(tip.x + dx + dy * 0.5, tip.y + dy - dx * 0.5);
    let p2 = egui::pos2(tip.x + dx - dy * 0.5, tip.y + dy + dx * 0.5);

    painter.add(egui::Shape::convex_polygon(
        vec![tip, p1, p2],
        color,
        egui::Stroke::NONE,
    ));
}
