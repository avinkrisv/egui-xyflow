use crate::types::edge::{EdgePathResult, EdgePosition};
use crate::types::position::Position;

const DEFAULT_BORDER_RADIUS: f32 = 5.0;
const DEFAULT_OFFSET: f32 = 20.0;

/// Get a smooth step (orthogonal) path between source and target.
pub fn get_smooth_step_path(
    pos: &EdgePosition,
    border_radius: Option<f32>,
    offset: Option<f32>,
) -> EdgePathResult {
    let border_radius = border_radius.unwrap_or(DEFAULT_BORDER_RADIUS);
    let offset = offset.unwrap_or(DEFAULT_OFFSET);

    let source = egui::pos2(pos.source_x, pos.source_y);
    let target = egui::pos2(pos.target_x, pos.target_y);

    // Offset source/target in handle direction
    let s = offset_point(source, pos.source_pos, offset);
    let t = offset_point(target, pos.target_pos, offset);

    let center_x = (s.x + t.x) / 2.0;
    let center_y = (s.y + t.y) / 2.0;

    let s_horizontal = pos.source_pos.is_horizontal();
    let t_horizontal = pos.target_pos.is_horizontal();

    let mut points = Vec::new();
    points.push(source);
    points.push(s);

    if s_horizontal && t_horizontal {
        // Both horizontal: go via shared x midpoint
        let mid_x = center_x;
        add_rounded_corner(&mut points, egui::pos2(mid_x, s.y), border_radius);
        add_rounded_corner(&mut points, egui::pos2(mid_x, t.y), border_radius);
    } else if !s_horizontal && !t_horizontal {
        // Both vertical: go via shared y midpoint
        let mid_y = center_y;
        add_rounded_corner(&mut points, egui::pos2(s.x, mid_y), border_radius);
        add_rounded_corner(&mut points, egui::pos2(t.x, mid_y), border_radius);
    } else if s_horizontal && !t_horizontal {
        // Source horizontal, target vertical: L-shaped
        add_rounded_corner(&mut points, egui::pos2(t.x, s.y), border_radius);
    } else {
        // Source vertical, target horizontal: L-shaped
        add_rounded_corner(&mut points, egui::pos2(s.x, t.y), border_radius);
    }

    points.push(t);
    points.push(target);

    let label_x = (source.x + target.x) / 2.0;
    let label_y = (source.y + target.y) / 2.0;

    EdgePathResult {
        points,
        label_pos: egui::pos2(label_x, label_y),
        center_x: label_x,
        center_y: label_y,
    }
}

/// Get a step path (no rounded corners).
pub fn get_step_path(pos: &EdgePosition, offset: Option<f32>) -> EdgePathResult {
    get_smooth_step_path(pos, Some(0.0), offset)
}

fn offset_point(p: egui::Pos2, position: Position, offset: f32) -> egui::Pos2 {
    match position {
        Position::Top => egui::pos2(p.x, p.y - offset),
        Position::Bottom => egui::pos2(p.x, p.y + offset),
        Position::Left => egui::pos2(p.x - offset, p.y),
        Position::Right => egui::pos2(p.x + offset, p.y),
        Position::Center | Position::Closest => p, // no offset for center/closest-connected edges
    }
}

fn add_rounded_corner(points: &mut Vec<egui::Pos2>, corner: egui::Pos2, _border_radius: f32) {
    // For simplicity, add the corner point directly.
    // The rendering step will handle smoothing with PathShape.
    points.push(corner);
}
