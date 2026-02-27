//! Arc diagram example for egui_xyflow.
//!
//! Implements a D3-style arc diagram: nodes are arranged along a horizontal
//! line, and edges are drawn as semicircular arcs above the line.  Arc height
//! is proportional to the distance between connected nodes.  Nodes are coloured
//! by group and ordered so that members of the same group are adjacent.
//!
//! Inspired by <https://observablehq.com/@d3/arc-diagram>.
//!
//! Run with: `cargo run --example arc_diagram`

use std::cell::Cell;
use std::collections::HashSet;


use eframe::egui;
use egui_xyflow::prelude::*;
use egui_xyflow::EdgePosition;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NODE_SIZE: f32 = 10.0;
const NODE_RADIUS: f32 = 4.0;
const NODE_SPACING: f32 = 28.0;
const LINE_Y: f32 = 0.0;
const LABEL_FONT_SIZE: f32 = 9.0;
const ARC_STROKE_WIDTH: f32 = 1.2;
const ARC_ALPHA: u8 = 100;
const HIGHLIGHT_ALPHA: u8 = 200;
const DIM_ALPHA: u8 = 20;
const BEZIER_K: f32 = 0.552; // cubic Bezier approximation of semicircle

// ---------------------------------------------------------------------------
// Colour palette (Tableau 10)
// ---------------------------------------------------------------------------

const PALETTE: [(u8, u8, u8); 10] = [
    (78, 121, 167),  // blue
    (242, 142, 43),  // orange
    (225, 87, 89),   // red
    (118, 183, 178), // teal
    (89, 161, 79),   // green
    (176, 122, 161), // purple
    (237, 201, 72),  // yellow
    (156, 117, 95),  // brown
    (255, 157, 167), // pink
    (186, 176, 172), // grey
];

fn group_color(group: usize) -> egui::Color32 {
    let (r, g, b) = PALETTE[group % PALETTE.len()];
    egui::Color32::from_rgb(r, g, b)
}

// ---------------------------------------------------------------------------
// Character / link data (Les Miserables network)
// ---------------------------------------------------------------------------

struct CharDef {
    name: &'static str,
    group: usize,
}

struct LinkDef {
    source: usize,
    target: usize,
}

fn build_data() -> (Vec<CharDef>, Vec<LinkDef>) {
    let chars = vec![
        // Group 0: Valjean circle
        CharDef { name: "Valjean", group: 0 },
        CharDef { name: "Javert", group: 0 },
        CharDef { name: "Fauchelevent", group: 0 },
        CharDef { name: "Bamatabois", group: 0 },
        CharDef { name: "Champmathieu", group: 0 },
        // Group 1: Myriel
        CharDef { name: "Myriel", group: 1 },
        CharDef { name: "Napoleon", group: 1 },
        CharDef { name: "MlleBaptistine", group: 1 },
        CharDef { name: "MmeMagloire", group: 1 },
        CharDef { name: "CountessDeLo", group: 1 },
        CharDef { name: "Geborand", group: 1 },
        CharDef { name: "Champtercier", group: 1 },
        CharDef { name: "Cravatte", group: 1 },
        CharDef { name: "OldMan", group: 1 },
        // Group 2: Fantine
        CharDef { name: "Fantine", group: 2 },
        CharDef { name: "Favourite", group: 2 },
        CharDef { name: "Dahlia", group: 2 },
        CharDef { name: "Zephine", group: 2 },
        CharDef { name: "Marguerite", group: 2 },
        CharDef { name: "Simplice", group: 2 },
        CharDef { name: "Perpetue", group: 2 },
        // Group 3: Thenardier
        CharDef { name: "Thenardier", group: 3 },
        CharDef { name: "MmeThenardier", group: 3 },
        CharDef { name: "Babet", group: 3 },
        CharDef { name: "Gueulemer", group: 3 },
        CharDef { name: "Claquesous", group: 3 },
        CharDef { name: "Montparnasse", group: 3 },
        CharDef { name: "Brujon", group: 3 },
        // Group 4: Cosette
        CharDef { name: "Cosette", group: 4 },
        CharDef { name: "Toussaint", group: 4 },
        CharDef { name: "Pontmercy", group: 4 },
        // Group 5: Marius
        CharDef { name: "Marius", group: 5 },
        CharDef { name: "Gillenormand", group: 5 },
        CharDef { name: "MlleGillenormand", group: 5 },
        CharDef { name: "MmeDeR", group: 5 },
        CharDef { name: "BaronessT", group: 5 },
        CharDef { name: "LtGillenormand", group: 5 },
        CharDef { name: "Mabeuf", group: 5 },
        // Group 6: Enjolras / ABC society
        CharDef { name: "Enjolras", group: 6 },
        CharDef { name: "Combeferre", group: 6 },
        CharDef { name: "Prouvaire", group: 6 },
        CharDef { name: "Feuilly", group: 6 },
        CharDef { name: "Courfeyrac", group: 6 },
        CharDef { name: "Bahorel", group: 6 },
        CharDef { name: "Bossuet", group: 6 },
        CharDef { name: "Joly", group: 6 },
        CharDef { name: "Grantaire", group: 6 },
        // Group 7: Gavroche
        CharDef { name: "Gavroche", group: 7 },
        CharDef { name: "Jondrette", group: 7 },
        CharDef { name: "MmeHucheloup", group: 7 },
        // Group 8: Secondary
        CharDef { name: "Labarre", group: 8 },
        CharDef { name: "MotherPlutarch", group: 8 },
        CharDef { name: "Anzelma", group: 8 },
        CharDef { name: "Eponine", group: 8 },
        CharDef { name: "Woman1", group: 8 },
        // Group 9: Misc
        CharDef { name: "Judge", group: 9 },
        CharDef { name: "Brevet", group: 9 },
        CharDef { name: "Chenildieu", group: 9 },
        CharDef { name: "Cochepaille", group: 9 },
        CharDef { name: "Isabeau", group: 9 },
        CharDef { name: "Scaufflaire", group: 9 },
        CharDef { name: "Gervais", group: 9 },
        CharDef { name: "Boulatruelle", group: 9 },
        CharDef { name: "MotherInnocent", group: 9 },
        CharDef { name: "Child1", group: 9 },
        CharDef { name: "Child2", group: 9 },
    ];

    let links = vec![
        // Myriel internal
        LinkDef { source: 5, target: 6 },
        LinkDef { source: 5, target: 7 },
        LinkDef { source: 5, target: 8 },
        LinkDef { source: 5, target: 9 },
        LinkDef { source: 5, target: 10 },
        LinkDef { source: 5, target: 11 },
        LinkDef { source: 5, target: 12 },
        LinkDef { source: 5, target: 13 },
        LinkDef { source: 7, target: 8 },
        // Myriel <-> Valjean
        LinkDef { source: 0, target: 5 },
        // Valjean circle internal
        LinkDef { source: 0, target: 1 },
        LinkDef { source: 0, target: 2 },
        LinkDef { source: 0, target: 3 },
        LinkDef { source: 0, target: 4 },
        // Fantine internal
        LinkDef { source: 14, target: 15 },
        LinkDef { source: 14, target: 16 },
        LinkDef { source: 14, target: 17 },
        LinkDef { source: 14, target: 18 },
        LinkDef { source: 14, target: 19 },
        LinkDef { source: 14, target: 20 },
        LinkDef { source: 15, target: 16 },
        LinkDef { source: 15, target: 17 },
        LinkDef { source: 15, target: 18 },
        LinkDef { source: 16, target: 17 },
        LinkDef { source: 16, target: 18 },
        LinkDef { source: 17, target: 18 },
        // Fantine <-> Valjean
        LinkDef { source: 0, target: 14 },
        LinkDef { source: 1, target: 14 },
        LinkDef { source: 3, target: 14 },
        LinkDef { source: 19, target: 0 },
        LinkDef { source: 19, target: 1 },
        // Thenardier internal
        LinkDef { source: 21, target: 22 },
        LinkDef { source: 21, target: 23 },
        LinkDef { source: 21, target: 24 },
        LinkDef { source: 21, target: 25 },
        LinkDef { source: 21, target: 26 },
        LinkDef { source: 21, target: 27 },
        LinkDef { source: 23, target: 24 },
        LinkDef { source: 23, target: 25 },
        LinkDef { source: 23, target: 26 },
        LinkDef { source: 23, target: 27 },
        LinkDef { source: 24, target: 25 },
        LinkDef { source: 24, target: 26 },
        LinkDef { source: 24, target: 27 },
        LinkDef { source: 25, target: 26 },
        LinkDef { source: 25, target: 27 },
        LinkDef { source: 26, target: 27 },
        // Thenardier <-> Valjean
        LinkDef { source: 0, target: 21 },
        LinkDef { source: 0, target: 22 },
        LinkDef { source: 1, target: 21 },
        LinkDef { source: 1, target: 22 },
        LinkDef { source: 14, target: 21 },
        LinkDef { source: 14, target: 22 },
        // Cosette
        LinkDef { source: 0, target: 28 },
        LinkDef { source: 21, target: 28 },
        LinkDef { source: 22, target: 28 },
        LinkDef { source: 1, target: 28 },
        LinkDef { source: 14, target: 28 },
        LinkDef { source: 28, target: 29 },
        LinkDef { source: 0, target: 29 },
        LinkDef { source: 0, target: 30 },
        LinkDef { source: 28, target: 30 },
        // Marius
        LinkDef { source: 0, target: 31 },
        LinkDef { source: 28, target: 31 },
        LinkDef { source: 21, target: 31 },
        LinkDef { source: 30, target: 31 },
        LinkDef { source: 31, target: 32 },
        LinkDef { source: 31, target: 33 },
        LinkDef { source: 32, target: 33 },
        LinkDef { source: 33, target: 34 },
        LinkDef { source: 33, target: 35 },
        LinkDef { source: 31, target: 36 },
        LinkDef { source: 32, target: 36 },
        LinkDef { source: 0, target: 37 },
        LinkDef { source: 31, target: 37 },
        LinkDef { source: 47, target: 37 },
        // Enjolras / ABC
        LinkDef { source: 0, target: 38 },
        LinkDef { source: 31, target: 38 },
        LinkDef { source: 38, target: 39 },
        LinkDef { source: 38, target: 40 },
        LinkDef { source: 38, target: 41 },
        LinkDef { source: 38, target: 42 },
        LinkDef { source: 38, target: 43 },
        LinkDef { source: 38, target: 44 },
        LinkDef { source: 38, target: 45 },
        LinkDef { source: 38, target: 46 },
        LinkDef { source: 39, target: 42 },
        LinkDef { source: 39, target: 44 },
        LinkDef { source: 39, target: 45 },
        LinkDef { source: 41, target: 43 },
        LinkDef { source: 42, target: 43 },
        LinkDef { source: 42, target: 44 },
        LinkDef { source: 42, target: 45 },
        LinkDef { source: 42, target: 31 },
        LinkDef { source: 43, target: 44 },
        LinkDef { source: 43, target: 45 },
        LinkDef { source: 44, target: 45 },
        LinkDef { source: 44, target: 46 },
        LinkDef { source: 45, target: 46 },
        LinkDef { source: 1, target: 38 },
        LinkDef { source: 47, target: 38 },
        // Gavroche
        LinkDef { source: 0, target: 47 },
        LinkDef { source: 1, target: 47 },
        LinkDef { source: 21, target: 47 },
        LinkDef { source: 22, target: 47 },
        LinkDef { source: 42, target: 47 },
        LinkDef { source: 38, target: 47 },
        LinkDef { source: 47, target: 48 },
        LinkDef { source: 47, target: 49 },
        LinkDef { source: 44, target: 49 },
        LinkDef { source: 45, target: 49 },
        LinkDef { source: 46, target: 49 },
        LinkDef { source: 47, target: 64 },
        LinkDef { source: 47, target: 65 },
        LinkDef { source: 64, target: 65 },
        // Labarre / secondary
        LinkDef { source: 0, target: 50 },
        LinkDef { source: 37, target: 51 },
        LinkDef { source: 21, target: 52 },
        LinkDef { source: 22, target: 52 },
        LinkDef { source: 0, target: 53 },
        LinkDef { source: 21, target: 53 },
        LinkDef { source: 31, target: 53 },
        LinkDef { source: 28, target: 53 },
        LinkDef { source: 14, target: 54 },
        // Misc
        LinkDef { source: 0, target: 55 },
        LinkDef { source: 0, target: 56 },
        LinkDef { source: 0, target: 57 },
        LinkDef { source: 0, target: 58 },
        LinkDef { source: 4, target: 55 },
        LinkDef { source: 4, target: 56 },
        LinkDef { source: 4, target: 57 },
        LinkDef { source: 4, target: 58 },
        LinkDef { source: 56, target: 57 },
        LinkDef { source: 56, target: 58 },
        LinkDef { source: 57, target: 58 },
        LinkDef { source: 55, target: 56 },
        LinkDef { source: 55, target: 57 },
        LinkDef { source: 55, target: 58 },
        LinkDef { source: 3, target: 55 },
        LinkDef { source: 0, target: 59 },
        LinkDef { source: 0, target: 60 },
        LinkDef { source: 0, target: 61 },
        LinkDef { source: 0, target: 62 },
        LinkDef { source: 0, target: 63 },
    ];

    (chars, links)
}

// ---------------------------------------------------------------------------
// Layout: sort nodes by group then alphabetically, space along x
// ---------------------------------------------------------------------------

/// Returns a mapping from original char index to sorted position index.
fn compute_order(chars: &[CharDef]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..chars.len()).collect();
    indices.sort_by(|&a, &b| {
        chars[a]
            .group
            .cmp(&chars[b].group)
            .then_with(|| chars[a].name.cmp(&chars[b].name))
    });

    // order_of[original_index] = position in the sorted line
    let mut order_of = vec![0_usize; chars.len()];
    for (pos, &orig) in indices.iter().enumerate() {
        order_of[orig] = pos;
    }
    order_of
}

// ---------------------------------------------------------------------------
// FlowState data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    color: egui::Color32,
}

#[derive(Debug, Clone, Default)]
struct ArcData {
    /// Source center x in flow space
    source_x: f32,
    /// Target center x in flow space
    target_x: f32,
    /// Center y of the horizontal line (flow space)
    line_y: f32,
    /// Arc colour (source group)
    color: egui::Color32,
    /// Source original index (for hover highlighting)
    source_idx: usize,
    /// Target original index (for hover highlighting)
    target_idx: usize,
}

// ---------------------------------------------------------------------------
// Custom NodeWidget: small filled circle + horizontal label below
// ---------------------------------------------------------------------------

struct ArcNodeWidget;

impl NodeWidget<NodeData> for ArcNodeWidget {
    fn size(&self, _node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(NODE_SIZE, NODE_SIZE)
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
        let center = screen_rect.center();

        // Highlight ring when selected or hovered
        if node.selected || hovered {
            painter.circle_filled(
                center,
                NODE_RADIUS + 3.0,
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 80),
            );
        }

        // Filled circle
        painter.circle_filled(center, NODE_RADIUS, node.data.color);

        // Label below the node
        if !node.data.label.is_empty() {
            let font = egui::FontId::proportional(LABEL_FONT_SIZE);
            let galley =
                painter.layout_no_wrap(node.data.label.clone(), font, config.node_text_color);

            let text_pos = egui::pos2(
                center.x - galley.size().x / 2.0,
                center.y + NODE_RADIUS + 4.0,
            );

            // White halo for readability
            let halo = egui::Rect::from_min_size(
                text_pos - egui::vec2(1.0, 0.0),
                galley.size() + egui::vec2(2.0, 0.0),
            );
            painter.rect_filled(
                halo,
                0.0,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220),
            );

            painter.galley(text_pos, galley, config.node_text_color);
        }
    }
}

// ---------------------------------------------------------------------------
// Custom EdgeWidget: semicircular arcs above the line with hover highlighting
// ---------------------------------------------------------------------------

struct ArcEdgeWidget {
    /// Currently hovered node index (original char index), if any.
    hovered_node: Cell<Option<usize>>,
}

impl ArcEdgeWidget {
    fn new() -> Self {
        Self {
            hovered_node: Cell::new(None),
        }
    }
}

impl EdgeWidget<ArcData> for ArcEdgeWidget {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &Edge<ArcData>,
        _pos: &EdgePosition,
        config: &FlowConfig,
        _time: f64,
        transform: &Transform,
    ) {
        let d = match edge.data.as_ref() {
            Some(d) => d,
            None => return,
        };

        // Determine left and right endpoints so arc always goes upward
        let left_x = d.source_x.min(d.target_x);
        let right_x = d.source_x.max(d.target_x);
        let span = right_x - left_x;
        if span < 0.01 {
            return;
        }

        let s = egui::pos2(left_x * transform.scale + transform.x, d.line_y * transform.scale + transform.y);
        let t = egui::pos2(right_x * transform.scale + transform.x, d.line_y * transform.scale + transform.y);

        let h = span * transform.scale / 2.0;

        // Hover-based highlighting
        let hovered = self.hovered_node.get();
        let is_connected = match hovered {
            Some(idx) => d.source_idx == idx || d.target_idx == idx,
            None => false,
        };

        let (color, width) = if edge.selected {
            (config.edge_selected_color, 2.5)
        } else if hovered.is_some() {
            if is_connected {
                let c = d.color;
                (
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), HIGHLIGHT_ALPHA),
                    2.0,
                )
            } else {
                let c = d.color;
                (
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), DIM_ALPHA),
                    ARC_STROKE_WIDTH,
                )
            }
        } else {
            (d.color, ARC_STROKE_WIDTH)
        };

        let stroke = egui::Stroke::new(width, color);

        // Cubic Bezier approximation of a semicircular arc above the line.
        let p0 = s;
        let p1 = egui::pos2(s.x, s.y - h * BEZIER_K);
        let p2 = egui::pos2(t.x, t.y - h * BEZIER_K);
        let p3 = t;

        let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
            [p0, p1, p2, p3],
            false,
            egui::Color32::TRANSPARENT,
            stroke,
        );
        painter.add(bezier);
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct ArcDiagramApp {
    state: FlowState<NodeData, ArcData>,
    edge_widget: ArcEdgeWidget,
    first_frame: bool,
    node_count: usize,
    edge_count: usize,
    group_count: usize,
    /// For each original char index, the set of connected original indices.
    #[allow(dead_code)]
    connections: Vec<HashSet<usize>>,
}

impl ArcDiagramApp {
    fn new() -> Self {
        let (chars, links) = build_data();
        let order = compute_order(&chars);
        let n = chars.len();
        let group_count = chars.iter().map(|c| c.group).max().unwrap_or(0) + 1;

        // Build connection sets for hover highlighting
        let mut connections: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        for link in &links {
            connections[link.source].insert(link.target);
            connections[link.target].insert(link.source);
        }

        // Compute flow-space center positions for each node
        let half = NODE_SIZE / 2.0;
        let center_y = LINE_Y + half;
        let node_centers: Vec<egui::Pos2> = (0..n)
            .map(|orig| {
                let cx = order[orig] as f32 * NODE_SPACING + half;
                egui::pos2(cx, center_y)
            })
            .collect();

        let config = FlowConfig {
            nodes_draggable: false,
            nodes_connectable: false,
            nodes_selectable: true,
            nodes_resizable: false,
            show_background: false,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            default_source_position: Position::Center,
            default_target_position: Position::Center,
            default_node_width: NODE_SIZE,
            default_node_height: NODE_SIZE,
            edge_stroke_width: ARC_STROKE_WIDTH,
            min_zoom: 0.1,
            max_zoom: 5.0,
            snap_to_grid: false,
            ..FlowConfig::default()
        };

        let mut state = FlowState::new(config);

        // Add nodes — position is top-left, offset so visual center is on the line
        for (orig_idx, ch) in chars.iter().enumerate() {
            let cx = node_centers[orig_idx].x;
            let cy = node_centers[orig_idx].y;
            state.add_node(
                Node::builder(format!("n{}", orig_idx))
                    .position(egui::pos2(cx - half, cy - half))
                    .data(NodeData {
                        label: ch.name.to_string(),
                        color: group_color(ch.group),
                    })
                    .size(NODE_SIZE, NODE_SIZE)
                    .build(),
            );
        }

        // Add edges — arc endpoints at node centers
        let edge_count = links.len();
        for (i, link) in links.iter().enumerate() {
            let sx = node_centers[link.source].x;
            let tx = node_centers[link.target].x;

            let mut edge = Edge::new(
                format!("e{}", i),
                format!("n{}", link.source),
                format!("n{}", link.target),
            )
            .edge_type(EdgeType::Straight);
            edge.data = Some(ArcData {
                source_x: sx,
                target_x: tx,
                line_y: center_y,
                color: {
                    let (r, g, b) = PALETTE[chars[link.source].group % PALETTE.len()];
                    egui::Color32::from_rgba_unmultiplied(r, g, b, ARC_ALPHA)
                },
                source_idx: link.source,
                target_idx: link.target,
            });
            state.add_edge(edge);
        }

        Self {
            state,
            edge_widget: ArcEdgeWidget::new(),
            first_frame: true,
            node_count: n,
            edge_count,
            group_count,
            connections,
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for ArcDiagramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Arc Diagram — Les Miserables");
                ui.separator();
                ui.label(format!(
                    "{} characters, {} connections, {} groups",
                    self.node_count, self.edge_count, self.group_count
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

                let events = FlowCanvas::new(&mut self.state, &ArcNodeWidget)
                    .edge_widget(&self.edge_widget)
                    .show(ui);

                // Map hovered NodeId to original char index for edge highlighting
                let hovered_idx = events.node_hovered.as_ref().and_then(|nid| {
                    nid.0.strip_prefix('n').and_then(|s| s.parse::<usize>().ok())
                });
                self.edge_widget.hovered_node.set(hovered_idx);

                if hovered_idx.is_some() {
                    ctx.request_repaint();
                }
            });
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow — Arc Diagram")
            .with_inner_size([1200.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Arc Diagram",
        options,
        Box::new(|_cc| Ok(Box::new(ArcDiagramApp::new()))),
    )
}
