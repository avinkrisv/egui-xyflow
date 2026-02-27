//! Neural Network Architecture Visualizer
//!
//! Demonstrates using egui_xyflow to visualize a neural network as a sequential
//! chain of layer nodes connected by animated edges.
//!
//! Architecture (default)
//! ----------------------
//!   Input (784) -> Dense (128) -> ReLU -> Dropout (0.2) ->
//!   Dense (64) -> ReLU -> Dense (10) -> Softmax
//!
//! Features demonstrated
//! ---------------------
//! - Custom NodeWidget implementation for layer-specific styling
//! - ConnectionValidator that enforces sequential wiring (max 1 input, 1 output)
//! - Animated edges showing data-flow direction
//! - Side panel with architecture summary and parameter count
//! - Toolbar to add common layer types at the end of the chain
//! - Auto-generated edges when new layers are appended

use eframe::egui;
use egui_xyflow::prelude::*;

// ---------------------------------------------------------------------------
// Layer metadata
// ---------------------------------------------------------------------------

/// Describes a single neural-network layer.
#[derive(Debug, Clone)]
struct LayerInfo {
    /// Human-readable layer name, e.g. "Dense (128)".
    label: String,
    /// Output dimensionality as a descriptive string, e.g. "(batch, 128)".
    output_shape: String,
    /// Trainable parameter count for this layer.
    params: usize,
    /// Visual category (used for colouring).
    category: LayerCategory,
}

impl Default for LayerInfo {
    fn default() -> Self {
        Self {
            label: String::new(),
            output_shape: String::new(),
            params: 0,
            category: LayerCategory::Dense,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerCategory {
    Input,
    Dense,
    Activation,
    Regularization,
    Convolution,
    Output,
}

impl LayerCategory {
    fn color(&self) -> egui::Color32 {
        match self {
            LayerCategory::Input => egui::Color32::from_rgb(66, 133, 244),
            LayerCategory::Dense => egui::Color32::from_rgb(52, 168, 83),
            LayerCategory::Activation => egui::Color32::from_rgb(251, 188, 4),
            LayerCategory::Regularization => egui::Color32::from_rgb(234, 67, 53),
            LayerCategory::Convolution => egui::Color32::from_rgb(171, 71, 188),
            LayerCategory::Output => egui::Color32::from_rgb(0, 172, 193),
        }
    }
}

// ---------------------------------------------------------------------------
// Custom node widget
// ---------------------------------------------------------------------------

/// Renders each layer node with a coloured header bar, layer name, and output
/// shape text underneath.
struct LayerNodeWidget;

impl NodeWidget<LayerInfo> for LayerNodeWidget {
    fn size(&self, node: &Node<LayerInfo>, config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(
            node.width.unwrap_or(config.default_node_width),
            node.height.unwrap_or(config.default_node_height),
        )
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<LayerInfo>,
        screen_rect: egui::Rect,
        config: &FlowConfig,
        _hovered: bool,
        _transform: &Transform,
    ) {
        let info = &node.data;
        let cat_color = info.category.color();
        let rounding = config.node_corner_radius;

        // -- Shadow when selected -----------------------------------------------
        if node.selected {
            let shadow_rect = screen_rect.expand(3.0);
            painter.rect_filled(
                shadow_rect,
                rounding + 1.0,
                egui::Color32::from_rgba_unmultiplied(
                    cat_color.r(),
                    cat_color.g(),
                    cat_color.b(),
                    50,
                ),
            );
        }

        // -- Background ---------------------------------------------------------
        let bg = if node.selected {
            config.node_selected_bg_color
        } else {
            config.node_bg_color
        };
        painter.rect_filled(screen_rect, rounding, bg);

        // -- Coloured header bar (top 6px) --------------------------------------
        let header_rect = egui::Rect::from_min_size(
            screen_rect.min,
            egui::vec2(screen_rect.width(), 6.0),
        );
        // Use the same rounding for the top bar; the bottom overlaps with the
        // card body so the visual effect is a rounded top strip.
        painter.rect_filled(header_rect, rounding, cat_color);

        // -- Border -------------------------------------------------------------
        let border_color = if node.selected {
            cat_color
        } else {
            config.node_border_color
        };
        let border_width = if node.selected {
            config.node_border_width * 2.0
        } else {
            config.node_border_width
        };
        painter.rect_stroke(
            screen_rect,
            rounding,
            egui::Stroke::new(border_width, border_color),
            egui::StrokeKind::Middle,
        );

        // -- Layer name ---------------------------------------------------------
        let name_galley = painter.layout_no_wrap(
            info.label.clone(),
            egui::FontId::proportional(13.0),
            config.node_text_color,
        );
        let name_y = screen_rect.min.y + 10.0;
        let name_x = screen_rect.center().x - name_galley.size().x / 2.0;
        painter.galley(egui::pos2(name_x, name_y), name_galley, config.node_text_color);

        // -- Output shape -------------------------------------------------------
        let shape_galley = painter.layout_no_wrap(
            info.output_shape.clone(),
            egui::FontId::proportional(10.0),
            egui::Color32::GRAY,
        );
        let shape_y = screen_rect.min.y + 28.0;
        let shape_x = screen_rect.center().x - shape_galley.size().x / 2.0;
        painter.galley(egui::pos2(shape_x, shape_y), shape_galley, egui::Color32::GRAY);

        // -- Param count (small, bottom-right) ----------------------------------
        if info.params > 0 {
            let params_text = format!("{}p", format_params(info.params));
            let params_galley = painter.layout_no_wrap(
                params_text,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgb(140, 140, 140),
            );
            let px = screen_rect.max.x - params_galley.size().x - 4.0;
            let py = screen_rect.max.y - params_galley.size().y - 3.0;
            painter.galley(
                egui::pos2(px, py),
                params_galley,
                egui::Color32::from_rgb(140, 140, 140),
            );
        }
    }
}

/// Format a parameter count as a short human-readable string (e.g. "100.5K").
fn format_params(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

// ---------------------------------------------------------------------------
// Connection validator -- sequential only
// ---------------------------------------------------------------------------

/// Allows a connection only when:
/// - source != target (no self-loops)
/// - the source node does not already have an outgoing edge
/// - the target node does not already have an incoming edge
///
/// Uses the `existing_edges` slice provided by the framework.
struct SequentialValidator;

impl ConnectionValidator for SequentialValidator {
    fn is_valid_connection(&self, connection: &Connection, existing_edges: &[EdgeInfo<'_>]) -> bool {
        if connection.source == connection.target {
            return false;
        }
        // Each node may have at most 1 outgoing and 1 incoming edge.
        let src_free = !existing_edges.iter().any(|e| *e.source == connection.source);
        let tgt_free = !existing_edges.iter().any(|e| *e.target == connection.target);
        src_free && tgt_free
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct NeuralNetApp {
    state: FlowState<LayerInfo, ()>,
    validator: SequentialValidator,
    layer_widget: LayerNodeWidget,
    /// Auto-incrementing counter for node IDs.
    next_id: usize,
}


fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- Neural Network Visualizer")
            .with_inner_size([1400.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Neural Network Visualizer",
        options,
        Box::new(|_cc| Ok(Box::new(NeuralNetApp::new()))),
    )
}

impl NeuralNetApp {
    fn new() -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            show_minimap: true,
            default_node_width: 160.0,
            default_node_height: 54.0,
            node_corner_radius: 6.0,
            ..FlowConfig::default()
        };

        let mut state: FlowState<LayerInfo, ()> = FlowState::new(config);

        // -- Define the default architecture layers -----------------------------
        let layers: Vec<LayerInfo> = vec![
            LayerInfo {
                label: "Input (784)".into(),
                output_shape: "(batch, 784)".into(),
                params: 0,
                category: LayerCategory::Input,
            },
            LayerInfo {
                label: "Dense (128)".into(),
                output_shape: "(batch, 128)".into(),
                params: 784 * 128 + 128, // weights + biases
                category: LayerCategory::Dense,
            },
            LayerInfo {
                label: "ReLU".into(),
                output_shape: "(batch, 128)".into(),
                params: 0,
                category: LayerCategory::Activation,
            },
            LayerInfo {
                label: "Dropout (0.2)".into(),
                output_shape: "(batch, 128)".into(),
                params: 0,
                category: LayerCategory::Regularization,
            },
            LayerInfo {
                label: "Dense (64)".into(),
                output_shape: "(batch, 64)".into(),
                params: 128 * 64 + 64,
                category: LayerCategory::Dense,
            },
            LayerInfo {
                label: "ReLU".into(),
                output_shape: "(batch, 64)".into(),
                params: 0,
                category: LayerCategory::Activation,
            },
            LayerInfo {
                label: "Dense (10)".into(),
                output_shape: "(batch, 10)".into(),
                params: 64 * 10 + 10,
                category: LayerCategory::Dense,
            },
            LayerInfo {
                label: "Softmax".into(),
                output_shape: "(batch, 10)".into(),
                params: 0,
                category: LayerCategory::Output,
            },
        ];

        // -- Create nodes in a horizontal chain ---------------------------------
        let x_start = 60.0;
        let x_spacing = 200.0;
        let y_pos = 200.0;
        let node_w = 160.0;
        let node_h = 54.0;

        for (i, layer) in layers.iter().enumerate() {
            let id = format!("{}", i + 1);
            let x = x_start + i as f32 * x_spacing;

            let mut builder = Node::builder(&id)
                .position(egui::pos2(x, y_pos))
                .data(layer.clone())
                .size(node_w, node_h);

            // First node: source handle only.
            // Last node: target handle only.
            // Middle nodes: both.
            if i > 0 {
                builder = builder.handle(NodeHandle::target(Position::Left));
            }
            if i < layers.len() - 1 {
                builder = builder.handle(NodeHandle::source(Position::Right));
            }

            state.add_node(builder.build());
        }

        // -- Create edges between consecutive layers ----------------------------
        for i in 0..layers.len() - 1 {
            let src = format!("{}", i + 1);
            let tgt = format!("{}", i + 2);
            let eid = format!("e{}-{}", i + 1, i + 2);

            state.add_edge(
                Edge::new(eid, src, tgt)
                    .edge_type(EdgeType::SmoothStep)
                    .animated(true)
                    .marker_end_arrow(),
            );
        }

        Self {
            state,
            validator: SequentialValidator,
            layer_widget: LayerNodeWidget,
            next_id: layers.len() + 1,
        }
    }

    /// Append a new layer node at the end of the chain, automatically wiring it
    /// to the previous tail node.
    fn add_layer(&mut self, info: LayerInfo) {
        let new_id = format!("{}", self.next_id);
        self.next_id += 1;

        // Find the current tail node (no outgoing edge).
        let sources: Vec<String> = self.state.edges.iter().map(|e| e.source.0.clone()).collect();
        let tail_id = self
            .state
            .nodes
            .iter()
            .rev()
            .find(|n| !sources.contains(&n.id.0))
            .map(|n| n.id.0.clone());

        // Position the new node to the right of the tail.
        let (x, y) = if let Some(ref tid) = tail_id {
            if let Some(tail) = self.state.nodes.iter().find(|n| n.id.0 == *tid) {
                (tail.position.x + 200.0, tail.position.y)
            } else {
                (60.0, 200.0)
            }
        } else {
            (60.0, 200.0)
        };

        // Add a source handle to the old tail if it does not have one.
        if let Some(ref tid) = tail_id {
            if let Some(tail) = self.state.nodes.iter_mut().find(|n| n.id.0 == *tid) {
                let has_source = tail.handles.iter().any(|h| h.handle_type == HandleType::Source);
                if !has_source {
                    tail.handles.push(NodeHandle::source(Position::Right));
                }
            }
            // Rebuild lookup after mutating handles.
            self.state.rebuild_lookup();
        }

        // Build the new node with a target handle (source may be added later if
        // another layer is appended).
        let node = Node::builder(&new_id)
            .position(egui::pos2(x, y))
            .data(info)
            .handle(NodeHandle::target(Position::Left))
            .size(160.0, 54.0)
            .build();
        self.state.add_node(node);

        // Wire the old tail to the new node.
        if let Some(tid) = tail_id {
            let eid = format!("e{}-{}", tid, new_id);
            self.state.add_edge(
                Edge::new(eid, tid, new_id)
                    .edge_type(EdgeType::SmoothStep)
                    .animated(true)
                    .marker_end_arrow(),
            );
        }

    }

    /// Compute total trainable parameters across all layers.
    fn total_params(&self) -> usize {
        self.state.nodes.iter().map(|n| n.data.params).sum()
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for NeuralNetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -- Toolbar ------------------------------------------------------------
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Neural Network Visualizer");
                ui.separator();

                // Viewport controls
                if ui.button("Fit All").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 40.0, t);
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }

                ui.separator();

                // -- Add-layer buttons ------------------------------------------
                ui.label("Add layer:");

                if ui.button("Dense (256)").clicked() {
                    self.add_layer(LayerInfo {
                        label: "Dense (256)".into(),
                        output_shape: "(batch, 256)".into(),
                        params: 0, // user would compute based on previous layer
                        category: LayerCategory::Dense,
                    });
                }
                if ui.button("Conv2D (3x3)").clicked() {
                    self.add_layer(LayerInfo {
                        label: "Conv2D (3x3)".into(),
                        output_shape: "(batch, H, W, C)".into(),
                        params: 0,
                        category: LayerCategory::Convolution,
                    });
                }
                if ui.button("ReLU").clicked() {
                    self.add_layer(LayerInfo {
                        label: "ReLU".into(),
                        output_shape: "(same)".into(),
                        params: 0,
                        category: LayerCategory::Activation,
                    });
                }
                if ui.button("Dropout (0.5)").clicked() {
                    self.add_layer(LayerInfo {
                        label: "Dropout (0.5)".into(),
                        output_shape: "(same)".into(),
                        params: 0,
                        category: LayerCategory::Regularization,
                    });
                }
                if ui.button("Softmax").clicked() {
                    self.add_layer(LayerInfo {
                        label: "Softmax".into(),
                        output_shape: "(same)".into(),
                        params: 0,
                        category: LayerCategory::Output,
                    });
                }
            });
        });

        // -- Side panel: architecture summary -----------------------------------
        egui::SidePanel::right("summary")
            .resizable(true)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Architecture");
                ui.separator();

                // Parameter summary
                let total = self.total_params();
                ui.label(
                    egui::RichText::new(format!("Total parameters: {}", format_params(total)))
                        .strong()
                        .size(14.0),
                );
                ui.label(
                    egui::RichText::new(format!("Layers: {}", self.state.nodes.len()))
                        .size(13.0),
                );
                ui.label(
                    egui::RichText::new(format!("Connections: {}", self.state.edges.len()))
                        .size(13.0),
                );
                ui.add_space(10.0);

                // Legend
                ui.separator();
                ui.label(egui::RichText::new("Layer types").strong());
                ui.add_space(4.0);
                for (cat, label) in [
                    (LayerCategory::Input, "Input"),
                    (LayerCategory::Dense, "Dense / Linear"),
                    (LayerCategory::Activation, "Activation"),
                    (LayerCategory::Regularization, "Regularization"),
                    (LayerCategory::Convolution, "Convolution"),
                    (LayerCategory::Output, "Output"),
                ] {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(12.0, 12.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 2.0, cat.color());
                        ui.label(label);
                    });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.label(egui::RichText::new("Layer sequence").strong());
                ui.add_space(4.0);

                // Walk the chain in order (follow edges from source to target).
                let ordered = ordered_layer_ids(&self.state);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, nid) in ordered.iter().enumerate() {
                            if let Some(node) = self.state.nodes.iter().find(|n| n.id.0 == *nid) {
                                let info = &node.data;
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{}.", idx + 1))
                                            .monospace()
                                            .size(11.0),
                                    );
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(8.0, 8.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 1.0, info.category.color());
                                    ui.label(
                                        egui::RichText::new(&info.label).monospace().size(11.0),
                                    );
                                });
                                ui.label(
                                    egui::RichText::new(format!(
                                        "   out: {}  params: {}",
                                        info.output_shape,
                                        format_params(info.params)
                                    ))
                                    .monospace()
                                    .size(10.0)
                                    .color(egui::Color32::GRAY),
                                );
                            }
                        }
                    });
            });

        // -- Main canvas -------------------------------------------------------
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(245, 245, 250)))
            .show(ctx, |ui| {
                let _events = FlowCanvas::new(&mut self.state, &self.layer_widget)
                    .connection_validator(&self.validator)
                    .show(ui);
            });
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk the layer chain in topological (sequential) order.
/// Returns a vec of node ID strings starting from the head (no incoming edge)
/// and following outgoing edges.
fn ordered_layer_ids<ND: Clone>(state: &FlowState<ND, ()>) -> Vec<String> {
    use std::collections::HashMap;

    // Build source->target map.
    let mut fwd: HashMap<String, String> = HashMap::new();
    let mut has_incoming: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in &state.edges {
        fwd.insert(e.source.0.clone(), e.target.0.clone());
        has_incoming.insert(e.target.0.clone());
    }

    // Find the head node (no incoming edge).
    let head = state
        .nodes
        .iter()
        .find(|n| !has_incoming.contains(&n.id.0))
        .map(|n| n.id.0.clone());

    let mut result = Vec::new();
    if let Some(mut current) = head {
        result.push(current.clone());
        while let Some(next) = fwd.get(&current) {
            result.push(next.clone());
            current = next.clone();
        }
    }

    // Append any orphan nodes not reached by the chain walk.
    for n in &state.nodes {
        if !result.contains(&n.id.0) {
            result.push(n.id.0.clone());
        }
    }

    result
}
