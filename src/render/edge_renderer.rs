use crate::config::FlowConfig;
use crate::edges::bezier::{get_bezier_path, sample_bezier};
use crate::edges::positions::get_edge_position;
use crate::edges::smooth_step::{get_smooth_step_path, get_step_path};
use crate::edges::straight::get_straight_path;
use crate::graph::node_position::flow_to_screen;
use crate::types::edge::{Edge, EdgeType};
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::Transform;
use std::collections::HashMap;

pub fn render_edges<ND, ED>(
    painter: &egui::Painter,
    edges: &[Edge<ED>],
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
) {
    for edge in edges {
        if edge.hidden {
            continue;
        }
        render_single_edge(painter, edge, node_lookup, transform, config, time);
    }
}

fn render_single_edge<ND, ED>(
    painter: &egui::Painter,
    edge: &Edge<ED>,
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
) {
    let default_source_pos = config.default_source_position;
    let default_target_pos = config.default_target_position;

    let edge_pos = match get_edge_position(
        &edge.source,
        &edge.target,
        edge.source_handle.as_deref(),
        edge.target_handle.as_deref(),
        node_lookup,
        default_source_pos,
        default_target_pos,
        edge.source_anchor.as_ref(),
        edge.target_anchor.as_ref(),
    ) {
        Some(pos) => pos,
        None => return,
    };

    let edge_type = edge.edge_type.unwrap_or(config.default_edge_type);
    let color = if edge.selected {
        config.edge_selected_color
    } else {
        config.edge_color
    };
    let width = config.edge_stroke_width * if edge.selected { 2.0 } else { 1.0 };
    let stroke = egui::Stroke::new(width, color);

    match edge_type {
        EdgeType::Bezier | EdgeType::SimpleBezier => {
            let result = get_bezier_path(&edge_pos, None);
            if result.points.len() == 4 {
                let p0 = flow_to_screen(result.points[0], transform);
                let p1 = flow_to_screen(result.points[1], transform);
                let p2 = flow_to_screen(result.points[2], transform);
                let p3 = flow_to_screen(result.points[3], transform);

                if edge.animated {
                    draw_animated_bezier(painter, p0, p1, p2, p3, stroke, config, time);
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
            let from = flow_to_screen(result.points[0], transform);
            let to = flow_to_screen(result.points[1], transform);

            if edge.animated {
                draw_animated_line(painter, &[from, to], stroke, config, time);
            } else {
                painter.line_segment([from, to], stroke);
            }
        }
        EdgeType::SmoothStep => {
            let result = get_smooth_step_path(&edge_pos, None, None);
            let screen_points: Vec<egui::Pos2> =
                result.points.iter().map(|p| flow_to_screen(*p, transform)).collect();

            if edge.animated {
                draw_animated_line(painter, &screen_points, stroke, config, time);
            } else {
                draw_polyline(painter, &screen_points, stroke);
            }
        }
        EdgeType::Step => {
            let result = get_step_path(&edge_pos, None);
            let screen_points: Vec<egui::Pos2> =
                result.points.iter().map(|p| flow_to_screen(*p, transform)).collect();

            if edge.animated {
                draw_animated_line(painter, &screen_points, stroke, config, time);
            } else {
                draw_polyline(painter, &screen_points, stroke);
            }
        }
    }

    // Render arrow markers
    if edge.marker_end.is_some() {
        let target_screen = flow_to_screen(
            egui::pos2(edge_pos.target_x, edge_pos.target_y),
            transform,
        );
        super::markers::render_arrow(painter, target_screen, edge_pos.target_pos, color, width);
    }
}

fn draw_polyline(painter: &egui::Painter, points: &[egui::Pos2], stroke: egui::Stroke) {
    if points.len() < 2 {
        return;
    }
    for i in 0..points.len() - 1 {
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

fn draw_animated_bezier(
    painter: &egui::Painter,
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    stroke: egui::Stroke,
    config: &FlowConfig,
    time: f64,
) {
    // Sample bezier into line segments for dashed rendering
    let sampled = sample_bezier(p0, p1, p2, p3, 64);
    draw_animated_line(painter, &sampled, stroke, config, time);
}
