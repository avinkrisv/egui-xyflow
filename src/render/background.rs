use crate::config::{BackgroundVariant, FlowConfig};
use crate::types::viewport::Viewport;

pub(crate) fn render_background(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    viewport: &Viewport,
    config: &FlowConfig,
) {
    if !config.show_background {
        return;
    }

    let gap = config.background_gap * viewport.zoom;
    let size = config.background_size * viewport.zoom;
    let color = config.background_color;

    // Offset by viewport translation
    let offset_x = viewport.x % gap;
    let offset_y = viewport.y % gap;

    match config.background_variant {
        BackgroundVariant::Dots => {
            let mut x = canvas_rect.min.x + offset_x;
            while x < canvas_rect.max.x {
                let mut y = canvas_rect.min.y + offset_y;
                while y < canvas_rect.max.y {
                    painter.circle_filled(egui::pos2(x, y), size * 0.5, color);
                    y += gap;
                }
                x += gap;
            }
        }
        BackgroundVariant::Lines => {
            let stroke = egui::Stroke::new(size * 0.5, color);
            let mut x = canvas_rect.min.x + offset_x;
            while x < canvas_rect.max.x {
                painter.line_segment(
                    [egui::pos2(x, canvas_rect.min.y), egui::pos2(x, canvas_rect.max.y)],
                    stroke,
                );
                x += gap;
            }
            let mut y = canvas_rect.min.y + offset_y;
            while y < canvas_rect.max.y {
                painter.line_segment(
                    [egui::pos2(canvas_rect.min.x, y), egui::pos2(canvas_rect.max.x, y)],
                    stroke,
                );
                y += gap;
            }
        }
        BackgroundVariant::Cross => {
            let stroke = egui::Stroke::new(size * 0.5, color);
            let arm = gap * 0.15;
            let mut x = canvas_rect.min.x + offset_x;
            while x < canvas_rect.max.x {
                let mut y = canvas_rect.min.y + offset_y;
                while y < canvas_rect.max.y {
                    painter.line_segment(
                        [egui::pos2(x - arm, y), egui::pos2(x + arm, y)],
                        stroke,
                    );
                    painter.line_segment(
                        [egui::pos2(x, y - arm), egui::pos2(x, y + arm)],
                        stroke,
                    );
                    y += gap;
                }
                x += gap;
            }
        }
    }
}
