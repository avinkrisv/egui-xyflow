use egui::{Color32, Rect, Stroke};

/// Render the selection rectangle overlay (solid border + semi-transparent fill).
pub fn render_selection_rect(painter: &egui::Painter, selection_rect: Rect) {
    // Semi-transparent fill
    painter.rect_filled(
        selection_rect,
        0.0,
        Color32::from_rgba_unmultiplied(59, 130, 246, 30),
    );

    // Solid border
    painter.rect_stroke(
        selection_rect,
        0.0,
        Stroke::new(1.0, Color32::from_rgb(59, 130, 246)),
        egui::StrokeKind::Middle,
    );
}
