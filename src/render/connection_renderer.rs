use crate::config::FlowConfig;
use crate::edges::bezier::{get_bezier_path, sample_bezier};
use crate::edges::smooth_step::get_smooth_step_path;
use crate::edges::straight::get_straight_path;
use crate::graph::node_position::flow_to_screen;
use crate::types::connection::ConnectionState;
use crate::types::edge::{EdgePosition, EdgeType};
use crate::types::position::Transform;

/// Render the in-progress connection line (from a dragged handle to the pointer).
pub(crate) fn render_connection_line(
    painter: &egui::Painter,
    connection_state: &ConnectionState,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
) {
    let (from, from_position, to, to_position, is_valid) = match connection_state {
        ConnectionState::InProgress {
            from,
            from_position,
            to,
            to_position,
            is_valid,
            ..
        } => (*from, *from_position, *to, *to_position, *is_valid),
        ConnectionState::None => return,
    };

    // Choose color based on validity
    let color = match is_valid {
        Some(true) => egui::Color32::from_rgb(34, 197, 94), // green
        Some(false) => egui::Color32::from_rgb(239, 68, 68), // red
        None => config.edge_color,
    };
    let stroke = egui::Stroke::new(config.edge_stroke_width, color);

    // Build a fake EdgePosition to reuse the path math
    let from_screen = flow_to_screen(from, transform);
    let to_screen = flow_to_screen(to, transform);

    // We work in screen space directly for the connection line
    let edge_pos = EdgePosition {
        source_x: from_screen.x,
        source_y: from_screen.y,
        target_x: to_screen.x,
        target_y: to_screen.y,
        source_pos: from_position,
        target_pos: to_position,
    };

    let edge_type = config.connection_line_type;

    match edge_type {
        EdgeType::Bezier | EdgeType::SimpleBezier => {
            let result = get_bezier_path(&edge_pos, None);
            if result.points.len() == 4 {
                // Points are already in screen space
                let p0 = result.points[0];
                let p1 = result.points[1];
                let p2 = result.points[2];
                let p3 = result.points[3];

                if config.connection_line_animated {
                    let sampled = sample_bezier(p0, p1, p2, p3, 64);
                    draw_animated_line(painter, &sampled, stroke, config, time);
                } else {
                    let bezier = epaint::CubicBezierShape::from_points_stroke(
                        [p0, p1, p2, p3],
                        false,
                        egui::Color32::TRANSPARENT,
                        stroke,
                    );
                    painter.add(bezier);
                }
            }
        }
        EdgeType::Straight => {
            let result = get_straight_path(&edge_pos);
            let from_pt = result.points[0];
            let to_pt = result.points[1];
            if config.connection_line_animated {
                draw_animated_line(painter, &[from_pt, to_pt], stroke, config, time);
            } else {
                painter.line_segment([from_pt, to_pt], stroke);
            }
        }
        EdgeType::SmoothStep | EdgeType::Step => {
            let result = get_smooth_step_path(&edge_pos, None, None);
            if config.connection_line_animated {
                draw_animated_line(painter, &result.points, stroke, config, time);
            } else {
                draw_polyline(painter, &result.points, stroke);
            }
        }
    }

    // Draw a small circle at the origin handle
    painter.circle_filled(from_screen, 4.0 * transform.scale.max(0.5), color);
}

fn draw_polyline(painter: &egui::Painter, points: &[egui::Pos2], stroke: egui::Stroke) {
    for i in 0..points.len().saturating_sub(1) {
        painter.line_segment([points[i], points[i + 1]], stroke);
    }
}

fn draw_animated_line(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    stroke: egui::Stroke,
    config: &FlowConfig,
    time: f64,
) {
    if points.len() < 2 {
        return;
    }
    let dash_offset = (time as f32 * config.animated_edge_speed)
        % (config.animated_edge_dash_length + config.animated_edge_gap_length);

    let shapes = egui::Shape::dashed_line_with_offset(
        points,
        stroke,
        &[config.animated_edge_dash_length],
        &[config.animated_edge_gap_length],
        dash_offset,
    );
    for shape in shapes {
        painter.add(shape);
    }
}
