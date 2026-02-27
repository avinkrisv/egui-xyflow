//! D3-style collapsible/expandable tree visualization.
//!
//! An interactive tree where clicking a non-leaf node toggles its children
//! visible/hidden.  Nodes are laid out horizontally (root on left, leaves on
//! right) with smooth Bezier edges.
//!
//! This example demonstrates dynamic node/edge add/remove, a core capability
//! of egui_xyflow.
//!
//! Reference: <https://observablehq.com/@d3/collapsible-tree>

use eframe::egui;
use egui_xyflow::prelude::*;

// ---------------------------------------------------------------------------
// Node data attached to every flow node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    is_leaf: bool,
    is_expanded: bool,
}

// ---------------------------------------------------------------------------
// Tree data stored *outside* FlowState
// ---------------------------------------------------------------------------

struct TreeNode {
    label: String,
    children: Vec<usize>,
    parent: Option<usize>,
    expanded: bool,
    depth: usize,
}

// ---------------------------------------------------------------------------
// Custom NodeWidget — circles with labels
// ---------------------------------------------------------------------------

struct TreeNodeWidget;

const NODE_RADIUS: f32 = 6.0;
const NODE_SIZE: f32 = NODE_RADIUS * 2.0;

impl NodeWidget<NodeData> for TreeNodeWidget {
    fn size(&self, node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        let _ = node;
        egui::vec2(NODE_SIZE, NODE_SIZE)
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
        let d = &node.data;

        // Branch nodes are dark steel-blue; leaves are lighter.
        let fill = if d.is_leaf {
            egui::Color32::from_rgb(200, 220, 240)
        } else if hovered {
            egui::Color32::from_rgb(50, 100, 160)
        } else {
            egui::Color32::from_rgb(70, 130, 180)
        };

        let stroke_color = if node.selected {
            egui::Color32::from_rgb(30, 80, 200)
        } else {
            egui::Color32::from_rgb(55, 105, 150)
        };

        // Draw filled circle
        painter.circle_filled(center, NODE_RADIUS, fill);
        painter.circle_stroke(
            center,
            NODE_RADIUS,
            egui::Stroke::new(1.2, stroke_color),
        );

        // For branch nodes, draw a small +/- indicator inside the circle
        if !d.is_leaf {
            let indicator_color = egui::Color32::WHITE;
            let half = 3.0_f32;
            // Horizontal bar (always present = minus)
            painter.line_segment(
                [
                    egui::pos2(center.x - half, center.y),
                    egui::pos2(center.x + half, center.y),
                ],
                egui::Stroke::new(1.4, indicator_color),
            );
            // Vertical bar (only when collapsed = plus)
            if !d.is_expanded {
                painter.line_segment(
                    [
                        egui::pos2(center.x, center.y - half),
                        egui::pos2(center.x, center.y + half),
                    ],
                    egui::Stroke::new(1.4, indicator_color),
                );
            }
        }

        // Label to the right (or left for the root, to keep it visible)
        let label_text = d.label.clone();
        let font = egui::FontId::proportional(11.0);
        let text_color = egui::Color32::from_rgb(40, 40, 40);
        let galley = painter.layout_no_wrap(label_text, font, text_color);

        let text_x = if d.is_leaf {
            // Leaf: label to the right
            screen_rect.right() + 4.0
        } else {
            // Branch: label above
            center.x - galley.size().x / 2.0
        };
        let text_y = if d.is_leaf {
            center.y - galley.size().y / 2.0
        } else {
            screen_rect.top() - galley.size().y - 2.0
        };

        painter.galley(egui::pos2(text_x, text_y), galley, text_color);
    }
}

// ---------------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------------

struct CollapsibleTreeApp {
    state: FlowState<NodeData, ()>,
    tree: Vec<TreeNode>,
    first_frame: bool,
}

impl CollapsibleTreeApp {
    fn new() -> Self {
        let config = FlowConfig {
            nodes_draggable: false,
            nodes_connectable: false,
            nodes_selectable: true,
            nodes_resizable: false,
            show_background: false,
            show_minimap: false,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_corner_radius: 0.0,
            min_zoom: 0.1,
            max_zoom: 5.0,
            default_source_position: Position::Right,
            default_target_position: Position::Left,
            default_node_width: NODE_SIZE,
            default_node_height: NODE_SIZE,
            handle_size: 0.1,
            edge_color: egui::Color32::from_rgb(160, 180, 200),
            edge_stroke_width: 1.2,
            ..FlowConfig::default()
        };

        let tree = build_tree();
        let state: FlowState<NodeData, ()> = FlowState::new(config);

        let mut app = CollapsibleTreeApp {
            state,
            tree,
            first_frame: true,
        };

        app.rebuild_state();
        app
    }

    // -----------------------------------------------------------------------
    // Determine which nodes are visible (all ancestors expanded)
    // -----------------------------------------------------------------------
    fn visible_nodes(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut stack: Vec<usize> = vec![0]; // root is always visible
        while let Some(idx) = stack.pop() {
            visible.push(idx);
            let node = &self.tree[idx];
            if node.expanded {
                // Push children in reverse so they come out in order
                for &child in node.children.iter().rev() {
                    stack.push(child);
                }
            }
        }
        visible.sort();
        visible
    }

    // -----------------------------------------------------------------------
    // Layout: horizontal tree, root on left
    // -----------------------------------------------------------------------
    fn compute_layout(&self, visible: &[usize]) -> Vec<(usize, f32, f32)> {
        let h_spacing = 180.0_f32;
        let v_spacing = 30.0_f32;

        // Compute the height (number of leaf-level slots) for each visible node
        // by a bottom-up pass.
        let mut subtree_height: Vec<f32> = vec![0.0; self.tree.len()];

        // Process in reverse depth order (leaves first).
        let mut sorted_vis: Vec<usize> = visible.to_vec();
        sorted_vis.sort_by(|a, b| self.tree[*b].depth.cmp(&self.tree[*a].depth));

        let vis_set: std::collections::HashSet<usize> = visible.iter().copied().collect();

        for &idx in &sorted_vis {
            let node = &self.tree[idx];
            if node.children.is_empty() || !node.expanded {
                // Leaf or collapsed: occupies 1 slot
                subtree_height[idx] = 1.0;
            } else {
                let sum: f32 = node
                    .children
                    .iter()
                    .filter(|c| vis_set.contains(c))
                    .map(|c| subtree_height[*c])
                    .sum();
                subtree_height[idx] = sum.max(1.0);
            }
        }

        // Top-down pass to assign y positions.
        let mut positions: Vec<(usize, f32, f32)> = Vec::new();
        let mut y_offset: Vec<f32> = vec![0.0; self.tree.len()];

        // Root starts at y=0
        y_offset[0] = 0.0;

        // Process top-down (by depth order)
        let mut by_depth: Vec<usize> = visible.to_vec();
        by_depth.sort_by_key(|&idx| self.tree[idx].depth);

        for &idx in &by_depth {
            let node = &self.tree[idx];
            let x = node.depth as f32 * h_spacing;
            let height = subtree_height[idx] * v_spacing;
            let y = y_offset[idx] + height / 2.0;

            positions.push((idx, x, y));

            // Distribute children vertically within this node's allocation
            if node.expanded {
                let mut child_y = y_offset[idx];
                for &child in &node.children {
                    if vis_set.contains(&child) {
                        y_offset[child] = child_y;
                        child_y += subtree_height[child] * v_spacing;
                    }
                }
            }
        }

        positions
    }

    // -----------------------------------------------------------------------
    // Rebuild FlowState from current tree expand/collapse state
    // -----------------------------------------------------------------------
    fn rebuild_state(&mut self) {
        // Create a fresh state with the same config
        let config = self.state.config.clone();
        self.state = FlowState::new(config);

        let visible = self.visible_nodes();
        let layout = self.compute_layout(&visible);

        let vis_set: std::collections::HashSet<usize> =
            visible.iter().copied().collect();

        // Add nodes
        for &(idx, x, y) in &layout {
            let tree_node = &self.tree[idx];
            let is_leaf = tree_node.children.is_empty();
            let node_id = format!("n{}", idx);

            let node_data = NodeData {
                label: tree_node.label.clone(),
                is_leaf,
                is_expanded: tree_node.expanded,
            };

            self.state.add_node(
                Node::builder(node_id)
                    .position(egui::pos2(x, y))
                    .data(node_data)
                    .size(NODE_SIZE, NODE_SIZE)
                    .build(),
            );
        }

        // Add edges (parent -> child for all visible pairs)
        for &(idx, _x, _y) in &layout {
            let tree_node = &self.tree[idx];
            if tree_node.expanded {
                for &child in &tree_node.children {
                    if vis_set.contains(&child) {
                        let edge_id = format!("e{}-{}", idx, child);
                        let source_id = format!("n{}", idx);
                        let target_id = format!("n{}", child);
                        self.state.add_edge(
                            Edge::new(edge_id, source_id, target_id)
                                .edge_type(EdgeType::Bezier),
                        );
                    }
                }
            }
        }
    }

    fn expand_all(&mut self) {
        for node in &mut self.tree {
            if !node.children.is_empty() {
                node.expanded = true;
            }
        }
        self.rebuild_state();
    }

    fn collapse_all(&mut self) {
        for node in &mut self.tree {
            if node.children.is_empty() {
                continue;
            }
            // Keep root expanded so something is visible
            if node.parent.is_some() {
                node.expanded = false;
            }
        }
        self.rebuild_state();
    }

    fn total_node_count(&self) -> usize {
        self.tree.len()
    }

    fn visible_node_count(&self) -> usize {
        self.state.nodes.len()
    }
}

// ---------------------------------------------------------------------------
// Build the tree data (a company org chart, ~70 nodes, 5 levels)
// ---------------------------------------------------------------------------

fn build_tree() -> Vec<TreeNode> {
    let mut tree: Vec<TreeNode> = Vec::new();

    // Helper: add a node, return its index
    let mut add = |label: &str, parent: Option<usize>, depth: usize| -> usize {
        let id = tree.len();
        tree.push(TreeNode {
            label: label.to_string(),
            children: Vec::new(),
            parent,
            expanded: depth < 2, // first 2 levels expanded by default
            depth,
        });
        if let Some(p) = parent {
            tree[p].children.push(id);
        }
        id
    };

    // Level 0: root
    let root = add("Acme Corp", None, 0);

    // Level 1: departments
    let eng = add("Engineering", Some(root), 1);
    let sales = add("Sales", Some(root), 1);
    let ops = add("Operations", Some(root), 1);
    let hr = add("Human Resources", Some(root), 1);
    let finance = add("Finance", Some(root), 1);

    // Level 2: teams under Engineering
    let frontend = add("Frontend", Some(eng), 2);
    let backend = add("Backend", Some(eng), 2);
    let infra = add("Infrastructure", Some(eng), 2);
    let mobile = add("Mobile", Some(eng), 2);
    let qa = add("QA", Some(eng), 2);

    // Level 2: teams under Sales
    let enterprise = add("Enterprise", Some(sales), 2);
    let smb = add("SMB", Some(sales), 2);
    let partnerships = add("Partnerships", Some(sales), 2);

    // Level 2: teams under Operations
    let logistics = add("Logistics", Some(ops), 2);
    let support = add("Support", Some(ops), 2);
    let security = add("Security", Some(ops), 2);

    // Level 2: teams under HR
    let recruiting = add("Recruiting", Some(hr), 2);
    let benefits = add("Benefits", Some(hr), 2);
    let training = add("Training", Some(hr), 2);

    // Level 2: teams under Finance
    let accounting = add("Accounting", Some(finance), 2);
    let payroll = add("Payroll", Some(finance), 2);
    let auditing = add("Auditing", Some(finance), 2);

    // Level 3: members under Frontend
    add("Alice", Some(frontend), 3);
    add("Bob", Some(frontend), 3);
    add("Carol", Some(frontend), 3);

    // Level 3: members under Backend
    add("Dave", Some(backend), 3);
    add("Eve", Some(backend), 3);
    add("Frank", Some(backend), 3);
    add("Grace", Some(backend), 3);

    // Level 3: members under Infrastructure
    add("Heidi", Some(infra), 3);
    add("Ivan", Some(infra), 3);

    // Level 3: members under Mobile
    let ios = add("iOS Team", Some(mobile), 3);
    let android = add("Android Team", Some(mobile), 3);

    // Level 3: members under QA
    add("Judy", Some(qa), 3);
    add("Karl", Some(qa), 3);
    add("Liam", Some(qa), 3);

    // Level 3: members under Enterprise
    add("Mia", Some(enterprise), 3);
    add("Noah", Some(enterprise), 3);

    // Level 3: under SMB
    add("Olivia", Some(smb), 3);
    add("Pete", Some(smb), 3);

    // Level 3: under Partnerships
    add("Quinn", Some(partnerships), 3);

    // Level 3: under Logistics
    add("Ruth", Some(logistics), 3);
    add("Sam", Some(logistics), 3);

    // Level 3: under Support
    let tier1 = add("Tier 1", Some(support), 3);
    let tier2 = add("Tier 2", Some(support), 3);

    // Level 3: under Security
    add("Tina", Some(security), 3);
    add("Uma", Some(security), 3);

    // Level 3: under Recruiting
    add("Victor", Some(recruiting), 3);
    add("Wendy", Some(recruiting), 3);

    // Level 3: under Benefits
    add("Xavier", Some(benefits), 3);

    // Level 3: under Training
    add("Yara", Some(training), 3);
    add("Zack", Some(training), 3);

    // Level 3: under Accounting
    add("Amy", Some(accounting), 3);
    add("Brian", Some(accounting), 3);

    // Level 3: under Payroll
    add("Cindy", Some(payroll), 3);

    // Level 3: under Auditing
    add("Derek", Some(auditing), 3);
    add("Ella", Some(auditing), 3);

    // Level 4: under iOS Team
    add("Fiona", Some(ios), 4);
    add("George", Some(ios), 4);
    add("Hannah", Some(ios), 4);

    // Level 4: under Android Team
    add("Ian", Some(android), 4);
    add("Jasmine", Some(android), 4);

    // Level 4: under Tier 1
    add("Kyle", Some(tier1), 4);
    add("Luna", Some(tier1), 4);
    add("Mason", Some(tier1), 4);

    // Level 4: under Tier 2
    add("Nora", Some(tier2), 4);
    add("Oscar", Some(tier2), 4);

    tree
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for CollapsibleTreeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -- Top panel with controls --
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Collapsible Tree");
                ui.separator();

                ui.label(format!(
                    "Visible: {} / {}",
                    self.visible_node_count(),
                    self.total_node_count()
                ));
                ui.separator();

                if ui.button("Expand All").clicked() {
                    self.expand_all();
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 80.0, t);
                }
                if ui.button("Collapse All").clicked() {
                    self.collapse_all();
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 80.0, t);
                }

                ui.separator();

                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 80.0, t);
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }

                ui.separator();

                let z = self.state.viewport.zoom;
                ui.label(format!("Zoom: {z:.2}"));
            });
        });

        // -- Main canvas --
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                // Fit view on first frame
                if self.first_frame {
                    let rect = ui.available_rect_before_wrap();
                    self.state.fit_view(rect, 80.0, ctx.input(|i| i.time));
                    self.first_frame = false;
                }

                let events =
                    FlowCanvas::new(&mut self.state, &TreeNodeWidget).show(ui);

                // Handle node clicks for expand/collapse
                if !events.nodes_clicked.is_empty() {
                    let mut toggled = false;
                    for node_id in &events.nodes_clicked {
                        // Parse the tree index from the node ID "nXX"
                        if let Some(idx_str) = node_id.0.strip_prefix('n') {
                            if let Ok(idx) = idx_str.parse::<usize>() {
                                if idx < self.tree.len()
                                    && !self.tree[idx].children.is_empty()
                                {
                                    self.tree[idx].expanded =
                                        !self.tree[idx].expanded;
                                    toggled = true;
                                }
                            }
                        }
                    }
                    if toggled {
                        self.rebuild_state();
                    }
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
            .with_title("egui_xyflow - Collapsible Tree")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Collapsible Tree",
        options,
        Box::new(|_cc| Ok(Box::new(CollapsibleTreeApp::new()))),
    )
}
