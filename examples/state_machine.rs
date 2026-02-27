//! State Machine / Finite Automaton Visualizer
//!
//! Demonstrates a state machine with five states and transitions between them,
//! using egui_xyflow for the graph visualization.
//!
//!   Idle ──start──> Running ──complete──> Complete
//!    ^                 │  ^
//!    │                 │  │
//!   reset           pause resume
//!    │                 │  │
//!    └──── Error <─fail── Paused
//!
//! Features demonstrated
//! ---------------------
//! - Custom NodeWidget that colors nodes based on whether they are the current state
//! - ConnectionValidator that enforces valid state transitions
//! - Side panel showing current state and available transition buttons
//! - Clicking a transition button briefly animates the corresponding edge
//! - Different EdgeType for each transition category

use eframe::egui;
use egui_xyflow::prelude::*;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- state machine visualizer")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "State Machine",
        options,
        Box::new(|_cc| Ok(Box::new(StateMachineApp::new()))),
    )
}

// ---------------------------------------------------------------------------
// State definitions
// ---------------------------------------------------------------------------

/// All possible states in our finite automaton.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Running,
    Paused,
    Error,
    Complete,
}

impl State {
    const ALL: [State; 5] = [
        State::Idle,
        State::Running,
        State::Paused,
        State::Error,
        State::Complete,
    ];

    fn name(self) -> &'static str {
        match self {
            State::Idle => "Idle",
            State::Running => "Running",
            State::Paused => "Paused",
            State::Error => "Error",
            State::Complete => "Complete",
        }
    }

    fn node_id(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Running => "running",
            State::Paused => "paused",
            State::Error => "error",
            State::Complete => "complete",
        }
    }

    fn from_node_id(id: &str) -> Option<State> {
        match id {
            "idle" => Some(State::Idle),
            "running" => Some(State::Running),
            "paused" => Some(State::Paused),
            "error" => Some(State::Error),
            "complete" => Some(State::Complete),
            _ => None,
        }
    }
}

/// A transition between two states.
#[derive(Debug, Clone)]
struct Transition {
    label: &'static str,
    from: State,
    to: State,
    edge_type: EdgeType,
}

/// All valid transitions in the state machine.
fn transitions() -> Vec<Transition> {
    vec![
        Transition {
            label: "start",
            from: State::Idle,
            to: State::Running,
            edge_type: EdgeType::Bezier,
        },
        Transition {
            label: "pause",
            from: State::Running,
            to: State::Paused,
            edge_type: EdgeType::SmoothStep,
        },
        Transition {
            label: "resume",
            from: State::Paused,
            to: State::Running,
            edge_type: EdgeType::SmoothStep,
        },
        Transition {
            label: "fail",
            from: State::Running,
            to: State::Error,
            edge_type: EdgeType::Straight,
        },
        Transition {
            label: "reset",
            from: State::Error,
            to: State::Idle,
            edge_type: EdgeType::Step,
        },
        Transition {
            label: "complete",
            from: State::Running,
            to: State::Complete,
            edge_type: EdgeType::Bezier,
        },
    ]
}

fn edge_id_for(t: &Transition) -> String {
    format!("e-{}-{}", t.from.node_id(), t.to.node_id())
}

// ---------------------------------------------------------------------------
// Connection validator -- only allows valid state transitions
// ---------------------------------------------------------------------------

struct StateMachineValidator;

impl ConnectionValidator for StateMachineValidator {
    fn is_valid_connection(&self, connection: &Connection, _existing_edges: &[EdgeInfo<'_>]) -> bool {
        let source = connection.source.as_str();
        let target = connection.target.as_str();

        // Only allow connections that match one of our defined transitions.
        transitions()
            .iter()
            .any(|t| t.from.node_id() == source && t.to.node_id() == target)
    }
}

// ---------------------------------------------------------------------------
// Custom node widget -- colors nodes based on current state
// ---------------------------------------------------------------------------

struct StateNodeWidget {
    current_state: State,
}

impl StateNodeWidget {
    fn color_for(&self, state: State) -> egui::Color32 {
        if state == self.current_state {
            // Current state gets a vivid fill
            match state {
                State::Idle => egui::Color32::from_rgb(100, 180, 255),    // blue
                State::Running => egui::Color32::from_rgb(80, 200, 120),  // green
                State::Paused => egui::Color32::from_rgb(255, 200, 60),   // amber
                State::Error => egui::Color32::from_rgb(240, 80, 80),     // red
                State::Complete => egui::Color32::from_rgb(160, 100, 240),// purple
            }
        } else {
            // Non-current states get a desaturated fill
            egui::Color32::from_rgb(235, 235, 240)
        }
    }

    fn border_for(&self, state: State, selected: bool) -> egui::Color32 {
        if selected {
            egui::Color32::from_rgb(59, 130, 246)
        } else if state == self.current_state {
            egui::Color32::from_rgb(40, 40, 50)
        } else {
            egui::Color32::from_rgb(177, 177, 183)
        }
    }

    fn text_color_for(&self, state: State) -> egui::Color32 {
        if state == self.current_state {
            match state {
                State::Error => egui::Color32::WHITE,
                _ => egui::Color32::from_rgb(20, 20, 30),
            }
        } else {
            egui::Color32::from_rgb(100, 100, 110)
        }
    }
}

impl NodeWidget<String> for StateNodeWidget {
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
        let state = State::from_node_id(node.id.as_str()).unwrap_or(State::Idle);
        let bg = self.color_for(state);
        let border = self.border_for(state, node.selected);
        let text_color = self.text_color_for(state);
        let rounding = config.node_corner_radius;

        // Glow for current state
        if state == self.current_state {
            let glow_rect = screen_rect.expand(4.0);
            let glow_color = match state {
                State::Idle => egui::Color32::from_rgba_unmultiplied(100, 180, 255, 50),
                State::Running => egui::Color32::from_rgba_unmultiplied(80, 200, 120, 50),
                State::Paused => egui::Color32::from_rgba_unmultiplied(255, 200, 60, 50),
                State::Error => egui::Color32::from_rgba_unmultiplied(240, 80, 80, 50),
                State::Complete => egui::Color32::from_rgba_unmultiplied(160, 100, 240, 50),
            };
            painter.rect_filled(glow_rect, rounding + 2.0, glow_color);
        }

        // Selection shadow
        if node.selected {
            let shadow_rect = screen_rect.expand(2.0);
            painter.rect_filled(
                shadow_rect,
                rounding + 1.0,
                egui::Color32::from_rgba_unmultiplied(59, 130, 246, 40),
            );
        }

        // Background
        painter.rect_filled(screen_rect, rounding, bg);

        // Border
        let border_width = if state == self.current_state || node.selected {
            config.node_border_width * 2.5
        } else {
            config.node_border_width
        };
        painter.rect_stroke(
            screen_rect,
            rounding,
            egui::Stroke::new(border_width, border),
            egui::StrokeKind::Middle,
        );

        // Label: state name
        let galley = painter.layout_no_wrap(
            node.data.clone(),
            egui::FontId::proportional(14.0),
            text_color,
        );
        let text_pos = egui::pos2(
            screen_rect.center().x - galley.size().x / 2.0,
            screen_rect.center().y - galley.size().y / 2.0,
        );
        painter.galley(text_pos, galley, text_color);
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct StateMachineApp {
    state: FlowState<String, ()>,
    validator: StateMachineValidator,
    current_state: State,

    /// Edge ID that is temporarily animated (to highlight a transition).
    animated_edge: Option<String>,
    /// Time when animation was triggered.
    animation_start: f64,
    /// Duration of the highlight animation in seconds.
    animation_duration: f64,

    /// Event log
    event_log: Vec<String>,
}

impl StateMachineApp {
    fn new() -> Self {
        let config = FlowConfig {
            show_background: true,
            background_variant: BackgroundVariant::Cross,
            show_minimap: true,
            node_corner_radius: 8.0,
            default_node_width: 140.0,
            default_node_height: 50.0,
            ..FlowConfig::default()
        };

        let mut state: FlowState<String, ()> = FlowState::new(config);

        // -- Nodes (state machine states) laid out in a nice arrangement --
        //
        //        Idle          Running         Complete
        //                       Paused
        //        Error
        //
        state.add_node(
            Node::builder("idle")
                .position(egui::pos2(80.0, 150.0))
                .data("Idle".to_string())
                .handle(NodeHandle::source(Position::Right))
                .handle(NodeHandle::target(Position::Left))
                .size(140.0, 50.0)
                .build(),
        );

        state.add_node(
            Node::builder("running")
                .position(egui::pos2(350.0, 150.0))
                .data("Running".to_string())
                .handle(NodeHandle::target(Position::Left))
                .handle(NodeHandle::source(Position::Right))
                .handle(NodeHandle::source(Position::Bottom))
                .size(140.0, 50.0)
                .build(),
        );

        state.add_node(
            Node::builder("paused")
                .position(egui::pos2(350.0, 330.0))
                .data("Paused".to_string())
                .handle(NodeHandle::target(Position::Top))
                .handle(NodeHandle::source(Position::Top).with_id("paused-resume"))
                .size(140.0, 50.0)
                .build(),
        );

        state.add_node(
            Node::builder("error")
                .position(egui::pos2(80.0, 330.0))
                .data("Error".to_string())
                .handle(NodeHandle::target(Position::Right))
                .handle(NodeHandle::source(Position::Top))
                .size(140.0, 50.0)
                .build(),
        );

        state.add_node(
            Node::builder("complete")
                .position(egui::pos2(620.0, 150.0))
                .data("Complete".to_string())
                .handle(NodeHandle::target(Position::Left))
                .size(140.0, 50.0)
                .build(),
        );

        // -- Edges (transitions) --
        for t in &transitions() {
            let edge = Edge::new(
                edge_id_for(t),
                t.from.node_id(),
                t.to.node_id(),
            )
            .edge_type(t.edge_type)
            .marker_end_arrow();
            state.add_edge(edge);
        }

        Self {
            state,
            validator: StateMachineValidator,
            current_state: State::Idle,
            animated_edge: None,
            animation_start: 0.0,
            animation_duration: 1.2,
            event_log: Vec::new(),
        }
    }

    /// Push a message to the event log (capped at 40 entries).
    fn log(&mut self, msg: impl Into<String>) {
        if self.event_log.len() >= 40 {
            self.event_log.remove(0);
        }
        self.event_log.push(msg.into());
    }

    /// Get transitions available from the current state.
    fn available_transitions(&self) -> Vec<Transition> {
        transitions()
            .into_iter()
            .filter(|t| t.from == self.current_state)
            .collect()
    }

    /// Fire a transition: change state and animate the corresponding edge.
    fn fire_transition(&mut self, transition: &Transition, time: f64) {
        let old = self.current_state;
        self.current_state = transition.to;

        // Animate the edge
        let eid = edge_id_for(transition);
        self.animated_edge = Some(eid.clone());
        self.animation_start = time;

        self.log(format!(
            "{} --[{}]--> {}",
            old.name(),
            transition.label,
            transition.to.name()
        ));
    }

    /// Update edge animation state each frame.
    fn tick_animation(&mut self, time: f64) {
        if let Some(ref eid) = self.animated_edge.clone() {
            let elapsed = time - self.animation_start;
            if elapsed > self.animation_duration {
                // Animation done -- turn off the animated flag
                if let Some(edge) = self.state.edges.iter_mut().find(|e| e.id.as_str() == *eid) {
                    edge.animated = false;
                }
                self.animated_edge = None;
                self.state.has_animated_edges = self.state.edges.iter().any(|e| e.animated);
            } else {
                // Ensure the edge is animated
                if let Some(edge) = self.state.edges.iter_mut().find(|e| e.id.as_str() == *eid) {
                    if !edge.animated {
                        edge.animated = true;
                        self.state.has_animated_edges = true;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for StateMachineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = ctx.input(|i| i.time);

        // Tick edge highlight animation
        self.tick_animation(time);

        // -- Top toolbar --
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("State Machine Visualizer");
                ui.separator();

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

                let n = self.state.nodes.len();
                let e = self.state.edges.len();
                let z = self.state.viewport.zoom;
                ui.label(format!("{n} states  |  {e} transitions  |  zoom {z:.2}"));
            });
        });

        // -- Side panel: current state + transitions --
        egui::SidePanel::right("control_panel")
            .resizable(true)
            .min_width(240.0)
            .show(ctx, |ui| {
                ui.heading("Current State");
                ui.add_space(4.0);

                // Show current state with colored label
                let state_color = match self.current_state {
                    State::Idle => egui::Color32::from_rgb(100, 180, 255),
                    State::Running => egui::Color32::from_rgb(80, 200, 120),
                    State::Paused => egui::Color32::from_rgb(255, 200, 60),
                    State::Error => egui::Color32::from_rgb(240, 80, 80),
                    State::Complete => egui::Color32::from_rgb(160, 100, 240),
                };
                ui.colored_label(
                    state_color,
                    egui::RichText::new(self.current_state.name())
                        .size(22.0)
                        .strong(),
                );
                ui.add_space(12.0);

                ui.separator();
                ui.add_space(4.0);
                ui.heading("Available Transitions");
                ui.add_space(4.0);

                let available = self.available_transitions();
                if available.is_empty() {
                    ui.label(
                        egui::RichText::new("No transitions available (terminal state)")
                            .italics()
                            .color(egui::Color32::from_rgb(150, 150, 160)),
                    );
                } else {
                    // We need to collect transitions first because firing
                    // borrows self mutably.
                    let mut to_fire: Option<Transition> = None;
                    for t in &available {
                        let label = format!("[{}] --> {}", t.label, t.to.name());
                        if ui
                            .add(egui::Button::new(
                                egui::RichText::new(&label).size(14.0),
                            ).min_size(egui::vec2(200.0, 30.0)))
                            .clicked()
                        {
                            to_fire = Some(t.clone());
                        }
                    }
                    if let Some(t) = to_fire {
                        self.fire_transition(&t, time);
                    }
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(4.0);

                // Reset button
                if self.current_state != State::Idle {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Reset to Idle").size(13.0),
                            )
                            .min_size(egui::vec2(200.0, 28.0)),
                        )
                        .clicked()
                    {
                        self.current_state = State::Idle;
                        self.log("-- manually reset to Idle --".to_string());
                    }
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);
                }

                // State legend
                ui.heading("Legend");
                ui.add_space(4.0);
                for s in &State::ALL {
                    let color = match s {
                        State::Idle => egui::Color32::from_rgb(100, 180, 255),
                        State::Running => egui::Color32::from_rgb(80, 200, 120),
                        State::Paused => egui::Color32::from_rgb(255, 200, 60),
                        State::Error => egui::Color32::from_rgb(240, 80, 80),
                        State::Complete => egui::Color32::from_rgb(160, 100, 240),
                    };
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(14.0, 14.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 3.0, color);
                        ui.label(s.name());
                    });
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(4.0);

                // Transition log
                ui.heading("Transition Log");
                ui.add_space(4.0);
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

        // -- Main canvas --
        let current_state = self.current_state;
        let node_widget = StateNodeWidget {
            current_state,
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(250, 250, 255)))
            .show(ctx, |ui| {
                let events = FlowCanvas::new(&mut self.state, &node_widget)
                    .connection_validator(&self.validator)
                    .show(ui);

                // Log interesting events
                if !events.is_empty() {
                    for conn in &events.connections_made {
                        self.log(format!(
                            "edge created: {} --> {}",
                            conn.source, conn.target
                        ));
                    }
                    for id in &events.nodes_clicked {
                        if let Some(s) = State::from_node_id(id.as_str()) {
                            self.log(format!("clicked state: {}", s.name()));
                        }
                    }
                    for id in &events.edges_clicked {
                        self.log(format!("clicked edge: {}", id));
                    }
                }
            });

        // Request repaint while animation is active
        if self.animated_edge.is_some() {
            ctx.request_repaint();
        }
    }
}
