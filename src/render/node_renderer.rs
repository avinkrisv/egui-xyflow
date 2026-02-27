//! Node rendering via the [`NodeWidget`] trait.
//!
//! Implement [`NodeWidget`] for custom node appearance; use
//! [`DefaultNodeWidget`] (for `String` data) or [`UnitNodeWidget`] (for `()`)
//! for built-in rectangle rendering.

use crate::config::FlowConfig;
use crate::types::node::Node;
use crate::types::position::Transform;

/// Trait for custom node rendering.
pub trait NodeWidget<D> {
    fn size(&self, node: &Node<D>, config: &FlowConfig) -> egui::Vec2;
    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<D>,
        screen_rect: egui::Rect,
        config: &FlowConfig,
        hovered: bool,
        transform: &Transform,
    );
}

/// Apply `config.node_bg_opacity` to a colour's alpha channel.
fn apply_bg_opacity(color: egui::Color32, opacity: f32) -> egui::Color32 {
    if opacity >= 1.0 {
        return color;
    }
    let [r, g, b, a] = color.to_array();
    let new_a = (a as f32 * opacity.clamp(0.0, 1.0)) as u8;
    egui::Color32::from_rgba_unmultiplied(r, g, b, new_a)
}

/// Default node renderer: rounded rectangle with label.
pub struct DefaultNodeWidget;

impl NodeWidget<String> for DefaultNodeWidget {
    fn size(&self, node: &Node<String>, config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(
            node.width.unwrap_or(config.default_node_width),
            node.height.unwrap_or(config.default_node_height),
        )
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<String>,
        screen_rect: egui::Rect,
        config: &FlowConfig,
        _hovered: bool,
        _transform: &Transform,
    ) {
        let bg = apply_bg_opacity(
            if node.selected {
                config.node_selected_bg_color
            } else {
                config.node_bg_color
            },
            config.node_bg_opacity,
        );
        let border = if node.selected {
            config.node_selected_border_color
        } else {
            config.node_border_color
        };

        let rounding = config.node_corner_radius;

        // Shadow
        if node.selected {
            let shadow_rect = screen_rect.expand(2.0);
            painter.rect_filled(
                shadow_rect,
                rounding + 1.0,
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 40),
            );
        }

        // Background
        painter.rect_filled(screen_rect, rounding, bg);

        // Border
        painter.rect_stroke(
            screen_rect,
            rounding,
            egui::Stroke::new(
                if node.selected {
                    config.node_border_width * 2.0
                } else {
                    config.node_border_width
                },
                border,
            ),
            egui::StrokeKind::Middle,
        );

        // Label
        let galley = painter.layout_no_wrap(
            node.data.clone(),
            egui::FontId::proportional(13.0),
            config.node_text_color,
        );
        let text_pos = egui::pos2(
            screen_rect.center().x - galley.size().x / 2.0,
            screen_rect.center().y - galley.size().y / 2.0,
        );
        painter.galley(text_pos, galley, config.node_text_color);
    }
}

/// Default unit node widget (for Node<()>).
pub struct UnitNodeWidget;

impl NodeWidget<()> for UnitNodeWidget {
    fn size(&self, node: &Node<()>, config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(
            node.width.unwrap_or(config.default_node_width),
            node.height.unwrap_or(config.default_node_height),
        )
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<()>,
        screen_rect: egui::Rect,
        config: &FlowConfig,
        _hovered: bool,
        _transform: &Transform,
    ) {
        let bg = apply_bg_opacity(
            if node.selected {
                config.node_selected_bg_color
            } else {
                config.node_bg_color
            },
            config.node_bg_opacity,
        );
        let border = if node.selected {
            config.node_selected_border_color
        } else {
            config.node_border_color
        };

        painter.rect_filled(screen_rect, config.node_corner_radius, bg);
        painter.rect_stroke(
            screen_rect,
            config.node_corner_radius,
            egui::Stroke::new(config.node_border_width, border),
            egui::StrokeKind::Middle,
        );

        let label = node.node_type.as_deref().unwrap_or("Node");
        let galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(13.0),
            config.node_text_color,
        );
        let text_pos = egui::pos2(
            screen_rect.center().x - galley.size().x / 2.0,
            screen_rect.center().y - galley.size().y / 2.0,
        );
        painter.galley(text_pos, galley, config.node_text_color);
    }
}
