//! Radial tidy tree example for egui_xyflow.
//!
//! Implements a D3-style radial tree layout: nodes are arranged in concentric
//! rings around a central root, with curved radial links connecting
//! parent-child pairs. Uses a software-framework hierarchy as sample data.
//!
//! Inspired by <https://observablehq.com/@d3/radial-tree/2>.
//!
//! Run with: `cargo run --example radial_tree`

use eframe::egui;
use egui_xyflow::prelude::*;
use egui_xyflow::EdgePosition;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NODE_SIZE: f32 = 8.0;
const CIRCLE_RADIUS: f32 = 3.0;
const RADIUS_STEP: f32 = 120.0; // flow-space distance per depth level
const LABEL_OFFSET: f32 = 8.0; // px from node center to label start

// ---------------------------------------------------------------------------
// Tree data builder
// ---------------------------------------------------------------------------

struct TreeBuilder {
    labels: Vec<String>,
    parents: Vec<Option<usize>>,
}

impl TreeBuilder {
    fn new(root: &str) -> Self {
        Self {
            labels: vec![root.to_string()],
            parents: vec![None],
        }
    }

    fn add(&mut self, parent: usize, label: &str) -> usize {
        let id = self.labels.len();
        self.labels.push(label.to_string());
        self.parents.push(Some(parent));
        id
    }

    fn add_many(&mut self, parent: usize, labels: &[&str]) -> Vec<usize> {
        labels.iter().map(|l| self.add(parent, l)).collect()
    }
}

/// Build a sample software-framework hierarchy (~90 nodes).
fn build_tree() -> TreeBuilder {
    let mut t = TreeBuilder::new("framework");

    let mods = t.add_many(0, &["core", "ui", "network", "data", "tools", "testing"]);

    // core
    let core_sub = t.add_many(mods[0], &["parser", "compiler", "runtime"]);
    t.add_many(core_sub[0], &["lexer", "ast", "validator", "scanner"]);
    t.add_many(core_sub[1], &["optimizer", "codegen", "linker", "ir"]);
    t.add_many(core_sub[2], &["gc", "jit", "debugger", "profiler"]);

    // ui
    let ui_sub = t.add_many(mods[1], &["widgets", "layout", "theme", "animation"]);
    t.add_many(ui_sub[0], &["button", "input", "slider", "dropdown", "checkbox"]);
    t.add_many(ui_sub[1], &["flex", "grid", "stack"]);
    t.add_many(ui_sub[2], &["colors", "fonts", "icons"]);
    t.add_many(ui_sub[3], &["tween", "spring", "keyframe"]);

    // network
    let net_sub = t.add_many(mods[2], &["http", "websocket", "rpc"]);
    t.add_many(net_sub[0], &["client", "server", "middleware", "router"]);
    t.add_many(net_sub[1], &["connection", "protocol", "frame"]);
    t.add_many(net_sub[2], &["codec", "transport", "registry"]);

    // data
    let data_sub = t.add_many(mods[3], &["storage", "query", "migration"]);
    t.add_many(data_sub[0], &["sql", "nosql", "cache", "blob"]);
    t.add_many(data_sub[1], &["builder", "executor", "planner"]);
    t.add_many(data_sub[2], &["schema", "seed", "rollback"]);

    // tools
    let tools_sub = t.add_many(mods[4], &["cli", "logger", "config"]);
    t.add_many(tools_sub[0], &["args", "output", "prompt"]);
    t.add_many(tools_sub[1], &["format", "rotate", "filter"]);
    t.add_many(tools_sub[2], &["loader", "validator", "merge"]);

    // testing
    let test_sub = t.add_many(mods[5], &["unit", "integration", "benchmark"]);
    t.add_many(test_sub[0], &["assert", "mock", "fixture"]);
    t.add_many(test_sub[1], &["runner", "report", "coverage"]);
    t.add_many(test_sub[2], &["timer", "stats", "compare"]);

    t
}

// ---------------------------------------------------------------------------
// Radial tree layout
// ---------------------------------------------------------------------------

/// Compute Cartesian positions for a radial tree centred at origin.
///
/// Returns `(positions, children, is_leaf, depths)` for each node index.
fn radial_layout(
    parents: &[Option<usize>],
) -> (Vec<egui::Pos2>, Vec<Vec<usize>>, Vec<bool>, Vec<usize>) {
    let n = parents.len();

    // Build children lists and depths
    let mut children = vec![Vec::<usize>::new(); n];
    let mut depths = vec![0_usize; n];
    for i in 1..n {
        if let Some(p) = parents[i] {
            children[p].push(i);
            depths[i] = depths[p] + 1;
        }
    }

    // Count leaves in each subtree
    fn count_leaves(node: usize, children: &[Vec<usize>]) -> usize {
        if children[node].is_empty() {
            1
        } else {
            children[node]
                .iter()
                .map(|&c| count_leaves(c, children))
                .sum()
        }
    }
    let leaf_counts: Vec<usize> = (0..n).map(|i| count_leaves(i, &children)).collect();

    // Assign angles: each subtree gets angular range proportional to leaf count
    let mut angles = vec![0.0_f32; n];
    fn assign_angles(
        node: usize,
        start: f32,
        end: f32,
        children: &[Vec<usize>],
        leaf_counts: &[usize],
        angles: &mut [f32],
    ) {
        angles[node] = (start + end) / 2.0;
        let total = leaf_counts[node] as f32;
        if total <= 0.0 {
            return;
        }
        let mut cur = start;
        for &child in &children[node] {
            let share = leaf_counts[child].max(1) as f32 / total;
            let span = (end - start) * share;
            assign_angles(child, cur, cur + span, children, leaf_counts, angles);
            cur += span;
        }
    }
    assign_angles(
        0,
        0.0,
        std::f32::consts::TAU,
        &children,
        &leaf_counts,
        &mut angles,
    );

    // Convert polar (angle, depth) → Cartesian
    let is_leaf: Vec<bool> = children.iter().map(|c| c.is_empty()).collect();
    let positions: Vec<egui::Pos2> = (0..n)
        .map(|i| {
            let r = depths[i] as f32 * RADIUS_STEP;
            let theta = angles[i];
            egui::pos2(r * theta.cos(), r * theta.sin())
        })
        .collect();

    (positions, children, is_leaf, depths)
}

// ---------------------------------------------------------------------------
// Node data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    is_leaf: bool,
}

// ---------------------------------------------------------------------------
// Custom NodeWidget — small circle + label
// ---------------------------------------------------------------------------

struct RadialNodeWidget;

impl NodeWidget<NodeData> for RadialNodeWidget {
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

        // Node circle
        let fill = if node.data.is_leaf {
            egui::Color32::from_rgb(153, 153, 153) // #999
        } else {
            egui::Color32::from_rgb(85, 85, 85) // #555
        };

        if node.selected || hovered {
            painter.circle_filled(
                center,
                CIRCLE_RADIUS + 3.0,
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 60),
            );
        }
        painter.circle_filled(center, CIRCLE_RADIUS, fill);

        // Label
        if node.data.label.is_empty() {
            return;
        }

        // Determine text alignment from flow-space angle
        let angle = node.position.y.atan2(node.position.x);
        let is_right = angle.abs() < std::f32::consts::FRAC_PI_2;
        let is_root = node.position.x.abs() < 0.01 && node.position.y.abs() < 0.01;

        let font = egui::FontId::proportional(10.0);
        let galley = painter.layout_no_wrap(
            node.data.label.clone(),
            font,
            config.node_text_color,
        );

        let text_pos = if is_root {
            // Center label below root
            egui::pos2(
                center.x - galley.size().x / 2.0,
                center.y + LABEL_OFFSET,
            )
        } else if is_right {
            egui::pos2(center.x + LABEL_OFFSET, center.y - galley.size().y / 2.0)
        } else {
            egui::pos2(
                center.x - LABEL_OFFSET - galley.size().x,
                center.y - galley.size().y / 2.0,
            )
        };

        // White halo behind text for readability
        let halo_rect = egui::Rect::from_min_size(
            text_pos - egui::vec2(1.0, 0.0),
            galley.size() + egui::vec2(2.0, 0.0),
        );
        painter.rect_filled(halo_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 210));

        painter.galley(text_pos, galley, config.node_text_color);
    }
}

// ---------------------------------------------------------------------------
// Custom EdgeWidget — radial Bézier curves
// ---------------------------------------------------------------------------

struct RadialEdgeWidget;

impl EdgeWidget<()> for RadialEdgeWidget {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &Edge<()>,
        pos: &EdgePosition,
        config: &FlowConfig,
        _time: f64,
        transform: &Transform,
    ) {
        // Flow origin (0,0) maps to screen (transform.x, transform.y)
        let center = egui::pos2(transform.x, transform.y);
        let source = egui::pos2(pos.source_x, pos.source_y);
        let target = egui::pos2(pos.target_x, pos.target_y);

        // Compute radial angles and radii in screen space
        let r_s = source.distance(center);
        let r_t = target.distance(center);
        let angle_s = (source.y - center.y).atan2(source.x - center.x);
        let angle_t = (target.y - center.y).atan2(target.x - center.x);
        let mid_r = (r_s + r_t) / 2.0;

        // Control points at midpoint radius, at source/target angles
        let cp1 = egui::pos2(
            center.x + mid_r * angle_s.cos(),
            center.y + mid_r * angle_s.sin(),
        );
        let cp2 = egui::pos2(
            center.x + mid_r * angle_t.cos(),
            center.y + mid_r * angle_t.sin(),
        );

        let color = if edge.selected {
            config.edge_selected_color
        } else {
            // #555 at 40 % opacity
            egui::Color32::from_rgba_unmultiplied(85, 85, 85, 102)
        };
        let stroke = egui::Stroke::new(config.edge_stroke_width, color);

        let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
            [source, cp1, cp2, target],
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

struct RadialTreeApp {
    state: FlowState<NodeData, ()>,
    edge_widget: RadialEdgeWidget,
    first_frame: bool,
    node_count: usize,
    leaf_count: usize,
    depth_max: usize,
}

impl RadialTreeApp {
    fn new() -> Self {
        let tree = build_tree();
        let (positions, children, is_leaf, depths) = radial_layout(&tree.parents);
        let n = tree.labels.len();
        let leaf_count = is_leaf.iter().filter(|&&l| l).count();
        let depth_max = *depths.iter().max().unwrap_or(&0);

        let config = FlowConfig {
            snap_to_grid: false,
            show_background: false,
            nodes_draggable: false,
            nodes_connectable: false,
            nodes_selectable: true,
            min_zoom: 0.1,
            max_zoom: 5.0,
            default_node_width: NODE_SIZE,
            default_node_height: NODE_SIZE,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            edge_stroke_width: 1.5,
            nodes_resizable: false,
            default_source_position: Position::Center,
            default_target_position: Position::Center,
            ..FlowConfig::default()
        };

        let mut state = FlowState::new(config);

        // Add nodes
        for i in 0..n {
            state.add_node(
                Node::builder(format!("n{}", i))
                    .position(positions[i])
                    .data(NodeData {
                        label: tree.labels[i].clone(),
                        is_leaf: is_leaf[i],
                    })
                    .size(NODE_SIZE, NODE_SIZE)
                    .build(),
            );
        }

        // Add edges (parent → child)
        let mut eid = 0;
        for (i, kids) in children.iter().enumerate() {
            for &child in kids {
                state.add_edge(
                    Edge::new(format!("e{}", eid), format!("n{}", i), format!("n{}", child))
                        .edge_type(EdgeType::Straight), // type ignored; custom widget draws
                );
                eid += 1;
            }
        }

        Self {
            state,
            edge_widget: RadialEdgeWidget,
            first_frame: true,
            node_count: n,
            leaf_count,
            depth_max,
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for RadialTreeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top bar
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Radial Tidy Tree");
                ui.separator();
                ui.label(format!(
                    "{} nodes, {} leaves, depth {}",
                    self.node_count, self.leaf_count, self.depth_max
                ));
                ui.separator();
                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    self.state
                        .fit_view(rect, 60.0, ctx.input(|i| i.time));
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }
            });
        });

        // Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                // Fit view on first frame
                if self.first_frame {
                    let rect = ui.available_rect_before_wrap();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 80.0, t);
                    self.first_frame = false;
                }

                let _events = FlowCanvas::new(&mut self.state, &RadialNodeWidget)
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
            .with_title("egui_xyflow -- Radial Tidy Tree")
            .with_inner_size([1100.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Radial Tree",
        options,
        Box::new(|_cc| Ok(Box::new(RadialTreeApp::new()))),
    )
}
