//! Temporal force-directed graph example for egui_xyflow.
//!
//! A network of conference attendees evolves over time. Nodes and edges
//! appear/disappear based on a time scrubber while a force simulation
//! (charge repulsion, link springs, centering) continuously repositions
//! nodes. Drag nodes to pin them in place.
//!
//! Inspired by D3's temporal force-directed graph.
//!
//! Run with: `cargo run --example temporal_force_graph`

use std::collections::{HashMap, HashSet};

use eframe::egui;
use egui_xyflow::prelude::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NUM_ATTENDEES: usize = 55;
const NUM_INTERACTIONS: usize = 180;
const NUM_GROUPS: usize = 5;
const NODE_DIAMETER: f32 = 20.0;

// Force simulation
const CHARGE_STRENGTH: f32 = -150.0;
const CHARGE_MIN_DIST: f32 = 20.0;
const CHARGE_MAX_DIST: f32 = 400.0;
const LINK_STRENGTH: f32 = 0.3;
const LINK_DISTANCE: f32 = 80.0;
const CENTER_STRENGTH: f32 = 0.03;
const VELOCITY_DECAY: f32 = 0.4;
const ALPHA_INITIAL: f32 = 1.0;
const ALPHA_DECAY: f32 = 0.02;
const ALPHA_MIN: f32 = 0.001;

// Time
const DEFAULT_PLAY_SPEED: f32 = 0.03;
const CLOCK_START: f32 = 9.0;
const CLOCK_END: f32 = 18.0;

// ---------------------------------------------------------------------------
// Simple deterministic RNG (xorshift64)
// ---------------------------------------------------------------------------

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() & 0x00FF_FFFF) as f32 / 0x0100_0000_u32 as f32
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }
}

// ---------------------------------------------------------------------------
// Groups (D3 category10 palette)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
enum Group {
    #[default]
    Researchers,
    Engineers,
    Designers,
    Managers,
    Students,
}


impl Group {
    const ALL: [Group; NUM_GROUPS] = [
        Self::Researchers,
        Self::Engineers,
        Self::Designers,
        Self::Managers,
        Self::Students,
    ];

    fn color(self) -> egui::Color32 {
        match self {
            Self::Researchers => egui::Color32::from_rgb(31, 119, 180),
            Self::Engineers => egui::Color32::from_rgb(255, 127, 14),
            Self::Designers => egui::Color32::from_rgb(44, 160, 44),
            Self::Managers => egui::Color32::from_rgb(214, 39, 40),
            Self::Students => egui::Color32::from_rgb(148, 103, 189),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Researchers => "Researchers",
            Self::Engineers => "Engineers",
            Self::Designers => "Designers",
            Self::Managers => "Managers",
            Self::Students => "Students",
        }
    }

    fn from_index(i: usize) -> Self {
        Self::ALL[i % NUM_GROUPS]
    }
}

// ---------------------------------------------------------------------------
// Source data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AttendeeData {
    id: usize,
    group: Group,
    start_time: f32,
    end_time: f32,
}

#[derive(Debug, Clone)]
struct InteractionData {
    source: usize,
    target: usize,
    start_time: f32,
    end_time: f32,
}

/// Node data stored inside `Node<NodeData>`.
#[derive(Debug, Clone, Default)]
struct NodeData {
    group: Group,
}

// ---------------------------------------------------------------------------
// Synthetic data generation
// ---------------------------------------------------------------------------

fn generate_conference_data(seed: u64) -> (Vec<AttendeeData>, Vec<InteractionData>) {
    let mut rng = SimpleRng::new(seed);

    // Attendees with staggered arrival/departure
    let mut attendees = Vec::with_capacity(NUM_ATTENDEES);
    for i in 0..NUM_ATTENDEES {
        let group = Group::from_index(i * NUM_GROUPS / NUM_ATTENDEES);
        let base_start = (group as usize as f32) * 0.04;
        let start = (base_start + rng.next_f32() * 0.3).clamp(0.0, 0.6);
        let duration = 0.25 + rng.next_f32() * 0.5;
        let end = (start + duration).min(1.0);
        attendees.push(AttendeeData {
            id: i,
            group,
            start_time: start,
            end_time: end,
        });
    }

    // Interactions biased 60 % intra-group
    let mut interactions = Vec::new();
    for _ in 0..NUM_INTERACTIONS * 3 {
        if interactions.len() >= NUM_INTERACTIONS {
            break;
        }

        let src = rng.next_usize(NUM_ATTENDEES);
        let mut tgt = rng.next_usize(NUM_ATTENDEES);
        if tgt == src {
            tgt = (tgt + 1) % NUM_ATTENDEES;
        }

        // Intra-group bias
        if rng.next_f32() < 0.6 {
            let same: Vec<usize> = attendees
                .iter()
                .filter(|a| a.group == attendees[src].group && a.id != src)
                .map(|a| a.id)
                .collect();
            if !same.is_empty() {
                tgt = same[rng.next_usize(same.len())];
            }
        }

        // Constrain to temporal overlap
        let os = attendees[src].start_time.max(attendees[tgt].start_time);
        let oe = attendees[src].end_time.min(attendees[tgt].end_time);
        if oe - os < 0.02 {
            continue;
        }

        let ist = os + rng.next_f32() * (oe - os) * 0.6;
        let idur = 0.02 + rng.next_f32() * ((oe - ist) * 0.5).max(0.02);
        let ien = (ist + idur).min(oe);

        interactions.push(InteractionData {
            source: src,
            target: tgt,
            start_time: ist,
            end_time: ien,
        });
    }

    (attendees, interactions)
}

// ---------------------------------------------------------------------------
// Phyllotaxis spiral for node entry positions
// ---------------------------------------------------------------------------

fn phyllotaxis_position(index: usize, scale: f32) -> egui::Pos2 {
    let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let theta = index as f32 * golden_angle;
    let r = scale * (index as f32).sqrt();
    egui::pos2(r * theta.cos(), r * theta.sin())
}

// ---------------------------------------------------------------------------
// Force simulation
// ---------------------------------------------------------------------------

struct SimNode {
    id: usize,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    pinned: bool,
}

struct ForceSimulation {
    nodes: Vec<SimNode>,
    /// Active links as (source_sim_index, target_sim_index).
    active_links: Vec<(usize, usize)>,
    alpha: f32,
}

impl ForceSimulation {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            active_links: Vec::new(),
            alpha: ALPHA_INITIAL,
        }
    }

    fn tick(&mut self) {
        // Cool down
        self.alpha += (0.0 - self.alpha) * ALPHA_DECAY;
        if self.alpha < ALPHA_MIN {
            return;
        }

        let alpha = self.alpha;
        self.apply_charge(alpha);
        self.apply_links(alpha);
        self.apply_center(alpha);

        // Integrate velocities
        for node in &mut self.nodes {
            if node.pinned {
                node.vx = 0.0;
                node.vy = 0.0;
                continue;
            }
            node.vx *= 1.0 - VELOCITY_DECAY;
            node.vy *= 1.0 - VELOCITY_DECAY;
            node.x += node.vx;
            node.y += node.vy;
        }
    }

    /// All-pairs charge repulsion (O(N²), fine for ~55 nodes).
    fn apply_charge(&mut self, alpha: f32) {
        let n = self.nodes.len();
        let pos: Vec<(f32, f32)> = self.nodes.iter().map(|n| (n.x, n.y)).collect();

        for i in 0..n {
            if self.nodes[i].pinned {
                continue;
            }
            let (mut fx, mut fy) = (0.0_f32, 0.0_f32);

            for j in 0..n {
                if i == j {
                    continue;
                }
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let dist = (dx * dx + dy * dy).sqrt().max(CHARGE_MIN_DIST);
                if dist > CHARGE_MAX_DIST {
                    continue;
                }
                // Coulomb-like: negative strength → repulsion
                let f = CHARGE_STRENGTH * alpha / (dist * dist);
                fx -= f * dx / dist;
                fy -= f * dy / dist;
            }

            self.nodes[i].vx += fx;
            self.nodes[i].vy += fy;
        }
    }

    /// Spring attraction along active edges.
    fn apply_links(&mut self, alpha: f32) {
        let pos: Vec<(f32, f32)> = self.nodes.iter().map(|n| (n.x, n.y)).collect();

        for &(si, ti) in &self.active_links {
            if si >= pos.len() || ti >= pos.len() {
                continue;
            }
            let dx = pos[ti].0 - pos[si].0;
            let dy = pos[ti].1 - pos[si].1;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let displacement = dist - LINK_DISTANCE;
            let f = LINK_STRENGTH * alpha * displacement / dist;
            let fx = f * dx / dist;
            let fy = f * dy / dist;

            if !self.nodes[si].pinned {
                self.nodes[si].vx += fx * 0.5;
                self.nodes[si].vy += fy * 0.5;
            }
            if !self.nodes[ti].pinned {
                self.nodes[ti].vx -= fx * 0.5;
                self.nodes[ti].vy -= fy * 0.5;
            }
        }
    }

    /// Gentle pull toward the origin.
    fn apply_center(&mut self, alpha: f32) {
        let n = self.nodes.len();
        if n == 0 {
            return;
        }
        let (sx, sy) = self.nodes.iter().fold((0.0_f32, 0.0_f32), |(ax, ay), n| {
            (ax + n.x, ay + n.y)
        });
        let shift_x = -(sx / n as f32) * CENTER_STRENGTH * alpha;
        let shift_y = -(sy / n as f32) * CENTER_STRENGTH * alpha;

        for node in &mut self.nodes {
            if node.pinned {
                continue;
            }
            node.vx += shift_x;
            node.vy += shift_y;
        }
    }

    fn reheat(&mut self, alpha: f32) {
        self.alpha = self.alpha.max(alpha);
    }
}

// ---------------------------------------------------------------------------
// Custom NodeWidget — filled circles coloured by group
// ---------------------------------------------------------------------------

struct CircleNodeWidget;

impl NodeWidget<NodeData> for CircleNodeWidget {
    fn size(&self, _node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(NODE_DIAMETER, NODE_DIAMETER)
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
        let radius = screen_rect.width().min(screen_rect.height()) / 2.0;
        let color = node.data.group.color();

        // Glow on hover / selection
        if node.selected || hovered {
            let glow =
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60);
            painter.circle_filled(center, radius + 4.0, glow);
        }

        // Main circle
        painter.circle_filled(center, radius, color);

        // Border
        let stroke_color = if node.selected {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 80)
        };
        painter.circle_stroke(
            center,
            radius,
            egui::Stroke::new(if node.selected { 2.0 } else { 1.0 }, stroke_color),
        );
    }
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

fn time_to_clock(t: f32) -> String {
    let hours = CLOCK_START + t * (CLOCK_END - CLOCK_START);
    let h = hours as u32;
    let m = ((hours - h as f32) * 60.0) as u32;
    format!("{:02}:{:02}", h, m)
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct TemporalForceApp {
    state: FlowState<NodeData, ()>,

    // Source data
    attendees: Vec<AttendeeData>,
    interactions: Vec<InteractionData>,

    // Simulation
    simulation: ForceSimulation,

    // Active tracking
    active_nodes: HashSet<usize>,
    active_edges: HashSet<usize>,
    entry_counter: usize,

    // Time scrubber
    current_time: f32,
    playing: bool,
    play_speed: f32,
    looping: bool,
    last_wall_time: Option<f64>,
}

impl TemporalForceApp {
    fn new() -> Self {
        let (attendees, interactions) = generate_conference_data(42);

        let config = FlowConfig {
            snap_to_grid: false,
            auto_pan_on_node_drag: false,
            auto_pan_on_connect: false,
            nodes_draggable: true,
            nodes_connectable: false,
            min_zoom: 0.1,
            max_zoom: 4.0,
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            background_color: egui::Color32::from_rgb(50, 50, 55),
            background_gap: 30.0,
            background_size: 0.5,
            show_minimap: false,
            default_node_width: NODE_DIAMETER,
            default_node_height: NODE_DIAMETER,
            edge_color: egui::Color32::from_rgba_unmultiplied(180, 180, 190, 80),
            edge_stroke_width: 1.0,
            node_border_width: 0.0,
            nodes_resizable: false,
            default_source_position: Position::Center,
            default_target_position: Position::Center,
            ..FlowConfig::default()
        };

        Self {
            state: FlowState::new(config),
            attendees,
            interactions,
            simulation: ForceSimulation::new(),
            active_nodes: HashSet::new(),
            active_edges: HashSet::new(),
            entry_counter: 0,
            current_time: 0.0,
            playing: false,
            play_speed: DEFAULT_PLAY_SPEED,
            looping: true,
            last_wall_time: None,
        }
    }

    // -- Time ---------------------------------------------------------------

    fn tick_time(&mut self, wall_time: f64) {
        if !self.playing {
            self.last_wall_time = Some(wall_time);
            return;
        }
        if let Some(last) = self.last_wall_time {
            let dt = (wall_time - last) as f32;
            self.current_time += self.play_speed * dt;
            if self.current_time > 1.0 {
                if self.looping {
                    self.current_time = 0.0;
                } else {
                    self.current_time = 1.0;
                    self.playing = false;
                }
            }
        }
        self.last_wall_time = Some(wall_time);
    }

    // -- Temporal add/remove ------------------------------------------------

    fn update_active_elements(&mut self) {
        let t = self.current_time;
        let mut changed = false;

        // Which nodes belong at this point in time?
        let should_nodes: HashSet<usize> = self
            .attendees
            .iter()
            .filter(|a| a.start_time <= t && t <= a.end_time)
            .map(|a| a.id)
            .collect();

        // Remove departed nodes
        let nodes_to_remove: Vec<usize> = self
            .active_nodes
            .difference(&should_nodes)
            .cloned()
            .collect();
        for id in nodes_to_remove {
            let nid = format!("n{}", id);
            self.state.nodes.retain(|n| n.id.as_str() != nid);
            self.simulation.nodes.retain(|n| n.id != id);
            self.active_nodes.remove(&id);
            changed = true;
        }

        // Add arriving nodes at phyllotaxis positions
        let nodes_to_add: Vec<usize> = should_nodes
            .difference(&self.active_nodes)
            .cloned()
            .collect();
        for id in nodes_to_add {
            let att = &self.attendees[id];
            let pos = phyllotaxis_position(self.entry_counter, 8.0);
            self.entry_counter += 1;

            self.state.nodes.push(
                Node::builder(format!("n{}", id))
                    .position(pos)
                    .data(NodeData { group: att.group })
                    .size(NODE_DIAMETER, NODE_DIAMETER)
                    .build(),
            );

            self.simulation.nodes.push(SimNode {
                id,
                x: pos.x,
                y: pos.y,
                vx: 0.0,
                vy: 0.0,
                pinned: false,
            });

            self.active_nodes.insert(id);
            changed = true;
        }

        // Which edges belong at this point in time?
        let should_edges: HashSet<usize> = self
            .interactions
            .iter()
            .enumerate()
            .filter(|(_, int)| {
                int.start_time <= t
                    && t <= int.end_time
                    && self.active_nodes.contains(&int.source)
                    && self.active_nodes.contains(&int.target)
            })
            .map(|(idx, _)| idx)
            .collect();

        // Remove departed edges
        let edges_to_remove: Vec<usize> = self
            .active_edges
            .difference(&should_edges)
            .cloned()
            .collect();
        for eid in edges_to_remove {
            let edge_id = format!("e{}", eid);
            self.state.edges.retain(|e| e.id.as_str() != edge_id);
            self.active_edges.remove(&eid);
            changed = true;
        }

        // Add arriving edges
        let edges_to_add: Vec<usize> = should_edges
            .difference(&self.active_edges)
            .cloned()
            .collect();
        for eid in edges_to_add {
            let int = &self.interactions[eid];
            self.state.edges.push(
                Edge::new(
                    format!("e{}", eid),
                    format!("n{}", int.source),
                    format!("n{}", int.target),
                )
                .edge_type(EdgeType::Straight),
            );
            self.active_edges.insert(eid);
            changed = true;
        }

        if changed {
            // Rebuild simulation link indices
            let id_to_idx: HashMap<usize, usize> = self
                .simulation
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.id, i))
                .collect();
            self.simulation.active_links.clear();
            for &eid in &self.active_edges {
                let int = &self.interactions[eid];
                if let (Some(&si), Some(&ti)) =
                    (id_to_idx.get(&int.source), id_to_idx.get(&int.target))
                {
                    self.simulation.active_links.push((si, ti));
                }
            }

            self.state.rebuild_lookup();
            self.simulation.reheat(0.3);
        }
    }

    // -- Sync between flow state and simulation -----------------------------

    fn sync_pinned_from_state(&mut self) {
        for flow_node in &self.state.nodes {
            if let Some(id) = flow_node
                .id
                .as_str()
                .strip_prefix('n')
                .and_then(|s| s.parse::<usize>().ok())
            {
                if let Some(sim) = self.simulation.nodes.iter_mut().find(|n| n.id == id) {
                    sim.pinned = flow_node.dragging;
                    if flow_node.dragging {
                        sim.x = flow_node.position.x;
                        sim.y = flow_node.position.y;
                    }
                }
            }
        }
    }

    fn sync_positions_to_state(&mut self) {
        for sim in &self.simulation.nodes {
            let nid = format!("n{}", sim.id);
            if let Some(flow_node) = self.state.nodes.iter_mut().find(|n| n.id.as_str() == nid) {
                if !flow_node.dragging {
                    flow_node.position = egui::pos2(sim.x, sim.y);
                }
            }
        }
        self.state.rebuild_lookup();
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for TemporalForceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let wall_time = ctx.input(|i| i.time);

        // 1. Advance time
        self.tick_time(wall_time);

        // 2. Add/remove nodes & edges based on current time
        self.update_active_elements();

        // 3. Read drag state into simulation
        self.sync_pinned_from_state();

        // 4. Run force simulation
        self.simulation.tick();

        // 5. Write simulation positions back to flow state
        self.sync_positions_to_state();

        // ===== UI =====

        // -- Top panel: time scrubber --
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Temporal Force Graph");
                ui.separator();

                let play_label = if self.playing { "Pause" } else { "Play" };
                if ui.button(play_label).clicked() {
                    self.playing = !self.playing;
                }

                let resp = ui.add(
                    egui::Slider::new(&mut self.current_time, 0.0..=1.0).show_value(false),
                );
                if resp.changed() {
                    self.simulation.reheat(1.0);
                }

                ui.monospace(time_to_clock(self.current_time));

                ui.separator();
                ui.label("Speed:");
                ui.add(
                    egui::DragValue::new(&mut self.play_speed)
                        .speed(0.005)
                        .range(0.005..=0.3)
                        .suffix("x"),
                );

                ui.checkbox(&mut self.looping, "Loop");

                if ui.button("Reset").clicked() {
                    self.current_time = 0.0;
                    self.playing = false;
                    self.simulation.reheat(1.0);
                }
            });
        });

        // -- Right panel: stats & legend --
        egui::SidePanel::right("info")
            .resizable(true)
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Statistics");
                ui.separator();
                ui.label(format!("Active nodes: {}", self.active_nodes.len()));
                ui.label(format!("Active edges: {}", self.active_edges.len()));
                ui.label(format!("Sim alpha: {:.4}", self.simulation.alpha));
                ui.label(format!("Time: {}", time_to_clock(self.current_time)));

                ui.add_space(12.0);
                ui.separator();
                ui.heading("Groups");
                ui.add_space(4.0);

                for &group in &Group::ALL {
                    let count = self
                        .attendees
                        .iter()
                        .filter(|a| a.group == group && self.active_nodes.contains(&a.id))
                        .count();
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(rect.center(), 6.0, group.color());
                        ui.label(format!("{} ({})", group.name(), count));
                    });
                }

                ui.add_space(12.0);
                ui.separator();
                ui.heading("Controls");
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Drag nodes to pin them").size(11.0));
                ui.label(egui::RichText::new("Scroll to zoom").size(11.0));
                ui.label(egui::RichText::new("Drag canvas to pan").size(11.0));

                ui.add_space(12.0);
                if ui.button("Reheat Simulation").clicked() {
                    self.simulation.reheat(1.0);
                }
                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    self.state
                        .fit_view(rect, 60.0, ctx.input(|i| i.time));
                }
            });

        // -- Central panel: force-directed graph --
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(24, 24, 32)))
            .show(ctx, |ui| {
                let _events = FlowCanvas::new(&mut self.state, &CircleNodeWidget).show(ui);
            });

        // Keep repainting while simulation is active or playing
        if self.playing || self.simulation.alpha > ALPHA_MIN {
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
            .with_title("egui_xyflow -- Temporal Force-Directed Graph")
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Temporal Force Graph",
        options,
        Box::new(|_cc| Ok(Box::new(TemporalForceApp::new()))),
    )
}
