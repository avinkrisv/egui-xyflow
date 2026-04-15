//! Demonstrates the three issue fixes landed together:
//!
//! • `Edge::builder(...)` — alias for `Edge::new(...)` (#9).
//! • `.label(text)` on edges — centred label using the path's `label_pos` (#7).
//! • Viewport culling — a large grid of SmoothStep edges stays smooth under
//!   pan/zoom because off-screen edges are skipped each frame (#8). Toggle
//!   `FlowConfig::cull_offscreen_edges` from the toolbar to feel the
//!   difference.
//!
//! Run: `cargo run --example edge_labels`

use eframe::egui;
use egui_xyflow::prelude::*;

const GRID_COLS: usize = 12;
const GRID_ROWS: usize = 10;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow — edge labels + culling demo")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "edge_labels",
        options,
        Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
    )
}

struct DemoApp {
    state: FlowState<String, ()>,
}

impl DemoApp {
    fn new() -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            default_edge_type: EdgeType::SmoothStep,
            cull_offscreen_edges: true,
            ..FlowConfig::default()
        };
        let mut state: FlowState<String, ()> = FlowState::new(config);

        // ── Small labelled subgraph at the top-left ──────────────────────────
        state.add_node(
            Node::builder("in")
                .position(egui::pos2(40.0, 40.0))
                .data("Ingest".to_string())
                .handle(NodeHandle::source(Position::Right))
                .size(130.0, 44.0)
                .build(),
        );
        state.add_node(
            Node::builder("val")
                .position(egui::pos2(260.0, 40.0))
                .data("Validate".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .handle(NodeHandle::source(Position::Bottom))
                .size(130.0, 44.0)
                .build(),
        );
        state.add_node(
            Node::builder("ok")
                .position(egui::pos2(500.0, 0.0))
                .data("Accept".to_string())
                .handle(NodeHandle::target(Position::Left))
                .size(130.0, 44.0)
                .build(),
        );
        state.add_node(
            Node::builder("rej")
                .position(egui::pos2(260.0, 160.0))
                .data("Reject".to_string())
                .handle(NodeHandle::target(Position::Top))
                .size(130.0, 44.0)
                .build(),
        );

        // Edge::builder (#9) + .label (#7)
        state.add_edge(
            Edge::builder("ingest->validate", "in", "val")
                .edge_type(EdgeType::SmoothStep)
                .label("1,284 rows/s")
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::builder("validate->ok", "val", "ok")
                .edge_type(EdgeType::Bezier)
                .animated(true)
                .label("p = 0.97")
                .color(egui::Color32::from_rgb(34, 197, 94))
                .marker_end_arrow(),
        );
        state.add_edge(
            Edge::builder("validate->rej", "val", "rej")
                .edge_type(EdgeType::Straight)
                .label("p = 0.03")
                .color(egui::Color32::from_rgb(239, 68, 68))
                .marker_end_arrow(),
        );

        // ── Big grid of SmoothStep edges to show off culling (#8) ────────────
        //
        // GRID_COLS × GRID_ROWS lattice with right + down edges per cell.
        let x0 = 40.0_f32;
        let y0 = 320.0_f32;
        let dx = 180.0_f32;
        let dy = 110.0_f32;
        for r in 0..GRID_ROWS {
            for c in 0..GRID_COLS {
                let id = format!("g{r}-{c}");
                state.add_node(
                    Node::builder(id.clone())
                        .position(egui::pos2(x0 + c as f32 * dx, y0 + r as f32 * dy))
                        .data(format!("{r},{c}"))
                        .handle(NodeHandle::source(Position::Right))
                        .handle(NodeHandle::source(Position::Bottom))
                        .handle(NodeHandle::target(Position::Left))
                        .handle(NodeHandle::target(Position::Top))
                        .size(120.0, 36.0)
                        .build(),
                );
            }
        }
        for r in 0..GRID_ROWS {
            for c in 0..GRID_COLS {
                if c + 1 < GRID_COLS {
                    state.add_edge(
                        Edge::builder(
                            format!("eh-{r}-{c}"),
                            format!("g{r}-{c}"),
                            format!("g{r}-{}", c + 1),
                        )
                        .edge_type(EdgeType::SmoothStep),
                    );
                }
                if r + 1 < GRID_ROWS {
                    state.add_edge(
                        Edge::builder(
                            format!("ev-{r}-{c}"),
                            format!("g{r}-{c}"),
                            format!("g{}-{c}", r + 1),
                        )
                        .edge_type(EdgeType::SmoothStep),
                    );
                }
            }
        }

        Self { state }
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("edge labels + culling");
                ui.separator();
                if ui.button("Fit all").clicked() {
                    let rect = ctx.screen_rect();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 40.0, t);
                }
                ui.separator();
                ui.checkbox(
                    &mut self.state.config.cull_offscreen_edges,
                    "Cull off-screen edges",
                );
                ui.separator();
                let n = self.state.nodes.len();
                let e = self.state.edges.len();
                let z = self.state.viewport.zoom;
                ui.label(format!("{n} nodes · {e} edges · zoom {z:.2}"));
                ui.separator();
                ui.label(
                    egui::RichText::new("Pan/zoom around the grid — toggle culling to feel the cost.")
                        .small()
                        .italics(),
                );
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(245, 245, 250)))
            .show(ctx, |ui| {
                FlowCanvas::new(&mut self.state, &DefaultNodeWidget).show(ui);
            });
    }
}
