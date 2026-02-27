//! Minimap rendering for `FlowCanvas`.
//!
//! [`render_minimap`] draws a small overview panel in the bottom-right corner
//! of the canvas.  It returns the screen-space [`egui::Rect`] that the minimap
//! occupies so that the caller can register interaction (click-to-pan, etc.)
//! on that region.

use crate::config::FlowConfig;
use crate::types::node::{InternalNode, NodeId};
use crate::types::viewport::Viewport;
use std::collections::HashMap;

const MINIMAP_WIDTH: f32 = 180.0;
const MINIMAP_HEIGHT: f32 = 120.0;
const MINIMAP_MARGIN: f32 = 12.0;
const MINIMAP_PADDING: f32 = 8.0;

/// Render a minimap overview in the bottom-right corner of the canvas.
///
/// Returns the screen-space [`egui::Rect`] the minimap occupies, or `None`
/// when the minimap is disabled.  The caller can use the returned rect to
/// drive click-to-pan and other interactions via [`egui::Ui::interact`].
pub fn render_minimap<D>(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    viewport: &Viewport,
    node_lookup: &HashMap<NodeId, InternalNode<D>>,
    config: &FlowConfig,
) -> Option<MinimapInfo> {
    if !config.show_minimap {
        return None;
    }

    // ── Minimap screen rect ───────────────────────────────────────────────────
    let mm_rect = egui::Rect::from_min_size(
        egui::pos2(
            canvas_rect.max.x - MINIMAP_WIDTH - MINIMAP_MARGIN,
            canvas_rect.max.y - MINIMAP_HEIGHT - MINIMAP_MARGIN,
        ),
        egui::vec2(MINIMAP_WIDTH, MINIMAP_HEIGHT),
    );

    // ── Background ────────────────────────────────────────────────────────────
    painter.rect_filled(
        mm_rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(20, 20, 30, 210),
    );
    painter.rect_stroke(
        mm_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 100)),
        egui::StrokeKind::Middle,
    );

    // ── Compute flow-space bounding box of all visible nodes ─────────────────
    let (flow_min, flow_max) = get_flow_bounds(node_lookup);
    if flow_min.x > flow_max.x || flow_min.y > flow_max.y {
        // No visible nodes — return the rect so interactions still work.
        return Some(MinimapInfo {
            mm_rect,
            flow_min,
            flow_max,
            mm_scale: 1.0,
            origin: mm_rect.center(),
        });
    }

    let flow_w = (flow_max.x - flow_min.x).max(1.0);
    let flow_h = (flow_max.y - flow_min.y).max(1.0);

    let inner = mm_rect.shrink(MINIMAP_PADDING);
    let scale_x = inner.width() / flow_w;
    let scale_y = inner.height() / flow_h;
    let mm_scale = scale_x.min(scale_y);

    // Centre the graph within the inner minimap rect.
    let graph_screen_w = flow_w * mm_scale;
    let graph_screen_h = flow_h * mm_scale;
    let origin = egui::pos2(
        inner.min.x + (inner.width() - graph_screen_w) / 2.0,
        inner.min.y + (inner.height() - graph_screen_h) / 2.0,
    );

    let flow_to_mm = |fp: egui::Pos2| -> egui::Pos2 {
        egui::pos2(
            origin.x + (fp.x - flow_min.x) * mm_scale,
            origin.y + (fp.y - flow_min.y) * mm_scale,
        )
    };

    // ── Draw nodes as small rects ─────────────────────────────────────────────
    for node in node_lookup.values() {
        if node.node.hidden {
            continue;
        }
        let fp = node.internals.position_absolute;
        let fw = node.width();
        let fh = node.height();

        let mm_min = flow_to_mm(fp);
        let mm_max = flow_to_mm(egui::pos2(fp.x + fw, fp.y + fh));
        let node_rect = egui::Rect::from_min_max(mm_min, mm_max);

        // Clamp to the inner minimap area.
        let node_rect = node_rect.intersect(inner);
        if node_rect.width() <= 0.0 || node_rect.height() <= 0.0 {
            continue;
        }

        let color = if node.node.selected {
            config.node_selected_border_color
        } else {
            egui::Color32::from_rgb(180, 180, 200)
        };

        painter.rect_filled(node_rect, 1.0, color);
    }

    // ── Viewport indicator ────────────────────────────────────────────────────
    // The viewport maps: screen = flow * zoom + (vp.x, vp.y)
    // Inverse:           flow   = (screen - (vp.x, vp.y)) / zoom
    let vp = viewport;
    let canvas_flow_min = egui::pos2(
        (canvas_rect.min.x - vp.x) / vp.zoom,
        (canvas_rect.min.y - vp.y) / vp.zoom,
    );
    let canvas_flow_max = egui::pos2(
        (canvas_rect.max.x - vp.x) / vp.zoom,
        (canvas_rect.max.y - vp.y) / vp.zoom,
    );

    let vp_mm_min = flow_to_mm(canvas_flow_min);
    let vp_mm_max = flow_to_mm(canvas_flow_max);
    let vp_rect = egui::Rect::from_min_max(vp_mm_min, vp_mm_max);

    // Clamp to minimap bounds for display.
    let vp_rect_clamped = vp_rect.intersect(mm_rect.expand(2.0));

    // Viewport fill.
    painter.rect_filled(
        vp_rect_clamped,
        2.0,
        egui::Color32::from_rgba_unmultiplied(59, 130, 246, 35),
    );

    // Viewport border.
    painter.rect_stroke(
        vp_rect_clamped,
        2.0,
        egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(59, 130, 246, 200),
        ),
        egui::StrokeKind::Middle,
    );

    Some(MinimapInfo {
        mm_rect,
        flow_min,
        flow_max,
        mm_scale,
        origin,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// MinimapInfo — returned to the caller for interaction
// ─────────────────────────────────────────────────────────────────────────────

/// Geometry information about the rendered minimap, returned by
/// `render_minimap`.  Use the `mm_rect` and scale fields to convert a
/// click position inside the minimap into a flow-space coordinate.
#[derive(Clone, Copy, Debug)]
pub struct MinimapInfo {
    /// Screen-space rect of the entire minimap panel.
    pub mm_rect: egui::Rect,
    /// Flow-space bounding box of all visible nodes (min corner).
    pub flow_min: egui::Pos2,
    /// Flow-space bounding box of all visible nodes (max corner).
    pub flow_max: egui::Pos2,
    /// Scale factor: minimap-pixels per flow-unit.
    pub mm_scale: f32,
    /// Screen-space origin of the (possibly centred) graph area inside the
    /// minimap.
    pub origin: egui::Pos2,
}

impl MinimapInfo {
    /// Convert a screen-space position inside the minimap to the corresponding
    /// **flow-space** coordinate.
    ///
    /// Returns `None` when `screen_pos` is outside the minimap rect.
    pub fn screen_to_flow(&self, screen_pos: egui::Pos2) -> Option<egui::Pos2> {
        if !self.mm_rect.contains(screen_pos) {
            return None;
        }
        if self.mm_scale <= 0.0 {
            return None;
        }
        let flow_x = (screen_pos.x - self.origin.x) / self.mm_scale + self.flow_min.x;
        let flow_y = (screen_pos.y - self.origin.y) / self.mm_scale + self.flow_min.y;
        Some(egui::pos2(flow_x, flow_y))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `(min, max)` of all visible node positions in flow space.
/// When there are no visible nodes `min > max` (sentinel "empty" value).
fn get_flow_bounds<D>(node_lookup: &HashMap<NodeId, InternalNode<D>>) -> (egui::Pos2, egui::Pos2) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for node in node_lookup.values() {
        if node.node.hidden {
            continue;
        }
        let fp = node.internals.position_absolute;
        let fw = node.width();
        let fh = node.height();

        min_x = min_x.min(fp.x);
        min_y = min_y.min(fp.y);
        max_x = max_x.max(fp.x + fw);
        max_y = max_y.max(fp.y + fh);
    }

    (egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
}
