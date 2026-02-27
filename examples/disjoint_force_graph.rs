//! Disjoint force-directed graph example for egui_xyflow.
//!
//! Loads a citation network from a JSON file (nodes = papers & patents,
//! links = citations). Multiple disconnected components are kept in view
//! via position forces (forceX / forceY).
//!
//! The force simulation faithfully matches D3.js defaults:
//! - `forceManyBody()` with strength -30, repulsion proportional to 1/dist²
//! - `forceLink()` with degree-based strength and bias
//! - `forceX()` / `forceY()` with strength 0.1
//! - `alphaTarget(0.3)` during drag, `alphaTarget(0)` on release
//!
//! Inspired by <https://observablehq.com/@d3/disjoint-force-directed-graph/2>.
//!
//! Run with: `cargo run --example disjoint_force_graph`

use std::collections::HashMap;

use eframe::egui;
use egui_xyflow::prelude::*;
use egui_xyflow::EdgePosition;

// ---------------------------------------------------------------------------
// Embedded JSON data
// ---------------------------------------------------------------------------

const JSON_DATA: &str = include_str!("disjoint_force_graph_data.json");

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const BASE_RADIUS: f32 = 5.0; // minimum circle radius
const RADIUS_SCALE: f32 = 1.0; // extra radius per data-radius unit

// Force simulation — D3 defaults
const CHARGE_STRENGTH: f32 = -30.0;
const LINK_DISTANCE: f32 = 30.0;
const POSITION_STRENGTH: f32 = 0.1;
const VELOCITY_DECAY: f32 = 0.6; // D3 internal: 1 - api_decay(0.4)
const ALPHA_MIN: f32 = 0.001;
const ALPHA_DECAY: f32 = 0.0228; // 1 - pow(0.001, 1/300)
const CHARGE_DIST_MIN2: f32 = 1.0;
const INITIAL_RADIUS: f32 = 10.0; // phyllotaxis spiral scale

// Group colours — "Cited Works" = blue, "Citing Patents" = orange
const COLOR_CITED: egui::Color32 = egui::Color32::from_rgb(31, 119, 180);
const COLOR_PATENT: egui::Color32 = egui::Color32::from_rgb(255, 127, 14);

// ---------------------------------------------------------------------------
// JSON data loading
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct JsonData {
    nodes: Vec<JsonNode>,
    links: Vec<JsonLink>,
}

#[derive(serde::Deserialize)]
struct JsonNode {
    id: String,
    group: String,
    #[serde(default = "default_radius")]
    radius: f32,
    #[serde(default)]
    #[allow(dead_code)]
    citing_patents_count: u32,
}

fn default_radius() -> f32 {
    1.0
}

#[derive(serde::Deserialize)]
struct JsonLink {
    source: String,
    target: String,
    #[serde(default = "default_value")]
    value: f32,
}

fn default_value() -> f32 {
    1.0
}

struct LoadedGraph {
    labels: Vec<String>,
    groups: Vec<String>,
    radii: Vec<f32>,
    links: Vec<(usize, usize, f32)>,
}

fn load_graph() -> LoadedGraph {
    let data: JsonData = serde_json::from_str(JSON_DATA).expect("Failed to parse JSON data");

    // Map string IDs → numeric indices
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    let mut labels = Vec::with_capacity(data.nodes.len());
    let mut groups = Vec::with_capacity(data.nodes.len());
    let mut radii = Vec::with_capacity(data.nodes.len());

    for (i, node) in data.nodes.iter().enumerate() {
        id_to_idx.insert(node.id.clone(), i);
        labels.push(node.id.clone());
        groups.push(node.group.clone());
        radii.push(node.radius);
    }

    let mut links = Vec::with_capacity(data.links.len());
    for link in &data.links {
        if let (Some(&si), Some(&ti)) = (id_to_idx.get(&link.source), id_to_idx.get(&link.target))
        {
            links.push((si, ti, link.value));
        }
    }

    LoadedGraph {
        labels,
        groups,
        radii,
        links,
    }
}

// ---------------------------------------------------------------------------
// Flow node/edge data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    group: String,
    radius: f32,
}

#[derive(Debug, Clone, Default)]
struct LinkData {
    value: f32,
}

// ---------------------------------------------------------------------------
// Force simulation — matches D3 semantics
// ---------------------------------------------------------------------------

struct SimNode {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    fx: Option<f32>,
    fy: Option<f32>,
}

struct SimLink {
    source: usize,
    target: usize,
    strength: f32,
    bias: f32,
}

struct ForceSimulation {
    nodes: Vec<SimNode>,
    links: Vec<SimLink>,
    alpha: f32,
    alpha_target: f32,
}

impl ForceSimulation {
    fn new(nodes: Vec<SimNode>, raw_links: Vec<(usize, usize)>) -> Self {
        let n = nodes.len();
        let mut degree = vec![0_usize; n];
        for &(s, t) in &raw_links {
            degree[s] += 1;
            degree[t] += 1;
        }

        let links: Vec<SimLink> = raw_links
            .iter()
            .map(|&(s, t)| {
                let ds = degree[s].max(1);
                let dt = degree[t].max(1);
                SimLink {
                    source: s,
                    target: t,
                    strength: 1.0 / (ds.min(dt) as f32),
                    bias: ds as f32 / (ds + dt) as f32,
                }
            })
            .collect();

        Self {
            nodes,
            links,
            alpha: 1.0,
            alpha_target: 0.0,
        }
    }

    fn tick(&mut self) {
        self.alpha += (self.alpha_target - self.alpha) * ALPHA_DECAY;
        if self.alpha < ALPHA_MIN {
            return;
        }

        let alpha = self.alpha;
        self.apply_charge(alpha);
        self.apply_links(alpha);
        self.apply_position(alpha);

        for node in &mut self.nodes {
            if let Some(fx) = node.fx {
                node.x = fx;
                node.vx = 0.0;
            } else {
                node.vx *= VELOCITY_DECAY;
                node.x += node.vx;
            }
            if let Some(fy) = node.fy {
                node.y = fy;
                node.vy = 0.0;
            } else {
                node.vy *= VELOCITY_DECAY;
                node.y += node.vy;
            }
        }
    }

    fn apply_charge(&mut self, alpha: f32) {
        let n = self.nodes.len();
        let pos: Vec<(f32, f32)> = self.nodes.iter().map(|n| (n.x, n.y)).collect();

        for i in 0..n {
            if self.nodes[i].fx.is_some() {
                continue;
            }
            for j in 0..n {
                if i == j {
                    continue;
                }
                let dx = pos[j].0 - pos[i].0;
                let dy = pos[j].1 - pos[i].1;
                let mut l = dx * dx + dy * dy;

                if l == 0.0 {
                    let jx = 1.0e-6 * ((i * 131 + j * 97) % 1000) as f32 - 0.5e-3;
                    let jy = 1.0e-6 * ((i * 97 + j * 131) % 1000) as f32 - 0.5e-3;
                    self.nodes[i].vx += jx * CHARGE_STRENGTH * alpha;
                    self.nodes[i].vy += jy * CHARGE_STRENGTH * alpha;
                    continue;
                }

                if l < CHARGE_DIST_MIN2 {
                    l = (CHARGE_DIST_MIN2 * l).sqrt();
                }

                self.nodes[i].vx += dx * CHARGE_STRENGTH * alpha / l;
                self.nodes[i].vy += dy * CHARGE_STRENGTH * alpha / l;
            }
        }
    }

    fn apply_links(&mut self, alpha: f32) {
        let state: Vec<(f32, f32, f32, f32)> = self
            .nodes
            .iter()
            .map(|n| (n.x, n.y, n.vx, n.vy))
            .collect();

        for link in &self.links {
            let (sx, sy, svx, svy) = state[link.source];
            let (tx, ty, tvx, tvy) = state[link.target];

            let mut dx = tx + tvx - sx - svx;
            let mut dy = ty + tvy - sy - svy;

            if dx == 0.0 {
                dx = 1.0e-6;
            }
            if dy == 0.0 {
                dy = 1.0e-6;
            }

            let l = (dx * dx + dy * dy).sqrt();
            let f = (l - LINK_DISTANCE) / l * alpha * link.strength;
            let fx = dx * f;
            let fy = dy * f;

            let b = link.bias;
            if self.nodes[link.target].fx.is_none() {
                self.nodes[link.target].vx -= fx * b;
                self.nodes[link.target].vy -= fy * b;
            }
            if self.nodes[link.source].fx.is_none() {
                self.nodes[link.source].vx += fx * (1.0 - b);
                self.nodes[link.source].vy += fy * (1.0 - b);
            }
        }
    }

    fn apply_position(&mut self, alpha: f32) {
        for node in &mut self.nodes {
            if node.fx.is_some() {
                continue;
            }
            node.vx += (0.0 - node.x) * POSITION_STRENGTH * alpha;
            node.vy += (0.0 - node.y) * POSITION_STRENGTH * alpha;
        }
    }

    fn reheat(&mut self) {
        self.alpha = 1.0;
    }
}

// ---------------------------------------------------------------------------
// Custom NodeWidget — circle sized by data radius, coloured by group
// ---------------------------------------------------------------------------

struct CircleNodeWidget;

impl NodeWidget<NodeData> for CircleNodeWidget {
    fn size(&self, node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        let d = (BASE_RADIUS + node.data.radius * RADIUS_SCALE) * 2.0;
        egui::vec2(d, d)
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<NodeData>,
        screen_rect: egui::Rect,
        _config: &FlowConfig,
        hovered: bool,
        _transform: &Transform,
    ) {
        let center = screen_rect.center();
        let r = BASE_RADIUS + node.data.radius * RADIUS_SCALE;
        let color = if node.data.group == "Citing Patents" {
            COLOR_PATENT
        } else {
            COLOR_CITED
        };

        // Selection glow
        if node.selected {
            painter.circle_filled(
                center,
                r + 4.0,
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 70),
            );
        }

        // Main circle
        painter.circle_filled(center, r, color);

        // White stroke (D3 style)
        painter.circle_stroke(center, r, egui::Stroke::new(1.5, egui::Color32::WHITE));

        // Hover tooltip — truncate long paper titles
        if hovered {
            let label = if node.data.label.len() > 80 {
                format!("{}...", &node.data.label[..77])
            } else {
                node.data.label.clone()
            };
            let font = egui::FontId::proportional(11.0);
            let galley =
                painter.layout_no_wrap(label, font, egui::Color32::from_rgb(50, 50, 50));
            let pad = egui::vec2(6.0, 3.0);
            let text_pos = egui::pos2(
                center.x - galley.size().x / 2.0,
                center.y - r - 8.0 - galley.size().y,
            );
            let bg = egui::Rect::from_min_size(text_pos - pad, galley.size() + pad * 2.0);
            painter.rect_filled(bg, 3.0, egui::Color32::from_rgb(245, 245, 245));
            painter.rect_stroke(
                bg,
                3.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)),
                egui::StrokeKind::Middle,
            );
            painter.galley(text_pos, galley, egui::Color32::from_rgb(50, 50, 50));
        }
    }
}

// ---------------------------------------------------------------------------
// Custom EdgeWidget — line with width = sqrt(value)
// ---------------------------------------------------------------------------

struct LinkEdgeWidget;

impl EdgeWidget<LinkData> for LinkEdgeWidget {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &Edge<LinkData>,
        pos: &EdgePosition,
        config: &FlowConfig,
        _time: f64,
        _transform: &Transform,
    ) {
        let value = edge.data.as_ref().map(|d| d.value).unwrap_or(1.0);
        let width = value.sqrt();
        let color = if edge.selected {
            config.edge_selected_color
        } else {
            egui::Color32::from_rgba_unmultiplied(153, 153, 153, 153) // #999 @ 60%
        };
        let stroke = egui::Stroke::new(width, color);
        let from = egui::pos2(pos.source_x, pos.source_y);
        let to = egui::pos2(pos.target_x, pos.target_y);
        painter.line_segment([from, to], stroke);
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct DisjointForceApp {
    state: FlowState<NodeData, LinkData>,
    simulation: ForceSimulation,
    first_frame: bool,
    total_nodes: usize,
    total_edges: usize,
    cited_count: usize,
    patent_count: usize,
}

impl DisjointForceApp {
    fn new() -> Self {
        let graph = load_graph();
        let n = graph.labels.len();
        let num_edges = graph.links.len();
        let cited_count = graph.groups.iter().filter(|g| *g == "Cited Works").count();
        let patent_count = n - cited_count;

        // Use a larger default node size for the bounding-box hit area
        let max_diameter = 2.0
            * (BASE_RADIUS
                + graph.radii.iter().cloned().fold(0.0_f32, f32::max) * RADIUS_SCALE);

        let config = FlowConfig {
            snap_to_grid: false,
            auto_pan_on_node_drag: false,
            auto_pan_on_connect: false,
            nodes_draggable: true,
            nodes_connectable: false,
            nodes_selectable: true,
            min_zoom: 0.1,
            max_zoom: 5.0,
            show_background: false,
            default_node_width: max_diameter,
            default_node_height: max_diameter,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            edge_stroke_width: 1.5,
            nodes_resizable: false,
            default_source_position: Position::Center,
            default_target_position: Position::Center,
            ..FlowConfig::default()
        };

        let mut state: FlowState<NodeData, LinkData> = FlowState::new(config);

        // D3 phyllotaxis spiral for initial positions
        let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
        let mut sim_nodes = Vec::with_capacity(n);

        for (i, ((label, group), &radius)) in graph
            .labels
            .iter()
            .zip(graph.groups.iter())
            .zip(graph.radii.iter())
            .enumerate()
        {
            let r = INITIAL_RADIUS * (0.5 + i as f32).sqrt();
            let angle = i as f32 * golden_angle;
            let x = r * angle.cos();
            let y = r * angle.sin();
            let d = (BASE_RADIUS + radius * RADIUS_SCALE) * 2.0;

            state.add_node(
                Node::builder(format!("n{}", i))
                    .position(egui::pos2(x, y))
                    .data(NodeData {
                        label: label.clone(),
                        group: group.clone(),
                        radius,
                    })
                    .size(d, d)
                    .build(),
            );

            sim_nodes.push(SimNode {
                x,
                y,
                vx: 0.0,
                vy: 0.0,
                fx: None,
                fy: None,
            });
        }

        // Edges
        let mut raw_links = Vec::with_capacity(num_edges);
        for (i, &(src, tgt, value)) in graph.links.iter().enumerate() {
            let mut edge = Edge::new(
                format!("e{}", i),
                format!("n{}", src),
                format!("n{}", tgt),
            )
            .edge_type(EdgeType::Straight);
            edge.data = Some(LinkData { value });
            state.add_edge(edge);
            raw_links.push((src, tgt));
        }

        let simulation = ForceSimulation::new(sim_nodes, raw_links);

        Self {
            state,
            simulation,
            first_frame: true,
            total_nodes: n,
            total_edges: num_edges,
            cited_count,
            patent_count,
        }
    }

    fn sync_drag_state(&mut self) {
        let mut any_dragging = false;

        for (i, flow_node) in self.state.nodes.iter().enumerate() {
            if i >= self.simulation.nodes.len() {
                break;
            }
            let sim = &mut self.simulation.nodes[i];

            if flow_node.dragging {
                any_dragging = true;
                sim.fx = Some(flow_node.position.x);
                sim.fy = Some(flow_node.position.y);
            } else {
                sim.fx = None;
                sim.fy = None;
            }
        }

        if any_dragging {
            self.simulation.alpha_target = 0.3;
            if self.simulation.alpha < self.simulation.alpha_target {
                self.simulation.alpha = self.simulation.alpha_target;
            }
        } else {
            self.simulation.alpha_target = 0.0;
        }
    }

    fn sync_positions_to_state(&mut self) {
        for (i, sim) in self.simulation.nodes.iter().enumerate() {
            if i >= self.state.nodes.len() {
                break;
            }
            if !self.state.nodes[i].dragging {
                self.state.nodes[i].position = egui::pos2(sim.x, sim.y);
            }
        }
        self.state.rebuild_lookup();
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for DisjointForceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Physics pipeline
        self.sync_drag_state();
        self.simulation.tick();
        self.sync_positions_to_state();

        // -- Top bar --
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Disjoint Force-Directed Graph");
                ui.separator();
                ui.label(format!(
                    "{} nodes, {} links",
                    self.total_nodes, self.total_edges
                ));
                ui.separator();
                if ui.button("Reheat").clicked() {
                    self.simulation.reheat();
                }
                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    self.state
                        .fit_view(rect, 60.0, ctx.input(|i| i.time));
                }
            });
        });

        // -- Side panel --
        egui::SidePanel::right("info")
            .resizable(true)
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Legend");
                ui.separator();

                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 6.0, COLOR_CITED);
                    ui.label(format!("Cited Works ({})", self.cited_count));
                });
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(rect.center(), 6.0, COLOR_PATENT);
                    ui.label(format!("Citing Patents ({})", self.patent_count));
                });

                ui.add_space(12.0);
                ui.separator();
                ui.heading("Simulation");
                ui.label(format!("Alpha: {:.4}", self.simulation.alpha));
                ui.label(format!("Target: {:.1}", self.simulation.alpha_target));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Drag nodes to interact").size(11.0));
                ui.label(egui::RichText::new("Scroll to zoom").size(11.0));
                ui.label(egui::RichText::new("Drag canvas to pan").size(11.0));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Circle size = citing patents count")
                        .size(10.0)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(
                        "Position forces (forceX/Y) keep\ndisjoint components in view.",
                    )
                    .size(10.0)
                    .weak(),
                );
            });

        // -- Canvas --
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                if self.first_frame {
                    let rect = ui.available_rect_before_wrap();
                    self.state
                        .fit_view(rect, 80.0, ctx.input(|i| i.time));
                    self.first_frame = false;
                }

                let _events = FlowCanvas::new(&mut self.state, &CircleNodeWidget)
                    .edge_widget(&LinkEdgeWidget)
                    .show(ui);
            });

        // Keep repainting while simulation is active or nodes are being dragged
        let any_dragging = self.state.nodes.iter().any(|n| n.dragging);
        if self.simulation.alpha > ALPHA_MIN || any_dragging {
            ctx.request_repaint();
        }
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- Disjoint Force-Directed Graph")
            .with_inner_size([1200.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Disjoint Force Graph",
        options,
        Box::new(|_cc| Ok(Box::new(DisjointForceApp::new()))),
    )
}
