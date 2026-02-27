//! Stress test & benchmark harness for egui_xyflow.
//!
//! Two modes
//! ─────────
//! **Interactive** (default):
//!   `cargo run --example stress_test`
//!   Adjust node count, toggle auto-drag, watch FPS.
//!   Click "Run Benchmark" to execute the automated suite in-app.
//!
//! **Automated benchmark** (CI / pre-commit):
//!   `cargo run --example stress_test -- --bench`
//!   Runs through all scenarios, writes a JSON report to
//!   `benchmarks/v{version}_{timestamp}.json`, prints a comparison
//!   against the most recent previous report, then exits.

use eframe::egui;
use egui_xyflow::prelude::*;
use std::collections::BTreeMap;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
const WARMUP_FRAMES: usize = 60;
const MEASURE_FRAMES: usize = 180;

/// Predefined scenarios for the automated benchmark.
const SCENARIOS: &[ScenarioDef] = &[
    ScenarioDef { nodes: 100,   mode: ScenarioMode::Idle },
    ScenarioDef { nodes: 500,   mode: ScenarioMode::Idle },
    ScenarioDef { nodes: 1000,  mode: ScenarioMode::Idle },
    ScenarioDef { nodes: 2000,  mode: ScenarioMode::Idle },
    ScenarioDef { nodes: 10000, mode: ScenarioMode::Idle },
    ScenarioDef { nodes: 100,   mode: ScenarioMode::Drag },
    ScenarioDef { nodes: 500,   mode: ScenarioMode::Drag },
    ScenarioDef { nodes: 1000,  mode: ScenarioMode::Drag },
    ScenarioDef { nodes: 2000,  mode: ScenarioMode::Drag },
    ScenarioDef { nodes: 10000, mode: ScenarioMode::Drag },
    ScenarioDef { nodes: 500,   mode: ScenarioMode::AllSelected },
    ScenarioDef { nodes: 500,   mode: ScenarioMode::Glow },
    ScenarioDef { nodes: 1000,  mode: ScenarioMode::Glow },
    ScenarioDef { nodes: 2000,  mode: ScenarioMode::Glow },
];

// ─────────────────────────────────────────────────────────────────────────────
// Scenario types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ScenarioMode {
    Idle,
    Drag,
    AllSelected,
    Glow,
}

impl ScenarioMode {
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Drag => "drag",
            Self::AllSelected => "all_selected",
            Self::Glow => "glow",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScenarioDef {
    nodes: usize,
    mode: ScenarioMode,
}

impl ScenarioDef {
    fn name(self) -> String {
        format!("{}_nodes_{}", self.nodes, self.mode.label())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark result types (serde for JSON report)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchReport {
    version: String,
    timestamp: String,
    warmup_frames: usize,
    measure_frames: usize,
    scenarios: Vec<ScenarioResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ScenarioResult {
    name: String,
    nodes: usize,
    edges: usize,
    mode: String,
    avg_ms: f64,
    median_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

impl ScenarioResult {
    fn from_times(name: String, nodes: usize, edges: usize, mode: &str, times: &[f64]) -> Self {
        let mut sorted: Vec<f64> = times.iter().map(|t| t * 1000.0).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        Self {
            name,
            nodes,
            edges,
            mode: mode.to_string(),
            avg_ms: sorted.iter().sum::<f64>() / n as f64,
            median_ms: sorted[n / 2],
            p95_ms: sorted[(n as f64 * 0.95) as usize],
            p99_ms: sorted[(n as f64 * 0.99).min((n - 1) as f64) as usize],
            min_ms: sorted[0],
            max_ms: sorted[n - 1],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark state machine
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum BenchPhase {
    /// Not running (interactive mode).
    Inactive,
    /// Building the graph for this scenario index.
    Building { idx: usize },
    /// Discarding warmup frames.
    WarmingUp { idx: usize, remaining: usize },
    /// Recording measurement frames.
    Measuring { idx: usize, remaining: usize, times: Vec<f64> },
    /// All scenarios complete — report written.
    Done { report: BenchReport },
}

// ─────────────────────────────────────────────────────────────────────────────
// App
// ─────────────────────────────────────────────────────────────────────────────

struct StressApp {
    state: FlowState<String, ()>,
    target_node_count: usize,
    current_node_count: usize,
    current_edge_count: usize,

    // Interactive profiling
    frame_times: Vec<f64>,
    last_time: f64,

    // Interactive toggles
    auto_drag: bool,
    auto_drag_frame: u64,
    all_selected: bool,

    // Benchmark harness
    bench_phase: BenchPhase,
    bench_results: Vec<ScenarioResult>,
    auto_exit: bool,

    // Report comparison
    previous_report: Option<BenchReport>,
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let auto_bench = std::env::args().any(|a| a == "--bench");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow — stress test")
            .with_inner_size([1400.0, 900.0]),
        vsync: false, // Uncap frame rate for accurate profiling
        ..Default::default()
    };

    eframe::run_native(
        "egui_xyflow Stress Test",
        options,
        Box::new(move |_cc| Ok(Box::new(StressApp::new(200, auto_bench)))),
    )
}

impl StressApp {
    fn new(initial_nodes: usize, auto_bench: bool) -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            show_minimap: false,
            ..FlowConfig::default()
        };

        let previous_report = load_latest_report();

        let mut app = Self {
            state: FlowState::new(config),
            target_node_count: initial_nodes,
            current_node_count: 0,
            current_edge_count: 0,
            frame_times: Vec::with_capacity(120),
            last_time: 0.0,
            auto_drag: false,
            auto_drag_frame: 0,
            all_selected: false,
            bench_phase: if auto_bench { BenchPhase::Building { idx: 0 } } else { BenchPhase::Inactive },
            bench_results: Vec::with_capacity(SCENARIOS.len()),
            auto_exit: auto_bench,
            previous_report,
        };
        app.rebuild_graph();
        app
    }

    fn rebuild_graph(&mut self) {
        self.rebuild_graph_with_glow(false);
    }

    fn rebuild_graph_with_glow(&mut self, glow: bool) {
        let n = self.target_node_count.max(1);
        let cols = (n as f32).sqrt().ceil() as usize;
        let rows = n.div_ceil(cols);
        let spacing_x = 220.0_f32;
        let spacing_y = 100.0_f32;

        let config = self.state.config.clone();
        self.state = FlowState::new(config);
        let mut edge_count = 0usize;

        let glow_style = EdgeStyle {
            color: Some(egui::Color32::from_rgb(59, 130, 246)),
            glow: Some(EdgeGlow::new(
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 60),
                12.0,
            )),
            ..EdgeStyle::default()
        };

        for i in 0..n {
            let row = i / cols;
            let col = i % cols;
            let x = col as f32 * spacing_x;
            let y = row as f32 * spacing_y;

            self.state.add_node(
                Node::builder(format!("n{i}"))
                    .position(egui::pos2(x, y))
                    .data(format!("Node {i}"))
                    .handle(NodeHandle::source(Position::Right))
                    .handle(NodeHandle::target(Position::Left))
                    .handle(NodeHandle::source(Position::Bottom))
                    .handle(NodeHandle::target(Position::Top))
                    .size(150.0, 40.0)
                    .build(),
            );

            if col + 1 < cols && i + 1 < n {
                let target = i + 1;
                let mut edge = Edge::new(format!("eh{i}_{target}"), format!("n{i}"), format!("n{target}"))
                    .edge_type(EdgeType::Bezier)
                    .marker_end_arrow();
                if glow { edge = edge.style(glow_style); }
                self.state.add_edge(edge);
                edge_count += 1;
            }

            let below = i + cols;
            if row + 1 < rows && below < n {
                let mut edge = Edge::new(format!("ev{i}_{below}"), format!("n{i}"), format!("n{below}"))
                    .edge_type(EdgeType::SmoothStep)
                    .marker_end_arrow();
                if glow { edge = edge.style(glow_style); }
                self.state.add_edge(edge);
                edge_count += 1;
            }
        }

        self.current_node_count = n;
        self.current_edge_count = edge_count;
        self.all_selected = false;
    }

    fn build_scenario_graph(&mut self, def: ScenarioDef) {
        self.target_node_count = def.nodes;
        self.rebuild_graph_with_glow(def.mode == ScenarioMode::Glow);

        if def.mode == ScenarioMode::AllSelected {
            let changes: Vec<_> = self.state.nodes.iter().map(|n| {
                NodeChange::Select { id: n.id.clone(), selected: true }
            }).collect();
            self.state.apply_node_changes(&changes);
            self.all_selected = true;
        }
    }

    fn apply_drag_step(&mut self) {
        self.auto_drag_frame += 1;
        let count = (self.current_node_count / 10).max(1);
        let phase = self.auto_drag_frame as f32 * 0.05;
        let mut changes = Vec::with_capacity(count);
        for i in 0..count {
            let offset_x = (phase + i as f32 * 0.3).sin() * 2.0;
            let offset_y = (phase + i as f32 * 0.3).cos() * 2.0;
            if let Some(node) = self.state.nodes.get(i) {
                changes.push(NodeChange::Position {
                    id: node.id.clone(),
                    position: Some(egui::pos2(node.position.x + offset_x, node.position.y + offset_y)),
                    dragging: Some(true),
                });
            }
        }
        if !changes.is_empty() {
            self.state.apply_node_changes(&changes);
        }
    }

    fn record_frame_time(&mut self, time: f64) {
        if self.last_time > 0.0 {
            let dt = time - self.last_time;
            if self.frame_times.len() >= 120 {
                self.frame_times.remove(0);
            }
            self.frame_times.push(dt);
        }
        self.last_time = time;
    }

    fn avg_frame_time_ms(&self) -> f64 {
        if self.frame_times.is_empty() { return 0.0; }
        self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64 * 1000.0
    }

    fn fps(&self) -> f64 {
        let avg = self.avg_frame_time_ms();
        if avg > 0.0 { 1000.0 / avg } else { 0.0 }
    }

    fn worst_frame_time_ms(&self) -> f64 {
        self.frame_times.iter().copied().fold(0.0_f64, f64::max) * 1000.0
    }

    fn start_benchmark(&mut self) {
        self.bench_results.clear();
        self.bench_phase = BenchPhase::Building { idx: 0 };
    }

    fn finalize_benchmark(&mut self) -> BenchReport {
        let timestamp = chrono_timestamp();
        let report = BenchReport {
            version: CRATE_VERSION.to_string(),
            timestamp,
            warmup_frames: WARMUP_FRAMES,
            measure_frames: MEASURE_FRAMES,
            scenarios: self.bench_results.clone(),
        };

        // Write report to disk
        if let Err(e) = write_report(&report) {
            eprintln!("Failed to write benchmark report: {e}");
        }

        // Print to stdout
        print_report(&report, self.previous_report.as_ref());

        report
    }

    /// Advance the benchmark state machine by one frame.
    /// Returns true if a drag step should be applied.
    fn tick_bench(&mut self, frame_dt: f64) -> bool {
        let mut needs_drag = false;

        let next_phase = match &mut self.bench_phase {
            BenchPhase::Inactive | BenchPhase::Done { .. } => None,

            BenchPhase::Building { idx } => {
                let idx = *idx;
                if idx < SCENARIOS.len() {
                    self.build_scenario_graph(SCENARIOS[idx]);
                    self.auto_drag_frame = 0;
                    self.last_time = 0.0; // reset dt tracking
                    Some(BenchPhase::WarmingUp { idx, remaining: WARMUP_FRAMES })
                } else {
                    let report = self.finalize_benchmark();
                    Some(BenchPhase::Done { report })
                }
            }

            BenchPhase::WarmingUp { idx, remaining } => {
                let idx = *idx;
                needs_drag = SCENARIOS[idx].mode == ScenarioMode::Drag;

                if *remaining == 0 {
                    Some(BenchPhase::Measuring {
                        idx,
                        remaining: MEASURE_FRAMES,
                        times: Vec::with_capacity(MEASURE_FRAMES),
                    })
                } else {
                    *remaining -= 1;
                    None
                }
            }

            BenchPhase::Measuring { idx, remaining, times } => {
                let idx = *idx;
                let def = SCENARIOS[idx];
                needs_drag = def.mode == ScenarioMode::Drag;

                if frame_dt > 0.0 {
                    times.push(frame_dt);
                }

                if *remaining == 0 {
                    // Scenario complete — collect results
                    let result = ScenarioResult::from_times(
                        def.name(),
                        self.current_node_count,
                        self.current_edge_count,
                        def.mode.label(),
                        times,
                    );
                    eprintln!(
                        "  [{}/{}] {:30} avg={:.2}ms  p95={:.2}ms  max={:.2}ms",
                        idx + 1, SCENARIOS.len(), result.name,
                        result.avg_ms, result.p95_ms, result.max_ms,
                    );
                    self.bench_results.push(result);
                    Some(BenchPhase::Building { idx: idx + 1 })
                } else {
                    *remaining -= 1;
                    None
                }
            }
        };

        if let Some(phase) = next_phase {
            self.bench_phase = phase;
        }

        needs_drag
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// eframe::App
// ─────────────────────────────────────────────────────────────────────────────

impl eframe::App for StressApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = ctx.input(|i| i.time);
        let frame_dt = if self.last_time > 0.0 { time - self.last_time } else { 0.0 };
        self.record_frame_time(time);
        ctx.request_repaint();

        // ── Benchmark state machine ─────────────────────────────────────────
        let bench_active = !matches!(self.bench_phase, BenchPhase::Inactive | BenchPhase::Done { .. });
        let bench_drag = if bench_active {
            self.tick_bench(frame_dt)
        } else {
            false
        };

        // Auto-exit after benchmark completes
        if self.auto_exit {
            if let BenchPhase::Done { .. } = &self.bench_phase {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        // ── Interactive auto-drag ───────────────────────────────────────────
        if bench_drag || (self.auto_drag && !bench_active) {
            self.apply_drag_step();
        }

        // ── Top panel ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Stress Test");
                ui.separator();

                // Benchmark controls
                let bench_running = bench_active;
                ui.add_enabled_ui(!bench_running, |ui| {
                    if ui.button("Run Benchmark").clicked() {
                        self.start_benchmark();
                    }
                });

                if bench_running {
                    let (idx, phase_label) = match &self.bench_phase {
                        BenchPhase::Building { idx } => (*idx, "building"),
                        BenchPhase::WarmingUp { idx, .. } => (*idx, "warmup"),
                        BenchPhase::Measuring { idx, .. } => (*idx, "measuring"),
                        _ => (0, ""),
                    };
                    let total = SCENARIOS.len();
                    let name = if idx < total { SCENARIOS[idx].name() } else { "done".into() };
                    ui.colored_label(
                        egui::Color32::from_rgb(59, 130, 246),
                        format!("BENCH [{}/{}] {} — {}", (idx + 1).min(total), total, name, phase_label),
                    );
                }

                if let BenchPhase::Done { report } = &self.bench_phase {
                    ui.colored_label(
                        egui::Color32::from_rgb(34, 197, 94),
                        format!("Benchmark complete — {} scenarios", report.scenarios.len()),
                    );
                }

                ui.separator();

                // Interactive controls (only when benchmark not running)
                if !bench_running {
                    ui.label("Nodes:");
                    ui.add(egui::Slider::new(&mut self.target_node_count, 10..=10000).logarithmic(true));
                    if ui.button("Rebuild").clicked() {
                        self.rebuild_graph();
                        self.frame_times.clear();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.auto_drag, "Auto-Drag");
                    if ui.button(if self.all_selected { "Deselect All" } else { "Select All" }).clicked() {
                        self.all_selected = !self.all_selected;
                        let changes: Vec<_> = self.state.nodes.iter().map(|n| {
                            NodeChange::Select { id: n.id.clone(), selected: self.all_selected }
                        }).collect();
                        self.state.apply_node_changes(&changes);
                    }
                    if ui.button("Fit All").clicked() {
                        self.state.fit_view(ctx.screen_rect(), 40.0, time);
                    }
                    ui.separator();
                }

                // Metrics
                let fps = self.fps();
                let avg_ms = self.avg_frame_time_ms();
                let worst_ms = self.worst_frame_time_ms();
                let fps_color = if fps >= 55.0 {
                    egui::Color32::from_rgb(34, 197, 94)
                } else if fps >= 30.0 {
                    egui::Color32::from_rgb(234, 179, 8)
                } else {
                    egui::Color32::from_rgb(239, 68, 68)
                };
                ui.colored_label(fps_color, format!(
                    "{:.0} FPS | avg {:.1}ms | worst {:.1}ms | {} nodes {} edges",
                    fps, avg_ms, worst_ms, self.current_node_count, self.current_edge_count,
                ));
            });
        });

        // ── Bottom panel: sparkline + report table ──────────────────────────
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .min_height(60.0)
            .default_height(if matches!(self.bench_phase, BenchPhase::Done { .. }) { 260.0 } else { 60.0 })
            .show(ctx, |ui| {
                // Show report table if benchmark is done
                if let BenchPhase::Done { report } = &self.bench_phase {
                    ui.heading("Benchmark Report");
                    ui.label(format!("v{} — {}", report.version, report.timestamp));
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("report_grid")
                            .striped(true)
                            .min_col_width(80.0)
                            .show(ui, |ui| {
                                // Header
                                ui.strong("Scenario");
                                ui.strong("Nodes");
                                ui.strong("Edges");
                                ui.strong("Mode");
                                ui.strong("Avg (ms)");
                                ui.strong("Median");
                                ui.strong("P95");
                                ui.strong("P99");
                                ui.strong("Min");
                                ui.strong("Max");
                                if self.previous_report.is_some() {
                                    ui.strong("Delta");
                                }
                                ui.end_row();

                                // Rows
                                let prev_map: BTreeMap<String, &ScenarioResult> = self.previous_report
                                    .as_ref()
                                    .map(|r| r.scenarios.iter().map(|s| (s.name.clone(), s)).collect())
                                    .unwrap_or_default();

                                for s in &report.scenarios {
                                    ui.label(&s.name);
                                    ui.label(format!("{}", s.nodes));
                                    ui.label(format!("{}", s.edges));
                                    ui.label(&s.mode);
                                    ui.label(format!("{:.2}", s.avg_ms));
                                    ui.label(format!("{:.2}", s.median_ms));
                                    ui.label(format!("{:.2}", s.p95_ms));
                                    ui.label(format!("{:.2}", s.p99_ms));
                                    ui.label(format!("{:.2}", s.min_ms));
                                    ui.label(format!("{:.2}", s.max_ms));
                                    if let Some(prev) = prev_map.get(&s.name) {
                                        let delta_pct = (s.avg_ms - prev.avg_ms) / prev.avg_ms * 100.0;
                                        let color = if delta_pct < -5.0 {
                                            egui::Color32::from_rgb(34, 197, 94)  // faster
                                        } else if delta_pct > 5.0 {
                                            egui::Color32::from_rgb(239, 68, 68)  // slower
                                        } else {
                                            egui::Color32::GRAY
                                        };
                                        let sign = if delta_pct > 0.0 { "+" } else { "" };
                                        ui.colored_label(color, format!("{sign}{delta_pct:.1}%"));
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                } else {
                    // Sparkline
                    ui.horizontal(|ui| {
                        ui.label("Frame time (ms):");
                        let height = 30.0;
                        let width = ui.available_width();
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(width, height),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);
                        draw_sparkline(&painter, rect, &self.frame_times);
                    });
                }
            });

        // ── Canvas ──────────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(30, 30, 35)))
            .show(ctx, |ui| {
                let _events = FlowCanvas::new(&mut self.state, &DefaultNodeWidget).show(ui);
            });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sparkline drawing
// ─────────────────────────────────────────────────────────────────────────────

fn draw_sparkline(painter: &egui::Painter, rect: egui::Rect, frame_times: &[f64]) {
    if frame_times.len() < 2 {
        return;
    }
    let max_ms = frame_times.iter().copied().fold(0.0_f64, f64::max) * 1000.0;
    let max_ms = max_ms.max(16.7) as f32;
    let height = rect.height();
    let width = rect.width();
    let n = frame_times.len();
    let dx = width / n as f32;

    // 60fps reference line
    let y_60 = rect.max.y - (16.67 / max_ms) * height;
    painter.line_segment(
        [egui::pos2(rect.min.x, y_60), egui::pos2(rect.max.x, y_60)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
    );
    painter.text(
        egui::pos2(rect.min.x + 2.0, y_60 - 10.0),
        egui::Align2::LEFT_BOTTOM,
        "60fps",
        egui::FontId::monospace(9.0),
        egui::Color32::GRAY,
    );

    for (i, &dt) in frame_times.iter().enumerate() {
        let ms = (dt * 1000.0) as f32;
        let bar_h = (ms / max_ms) * height;
        let x = rect.min.x + i as f32 * dx;
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(x, rect.max.y - bar_h),
            egui::pos2(x + dx.max(1.0), rect.max.y),
        );
        let color = if ms <= 16.67 {
            egui::Color32::from_rgb(34, 197, 94)
        } else if ms <= 33.33 {
            egui::Color32::from_rgb(234, 179, 8)
        } else {
            egui::Color32::from_rgb(239, 68, 68)
        };
        painter.rect_filled(bar_rect, 0.0, color);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Report I/O
// ─────────────────────────────────────────────────────────────────────────────

fn benchmarks_dir() -> PathBuf {
    // Walk up from the executable to find the project root (contains Cargo.toml).
    let mut dir = std::env::current_dir().unwrap_or_default();
    for _ in 0..10 {
        if dir.join("Cargo.toml").exists() {
            return dir.join("benchmarks");
        }
        if !dir.pop() {
            break;
        }
    }
    // Fallback: cwd/benchmarks
    std::env::current_dir().unwrap_or_default().join("benchmarks")
}

fn chrono_timestamp() -> String {
    // Simple ISO-ish timestamp without pulling in the chrono crate.
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert to readable format: we'll just use the unix timestamp and a
    // rough conversion (good enough for filenames, not timezone-aware).
    let s = secs;
    let days = s / 86400;
    let time_of_day = s % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since 1970-01-01 → approximate date.
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Simplified Gregorian calendar conversion.
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn write_report(report: &BenchReport) -> std::io::Result<()> {
    let dir = benchmarks_dir();
    std::fs::create_dir_all(&dir)?;

    let safe_ts = report.timestamp.replace([':', '-'], "");
    let filename = format!("v{}_{}.json", report.version, safe_ts);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(report)
        .map_err(std::io::Error::other)?;
    let mut f = std::fs::File::create(&path)?;
    f.write_all(json.as_bytes())?;

    eprintln!("\nReport written to: {}", path.display());
    Ok(())
}

fn load_latest_report() -> Option<BenchReport> {
    let dir = benchmarks_dir();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    // Sort by filename (timestamps sort lexicographically).
    entries.sort_by_key(|e| e.file_name());

    let latest = entries.last()?;
    let data = std::fs::read_to_string(latest.path()).ok()?;
    let report: BenchReport = serde_json::from_str(&data).ok()?;
    eprintln!("Loaded previous report: {} (v{})", latest.file_name().to_string_lossy(), report.version);
    Some(report)
}

/// Load all reports from the benchmarks directory, sorted by timestamp.
fn load_all_reports(dir: &Path) -> Vec<BenchReport> {
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect(),
        Err(_) => return Vec::new(),
    };
    entries.sort_by_key(|e| e.file_name());
    entries
        .iter()
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            serde_json::from_str(&data).ok()
        })
        .collect()
}

fn print_report(report: &BenchReport, previous: Option<&BenchReport>) {
    eprintln!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    eprintln!("║  egui_xyflow benchmark report — v{:<10} {}  ║", report.version, &report.timestamp[..19]);
    eprintln!("║  warmup: {} frames   measure: {} frames                                ║", report.warmup_frames, report.measure_frames);
    eprintln!("╠══════════════════════════════════════════════════════════════════════════╣");
    eprintln!("║ {:30} {:>7} {:>7} {:>7} {:>7} {:>8} ║", "Scenario", "Avg", "Median", "P95", "Max", "Delta");
    eprintln!("╠══════════════════════════════════════════════════════════════════════════╣");

    let prev_map: BTreeMap<String, &ScenarioResult> = previous
        .map(|r| r.scenarios.iter().map(|s| (s.name.clone(), s)).collect())
        .unwrap_or_default();

    for s in &report.scenarios {
        let delta = if let Some(prev) = prev_map.get(&s.name) {
            let pct = (s.avg_ms - prev.avg_ms) / prev.avg_ms * 100.0;
            let sign = if pct > 0.0 { "+" } else { "" };
            format!("{sign}{pct:.1}%")
        } else {
            "—".to_string()
        };
        eprintln!(
            "║ {:30} {:>6.2}ms {:>6.2}ms {:>6.2}ms {:>6.2}ms {:>8} ║",
            s.name, s.avg_ms, s.median_ms, s.p95_ms, s.max_ms, delta,
        );
    }
    eprintln!("╚══════════════════════════════════════════════════════════════════════════╝");

    // Also update history.md
    if let Err(e) = update_history_md(report, previous) {
        eprintln!("Warning: could not update history.md: {e}");
    }
}

fn update_history_md(report: &BenchReport, _previous: Option<&BenchReport>) -> std::io::Result<()> {
    let dir = benchmarks_dir();
    let path = dir.join("history.md");

    let all_reports = load_all_reports(&dir);

    let mut md = String::new();
    md.push_str("# Benchmark History\n\n");
    md.push_str("Auto-generated by `cargo run --example stress_test -- --bench`.\n\n");

    // Build a table per scenario across all versions
    let mut scenario_names: Vec<String> = Vec::new();
    for r in &all_reports {
        for s in &r.scenarios {
            if !scenario_names.contains(&s.name) {
                scenario_names.push(s.name.clone());
            }
        }
    }
    // Also include current report if not already in all_reports
    for s in &report.scenarios {
        if !scenario_names.contains(&s.name) {
            scenario_names.push(s.name.clone());
        }
    }

    // Merge all_reports with current report (in case it hasn't been flushed to disk yet)
    let mut combined_reports = all_reports;
    // Only add current if its timestamp differs from the last
    let already_included = combined_reports.last().map(|r| r.timestamp == report.timestamp).unwrap_or(false);
    if !already_included {
        combined_reports.push(report.clone());
    }

    for scenario_name in &scenario_names {
        md.push_str(&format!("## {scenario_name}\n\n"));
        md.push_str("| Version | Timestamp | Avg (ms) | Median | P95 | P99 | Max |\n");
        md.push_str("|---------|-----------|----------|--------|-----|-----|-----|\n");

        for r in &combined_reports {
            if let Some(s) = r.scenarios.iter().find(|s| &s.name == scenario_name) {
                md.push_str(&format!(
                    "| v{} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                    r.version,
                    &r.timestamp[..19],
                    s.avg_ms, s.median_ms, s.p95_ms, s.p99_ms, s.max_ms,
                ));
            }
        }
        md.push('\n');
    }

    std::fs::write(&path, md)?;
    eprintln!("Updated: {}", path.display());
    Ok(())
}
