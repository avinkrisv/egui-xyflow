use crate::config::FlowConfig;
use crate::types::position::CoordinateExtent;
use crate::types::viewport::Viewport;

/// Result of a pan/zoom input frame.
pub(crate) struct PanZoomResult {
    /// True if the viewport was mutated (pan or instant zoom).
    pub(crate) changed: bool,
    /// If set, the canvas should start an animated zoom toward this screen
    /// position with this factor (used for double-click).
    pub(crate) animate_zoom: Option<(egui::Pos2, f32)>,
}

/// Handle pan/zoom input from egui.
///
/// Instant operations (scroll, pinch, middle-drag, left-drag on background)
/// are applied directly to `viewport`.  Double-click is NOT applied instantly —
/// instead the result carries `animate_zoom` so the caller can start a
/// `ViewportAnimation`.
///
/// After every mutation the viewport translation is clamped to
/// [`FlowConfig::translate_extent`] so the user cannot pan outside the
/// configured content boundary.
pub(crate) fn handle_pan_zoom(
    ui: &egui::Ui,
    response: &egui::Response,
    viewport: &mut Viewport,
    config: &FlowConfig,
    canvas_rect: egui::Rect,
    suppress_primary_pan: bool,
) -> PanZoomResult {
    let mut changed = false;
    let mut animate_zoom: Option<(egui::Pos2, f32)> = None;

    // ── Scroll wheel → instant zoom ──────────────────────────────────────────
    if config.zoom_on_scroll && response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            // Map raw pixel delta → zoom factor.
            // Raw scroll on most platforms is in pixels; 120 px ≈ one notch.
            let factor = (1.0 + scroll * 0.005).clamp(0.5, 2.0);
            let pointer = ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(canvas_rect.center());
            zoom_toward(viewport, pointer, factor, config.min_zoom, config.max_zoom);
            clamp_translate(viewport, &config.translate_extent, canvas_rect);
            changed = true;
        }
    }

    // ── Pinch → instant zoom ─────────────────────────────────────────────────
    if config.zoom_on_pinch && response.hovered() {
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > f32::EPSILON {
            let pointer = ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(canvas_rect.center());
            zoom_toward(
                viewport,
                pointer,
                zoom_delta,
                config.min_zoom,
                config.max_zoom,
            );
            clamp_translate(viewport, &config.translate_extent, canvas_rect);
            changed = true;
        }
    }

    // ── Double-click → animated zoom in ─────────────────────────────────────
    // We do NOT modify the viewport here; instead we signal the canvas to
    // start a ViewportAnimation so the zoom is smooth.
    if config.zoom_on_double_click && response.double_clicked() {
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .unwrap_or(canvas_rect.center());
        animate_zoom = Some((pointer, 1.5));
    }

    // ── Middle-mouse drag → pan ──────────────────────────────────────────────
    if config.pan_on_drag && response.dragged_by(egui::PointerButton::Middle) {
        let delta = response.drag_delta();
        viewport.x += delta.x;
        viewport.y += delta.y;
        clamp_translate(viewport, &config.translate_extent, canvas_rect);
        changed = true;
    }

    // ── Left-drag on background → pan ────────────────────────────────────────
    // Suppressed when a selection drag is in progress (Shift+drag).
    if config.pan_on_drag
        && !suppress_primary_pan
        && response.dragged_by(egui::PointerButton::Primary)
    {
        let delta = response.drag_delta();
        viewport.x += delta.x;
        viewport.y += delta.y;
        clamp_translate(viewport, &config.translate_extent, canvas_rect);
        changed = true;
    }

    // ── Scroll → pan (pan_on_scroll mode, mutually exclusive with zoom) ──────
    if config.pan_on_scroll && response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta);
        if scroll != egui::Vec2::ZERO {
            use crate::types::viewport::PanOnScrollMode;
            match config.pan_on_scroll_mode {
                PanOnScrollMode::Free => {
                    viewport.x += scroll.x;
                    viewport.y += scroll.y;
                }
                PanOnScrollMode::Horizontal => {
                    viewport.x += scroll.x + scroll.y;
                }
                PanOnScrollMode::Vertical => {
                    viewport.y += scroll.y + scroll.x;
                }
            }
            clamp_translate(viewport, &config.translate_extent, canvas_rect);
            changed = true;
        }
    }

    PanZoomResult {
        changed,
        animate_zoom,
    }
}

/// Zoom the viewport toward a screen-space point by `factor`.
pub(crate) fn zoom_toward(
    viewport: &mut Viewport,
    screen_pos: egui::Pos2,
    factor: f32,
    min: f32,
    max: f32,
) {
    let new_zoom = (viewport.zoom * factor).clamp(min, max);
    let actual_factor = new_zoom / viewport.zoom;
    viewport.x = screen_pos.x - (screen_pos.x - viewport.x) * actual_factor;
    viewport.y = screen_pos.y - (screen_pos.y - viewport.y) * actual_factor;
    viewport.zoom = new_zoom;
}

/// Clamp viewport translation so that the visible canvas area stays within
/// `translate_extent` (given in flow-space coordinates).
///
/// The extent describes the **flow-space** bounding box that the user is
/// allowed to pan into.  We derive the allowed viewport-offset range from the
/// canvas rect size and the current zoom.
///
/// If either extent bound is infinite (the default) the axis is unclamped.
pub(crate) fn clamp_translate(
    viewport: &mut Viewport,
    extent: &CoordinateExtent,
    canvas_rect: egui::Rect,
) {
    let z = viewport.zoom;

    // --- X axis ---
    let min_flow_x = extent.min.x;
    let max_flow_x = extent.max.x;

    if min_flow_x.is_finite() {
        // The left edge of the canvas (canvas_rect.min.x) must map to a flow-x
        // >= min_flow_x, i.e.  (canvas.min.x - vp.x) / z >= min_flow_x
        // →  vp.x <= canvas.min.x - min_flow_x * z
        let vp_x_upper = canvas_rect.min.x - min_flow_x * z;
        viewport.x = viewport.x.min(vp_x_upper);
    }

    if max_flow_x.is_finite() {
        // The right edge of the canvas must map to a flow-x <= max_flow_x.
        // (canvas.max.x - vp.x) / z <= max_flow_x
        // →  vp.x >= canvas.max.x - max_flow_x * z
        let vp_x_lower = canvas_rect.max.x - max_flow_x * z;
        viewport.x = viewport.x.max(vp_x_lower);
    }

    // --- Y axis ---
    let min_flow_y = extent.min.y;
    let max_flow_y = extent.max.y;

    if min_flow_y.is_finite() {
        let vp_y_upper = canvas_rect.min.y - min_flow_y * z;
        viewport.y = viewport.y.min(vp_y_upper);
    }

    if max_flow_y.is_finite() {
        let vp_y_lower = canvas_rect.max.y - max_flow_y * z;
        viewport.y = viewport.y.max(vp_y_lower);
    }
}

/// Calculate auto-pan velocity when dragging near the canvas edge.
///
/// Returns a velocity vector in screen pixels/frame that should be subtracted
/// from the viewport translation.
pub(crate) fn calc_auto_pan(
    pos: egui::Pos2,
    canvas_rect: egui::Rect,
    speed: f32,
    distance: f32,
) -> egui::Vec2 {
    let axis = |value: f32, min: f32, max: f32| -> f32 {
        if value - min < distance {
            let d = (value - min).max(1.0);
            -(distance - d) / distance * speed
        } else if max - value < distance {
            let d = (max - value).max(1.0);
            (distance - d) / distance * speed
        } else {
            0.0
        }
    };

    egui::vec2(
        axis(pos.x, canvas_rect.min.x, canvas_rect.max.x),
        axis(pos.y, canvas_rect.min.y, canvas_rect.max.y),
    )
}
