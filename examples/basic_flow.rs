//! Basic egui_xyflow example — updated to demonstrate all ported features.
//!
//! Nodes + Edges
//! ─────────────
//!   Input  →  Process  →  Output
//!                ↓
//!            Side Node
//!
//! Features demonstrated
//! ─────────────────────
//! • Multi-node drag (select several nodes with Shift-click or box-select,
//!   then drag any one of them — all selected nodes move together).
//! • Node resize (select exactly one node → 8 resize handles appear).
//! • Edge click-to-select (click an edge to select it; Delete removes it).
//! • Minimap click/drag to pan.
//! • FlowEvents — events returned by show() are logged in a side-panel.
//! • fit_bounds — frame a hand-crafted bounding box.
//! • fit_selected_nodes — zoom to the current selection.
//! • ConnectionMode toggle (Strict / Loose).
//! • Background variant toggle.
//!
//! Controls
//! ────────
//! • Scroll wheel / pinch         → zoom
//! • Double-click on background   → animated zoom in
//! • Left-drag on background      → pan
//! • Left-drag on node            → move node
//! • Shift + click node           → add/remove from selection
//! • Box-select on background     → multi-select
//! • Drag handle → handle         → create edge
//! • Click edge                   → select edge
//! • Select one node              → resize handles appear
//! • Click minimap                → pan to that region
//! • Ctrl/Cmd + A                 → select all
//! • Ctrl/Cmd + F                 → fit all nodes
//! • Ctrl/Cmd + Shift + F         → fit selected nodes
//! • Delete / Backspace           → delete selected nodes / edges

use eframe::egui;
use egui_xyflow::prelude::*;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow — feature showcase")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui_xyflow Example",
        options,
        Box::new(|_cc| Ok(Box::new(FlowApp::new()))),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Connection validator
// ─────────────────────────────────────────────────────────────────────────────

struct CapitalLetterValidator;

impl ConnectionValidator for CapitalLetterValidator {
    fn is_valid_connection(&self, _connection: &Connection, _existing_edges: &[EdgeInfo<'_>]) -> bool {
        // Always allow — the wiring is the point of the demo, not business rules.
        true
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// App state
// ─────────────────────────────────────────────────────────────────────────────

struct FlowApp {
    state: FlowState<String, ()>,
    validator: CapitalLetterValidator,

    // ── UI controls ──────────────────────────────────────────────────────────
    loose_connections: bool,

    // ── Event log (last N events shown in side panel) ────────────────────────
    event_log: Vec<String>,
}

impl FlowApp {
    fn new() -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            show_minimap: true,
            ..FlowConfig::default()
        };

        let mut state: FlowState<String, ()> = FlowState::new(config);

        // ── Nodes ────────────────────────────────────────────────────────────
        state.add_node(
            Node::builder("1")
                .position(egui::pos2(80.0, 160.0))
                .data("Input".to_string())
                .handle(NodeHandle::source(Position::Right))
                .size(130.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("2")
                .position(egui::pos2(340.0, 100.0))
                .data("Process".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .handle(NodeHandle::source(Position::Bottom))
                .size(140.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("3")
                .position(egui::pos2(610.0, 160.0))
                .data("Output".to_string())
                .handle(NodeHandle::target(Position::Left))
                .size(130.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("4")
                .position(egui::pos2(340.0, 300.0))
                .data("Side Node".to_string())
                .handle(NodeHandle::target(Position::Top))
                .handle(NodeHandle::source(Position::Bottom))
                .size(130.0, 44.0)
                .build(),
        );

        state.add_node(
            Node::builder("5")
                .position(egui::pos2(340.0, 470.0))
                .data("Leaf".to_string())
                .handle(NodeHandle::target(Position::Top))
                .size(130.0, 44.0)
                .build(),
        );

        // ── Edges ────────────────────────────────────────────────────────────
        state.add_edge(
            Edge::new("e1-2", "1", "2")
                .edge_type(EdgeType::Bezier)
                .animated(true)
                .marker_end_arrow(),
        );

        state.add_edge(
            Edge::new("e2-3", "2", "3")
                .edge_type(EdgeType::SmoothStep)
                .marker_end_arrow(),
        );

        state.add_edge(
            Edge::new("e2-4", "2", "4")
                .edge_type(EdgeType::Straight)
                .marker_end_arrow(),
        );

        state.add_edge(
            Edge::new("e4-5", "4", "5")
                .edge_type(EdgeType::Step)
                .marker_end_arrow(),
        );

        Self {
            state,
            validator: CapitalLetterValidator,
            loose_connections: false,
            event_log: Vec::new(),
        }
    }

    /// Push a message to the event log (capped at 60 entries).
    fn log(&mut self, msg: impl Into<String>) {
        if self.event_log.len() >= 60 {
            self.event_log.remove(0);
        }
        self.event_log.push(msg.into());
    }

    /// Process the FlowEvents returned by the canvas.
    fn handle_events(&mut self, events: FlowEvents) {
        for conn in &events.connections_made {
            self.log(format!("✅ connected {} → {}", conn.source, conn.target));
        }
        if let Some(ref nid) = events.connection_started {
            self.log(format!("🔗 drag started from {nid}"));
        }
        if events.connection_ended && events.connections_made.is_empty() {
            self.log("❌ connection cancelled".to_string());
        }
        for id in &events.nodes_drag_started {
            self.log(format!("▶ drag started: {id}"));
        }
        for (id, pos) in &events.nodes_dragged {
            self.log(format!("↔ dragging {id} → ({:.0}, {:.0})", pos.x, pos.y));
        }
        for id in &events.nodes_drag_stopped {
            self.log(format!("⏹ drag stopped: {id}"));
        }
        for (id, w, h) in &events.nodes_resized {
            self.log(format!("⇲ resized {id}: {w:.0}×{h:.0}"));
        }
        for id in &events.nodes_clicked {
            self.log(format!("🖱 node clicked: {id}"));
        }
        for id in &events.edges_clicked {
            self.log(format!("🖱 edge clicked: {id}"));
        }
        if events.selection_changed {
            let ns: Vec<_> = events
                .selected_nodes
                .iter()
                .map(|n| n.to_string())
                .collect();
            let es: Vec<_> = events
                .selected_edges
                .iter()
                .map(|e| e.to_string())
                .collect();
            self.log(format!(
                "◼ selection: nodes=[{}] edges=[{}]",
                ns.join(", "),
                es.join(", ")
            ));
        }
        if !events.nodes_deleted.is_empty() {
            let ids: Vec<_> = events.nodes_deleted.iter().map(|n| n.to_string()).collect();
            self.log(format!("🗑 deleted nodes: {}", ids.join(", ")));
        }
        if !events.edges_deleted.is_empty() {
            let ids: Vec<_> = events.edges_deleted.iter().map(|e| e.to_string()).collect();
            self.log(format!("🗑 deleted edges: {}", ids.join(", ")));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// eframe::App
// ─────────────────────────────────────────────────────────────────────────────

impl eframe::App for FlowApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Toolbar ───────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("egui_xyflow");
                ui.separator();

                // Viewport controls
                if ui.button("⟲  Fit All").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 40.0, t);
                }
                if ui.button("⊡  Fit Selected").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_selected_nodes(rect, 40.0, t);
                }
                if ui.button("⊕  Fit Bounds").clicked() {
                    // Fit a hand-crafted bounding box covering the left cluster.
                    use egui_xyflow::types::position::CoordinateExtent;
                    let bounds = CoordinateExtent {
                        min: egui::pos2(60.0, 80.0),
                        max: egui::pos2(500.0, 380.0),
                    };
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_bounds(bounds, rect, 40.0, t);
                    self.log("📐 fit_bounds called".to_string());
                }
                if ui.button("＋  Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("－  Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }
                if ui.button("↩  Reset").clicked() {
                    use egui_xyflow::animation::easing::ease_linear;
                    self.state.set_viewport(
                        Viewport {
                            x: 0.0,
                            y: 0.0,
                            zoom: 1.0,
                        },
                        0.4,
                        ease_linear,
                        ctx.input(|i| i.time),
                    );
                }

                ui.separator();

                // Background variant
                let bv = &mut self.state.config.background_variant;
                egui::ComboBox::from_label("BG")
                    .selected_text(format!("{bv:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(bv, BackgroundVariant::Dots, "Dots");
                        ui.selectable_value(bv, BackgroundVariant::Lines, "Lines");
                        ui.selectable_value(bv, BackgroundVariant::Cross, "Cross");
                    });

                ui.checkbox(&mut self.state.config.show_minimap, "Minimap");
                ui.checkbox(&mut self.state.config.snap_to_grid, "Snap");

                // ConnectionMode toggle
                let was_loose = self.loose_connections;
                ui.checkbox(&mut self.loose_connections, "Loose connections");
                if self.loose_connections != was_loose {
                    use egui_xyflow::types::connection::ConnectionMode;
                    self.state.config.connection_mode = if self.loose_connections {
                        ConnectionMode::Loose
                    } else {
                        ConnectionMode::Strict
                    };
                    self.log(format!(
                        "🔌 connection mode → {}",
                        if self.loose_connections {
                            "Loose"
                        } else {
                            "Strict"
                        }
                    ));
                }

                ui.separator();

                let n = self.state.nodes.len();
                let e = self.state.edges.len();
                let z = self.state.viewport.zoom;
                let sel_n = self.state.nodes.iter().filter(|n| n.selected).count();
                let sel_e = self.state.edges.iter().filter(|e| e.selected).count();
                ui.label(format!(
                    "{n} nodes  ·  {e} edges  ·  zoom {z:.2}  ·  sel {sel_n}N {sel_e}E"
                ));
            });
        });

        // ── Event log side panel ──────────────────────────────────────────────
        egui::SidePanel::right("events")
            .resizable(true)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Event log");
                if ui.small_button("Clear").clicked() {
                    self.event_log.clear();
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for msg in &self.event_log {
                            ui.label(egui::RichText::new(msg).monospace().size(11.0));
                        }
                    });
            });

        // ── Main canvas ───────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(245, 245, 250)))
            .show(ctx, |ui| {
                let events = FlowCanvas::new(&mut self.state, &DefaultNodeWidget)
                    .connection_validator(&self.validator)
                    .show(ui);

                // Process events (log interesting ones, skip viewport spam)
                if !events.is_empty() {
                    let mut filtered = events.clone();
                    // Don't spam the log with every viewport tick
                    filtered.viewport_changed = false;
                    // Don't log every single drag move — only start/stop
                    filtered.nodes_dragged.clear();
                    self.handle_events(filtered);
                }
            });
    }
}
