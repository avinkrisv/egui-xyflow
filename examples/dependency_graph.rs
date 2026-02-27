//! Dependency graph example for egui_xyflow.
//!
//! Demonstrates a package/task build pipeline with ~10 tasks and their
//! dependencies.  Features:
//!
//! - Cycle-prevention via a `ConnectionValidator` that runs DFS
//! - Side panel showing topological execution order
//! - Tasks have statuses: Pending, Running, Complete, Failed
//! - Click a node to cycle its status
//! - Nodes are coloured by status
//! - Bezier edges with arrows; "Running" edges are animated
//!
//! Node data is `String` encoded as "TaskName|status" where status is one of
//! `pending`, `running`, `complete`, `failed`.

use std::collections::{HashMap, HashSet};

use eframe::egui;
use egui_xyflow::prelude::*;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- dependency graph")
            .with_inner_size([1400.0, 860.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Dependency Graph",
        options,
        Box::new(|_cc| Ok(Box::new(DepGraphApp::new()))),
    )
}

// ---------------------------------------------------------------------------
// Data helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

impl TaskStatus {
    fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Complete => "complete",
            TaskStatus::Failed => "failed",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "running" => TaskStatus::Running,
            "complete" => TaskStatus::Complete,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        }
    }

    fn next(self) -> Self {
        match self {
            TaskStatus::Pending => TaskStatus::Running,
            TaskStatus::Running => TaskStatus::Complete,
            TaskStatus::Complete => TaskStatus::Failed,
            TaskStatus::Failed => TaskStatus::Pending,
        }
    }
}

/// Encode task name and status into the node data string.
fn encode_data(name: &str, status: TaskStatus) -> String {
    format!("{}|{}", name, status.as_str())
}

/// Decode node data string into (task_name, status).
fn decode_data(data: &str) -> (&str, TaskStatus) {
    if let Some(idx) = data.rfind('|') {
        let name = &data[..idx];
        let status = TaskStatus::from_str(&data[idx + 1..]);
        (name, status)
    } else {
        (data, TaskStatus::Pending)
    }
}

// ---------------------------------------------------------------------------
// Cycle-detecting ConnectionValidator
// ---------------------------------------------------------------------------

/// Cycle-detecting connection validator.  Uses the `existing_edges` slice
/// provided by the framework — no RefCell snapshot needed.
struct CyclePreventionValidator;

impl CyclePreventionValidator {
    /// Returns `true` if adding an edge from `source` to `target` would create
    /// a cycle in the directed graph.
    fn would_create_cycle(source: &str, target: &str, existing_edges: &[EdgeInfo<'_>]) -> bool {
        // If source == target, trivially a self-loop.
        if source == target {
            return true;
        }

        // Build adjacency list from current edges + the proposed edge.
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in existing_edges {
            adj.entry(e.source.as_str()).or_default().push(e.target.as_str());
        }
        adj.entry(source).or_default().push(target);

        // DFS from `target` looking for a path back to `source`.
        let mut visited = HashSet::new();
        let mut stack = vec![target];
        while let Some(node) = stack.pop() {
            if node == source {
                return true;
            }
            if visited.insert(node) {
                if let Some(neighbours) = adj.get(node) {
                    for &n in neighbours {
                        stack.push(n);
                    }
                }
            }
        }

        false
    }
}

impl ConnectionValidator for CyclePreventionValidator {
    fn is_valid_connection(&self, connection: &Connection, existing_edges: &[EdgeInfo<'_>]) -> bool {
        !Self::would_create_cycle(connection.source.as_str(), connection.target.as_str(), existing_edges)
    }
}

// ---------------------------------------------------------------------------
// Topological sort
// ---------------------------------------------------------------------------

/// Kahn's algorithm. Returns `None` if a cycle is detected (should not happen
/// in our graph because the validator prevents cycles).
fn topological_sort(node_ids: &[String], edges: &[(String, String)]) -> Option<Vec<String>> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for id in node_ids {
        in_degree.entry(id.as_str()).or_insert(0);
        adj.entry(id.as_str()).or_default();
    }

    for (s, t) in edges {
        adj.entry(s.as_str()).or_default().push(t.as_str());
        *in_degree.entry(t.as_str()).or_insert(0) += 1;
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    queue.sort(); // deterministic order

    let mut result = Vec::new();
    while let Some(n) = queue.pop() {
        result.push(n.to_string());
        if let Some(neighbours) = adj.get(n) {
            for &nb in neighbours {
                if let Some(deg) = in_degree.get_mut(nb) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(nb);
                        queue.sort();
                    }
                }
            }
        }
    }

    if result.len() == node_ids.len() {
        Some(result)
    } else {
        None // cycle
    }
}

// ---------------------------------------------------------------------------
// Custom node widget (colour by status)
// ---------------------------------------------------------------------------

struct TaskNodeWidget;

impl NodeWidget<String> for TaskNodeWidget {
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
        let (task_name, status) = decode_data(&node.data);

        // Background colour by status.
        let bg = match status {
            TaskStatus::Pending => egui::Color32::from_rgb(200, 200, 210), // gray
            TaskStatus::Running => egui::Color32::from_rgb(100, 160, 240), // blue
            TaskStatus::Complete => egui::Color32::from_rgb(100, 200, 120), // green
            TaskStatus::Failed => egui::Color32::from_rgb(230, 100, 100),  // red
        };

        let border_color = if node.selected {
            egui::Color32::from_rgb(40, 40, 40)
        } else {
            match status {
                TaskStatus::Pending => egui::Color32::from_rgb(140, 140, 155),
                TaskStatus::Running => egui::Color32::from_rgb(50, 110, 200),
                TaskStatus::Complete => egui::Color32::from_rgb(50, 160, 80),
                TaskStatus::Failed => egui::Color32::from_rgb(180, 50, 50),
            }
        };

        let rounding = config.node_corner_radius;

        // Selection glow.
        if node.selected {
            let shadow_rect = screen_rect.expand(3.0);
            painter.rect_filled(
                shadow_rect,
                rounding + 2.0,
                egui::Color32::from_rgba_unmultiplied(40, 40, 40, 50),
            );
        }

        // Background.
        painter.rect_filled(screen_rect, rounding, bg);

        // Border.
        painter.rect_stroke(
            screen_rect,
            rounding,
            egui::Stroke::new(
                if node.selected { 2.5 } else { 1.5 },
                border_color,
            ),
            egui::StrokeKind::Middle,
        );

        // Status indicator bar at the top of the node.
        let bar_rect = egui::Rect::from_min_size(
            screen_rect.min,
            egui::vec2(screen_rect.width(), 5.0),
        );
        let bar_color = match status {
            TaskStatus::Pending => egui::Color32::from_rgb(160, 160, 175),
            TaskStatus::Running => egui::Color32::from_rgb(30, 100, 220),
            TaskStatus::Complete => egui::Color32::from_rgb(30, 140, 60),
            TaskStatus::Failed => egui::Color32::from_rgb(200, 30, 30),
        };
        painter.rect_filled(bar_rect, rounding, bar_color);

        // Task name (bold).
        let text_color = match status {
            TaskStatus::Running | TaskStatus::Failed => egui::Color32::WHITE,
            _ => egui::Color32::from_rgb(30, 30, 30),
        };

        let galley = painter.layout_no_wrap(
            task_name.to_string(),
            egui::FontId::proportional(13.0),
            text_color,
        );
        let text_pos = egui::pos2(
            screen_rect.center().x - galley.size().x / 2.0,
            screen_rect.center().y - galley.size().y / 2.0 + 1.0,
        );
        painter.galley(text_pos, galley, text_color);

        // Small status label at the bottom-right.
        let status_text = status.as_str();
        let dim_color = match status {
            TaskStatus::Running | TaskStatus::Failed => egui::Color32::from_rgb(220, 220, 220),
            _ => egui::Color32::from_rgb(100, 100, 100),
        };
        let status_galley = painter.layout_no_wrap(
            status_text.to_string(),
            egui::FontId::proportional(9.0),
            dim_color,
        );
        let status_pos = egui::pos2(
            screen_rect.right() - status_galley.size().x - 4.0,
            screen_rect.bottom() - status_galley.size().y - 2.0,
        );
        painter.galley(status_pos, status_galley, dim_color);
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct DepGraphApp {
    state: FlowState<String, ()>,
    validator: CyclePreventionValidator,
    event_log: Vec<String>,
}

impl DepGraphApp {
    fn new() -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Lines,
            show_minimap: true,
            node_corner_radius: 6.0,
            default_node_width: 170.0,
            default_node_height: 50.0,
            ..FlowConfig::default()
        };

        let mut state: FlowState<String, ()> = FlowState::new(config);

        // -- Tasks (laid out roughly left-to-right in dependency order) ------
        //
        // Layer 0: Parse Config
        // Layer 1: Compile Frontend, Compile Backend
        // Layer 2: Run Tests, Build Docker
        // Layer 3: Integration Tests, Deploy Staging
        // Layer 4: Deploy Prod, Update Docs
        // Layer 5: Notify Team

        let tasks: Vec<(&str, &str, f32, f32)> = vec![
            ("parse_cfg",    "Parse Config",       80.0,  260.0),
            ("comp_fe",      "Compile Frontend",  340.0,  140.0),
            ("comp_be",      "Compile Backend",   340.0,  380.0),
            ("run_tests",    "Run Tests",         600.0,  200.0),
            ("build_docker", "Build Docker",      600.0,  380.0),
            ("int_tests",    "Integration Tests", 860.0,  140.0),
            ("deploy_stg",   "Deploy Staging",    860.0,  340.0),
            ("deploy_prod",  "Deploy Prod",      1120.0,  200.0),
            ("update_docs",  "Update Docs",      1120.0,  400.0),
            ("notify",       "Notify Team",      1360.0,  300.0),
        ];

        for (id, name, x, y) in &tasks {
            state.add_node(
                Node::builder(*id)
                    .position(egui::pos2(*x, *y))
                    .data(encode_data(name, TaskStatus::Pending))
                    .handle(NodeHandle::target(Position::Left))
                    .handle(NodeHandle::source(Position::Right))
                    .size(170.0, 50.0)
                    .build(),
            );
        }

        // -- Dependency edges ------------------------------------------------

        let deps: Vec<(&str, &str)> = vec![
            ("parse_cfg",    "comp_fe"),
            ("parse_cfg",    "comp_be"),
            ("comp_fe",      "run_tests"),
            ("comp_be",      "run_tests"),
            ("comp_be",      "build_docker"),
            ("run_tests",    "int_tests"),
            ("run_tests",    "deploy_stg"),
            ("build_docker", "deploy_stg"),
            ("int_tests",    "deploy_prod"),
            ("deploy_stg",   "deploy_prod"),
            ("deploy_stg",   "update_docs"),
            ("deploy_prod",  "notify"),
            ("update_docs",  "notify"),
        ];

        for (src, tgt) in &deps {
            let eid = format!("e-{}-{}", src, tgt);
            state.add_edge(
                Edge::new(eid, *src, *tgt)
                    .edge_type(EdgeType::Bezier)
                    .marker_end_arrow(),
            );
        }

        Self {
            state,
            validator: CyclePreventionValidator,
            event_log: Vec::new(),
        }
    }

    /// Push a message to the event log (capped at 80 entries).
    fn log(&mut self, msg: impl Into<String>) {
        if self.event_log.len() >= 80 {
            self.event_log.remove(0);
        }
        self.event_log.push(msg.into());
    }

    /// Collect (source_id, target_id) pairs from the current edge list.
    fn edge_pairs(&self) -> Vec<(String, String)> {
        self.state
            .edges
            .iter()
            .map(|e| (e.source.to_string(), e.target.to_string()))
            .collect()
    }

    /// Collect all node IDs.
    fn node_ids(&self) -> Vec<String> {
        self.state.nodes.iter().map(|n| n.id.to_string()).collect()
    }

    /// Look up a node's task name given its id string.
    fn task_name_for_id(&self, id: &str) -> String {
        self.state
            .nodes
            .iter()
            .find(|n| n.id.as_str() == id)
            .map(|n| {
                let (name, _) = decode_data(&n.data);
                name.to_string()
            })
            .unwrap_or_else(|| id.to_string())
    }

    /// Toggle the status of a clicked node and update animated edges.
    fn toggle_node_status(&mut self, node_id: &str) {
        if let Some(node) = self.state.nodes.iter_mut().find(|n| n.id.as_str() == node_id) {
            let (name, status) = decode_data(&node.data);
            let new_status = status.next();
            let name_owned = name.to_string();
            node.data = encode_data(&name_owned, new_status);
            self.log(format!(
                "[{}] {} -> {}",
                name_owned,
                status.as_str(),
                new_status.as_str()
            ));
        }
        // Rebuild lookup since we changed node data.
        self.state.rebuild_lookup();
        // Update edge animation: animate edges whose source is Running.
        self.update_edge_animation();
    }

    /// Set animated=true on edges whose source node has status Running.
    fn update_edge_animation(&mut self) {
        let running_nodes: HashSet<String> = self
            .state
            .nodes
            .iter()
            .filter(|n| {
                let (_, status) = decode_data(&n.data);
                status == TaskStatus::Running
            })
            .map(|n| n.id.to_string())
            .collect();

        let mut any_animated = false;
        for edge in &mut self.state.edges {
            edge.animated = running_nodes.contains(edge.source.as_str());
            if edge.animated {
                any_animated = true;
            }
        }
        self.state.has_animated_edges = any_animated;
    }

    /// Handle events returned by the canvas.
    fn handle_events(&mut self, events: &FlowEvents) {
        // Process node clicks -- toggle status.
        for nid in &events.nodes_clicked {
            let id_str = nid.to_string();
            self.toggle_node_status(&id_str);
        }

        // Log new connections.
        for conn in &events.connections_made {
            let src_name = self.task_name_for_id(conn.source.as_str());
            let tgt_name = self.task_name_for_id(conn.target.as_str());
            self.log(format!("+ edge: {} -> {}", src_name, tgt_name));
        }

        // Log deletions.
        if !events.nodes_deleted.is_empty() {
            let names: Vec<String> = events
                .nodes_deleted
                .iter()
                .map(|id| self.task_name_for_id(id.as_str()))
                .collect();
            self.log(format!("- deleted nodes: {}", names.join(", ")));
        }
        if !events.edges_deleted.is_empty() {
            let ids: Vec<String> = events.edges_deleted.iter().map(|e| e.to_string()).collect();
            self.log(format!("- deleted edges: {}", ids.join(", ")));
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for DepGraphApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -- Top toolbar -----------------------------------------------------
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Dependency Graph");
                ui.separator();

                if ui.button("Fit All").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 40.0, t);
                }
                if ui.button("Fit Selected").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_selected_nodes(rect, 40.0, t);
                }
                if ui.button("Reset All Pending").clicked() {
                    for node in &mut self.state.nodes {
                        let (name, _) = decode_data(&node.data);
                        let name_owned = name.to_string();
                        node.data = encode_data(&name_owned, TaskStatus::Pending);
                    }
                    self.state.rebuild_lookup();
                    self.update_edge_animation();
                    self.log("Reset all tasks to Pending".to_string());
                }

                ui.separator();

                let n = self.state.nodes.len();
                let e = self.state.edges.len();
                let z = self.state.viewport.zoom;
                ui.label(format!(
                    "{n} tasks  |  {e} deps  |  zoom {z:.2}"
                ));

                ui.separator();
                ui.label("Click a node to cycle: Pending -> Running -> Complete -> Failed");
            });
        });

        // -- Side panel: topo order + event log ------------------------------
        egui::SidePanel::right("info_panel")
            .resizable(true)
            .min_width(230.0)
            .default_width(280.0)
            .show(ctx, |ui| {
                // ---- Execution Order section ----
                ui.heading("Execution Order");
                ui.separator();

                let node_ids = self.node_ids();
                let edge_pairs = self.edge_pairs();
                if let Some(order) = topological_sort(&node_ids, &edge_pairs) {
                    egui::ScrollArea::vertical()
                        .id_salt("topo_scroll")
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (i, id) in order.iter().enumerate() {
                                let name = self.task_name_for_id(id);
                                let status = self
                                    .state
                                    .nodes
                                    .iter()
                                    .find(|n| n.id.as_str() == id.as_str())
                                    .map(|n| {
                                        let (_, s) = decode_data(&n.data);
                                        s
                                    })
                                    .unwrap_or(TaskStatus::Pending);

                                let status_icon = match status {
                                    TaskStatus::Pending  => "[.]",
                                    TaskStatus::Running  => "[>]",
                                    TaskStatus::Complete => "[v]",
                                    TaskStatus::Failed   => "[x]",
                                };

                                let color = match status {
                                    TaskStatus::Pending  => egui::Color32::GRAY,
                                    TaskStatus::Running  => egui::Color32::from_rgb(60, 130, 240),
                                    TaskStatus::Complete => egui::Color32::from_rgb(50, 170, 80),
                                    TaskStatus::Failed   => egui::Color32::from_rgb(220, 60, 60),
                                };

                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:>2}.", i + 1))
                                            .monospace()
                                            .size(12.0),
                                    );
                                    ui.label(
                                        egui::RichText::new(status_icon)
                                            .monospace()
                                            .size(12.0)
                                            .color(color),
                                    );
                                    ui.label(
                                        egui::RichText::new(&name)
                                            .size(12.0)
                                            .color(color),
                                    );
                                });
                            }
                        });
                } else {
                    ui.colored_label(
                        egui::Color32::RED,
                        "Cycle detected! (should not happen)",
                    );
                }

                ui.add_space(12.0);
                ui.separator();

                // ---- Status Summary ----
                ui.heading("Status Summary");
                ui.separator();
                let mut counts = [0u32; 4];
                for node in &self.state.nodes {
                    let (_, status) = decode_data(&node.data);
                    match status {
                        TaskStatus::Pending  => counts[0] += 1,
                        TaskStatus::Running  => counts[1] += 1,
                        TaskStatus::Complete => counts[2] += 1,
                        TaskStatus::Failed   => counts[3] += 1,
                    }
                }
                ui.label(format!("  Pending:  {}", counts[0]));
                ui.colored_label(
                    egui::Color32::from_rgb(60, 130, 240),
                    format!("  Running:  {}", counts[1]),
                );
                ui.colored_label(
                    egui::Color32::from_rgb(50, 170, 80),
                    format!("  Complete: {}", counts[2]),
                );
                ui.colored_label(
                    egui::Color32::from_rgb(220, 60, 60),
                    format!("  Failed:   {}", counts[3]),
                );

                ui.add_space(12.0);
                ui.separator();

                // ---- Event Log ----
                ui.heading("Event Log");
                if ui.small_button("Clear").clicked() {
                    self.event_log.clear();
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("event_log_scroll")
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for msg in &self.event_log {
                            ui.label(egui::RichText::new(msg).monospace().size(11.0));
                        }
                    });
            });

        // -- Main canvas -----------------------------------------------------
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(248, 248, 252)))
            .show(ctx, |ui| {
                let events = FlowCanvas::new(&mut self.state, &TaskNodeWidget)
                    .connection_validator(&self.validator)
                    .show(ui);

                if !events.is_empty() {
                    let mut filtered = events.clone();
                    // Don't spam the log with viewport ticks or drag moves.
                    filtered.viewport_changed = false;
                    filtered.nodes_dragged.clear();
                    self.handle_events(&filtered);
                }
            });
    }
}
