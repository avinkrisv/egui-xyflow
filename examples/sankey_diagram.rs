#![allow(clippy::needless_range_loop)]
//! Sankey diagram example for egui_xyflow.
//!
//! Implements a D3-style Sankey energy-flow diagram: nodes are tall narrow
//! rectangles arranged in columns with heights proportional to flow volume,
//! and links are drawn as filled Bézier bands whose width represents quantity.
//!
//! Inspired by <https://observablehq.com/@d3/sankey>.
//!
//! Run with: `cargo run --example sankey_diagram`

use eframe::egui;
use egui_xyflow::prelude::*;
use egui_xyflow::EdgePosition;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NODE_WIDTH: f32 = 20.0;
const NODE_PADDING: f32 = 12.0;
const CHART_WIDTH: f32 = 800.0;
const CHART_HEIGHT: f32 = 500.0;
const BEZIER_SAMPLES: usize = 48;
const LINK_ALPHA: u8 = 100;

// ---------------------------------------------------------------------------
// Colour palette (Tableau 10 inspired)
// ---------------------------------------------------------------------------

fn node_color(index: usize) -> egui::Color32 {
    const PALETTE: [(u8, u8, u8); 13] = [
        (78, 121, 167),  // 0  Oil
        (156, 117, 95),  // 1  Coal
        (242, 142, 43),  // 2  Natural Gas
        (225, 87, 89),   // 3  Nuclear
        (89, 161, 79),   // 4  Renewables
        (176, 122, 161), // 5  Electricity
        (237, 201, 72),  // 6  Refined Fuels
        (118, 183, 178), // 7  Direct Heat
        (255, 157, 167), // 8  Transport
        (140, 162, 182), // 9  Industry
        (158, 218, 229), // 10 Residential
        (199, 199, 199), // 11 Commercial
        (200, 82, 0),    // 12 Losses
    ];
    let (r, g, b) = PALETTE[index % PALETTE.len()];
    egui::Color32::from_rgb(r, g, b)
}

fn link_color(source_index: usize) -> egui::Color32 {
    let c = node_color(source_index);
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), LINK_ALPHA)
}

// ---------------------------------------------------------------------------
// Source data
// ---------------------------------------------------------------------------

struct NodeDef {
    label: &'static str,
    layer: usize,
}

struct LinkDef {
    source: usize,
    target: usize,
    value: f32,
}

fn build_data() -> (Vec<NodeDef>, Vec<LinkDef>) {
    let nodes = vec![
        // Layer 0: Energy sources
        NodeDef { label: "Oil", layer: 0 },           // 0
        NodeDef { label: "Coal", layer: 0 },          // 1
        NodeDef { label: "Natural Gas", layer: 0 },   // 2
        NodeDef { label: "Nuclear", layer: 0 },       // 3
        NodeDef { label: "Renewables", layer: 0 },    // 4
        // Layer 1: Conversion
        NodeDef { label: "Electricity", layer: 1 },   // 5
        NodeDef { label: "Refined Fuels", layer: 1 }, // 6
        NodeDef { label: "Direct Heat", layer: 1 },   // 7
        // Layer 2: End use
        NodeDef { label: "Transport", layer: 2 },     // 8
        NodeDef { label: "Industry", layer: 2 },      // 9
        NodeDef { label: "Residential", layer: 2 },   // 10
        NodeDef { label: "Commercial", layer: 2 },    // 11
        NodeDef { label: "Losses", layer: 2 },        // 12
    ];

    let links = vec![
        // Sources → Conversion
        LinkDef { source: 0, target: 5, value: 50.0 },
        LinkDef { source: 0, target: 6, value: 450.0 },
        LinkDef { source: 1, target: 5, value: 200.0 },
        LinkDef { source: 1, target: 6, value: 50.0 },
        LinkDef { source: 1, target: 7, value: 50.0 },
        LinkDef { source: 2, target: 5, value: 100.0 },
        LinkDef { source: 2, target: 6, value: 50.0 },
        LinkDef { source: 2, target: 7, value: 100.0 },
        LinkDef { source: 3, target: 5, value: 150.0 },
        LinkDef { source: 4, target: 5, value: 80.0 },
        LinkDef { source: 4, target: 7, value: 20.0 },
        // Conversion → End use
        LinkDef { source: 5, target: 8, value: 50.0 },
        LinkDef { source: 5, target: 9, value: 200.0 },
        LinkDef { source: 5, target: 10, value: 150.0 },
        LinkDef { source: 5, target: 11, value: 100.0 },
        LinkDef { source: 5, target: 12, value: 80.0 },
        LinkDef { source: 6, target: 8, value: 400.0 },
        LinkDef { source: 6, target: 9, value: 100.0 },
        LinkDef { source: 6, target: 11, value: 50.0 },
        LinkDef { source: 7, target: 9, value: 80.0 },
        LinkDef { source: 7, target: 10, value: 60.0 },
        LinkDef { source: 7, target: 11, value: 30.0 },
    ];

    (nodes, links)
}

// ---------------------------------------------------------------------------
// Sankey layout
// ---------------------------------------------------------------------------

struct LayoutNode {
    x0: f32,
    y0: f32,
    height: f32,
    value: f32,
}

struct LayoutLink {
    source_x: f32,
    source_y: f32,
    target_x: f32,
    target_y: f32,
    width: f32,
}

fn compute_layout(
    node_defs: &[NodeDef],
    link_defs: &[LinkDef],
) -> (Vec<LayoutNode>, Vec<LayoutLink>) {
    let n = node_defs.len();

    // Compute node values = max(sum_in, sum_out)
    let mut sum_out = vec![0.0_f32; n];
    let mut sum_in = vec![0.0_f32; n];
    for link in link_defs {
        sum_out[link.source] += link.value;
        sum_in[link.target] += link.value;
    }
    let node_values: Vec<f32> = (0..n).map(|i| sum_out[i].max(sum_in[i])).collect();

    // Group nodes by layer
    let num_layers = node_defs.iter().map(|d| d.layer).max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![vec![]; num_layers];
    for (i, nd) in node_defs.iter().enumerate() {
        layers[nd.layer].push(i);
    }

    // Global scale: ensure link widths are consistent across layers.
    // All layers have the same total value (balanced Sankey), but different
    // amounts of inter-node padding.
    let max_padding = layers
        .iter()
        .map(|l| (l.len() as f32 - 1.0).max(0.0) * NODE_PADDING)
        .fold(0.0_f32, f32::max);
    let max_layer_total = layers
        .iter()
        .map(|l| l.iter().map(|&i| node_values[i]).sum::<f32>())
        .fold(0.0_f32, f32::max);
    let scale = (CHART_HEIGHT - max_padding) / max_layer_total;

    // Position nodes within each layer
    let mut layout_nodes: Vec<LayoutNode> = (0..n)
        .map(|_| LayoutNode {
            x0: 0.0,
            y0: 0.0,
            height: 0.0,
            value: 0.0,
        })
        .collect();

    for (layer_idx, node_ids) in layers.iter().enumerate() {
        let x0 = if num_layers > 1 {
            layer_idx as f32 * (CHART_WIDTH - NODE_WIDTH) / (num_layers - 1) as f32
        } else {
            (CHART_WIDTH - NODE_WIDTH) / 2.0
        };

        let layer_total_height: f32 = node_ids.iter().map(|&i| node_values[i] * scale).sum();
        let layer_padding = (node_ids.len() as f32 - 1.0).max(0.0) * NODE_PADDING;
        let y_offset = (CHART_HEIGHT - layer_total_height - layer_padding) / 2.0;

        let mut y = y_offset;
        for &i in node_ids {
            let h = node_values[i] * scale;
            layout_nodes[i] = LayoutNode {
                x0,
                y0: y,
                height: h,
                value: node_values[i],
            };
            y += h + NODE_PADDING;
        }
    }

    // Compute link positions.
    // For each source: sort outgoing links by target y-centre, stack downward.
    // For each target: sort incoming links by source y-centre, stack downward.
    let node_cy: Vec<f32> = layout_nodes
        .iter()
        .map(|ln| ln.y0 + ln.height / 2.0)
        .collect();
    let link_widths: Vec<f32> = link_defs.iter().map(|l| l.value * scale).collect();

    let mut layout_links: Vec<LayoutLink> = (0..link_defs.len())
        .map(|_| LayoutLink {
            source_x: 0.0,
            source_y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            width: 0.0,
        })
        .collect();

    // Source-side offsets
    for i in 0..n {
        let mut outgoing: Vec<usize> = (0..link_defs.len())
            .filter(|&li| link_defs[li].source == i)
            .collect();
        outgoing.sort_by(|&a, &b| {
            node_cy[link_defs[a].target]
                .partial_cmp(&node_cy[link_defs[b].target])
                .unwrap()
        });
        let mut y = layout_nodes[i].y0;
        for li in outgoing {
            let w = link_widths[li];
            layout_links[li].source_x = layout_nodes[i].x0 + NODE_WIDTH;
            layout_links[li].source_y = y + w / 2.0;
            layout_links[li].width = w;
            y += w;
        }
    }

    // Target-side offsets
    for i in 0..n {
        let mut incoming: Vec<usize> = (0..link_defs.len())
            .filter(|&li| link_defs[li].target == i)
            .collect();
        incoming.sort_by(|&a, &b| {
            node_cy[link_defs[a].source]
                .partial_cmp(&node_cy[link_defs[b].source])
                .unwrap()
        });
        let mut y = layout_nodes[i].y0;
        for li in incoming {
            let w = link_widths[li];
            layout_links[li].target_x = layout_nodes[i].x0;
            layout_links[li].target_y = y + w / 2.0;
            y += w;
        }
    }

    (layout_nodes, layout_links)
}

// ---------------------------------------------------------------------------
// FlowState data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    color: egui::Color32,
    #[allow(dead_code)]
    value: f32,
    height: f32,
    layer: usize,
    num_layers: usize,
}

#[derive(Debug, Clone, Default)]
struct LinkData {
    source_x: f32,
    source_y: f32,
    target_x: f32,
    target_y: f32,
    width: f32,
    color: egui::Color32,
}

// ---------------------------------------------------------------------------
// Custom NodeWidget — tall coloured rectangles with labels
// ---------------------------------------------------------------------------

struct SankeyNodeWidget;

impl NodeWidget<NodeData> for SankeyNodeWidget {
    fn size(&self, node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(NODE_WIDTH, node.data.height)
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<NodeData>,
        screen_rect: egui::Rect,
        config: &FlowConfig,
        hovered: bool,
        _transform: &Transform,
    ) {
        let color = if node.selected || hovered {
            let c = node.data.color;
            egui::Color32::from_rgb(
                (c.r() as u16 + 40).min(255) as u8,
                (c.g() as u16 + 40).min(255) as u8,
                (c.b() as u16 + 40).min(255) as u8,
            )
        } else {
            node.data.color
        };

        painter.rect_filled(screen_rect, 0.0, color);

        if node.selected || hovered {
            painter.rect_stroke(
                screen_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::WHITE),
                egui::StrokeKind::Middle,
            );
        }

        if node.data.label.is_empty() {
            return;
        }

        let font = egui::FontId::proportional(11.0);
        let galley =
            painter.layout_no_wrap(node.data.label.clone(), font, config.node_text_color);

        let is_last_layer = node.data.layer == node.data.num_layers - 1;
        let text_pos = if is_last_layer {
            // Right-most column: label on the left
            egui::pos2(
                screen_rect.left() - galley.size().x - 6.0,
                screen_rect.center().y - galley.size().y / 2.0,
            )
        } else {
            // All other columns: label on the right
            egui::pos2(
                screen_rect.right() + 6.0,
                screen_rect.center().y - galley.size().y / 2.0,
            )
        };

        painter.galley(text_pos, galley, config.node_text_color);
    }
}

// ---------------------------------------------------------------------------
// Custom EdgeWidget — filled Bézier bands
// ---------------------------------------------------------------------------

struct SankeyEdgeWidget;

impl EdgeWidget<LinkData> for SankeyEdgeWidget {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &Edge<LinkData>,
        _pos: &EdgePosition,
        config: &FlowConfig,
        _time: f64,
        transform: &Transform,
    ) {
        let d = match edge.data.as_ref() {
            Some(d) => d,
            None => return,
        };
        let scale = transform.scale;

        let s = egui::pos2(d.source_x * scale + transform.x, d.source_y * scale + transform.y);
        let t = egui::pos2(d.target_x * scale + transform.x, d.target_y * scale + transform.y);
        let w = d.width * scale;

        if w < 0.3 {
            return; // too thin to see
        }

        let hw = w / 2.0;
        let mx = (s.x + t.x) / 2.0;

        let color = if edge.selected {
            config.edge_selected_color
        } else {
            d.color
        };

        // Top Bézier: (source, above centre) → (target, above centre)
        // Bottom Bézier: (target, below centre) → (source, below centre)
        let mut points = Vec::with_capacity((BEZIER_SAMPLES + 1) * 2);

        for i in 0..=BEZIER_SAMPLES {
            let t_param = i as f32 / BEZIER_SAMPLES as f32;
            points.push(cubic_bezier_point(
                egui::pos2(s.x, s.y - hw),
                egui::pos2(mx, s.y - hw),
                egui::pos2(mx, t.y - hw),
                egui::pos2(t.x, t.y - hw),
                t_param,
            ));
        }

        for i in 0..=BEZIER_SAMPLES {
            let t_param = i as f32 / BEZIER_SAMPLES as f32;
            points.push(cubic_bezier_point(
                egui::pos2(t.x, t.y + hw),
                egui::pos2(mx, t.y + hw),
                egui::pos2(mx, s.y + hw),
                egui::pos2(s.x, s.y + hw),
                t_param,
            ));
        }

        painter.add(egui::epaint::PathShape {
            points,
            closed: true,
            fill: color,
            stroke: egui::epaint::PathStroke::NONE,
        });
    }
}

fn cubic_bezier_point(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    t: f32,
) -> egui::Pos2 {
    let u = 1.0 - t;
    let u2 = u * u;
    let t2 = t * t;
    egui::pos2(
        u2 * u * p0.x + 3.0 * u2 * t * p1.x + 3.0 * u * t2 * p2.x + t2 * t * p3.x,
        u2 * u * p0.y + 3.0 * u2 * t * p1.y + 3.0 * u * t2 * p2.y + t2 * t * p3.y,
    )
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct SankeyApp {
    state: FlowState<NodeData, LinkData>,
    edge_widget: SankeyEdgeWidget,
    first_frame: bool,
    node_count: usize,
    link_count: usize,
    total_flow: f32,
}

impl SankeyApp {
    fn new() -> Self {
        let (node_defs, link_defs) = build_data();
        let (layout_nodes, layout_links) = compute_layout(&node_defs, &link_defs);
        let n = node_defs.len();
        let num_layers = node_defs.iter().map(|d| d.layer).max().unwrap_or(0) + 1;
        let total_flow: f32 = link_defs
            .iter()
            .filter(|l| node_defs[l.source].layer == 0)
            .map(|l| l.value)
            .sum();

        let config = FlowConfig {
            snap_to_grid: false,
            nodes_draggable: false,
            nodes_connectable: false,
            nodes_selectable: true,
            nodes_resizable: false,
            min_zoom: 0.2,
            max_zoom: 5.0,
            show_background: false,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            edge_stroke_width: 1.0,
            default_source_position: Position::Right,
            default_target_position: Position::Left,
            ..FlowConfig::default()
        };

        let mut state = FlowState::new(config);

        for (i, nd) in node_defs.iter().enumerate() {
            let ln = &layout_nodes[i];
            state.add_node(
                Node::builder(format!("n{}", i))
                    .position(egui::pos2(ln.x0, ln.y0))
                    .data(NodeData {
                        label: nd.label.to_string(),
                        color: node_color(i),
                        value: ln.value,
                        height: ln.height,
                        layer: nd.layer,
                        num_layers,
                    })
                    .size(NODE_WIDTH, ln.height)
                    .build(),
            );
        }

        for (i, ld) in link_defs.iter().enumerate() {
            let ll = &layout_links[i];
            state.add_edge(
                {
                    let mut edge = Edge::new(
                        format!("e{}", i),
                        format!("n{}", ld.source),
                        format!("n{}", ld.target),
                    )
                    .edge_type(EdgeType::Straight);
                    edge.data = Some(LinkData {
                        source_x: ll.source_x,
                        source_y: ll.source_y,
                        target_x: ll.target_x,
                        target_y: ll.target_y,
                        width: ll.width,
                        color: link_color(ld.source),
                    });
                    edge
                },
            );
        }

        Self {
            state,
            edge_widget: SankeyEdgeWidget,
            first_frame: true,
            node_count: n,
            link_count: link_defs.len(),
            total_flow,
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for SankeyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Sankey Diagram — Energy Flow");
                ui.separator();
                ui.label(format!(
                    "{} nodes, {} links, {:.0} total flow",
                    self.node_count, self.link_count, self.total_flow
                ));
                ui.separator();
                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    self.state.fit_view(rect, 80.0, ctx.input(|i| i.time));
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                if self.first_frame {
                    let rect = ui.available_rect_before_wrap();
                    self.state
                        .fit_view(rect, 80.0, ctx.input(|i| i.time));
                    self.first_frame = false;
                }

                let _events = FlowCanvas::new(&mut self.state, &SankeyNodeWidget)
                    .edge_widget(&self.edge_widget)
                    .show(ui);
            });
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow — Sankey Diagram")
            .with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Sankey Diagram",
        options,
        Box::new(|_cc| Ok(Box::new(SankeyApp::new()))),
    )
}
