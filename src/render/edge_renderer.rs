//! Edge rendering: path computation, stroke painting, and animation.

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

/// Screen-space endpoint info collected during edge rendering so that contact
/// indicators can be drawn in a later pass (on top of nodes).
pub(crate) struct EdgeEndpoints {
    #[allow(dead_code)]
    pub(crate) edge_id: crate::types::edge::EdgeId,
    pub(crate) source_screen: egui::Pos2,
    pub(crate) target_screen: egui::Pos2,
}

pub(crate) fn render_edges<ND, ED>(
    painter: &egui::Painter,
    edges: &[Edge<ED>],
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
) -> Vec<EdgeEndpoints> {
    let mut endpoints = Vec::with_capacity(edges.len());
    for edge in edges {
        if edge.hidden {
            continue;
        }
        if let Some(ep) = render_single_edge(painter, edge, node_lookup, transform, config, time) {
            endpoints.push(ep);
        }
    }
    endpoints
}

fn render_single_edge<ND, ED>(
    painter: &egui::Painter,
    edge: &Edge<ED>,
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
) -> Option<EdgeEndpoints> {
    let default_source_pos = config.default_source_position;
    let default_target_pos = config.default_target_position;

    let edge_pos = get_edge_position(
        &edge.source,
        &edge.target,
        edge.source_handle.as_deref(),
        edge.target_handle.as_deref(),
        node_lookup,
        default_source_pos,
        default_target_pos,
        edge.source_anchor.as_ref(),
        edge.target_anchor.as_ref(),
    )?;

    let edge_type = edge.edge_type.unwrap_or(config.default_edge_type);
    let style = edge.style.as_ref();
    let color = if edge.selected {
        style.and_then(|s| s.selected_color).unwrap_or(config.edge_selected_color)
    } else {
        style.and_then(|s| s.color).unwrap_or(config.edge_color)
    };
    let base_width = style.and_then(|s| s.stroke_width).unwrap_or(config.edge_stroke_width);
    let width = base_width * if edge.selected { 2.0 } else { 1.0 };
    let stroke = egui::Stroke::new(width, color);
    let glow = style.and_then(|s| s.glow);

    match edge_type {
        EdgeType::Bezier | EdgeType::SimpleBezier => {
            let result = get_bezier_path(&edge_pos, None);
            if result.points.len() == 4 {
                let p0 = flow_to_screen(result.points[0], transform);
                let p1 = flow_to_screen(result.points[1], transform);
                let p2 = flow_to_screen(result.points[2], transform);
                let p3 = flow_to_screen(result.points[3], transform);

                // Glow pass (wider, semi-transparent, behind the main stroke)
                if let Some(g) = glow {
                    let glow_stroke = egui::Stroke::new(g.width, g.color);
                    if edge.animated {
                        draw_animated_bezier(painter, p0, p1, p2, p3, glow_stroke, config, time);
                    } else {
                        let bezier = epaint::CubicBezierShape::from_points_stroke(
                            [p0, p1, p2, p3],
                            false,
                            egui::Color32::TRANSPARENT,
                            glow_stroke,
                        );
                        painter.add(bezier);
                    }
                }

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

            if let Some(g) = glow {
                let glow_stroke = egui::Stroke::new(g.width, g.color);
                if edge.animated {
                    draw_animated_line(painter, &[from, to], glow_stroke, config, time);
                } else {
                    painter.line_segment([from, to], glow_stroke);
                }
            }

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

            if let Some(g) = glow {
                let glow_stroke = egui::Stroke::new(g.width, g.color);
                if edge.animated {
                    draw_animated_line(painter, &screen_points, glow_stroke, config, time);
                } else {
                    draw_polyline(painter, &screen_points, glow_stroke);
                }
            }

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

            if let Some(g) = glow {
                let glow_stroke = egui::Stroke::new(g.width, g.color);
                if edge.animated {
                    draw_animated_line(painter, &screen_points, glow_stroke, config, time);
                } else {
                    draw_polyline(painter, &screen_points, glow_stroke);
                }
            }

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

    // Return endpoint screen positions for contact indicator rendering
    // (drawn later, on top of nodes).
    let src_screen = flow_to_screen(
        egui::pos2(edge_pos.source_x, edge_pos.source_y),
        transform,
    );
    let tgt_screen = flow_to_screen(
        egui::pos2(edge_pos.target_x, edge_pos.target_y),
        transform,
    );
    Some(EdgeEndpoints {
        edge_id: edge.id.clone(),
        source_screen: src_screen,
        target_screen: tgt_screen,
    })
}

/// Render contact indicator circles at edge endpoints.
/// Called in a later pass so indicators appear on top of nodes.
pub(crate) fn render_edge_contact_indicators(
    painter: &egui::Painter,
    endpoints: &[EdgeEndpoints],
    transform: &Transform,
    config: &FlowConfig,
) {
    if !config.show_edge_contact_indicators {
        return;
    }
    let r = config.edge_contact_indicator_radius * transform.scale;
    let fill = config.edge_contact_indicator_color;
    let stroke = egui::Stroke::new(1.0 * transform.scale, egui::Color32::WHITE);

    for ep in endpoints {
        painter.circle_filled(ep.source_screen, r, fill);
        painter.circle_stroke(ep.source_screen, r, stroke);
        painter.circle_filled(ep.target_screen, r, fill);
        painter.circle_stroke(ep.target_screen, r, stroke);
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

    // Draw short solid caps at both endpoints so the edge always visually
    // connects to the node border even when a dash gap falls at the end.
    let cap_len = config.animated_edge_dash_length * 0.5;
    if let (Some(&first), Some(&second)) = (points.first(), points.get(1)) {
        let d = second - first;
        let len = d.length();
        if len > 0.0 {
            let cap_end = first + d / len * cap_len.min(len);
            painter.line_segment([first, cap_end], stroke);
        }
    }
    if let (Some(&last), Some(&prev)) = (points.last(), points.get(points.len().saturating_sub(2)))
    {
        let d = prev - last;
        let len = d.length();
        if len > 0.0 {
            let cap_end = last + d / len * cap_len.min(len);
            painter.line_segment([last, cap_end], stroke);
        }
    }
}

#[allow(clippy::too_many_arguments)]
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
