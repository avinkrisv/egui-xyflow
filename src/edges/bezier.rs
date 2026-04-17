//! Cubic Bezier and SimpleBezier edge path computation.

use smallvec::smallvec;

use crate::types::edge::{EdgePathResult, EdgePosition};
use crate::types::position::Position;

const DEFAULT_CURVATURE: f32 = 0.25;

fn calculate_control_offset(distance: f32, curvature: f32) -> f32 {
    if distance >= 0.0 {
        0.5 * distance
    } else {
        curvature * 25.0 * (-distance).sqrt()
    }
}

fn get_control_with_curvature(
    pos: Position,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    c: f32,
) -> egui::Pos2 {
    match pos {
        Position::Left => egui::pos2(x1 - calculate_control_offset(x1 - x2, c), y1),
        Position::Right => egui::pos2(x1 + calculate_control_offset(x2 - x1, c), y1),
        Position::Top => egui::pos2(x1, y1 - calculate_control_offset(y1 - y2, c)),
        Position::Bottom => egui::pos2(x1, y1 + calculate_control_offset(y2 - y1, c)),
        Position::Center | Position::Closest => {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let offset = calculate_control_offset(dist, c);
            let sign = if (x1, y1) < (x2, y2) { 1.0 } else { -1.0 };
            egui::pos2(
                x1 + sign * (-dy / dist) * offset,
                y1 + sign * (dx / dist) * offset,
            )
        }
    }
}

/// Compute a cubic Bezier edge path between source and target positions.
pub fn get_bezier_path(pos: &EdgePosition, curvature: Option<f32>) -> EdgePathResult {
    let c = curvature.unwrap_or(DEFAULT_CURVATURE);
    let source = egui::pos2(pos.source_x, pos.source_y);
    let target = egui::pos2(pos.target_x, pos.target_y);

    let cp1 = get_control_with_curvature(
        pos.source_pos,
        pos.source_x,
        pos.source_y,
        pos.target_x,
        pos.target_y,
        c,
    );
    let cp2 = get_control_with_curvature(
        pos.target_pos,
        pos.target_x,
        pos.target_y,
        pos.source_x,
        pos.source_y,
        c,
    );

    // Center at t=0.5 using cubic bezier formula
    let center_x = 0.125 * source.x + 0.375 * cp1.x + 0.375 * cp2.x + 0.125 * target.x;
    let center_y = 0.125 * source.y + 0.375 * cp1.y + 0.375 * cp2.y + 0.125 * target.y;

    EdgePathResult {
        points: smallvec![source, cp1, cp2, target],
        label_pos: egui::pos2(center_x, center_y),
        center_x,
        center_y,
    }
}

/// Sample N points along a cubic bezier for dashed line rendering.
pub fn sample_bezier(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    segments: usize,
) -> Vec<egui::Pos2> {
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let inv = 1.0 - t;
        let x = inv * inv * inv * p0.x
            + 3.0 * inv * inv * t * p1.x
            + 3.0 * inv * t * t * p2.x
            + t * t * t * p3.x;
        let y = inv * inv * inv * p0.y
            + 3.0 * inv * inv * t * p1.y
            + 3.0 * inv * t * t * p2.y
            + t * t * t * p3.y;
        points.push(egui::pos2(x, y));
    }
    points
}
