//! Data processing pipeline builder using egui_xyflow.
//!
//! Demonstrates building a left-to-right data pipeline with three node
//! categories:
//!
//!   Sources  -->  Transforms  -->  Sinks
//!
//! Features demonstrated
//! ---------------------
//! * Three node categories with different handle configurations:
//!   - Source nodes (output only): CSV File, Database, API Endpoint
//!   - Transform nodes (input + output): Filter, Map, Aggregate, Join
//!   - Sink nodes (input only): Dashboard, Export CSV, Alert
//! * ConnectionValidator that enforces flow direction (sources cannot
//!   receive connections, sinks cannot produce them).
//! * Toolbar for adding new nodes of each type.
//! * SmoothStep edges with arrows.
//! * Side panel showing connection counts per node.
//!
//! Controls
//! --------
//! * Scroll wheel / pinch         -> zoom
//! * Left-drag on background      -> pan
//! * Left-drag on node            -> move node
//! * Drag handle -> handle        -> create edge
//! * Delete / Backspace           -> delete selected nodes / edges
//! * Toolbar buttons              -> add new nodes

use eframe::egui;
use egui_xyflow::prelude::*;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- data pipeline builder")
            .with_inner_size([1400.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Data Pipeline Builder",
        options,
        Box::new(|_cc| Ok(Box::new(PipelineApp::new()))),
    )
}

// ---------------------------------------------------------------------------
// Node categories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeCategory {
    Source,
    Transform,
    Sink,
}

/// Determine the category of a node from the data label stored in it.
fn node_category(label: &str) -> NodeCategory {
    match label {
        "CSV File" | "Database" | "API Endpoint" => NodeCategory::Source,
        "Dashboard" | "Export CSV" | "Alert" => NodeCategory::Sink,
        _ => NodeCategory::Transform,
    }
}

// ---------------------------------------------------------------------------
// Connection validator -- enforces flow direction
// ---------------------------------------------------------------------------

/// Validates that connections respect the pipeline direction:
/// - Source nodes may only have outgoing connections (never be a target).
/// - Sink nodes may only have incoming connections (never be a source).
/// - A node cannot connect to itself.
struct PipelineValidator {
    /// Maps node ID -> label so we can look up categories.
    node_labels: std::collections::HashMap<String, String>,
}

impl ConnectionValidator for PipelineValidator {
    fn is_valid_connection(&self, connection: &Connection, _existing_edges: &[EdgeInfo<'_>]) -> bool {
        let source_id = connection.source.as_str();
        let target_id = connection.target.as_str();

        // No self-connections.
        if source_id == target_id {
            return false;
        }

        // Look up categories.
        if let (Some(src_label), Some(tgt_label)) =
            (self.node_labels.get(source_id), self.node_labels.get(target_id))
        {
            let src_cat = node_category(src_label);
            let tgt_cat = node_category(tgt_label);

            // Sinks must not be a source of a connection.
            if src_cat == NodeCategory::Sink {
                return false;
            }
            // Sources must not be a target of a connection.
            if tgt_cat == NodeCategory::Source {
                return false;
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct PipelineApp {
    state: FlowState<String, ()>,
    validator: PipelineValidator,
    next_node_id: u32,
}

impl PipelineApp {
    fn new() -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            show_minimap: true,
            snap_to_grid: true,
            default_edge_type: EdgeType::SmoothStep,
            ..FlowConfig::default()
        };

        let mut state: FlowState<String, ()> = FlowState::new(config);

        // -- Source nodes (left column, output handle on Right) --------------

        state.add_node(
            Node::builder("src_csv")
                .position(egui::pos2(60.0, 80.0))
                .data("CSV File".to_string())
                .handle(NodeHandle::source(Position::Right))
                .size(140.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("src_db")
                .position(egui::pos2(60.0, 200.0))
                .data("Database".to_string())
                .handle(NodeHandle::source(Position::Right))
                .size(140.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("src_api")
                .position(egui::pos2(60.0, 320.0))
                .data("API Endpoint".to_string())
                .handle(NodeHandle::source(Position::Right))
                .size(140.0, 44.0)
                .build(),
        );

        // -- Transform nodes (middle column, handles on Left + Right) --------

        state.add_node(
            Node::builder("tx_filter")
                .position(egui::pos2(340.0, 60.0))
                .data("Filter".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .size(130.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("tx_map")
                .position(egui::pos2(340.0, 170.0))
                .data("Map".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .size(130.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("tx_agg")
                .position(egui::pos2(340.0, 280.0))
                .data("Aggregate".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .size(130.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("tx_join")
                .position(egui::pos2(340.0, 390.0))
                .data("Join".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .size(130.0, 44.0)
                .build(),
        );

        // -- Sink nodes (right column, input handle on Left) -----------------

        state.add_node(
            Node::builder("sk_dash")
                .position(egui::pos2(620.0, 80.0))
                .data("Dashboard".to_string())
                .handle(NodeHandle::target(Position::Left))
                .size(140.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("sk_csv")
                .position(egui::pos2(620.0, 230.0))
                .data("Export CSV".to_string())
                .handle(NodeHandle::target(Position::Left))
                .size(140.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("sk_alert")
                .position(egui::pos2(620.0, 380.0))
                .data("Alert".to_string())
                .handle(NodeHandle::target(Position::Left))
                .size(140.0, 44.0)
                .build(),
        );

        // -- Edges (SmoothStep with arrows) ----------------------------------
        // Wire up a representative pipeline:
        //   CSV File -> Filter -> Dashboard
        //   Database -> Map -> Export CSV
        //   API Endpoint -> Aggregate -> Alert
        //   Database -> Join -> Dashboard

        state.add_edge(
            Edge::new("e1", "src_csv", "tx_filter")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e2", "tx_filter", "sk_dash")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e3", "src_db", "tx_map")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e4", "tx_map", "sk_csv")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e5", "src_api", "tx_agg")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e6", "tx_agg", "sk_alert")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e7", "src_db", "tx_join")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::new("e8", "tx_join", "sk_dash")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );

        // Build the label lookup for the validator.
        let node_labels: std::collections::HashMap<String, String> = state
            .nodes
            .iter()
            .map(|n| (n.id.to_string(), n.data.clone()))
            .collect();

        Self {
            state,
            validator: PipelineValidator { node_labels },
            next_node_id: 100,
        }
    }

    /// Add a node of the given label at a computed position.
    fn add_pipeline_node(&mut self, label: &str) {
        let id = format!("n{}", self.next_node_id);
        self.next_node_id += 1;

        let cat = node_category(label);

        // Stagger new nodes so they don't overlap.
        let base_x = match cat {
            NodeCategory::Source => 60.0,
            NodeCategory::Transform => 340.0,
            NodeCategory::Sink => 620.0,
        };
        let y_offset = (self.next_node_id as f32) * 30.0;
        let pos = egui::pos2(base_x, 450.0 + y_offset);

        let mut builder = Node::builder(id.as_str())
            .position(pos)
            .data(label.to_string())
            .size(140.0, 44.0);

        match cat {
            NodeCategory::Source => {
                builder = builder.handle(NodeHandle::source(Position::Right));
            }
            NodeCategory::Transform => {
                builder = builder
                    .handle(NodeHandle::target(Position::Left))
                    .handle(NodeHandle::source(Position::Right));
            }
            NodeCategory::Sink => {
                builder = builder.handle(NodeHandle::target(Position::Left));
            }
        }

        self.state.add_node(builder.build());

        // Update the validator label map.
        self.validator
            .node_labels
            .insert(id, label.to_string());
    }

    /// Count incoming and outgoing connections for each node.
    fn connection_counts(&self) -> std::collections::HashMap<String, (usize, usize)> {
        let mut counts: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();

        // Initialize all nodes with zero counts.
        for node in &self.state.nodes {
            counts.entry(node.id.to_string()).or_insert((0, 0));
        }

        // Tally edges.
        for edge in &self.state.edges {
            counts
                .entry(edge.source.to_string())
                .or_insert((0, 0))
                .1 += 1; // outgoing
            counts
                .entry(edge.target.to_string())
                .or_insert((0, 0))
                .0 += 1; // incoming
        }

        counts
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for PipelineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -- Toolbar ---------------------------------------------------------
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Data Pipeline Builder");
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

                // -- Add Source nodes ----------------------------------------
                ui.label("Sources:");
                if ui.button("+ CSV File").clicked() {
                    self.add_pipeline_node("CSV File");
                }
                if ui.button("+ Database").clicked() {
                    self.add_pipeline_node("Database");
                }
                if ui.button("+ API Endpoint").clicked() {
                    self.add_pipeline_node("API Endpoint");
                }

                ui.separator();

                // -- Add Transform nodes -------------------------------------
                ui.label("Transforms:");
                if ui.button("+ Filter").clicked() {
                    self.add_pipeline_node("Filter");
                }
                if ui.button("+ Map").clicked() {
                    self.add_pipeline_node("Map");
                }
                if ui.button("+ Aggregate").clicked() {
                    self.add_pipeline_node("Aggregate");
                }
                if ui.button("+ Join").clicked() {
                    self.add_pipeline_node("Join");
                }

                ui.separator();

                // -- Add Sink nodes ------------------------------------------
                ui.label("Sinks:");
                if ui.button("+ Dashboard").clicked() {
                    self.add_pipeline_node("Dashboard");
                }
                if ui.button("+ Export CSV").clicked() {
                    self.add_pipeline_node("Export CSV");
                }
                if ui.button("+ Alert").clicked() {
                    self.add_pipeline_node("Alert");
                }

                ui.separator();

                let n = self.state.nodes.len();
                let e = self.state.edges.len();
                let z = self.state.viewport.zoom;
                ui.label(format!("{n} nodes  |  {e} edges  |  zoom {z:.2}"));
            });
        });

        // -- Side panel: connection counts -----------------------------------
        egui::SidePanel::right("connections")
            .resizable(true)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Connection Counts");
                ui.separator();

                let counts = self.connection_counts();

                // Group by category for readability.
                let mut sources: Vec<_> = Vec::new();
                let mut transforms: Vec<_> = Vec::new();
                let mut sinks: Vec<_> = Vec::new();

                for node in &self.state.nodes {
                    let label = &node.data;
                    let (inc, out) = counts.get(node.id.as_str()).copied().unwrap_or((0, 0));
                    let entry = (node.id.to_string(), label.clone(), inc, out);
                    match node_category(label) {
                        NodeCategory::Source => sources.push(entry),
                        NodeCategory::Transform => transforms.push(entry),
                        NodeCategory::Sink => sinks.push(entry),
                    }
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !sources.is_empty() {
                            ui.label(
                                egui::RichText::new("-- Sources --")
                                    .strong()
                                    .size(12.0),
                            );
                            for (_id, label, inc, out) in &sources {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  {label}:  in={inc}  out={out}"
                                    ))
                                    .monospace()
                                    .size(11.0),
                                );
                            }
                            ui.add_space(6.0);
                        }

                        if !transforms.is_empty() {
                            ui.label(
                                egui::RichText::new("-- Transforms --")
                                    .strong()
                                    .size(12.0),
                            );
                            for (_id, label, inc, out) in &transforms {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  {label}:  in={inc}  out={out}"
                                    ))
                                    .monospace()
                                    .size(11.0),
                                );
                            }
                            ui.add_space(6.0);
                        }

                        if !sinks.is_empty() {
                            ui.label(
                                egui::RichText::new("-- Sinks --")
                                    .strong()
                                    .size(12.0),
                            );
                            for (_id, label, inc, out) in &sinks {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  {label}:  in={inc}  out={out}"
                                    ))
                                    .monospace()
                                    .size(11.0),
                                );
                            }
                        }
                    });
            });

        // -- Main canvas -----------------------------------------------------
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(245, 245, 250)))
            .show(ctx, |ui| {
                let events = FlowCanvas::new(&mut self.state, &DefaultNodeWidget)
                    .connection_validator(&self.validator)
                    .show(ui);

                // When a new connection is made, auto-create the edge with
                // SmoothStep + arrow and update the validator label map.
                for conn in &events.connections_made {
                    // The canvas already added the edge internally; we just
                    // need to keep the validator up to date if new nodes were
                    // somehow added externally.  In practice the label map is
                    // already populated from add_pipeline_node, so this is a
                    // no-op -- but it keeps things robust.
                    let _ = &conn;
                }

                // If nodes were deleted, remove them from the validator map.
                for nid in &events.nodes_deleted {
                    self.validator.node_labels.remove(nid.as_str());
                }
            });
    }
}
