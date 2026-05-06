//! Edge rendering: path computation, stroke painting, and animation.

use crate::config::FlowConfig;
use crate::edges::bezier::{get_bezier_path, sample_bezier};
use crate::edges::positions::get_edge_position;
use crate::edges::smooth_step::{get_smooth_step_path, get_step_path};
use crate::edges::straight::get_straight_path;
use crate::graph::node_position::flow_to_screen;
use crate::state::flow_state::EdgePathCache;
use crate::types::edge::{Edge, EdgeId, EdgeMarker, EdgeType};
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::{Position, Transform};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Number of samples used when caching a bezier curve as a flow-space polyline.
const BEZIER_CACHE_SAMPLES: usize = 64;

/// Screen-space endpoint info collected during edge rendering so that contact
/// indicators and arrow markers can be drawn in a later pass (on top of nodes).
pub(crate) struct EdgeEndpoints {
    #[allow(dead_code)]
    pub(crate) edge_id: crate::types::edge::EdgeId,
    pub(crate) source_screen: egui::Pos2,
    pub(crate) target_screen: egui::Pos2,
    pub(crate) source_pos: Position,
    pub(crate) target_pos: Position,
    pub(crate) marker_start: Option<EdgeMarker>,
    pub(crate) marker_end: Option<EdgeMarker>,
    pub(crate) edge_color: egui::Color32,
    pub(crate) edge_stroke_width: f32,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_edges<ND, ED>(
    painter: &egui::Painter,
    edges: &[Edge<ED>],
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    edge_path_cache: &mut HashMap<EdgeId, EdgePathCache>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
    canvas_rect: egui::Rect,
) -> Vec<EdgeEndpoints> {
    let mut endpoints = Vec::with_capacity(edges.len());
    for edge in edges {
        if edge.hidden {
            continue;
        }
        if let Some(ep) = render_single_edge(
            painter,
            edge,
            node_lookup,
            edge_path_cache,
            transform,
            config,
            time,
            canvas_rect,
        ) {
            endpoints.push(ep);
        }
    }
    endpoints
}

/// Build (or refresh) a flow-space cache entry for `edge`. Returns `None`
/// when either endpoint node is missing from `node_lookup` — the caller
/// should skip rendering in that case.
fn build_path_cache_entry<ND, ED>(
    edge: &Edge<ED>,
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    config: &FlowConfig,
) -> Option<EdgePathCache> {
    let edge_pos = get_edge_position(
        &edge.source,
        &edge.target,
        edge.source_handle.as_deref(),
        edge.target_handle.as_deref(),
        node_lookup,
        config.default_source_position,
        config.default_target_position,
        edge.source_anchor.as_ref(),
        edge.target_anchor.as_ref(),
    )?;

    let edge_type = edge.edge_type.unwrap_or(config.default_edge_type);

    let (points, label_pos): (SmallVec<[egui::Pos2; 16]>, egui::Pos2) = match edge_type {
        EdgeType::Bezier | EdgeType::SimpleBezier => {
            let result = get_bezier_path(&edge_pos, None);
            if result.points.len() != 4 {
                // Degenerate bezier — fall back to straight line.
                let mut pts: SmallVec<[egui::Pos2; 16]> = SmallVec::new();
                pts.push(egui::pos2(edge_pos.source_x, edge_pos.source_y));
                pts.push(egui::pos2(edge_pos.target_x, edge_pos.target_y));
                (pts, result.label_pos)
            } else {
                let sampled = sample_bezier(
                    result.points[0],
                    result.points[1],
                    result.points[2],
                    result.points[3],
                    BEZIER_CACHE_SAMPLES,
                );
                let mut pts: SmallVec<[egui::Pos2; 16]> = SmallVec::with_capacity(sampled.len());
                pts.extend(sampled);
                (pts, result.label_pos)
            }
        }
        EdgeType::Straight => {
            let result = get_straight_path(&edge_pos);
            let mut pts: SmallVec<[egui::Pos2; 16]> = SmallVec::with_capacity(result.points.len());
            pts.extend(result.points.iter().copied());
            (pts, result.label_pos)
        }
        EdgeType::SmoothStep => {
            let result = get_smooth_step_path(&edge_pos, None, None);
            let mut pts: SmallVec<[egui::Pos2; 16]> = SmallVec::with_capacity(result.points.len());
            pts.extend(result.points.iter().copied());
            (pts, result.label_pos)
        }
        EdgeType::Step => {
            let result = get_step_path(&edge_pos, None);
            let mut pts: SmallVec<[egui::Pos2; 16]> = SmallVec::with_capacity(result.points.len());
            pts.extend(result.points.iter().copied());
            (pts, result.label_pos)
        }
    };

    Some(EdgePathCache {
        points,
        label_pos,
        source_pos: edge_pos.source_pos,
        target_pos: edge_pos.target_pos,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_single_edge<ND, ED>(
    painter: &egui::Painter,
    edge: &Edge<ED>,
    node_lookup: &HashMap<NodeId, InternalNode<ND>>,
    edge_path_cache: &mut HashMap<EdgeId, EdgePathCache>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
    canvas_rect: egui::Rect,
) -> Option<EdgeEndpoints> {
    // Lazy fill: build flow-space polyline on miss.
    if !edge_path_cache.contains_key(&edge.id) {
        let entry = build_path_cache_entry(edge, node_lookup, config)?;
        edge_path_cache.insert(edge.id.clone(), entry);
    }
    let cache = edge_path_cache.get(&edge.id)?;
    if cache.points.len() < 2 {
        return None;
    }

    let src_flow = *cache.points.first()?;
    let tgt_flow = *cache.points.last()?;

    // Viewport culling using the flow-space endpoints — same heuristic as
    // before, just sourced from the cache so we don't recompute geometry.
    if config.cull_offscreen_edges {
        let src_screen = flow_to_screen(src_flow, transform);
        let tgt_screen = flow_to_screen(tgt_flow, transform);
        let mut aabb = egui::Rect::from_two_pos(src_screen, tgt_screen);
        let margin = 64.0_f32.max(aabb.width().abs().max(aabb.height().abs()) * 0.5);
        aabb = aabb.expand(margin);
        if !aabb.intersects(canvas_rect) {
            return None;
        }
    }

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

    // Transform the cached flow-space polyline into screen space. This is the
    // dominant per-frame cost on static graphs — a vec2*scalar+vec2 per vertex.
    let screen_points: Vec<egui::Pos2> = cache
        .points
        .iter()
        .map(|p| flow_to_screen(*p, transform))
        .collect();

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

    // Markers (start + end) are deferred to a post-node pass — see
    // `render_edge_markers` — so they are not overpainted by node bodies.

    // Render text label at the cached `label_pos`.
    if let Some(label) = edge.label.as_deref() {
        if !label.is_empty() {
            let center = flow_to_screen(cache.label_pos, transform);
            let font = egui::FontId::proportional(config.edge_label_font_size * transform.scale);
            let galley = painter.layout_no_wrap(
                label.to_owned(),
                font,
                config.edge_label_color,
            );
            let pad = config.edge_label_padding * transform.scale;
            let text_size = galley.size();
            let rect = egui::Rect::from_center_size(
                center,
                egui::vec2(text_size.x + pad * 2.0, text_size.y + pad * 2.0),
            );
            if config.edge_label_bg_color != egui::Color32::TRANSPARENT {
                painter.rect_filled(rect, 3.0 * transform.scale, config.edge_label_bg_color);
            }
            let text_pos = egui::pos2(
                rect.center().x - text_size.x * 0.5,
                rect.center().y - text_size.y * 0.5,
            );
            painter.galley(text_pos, galley, config.edge_label_color);
        }
    }

    let src_screen = *screen_points.first()?;
    let tgt_screen = *screen_points.last()?;

    Some(EdgeEndpoints {
        edge_id: edge.id.clone(),
        source_screen: src_screen,
        target_screen: tgt_screen,
        source_pos: cache.source_pos,
        target_pos: cache.target_pos,
        marker_start: edge.marker_start.clone(),
        marker_end: edge.marker_end.clone(),
        edge_color: color,
        edge_stroke_width: width,
    })
}

/// Render arrow markers for all edges. Called after the nodes pass so the
/// arrows are not overpainted by `NodeWidget` implementations that fill their
/// full screen rect (e.g. circles, filled rectangles).
pub(crate) fn render_edge_markers(painter: &egui::Painter, endpoints: &[EdgeEndpoints]) {
    for ep in endpoints {
        if let Some(marker) = &ep.marker_end {
            super::markers::render_marker(
                painter,
                ep.target_screen,
                ep.target_pos,
                ep.edge_color,
                ep.edge_stroke_width,
                marker,
                Some(ep.source_screen),
            );
        }
        if let Some(marker) = &ep.marker_start {
            super::markers::render_marker(
                painter,
                ep.source_screen,
                ep.source_pos,
                ep.edge_color,
                ep.edge_stroke_width,
                marker,
                Some(ep.target_screen),
            );
        }
    }
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

