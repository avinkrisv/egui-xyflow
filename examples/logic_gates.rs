//! Logic Gate Circuit Simulator example for egui_xyflow.
//!
//! Digital logic gates (AND, OR, NOT, XOR, NAND, NOR) are wired together;
//! signal propagation is computed and visualized in real-time via topological
//! evaluation each frame.
//!
//! Features:
//! - Input switches toggle on click (0/1)
//! - Gates colored by output signal: green = high, dark gray = low
//! - Topological sort for cycle-safe evaluation (Kahn's algorithm)
//! - Connection validator prevents self-loops and occupied inputs
//! - Toolbar to add new gates; side panel with truth tables and signal summary
//!
//! Run with: `cargo run --example logic_gates`

use std::collections::{HashMap, VecDeque};

use eframe::egui;
use egui_xyflow::prelude::*;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- logic gates")
            .with_inner_size([1400.0, 860.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Logic Gates",
        options,
        Box::new(|_cc| Ok(Box::new(LogicGatesApp::new()))),
    )
}

// ---------------------------------------------------------------------------
// Gate helpers
// ---------------------------------------------------------------------------

/// Parse gate type label from node data. Input nodes store "INPUT:0" or
/// "INPUT:1"; all other gates store just the type name ("AND", "OR", etc.).
fn gate_label(data: &str) -> &str {
    if data.starts_with("INPUT:") {
        "INPUT"
    } else {
        data
    }
}

fn compute_gate(gate: &str, a: bool, b: bool) -> bool {
    match gate {
        "AND" => a && b,
        "OR" => a || b,
        "NOT" => !a,
        "XOR" => a ^ b,
        "NAND" => !(a && b),
        "NOR" => !(a || b),
        "OUTPUT" | "INPUT" => a,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Custom NodeWidget
// ---------------------------------------------------------------------------

struct GateNodeWidget<'a> {
    signals: &'a HashMap<String, bool>,
}

impl<'a> NodeWidget<String> for GateNodeWidget<'a> {
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
        let signal = self.signals.get(node.id.as_str()).copied().unwrap_or(false);
        let is_input = node.data.starts_with("INPUT:");
        let is_output = node.data == "OUTPUT";
        let label = gate_label(&node.data);

        // Background color: green when high, dark when low.
        // Inputs and outputs use slightly different palettes.
        let bg = if is_input {
            if node.data.ends_with(":1") {
                egui::Color32::from_rgb(76, 175, 80) // green
            } else {
                egui::Color32::from_rgb(85, 85, 90)
            }
        } else if is_output {
            if signal {
                egui::Color32::from_rgb(33, 150, 243) // blue
            } else {
                egui::Color32::from_rgb(85, 85, 90)
            }
        } else if signal {
            egui::Color32::from_rgb(56, 142, 60) // darker green
        } else {
            egui::Color32::from_rgb(60, 60, 65)
        };

        let rounding = config.node_corner_radius;

        // Selection glow
        if node.selected {
            painter.rect_filled(
                screen_rect.expand(3.0),
                rounding + 1.0,
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 60),
            );
        }

        // Background
        painter.rect_filled(screen_rect, rounding, bg);

        // Border
        let border_color = if node.selected {
            config.node_selected_border_color
        } else {
            egui::Color32::from_rgb(120, 120, 130)
        };
        painter.rect_stroke(
            screen_rect,
            rounding,
            egui::Stroke::new(
                if node.selected {
                    config.node_border_width * 2.0
                } else {
                    config.node_border_width
                },
                border_color,
            ),
            egui::StrokeKind::Middle,
        );

        // Display text
        let display = if is_input {
            let bit = if node.data.ends_with(":1") { "1" } else { "0" };
            format!("{label}  [{bit}]")
        } else if is_output {
            let bit = if signal { "1" } else { "0" };
            format!("OUT  [{bit}]")
        } else {
            let bit = if signal { "1" } else { "0" };
            format!("{label}  \u{2192}{bit}")
        };

        let galley =
            painter.layout_no_wrap(display, egui::FontId::proportional(14.0), egui::Color32::WHITE);
        let text_pos = egui::pos2(
            screen_rect.center().x - galley.size().x / 2.0,
            screen_rect.center().y - galley.size().y / 2.0,
        );
        painter.galley(text_pos, galley, egui::Color32::WHITE);
    }
}

// ---------------------------------------------------------------------------
// ConnectionValidator -- prevents self-loops & occupied input handles
// ---------------------------------------------------------------------------

/// Circuit connection validator — prevents self-loops and occupied input handles.
/// Uses the `existing_edges` slice provided by the framework.
struct CircuitValidator;

impl ConnectionValidator for CircuitValidator {
    fn is_valid_connection(&self, connection: &Connection, existing_edges: &[EdgeInfo<'_>]) -> bool {
        // No self-loops
        if connection.source == connection.target {
            return false;
        }

        // Reject if the target handle is already occupied
        if let Some(ref th) = connection.target_handle {
            for e in existing_edges {
                if e.target == &connection.target
                    && e.target_handle == Some(th.as_str())
                {
                    return false;
                }
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct LogicGatesApp {
    state: FlowState<String, ()>,
    signals: HashMap<String, bool>,
    validator: CircuitValidator,
    next_node_id: usize,
    next_edge_id: usize,
    add_position: egui::Pos2,
    selected_gate_type: Option<String>,
}

impl LogicGatesApp {
    fn new() -> Self {
        let config = FlowConfig {
            snap_to_grid: true,
            snap_grid: [20.0, 20.0],
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            background_color: egui::Color32::from_rgb(60, 60, 65),
            connection_line_type: EdgeType::SmoothStep,
            default_edge_type: EdgeType::SmoothStep,
            handle_size: 10.0,
            edge_color: egui::Color32::from_rgb(140, 140, 150),
            ..FlowConfig::default()
        };

        let mut state = FlowState::new(config);

        // --- Input switches ---
        state.add_node(Self::make_node("input_a", "INPUT:0", egui::pos2(50.0, 80.0)));
        state.add_node(Self::make_node("input_b", "INPUT:0", egui::pos2(50.0, 220.0)));
        state.add_node(Self::make_node("input_c", "INPUT:0", egui::pos2(50.0, 380.0)));

        // --- Gates ---
        // AND: inputs A, B
        state.add_node(Self::make_node("and1", "AND", egui::pos2(300.0, 100.0)));
        // NOT: input B
        state.add_node(Self::make_node("not1", "NOT", egui::pos2(300.0, 280.0)));
        // OR: inputs NOT(B), C
        state.add_node(Self::make_node("or1", "OR", egui::pos2(550.0, 300.0)));
        // XOR: inputs AND, OR
        state.add_node(Self::make_node("xor1", "XOR", egui::pos2(800.0, 200.0)));

        // --- Output probe ---
        state.add_node(Self::make_node("output1", "OUTPUT", egui::pos2(1050.0, 200.0)));

        // --- Wiring ---
        // Circuit computes: (A AND B) XOR ((NOT B) OR C)
        Self::add_wired_edge(&mut state, "e1", "input_a", "and1", "out", "in1");
        Self::add_wired_edge(&mut state, "e2", "input_b", "and1", "out", "in2");
        Self::add_wired_edge(&mut state, "e3", "input_b", "not1", "out", "in1");
        Self::add_wired_edge(&mut state, "e4", "not1", "or1", "out", "in1");
        Self::add_wired_edge(&mut state, "e5", "input_c", "or1", "out", "in2");
        Self::add_wired_edge(&mut state, "e6", "and1", "xor1", "out", "in1");
        Self::add_wired_edge(&mut state, "e7", "or1", "xor1", "out", "in2");
        Self::add_wired_edge(&mut state, "e8", "xor1", "output1", "out", "in1");

        let mut app = Self {
            state,
            signals: HashMap::new(),
            validator: CircuitValidator,
            next_node_id: 100,
            next_edge_id: 100,
            add_position: egui::pos2(450.0, 100.0),
            selected_gate_type: None,
        };
        app.evaluate_circuit();
        app
    }

    // -- Node factory -------------------------------------------------------

    fn make_node(id: &str, data: &str, pos: egui::Pos2) -> Node<String> {
        let kind = gate_label(data);
        let mut builder = Node::builder(id).position(pos).data(data.to_string());

        match kind {
            "INPUT" => {
                builder = builder
                    .handle(NodeHandle::source(Position::Right).with_id("out"))
                    .size(100.0, 50.0);
            }
            "NOT" => {
                builder = builder
                    .handle(NodeHandle::target(Position::Left).with_id("in1"))
                    .handle(NodeHandle::source(Position::Right).with_id("out"))
                    .size(100.0, 50.0);
            }
            "OUTPUT" => {
                builder = builder
                    .handle(NodeHandle::target(Position::Left).with_id("in1"))
                    .size(100.0, 50.0);
            }
            _ => {
                // Two-input gates: AND, OR, XOR, NAND, NOR
                builder = builder
                    .handle(NodeHandle::target(Position::Left).with_id("in1"))
                    .handle(NodeHandle::target(Position::Left).with_id("in2"))
                    .handle(NodeHandle::source(Position::Right).with_id("out"))
                    .size(120.0, 60.0);
            }
        }

        builder.build()
    }

    fn add_wired_edge(
        state: &mut FlowState<String, ()>,
        id: &str,
        source: &str,
        target: &str,
        src_handle: &str,
        tgt_handle: &str,
    ) {
        let mut edge = Edge::new(id, source, target)
            .edge_type(EdgeType::SmoothStep)
            .marker_end_arrow();
        edge.source_handle = Some(src_handle.into());
        edge.target_handle = Some(tgt_handle.into());
        state.add_edge(edge);
    }

    // -- Add gate from toolbar ----------------------------------------------

    fn add_gate(&mut self, gate_data: &str) {
        let id = format!("gate_{}", self.next_node_id);
        self.next_node_id += 1;

        self.state
            .add_node(Self::make_node(&id, gate_data, self.add_position));

        self.add_position += egui::vec2(30.0, 30.0);
        if self.add_position.x > 900.0 || self.add_position.y > 600.0 {
            self.add_position = egui::pos2(450.0, 100.0);
        }
    }

    // -- Signal evaluation via topological sort (Kahn's algorithm) ----------

    fn evaluate_circuit(&mut self) {
        // Build in-degree map and successor list
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut successors: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.state.nodes {
            in_degree.entry(node.id.as_str()).or_insert(0);
        }
        for edge in &self.state.edges {
            *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
            successors
                .entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
        }

        // Kahn's algorithm
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut order: Vec<&str> = Vec::new();
        while let Some(nid) = queue.pop_front() {
            order.push(nid);
            if let Some(targets) = successors.get(nid) {
                for &t in targets {
                    if let Some(deg) = in_degree.get_mut(t) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(t);
                        }
                    }
                }
            }
        }

        // Build reverse lookup: target_node → { target_handle → source_node }
        let mut input_map: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
        for edge in &self.state.edges {
            let th = edge.target_handle.as_deref().unwrap_or("");
            input_map
                .entry(edge.target.as_str())
                .or_default()
                .insert(th, edge.source.as_str());
        }

        // Evaluate in topological order
        let mut signals: HashMap<String, bool> = HashMap::new();

        for &nid in &order {
            let data = self
                .state
                .nodes
                .iter()
                .find(|n| n.id.as_str() == nid)
                .map(|n| n.data.as_str())
                .unwrap_or("");

            // Input switches provide their stored value directly
            if data.starts_with("INPUT:") {
                signals.insert(nid.to_string(), data.ends_with(":1"));
                continue;
            }

            let inputs = input_map.get(nid);
            let get = |handle: &str| -> bool {
                inputs
                    .and_then(|m| m.get(handle))
                    .and_then(|src| signals.get(*src))
                    .copied()
                    .unwrap_or(false)
            };

            let a = get("in1");
            let b = get("in2");
            let output = compute_gate(data, a, b);
            signals.insert(nid.to_string(), output);
        }

        self.signals = signals;
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for LogicGatesApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Evaluate circuit every frame so colours stay up-to-date
        self.evaluate_circuit();

        // -- Toolbar --------------------------------------------------------
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Add:");
                if ui.button("Input").clicked() {
                    self.add_gate("INPUT:0");
                }
                ui.separator();
                for gate in ["AND", "OR", "NOT", "XOR", "NAND", "NOR"] {
                    if ui.button(gate).clicked() {
                        self.add_gate(gate);
                    }
                }
                ui.separator();
                if ui.button("Output").clicked() {
                    self.add_gate("OUTPUT");
                }

                ui.add_space(20.0);
                ui.separator();
                if ui.button("Fit All").clicked() {
                    let rect = ctx.screen_rect();
                    self.state.fit_view(rect, 40.0, ctx.input(|i| i.time));
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }
            });
        });

        // -- Side panel -----------------------------------------------------
        egui::SidePanel::right("info_panel")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Circuit Info");
                ui.separator();

                // Truth table for selected gate
                if let Some(ref gt) = self.selected_gate_type {
                    ui.strong(format!("Selected: {gt}"));
                    ui.add_space(4.0);
                    show_truth_table(ui, gt);
                    ui.separator();
                }

                // Signal summary
                ui.heading("Signals");
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for node in &self.state.nodes {
                            let sig =
                                self.signals.get(node.id.as_str()).copied().unwrap_or(false);
                            let bit = if node.data.starts_with("INPUT:") {
                                if node.data.ends_with(":1") {
                                    "1"
                                } else {
                                    "0"
                                }
                            } else if sig {
                                "1"
                            } else {
                                "0"
                            };
                            let label = gate_label(&node.data);
                            let color = if bit == "1" {
                                egui::Color32::from_rgb(76, 175, 80)
                            } else {
                                egui::Color32::from_rgb(160, 160, 160)
                            };
                            ui.horizontal(|ui| {
                                ui.colored_label(color, "\u{25cf}");
                                ui.label(format!("{} ({}): {bit}", node.id, label));
                            });
                        }
                    });

                ui.separator();
                ui.heading("Legend");
                ui.label("Click input nodes to toggle 0/1");
                ui.label("Drag between handles to wire");
                ui.label("Delete key removes selection");
                ui.add_space(4.0);
                ui.label("AND  - both inputs high");
                ui.label("OR   - any input high");
                ui.label("NOT  - inverts input");
                ui.label("XOR  - inputs differ");
                ui.label("NAND - NOT of AND");
                ui.label("NOR  - NOT of OR");
            });

        // -- Canvas ---------------------------------------------------------
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(28, 28, 32)))
            .show(ctx, |ui| {
                let node_widget = GateNodeWidget {
                    signals: &self.signals,
                };
                let events = FlowCanvas::new(&mut self.state, &node_widget)
                    .connection_validator(&self.validator)
                    .show(ui);

                // Toggle input switches on click
                for nid in &events.nodes_clicked {
                    if let Some(node) =
                        self.state.nodes.iter_mut().find(|n| n.id == *nid)
                    {
                        if node.data == "INPUT:0" {
                            node.data = "INPUT:1".to_string();
                        } else if node.data == "INPUT:1" {
                            node.data = "INPUT:0".to_string();
                        }
                    }
                }

                // Create edges from new connections
                for conn in &events.connections_made {
                    let eid = format!("e_{}", self.next_edge_id);
                    self.next_edge_id += 1;
                    let mut edge = Edge::new(
                        &*eid,
                        conn.source.as_str(),
                        conn.target.as_str(),
                    )
                    .edge_type(EdgeType::SmoothStep)
                    .marker_end_arrow();
                    edge.source_handle = conn.source_handle.clone();
                    edge.target_handle = conn.target_handle.clone();
                    self.state.add_edge(edge);
                }

                // Track selected node for the side panel truth table
                if events.selection_changed {
                    self.selected_gate_type =
                        events.selected_nodes.first().and_then(|id| {
                            self.state
                                .nodes
                                .iter()
                                .find(|n| n.id == *id)
                                .map(|n| gate_label(&n.data).to_string())
                        });
                }
            });

        ctx.request_repaint();
    }
}

// ---------------------------------------------------------------------------
// Truth table display
// ---------------------------------------------------------------------------

fn show_truth_table(ui: &mut egui::Ui, gate_type: &str) {
    match gate_type {
        "AND" | "OR" | "XOR" | "NAND" | "NOR" => {
            ui.label("Truth table:");
            egui::Grid::new("truth_table")
                .striped(true)
                .min_col_width(30.0)
                .show(ui, |ui| {
                    ui.strong("A");
                    ui.strong("B");
                    ui.strong("Out");
                    ui.end_row();
                    for &a in &[false, true] {
                        for &b in &[false, true] {
                            let out = compute_gate(gate_type, a, b);
                            ui.label(if a { "1" } else { "0" });
                            ui.label(if b { "1" } else { "0" });
                            ui.label(if out { "1" } else { "0" });
                            ui.end_row();
                        }
                    }
                });
        }
        "NOT" => {
            ui.label("Truth table:");
            egui::Grid::new("truth_table")
                .striped(true)
                .min_col_width(30.0)
                .show(ui, |ui| {
                    ui.strong("In");
                    ui.strong("Out");
                    ui.end_row();
                    for &v in &[false, true] {
                        ui.label(if v { "1" } else { "0" });
                        ui.label(if !v { "1" } else { "0" });
                        ui.end_row();
                    }
                });
        }
        _ => {
            ui.label("No truth table for this node type.");
        }
    }
}
