//! # egui_xyflow
//!
//! An interactive node-graph editor widget for
//! [egui](https://github.com/emilk/egui), inspired by
//! [xyflow](https://xyflow.com/) (React Flow / Svelte Flow). Supports
//! drag-and-drop nodes, handle-to-handle edge connections, pan/zoom,
//! multi-select, resize, minimap, edge labels, and an optional
//! force-directed layout subsystem.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use eframe::egui;
//! use egui_xyflow::prelude::*;
//!
//! struct MyApp {
//!     state: FlowState<String, ()>,
//! }
//!
//! impl MyApp {
//!     fn new() -> Self {
//!         let mut state = FlowState::new(FlowConfig::default());
//!
//!         state.add_node(
//!             Node::builder("1")
//!                 .position(egui::pos2(100.0, 100.0))
//!                 .data("Input".to_string())
//!                 .handle(NodeHandle::source(Position::Right))
//!                 .build(),
//!         );
//!         state.add_node(
//!             Node::builder("2")
//!                 .position(egui::pos2(400.0, 100.0))
//!                 .data("Output".to_string())
//!                 .handle(NodeHandle::target(Position::Left))
//!                 .build(),
//!         );
//!         state.add_edge(Edge::builder("e1-2", "1", "2").label("flow"));
//!
//!         Self { state }
//!     }
//! }
//!
//! impl eframe::App for MyApp {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         egui::CentralPanel::default().show(ctx, |ui| {
//!             let events =
//!                 FlowCanvas::new(&mut self.state, &DefaultNodeWidget).show(ui);
//!             for conn in &events.connections_made {
//!                 println!("new edge: {} -> {}", conn.source, conn.target);
//!             }
//!         });
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! Every frame follows the same cycle:
//!
//! ```text
//! FlowState  →  FlowCanvas::show()  →  FlowEvents  →  apply changes  →  FlowState
//! ```
//!
//! 1. [`FlowState`] owns the graph: nodes, edges, viewport, config,
//!    selection state.
//! 2. [`FlowCanvas`] renders the graph and processes input, returning
//!    [`FlowEvents`] that describe what happened this frame (clicks,
//!    drags, connections, selection changes, deletions, etc.).
//! 3. Most mutations are represented as [`NodeChange`] / [`EdgeChange`]
//!    enum variants and applied atomically via
//!    [`FlowState::apply_node_changes`] and
//!    [`FlowState::apply_edge_changes`].
//!
//! ### Generic parameters
//!
//! The library is parameterised over two user-owned types:
//!
//! - `ND` — custom data attached to each [`Node`].
//! - `ED` — custom data attached to each [`Edge`].
//!
//! Use `()` when you don't need custom data, or `String` for simple
//! labels, or a struct of your own.
//!
//! ### Coordinate spaces
//!
//! Two coordinate systems convert via [`flow_to_screen`] and
//! [`screen_to_flow`] using a [`Transform`]:
//!
//! - **Flow space** — graph coordinates you set on nodes. Unbounded.
//! - **Screen space** — pixel coordinates inside the canvas rect.
//!
//! ### Render order
//!
//! Bottom-to-top: background → edges (with optional viewport culling) →
//! connection drag line → nodes (z-ordered) → handles → resize handles →
//! selection rectangle → minimap.
//!
//! ## Customisation points
//!
//! - [`NodeWidget<D>`](render::node_renderer::NodeWidget) — implement
//!   `size()` and `show()` for custom node rendering. Built-ins:
//!   [`DefaultNodeWidget`] (for `Node<String>`) and [`UnitNodeWidget`]
//!   (for `Node<()>`).
//! - [`EdgeWidget<ED>`](render::canvas::EdgeWidget) — implement to render
//!   custom edge paths instead of the built-in [`EdgeType`] algorithms.
//! - [`ConnectionValidator`] — reject prospective connections (e.g. no
//!   self-loops, typed handles).
//! - [`FlowConfig`] — 60+ knobs for pan/zoom, selection, colour, edge
//!   defaults, handle appearance, grid snapping, animation, viewport
//!   culling, edge labels, etc.
//!
//! ## Physics (force-directed layout)
//!
//! The [`physics`] module provides a D3-compatible force simulation
//! usable alongside (or instead of) hand-placed node positions:
//!
//! ```rust,no_run
//! # use egui_xyflow::prelude::*;
//! use egui_xyflow::physics::*;
//!
//! # let mut state: FlowState<(), ()> = FlowState::new(FlowConfig::default());
//! let mut sim = ForceSimulation::from_state(&state)
//!     .add_force("charge",   ManyBodyForce::new().strength(-30.0))
//!     .add_force("links",    LinkForce::from_state(&state).distance(30.0))
//!     .add_force("position", PositionForce::new().strength(0.1))
//!     .add_force("center",   CenterForce::new());
//!
//! // Each frame — `false` means the state was mutated, rebuild the sim.
//! if !sim.step(&mut state) {
//!     sim = ForceSimulation::from_state(&state);
//! }
//! ```
//!
//! The charge force uses a Barnes–Hut quadtree (θ = 0.9 by default), so
//! it scales roughly O(n log n) with node count. `physics` is **not**
//! re-exported from the prelude — import it explicitly.
//!
//! ## Feature flags
//!
//! - `serde` *(default)* — derive `Serialize` / `Deserialize` on
//!   [`FlowState`], [`Node`], [`Edge`], and most config/viewport types.
//!   Disable for no-std-adjacent builds or to drop the `serde`
//!   dependency.
//!
//! ## Examples
//!
//! The repository ships with 17 runnable examples:
//!
//! ```text
//! cargo run --example basic_flow                   # getting started
//! cargo run --example edge_labels                  # labels + viewport culling
//! cargo run --example data_pipeline                # pipeline with validation
//! cargo run --example disjoint_force_graph         # physics: citation network
//! cargo run --release --example physics_bench      # physics timing harness
//! ```
//!
//! See [the repo](https://github.com/avinkrisv/egui_xyflow) for the
//! complete list.
//!
//! ## Compatibility
//!
//! Built against `egui` 0.31. MSRV: 1.85 (edition 2024).

#![warn(missing_docs)]

// ── Module tree ───────────────────────────────────────────────────────────────

pub mod animation;
pub mod config;
pub mod edges;
pub mod events;
pub mod graph;
pub mod interaction;
pub mod physics;
pub mod render;
pub mod state;
pub mod types;

// ── Top-level re-exports (most commonly used items) ──────────────────────────

// Configuration
pub use config::{BackgroundVariant, FlowConfig, ZIndexMode};

// Core types
pub use types::changes::{EdgeChange, NodeChange};
pub use types::connection::{Connection, ConnectionMode, ConnectionState, EdgeInfo};
pub use types::edge::{
    AnchorEndpoint, Edge, EdgeAnchor, EdgeGlow, EdgeId, EdgeMarker, EdgePathResult, EdgePosition,
    EdgeStyle, EdgeType, MarkerType,
};
pub use types::handle::{Handle, HandleType, NodeHandle};
pub use types::node::{
    InternalNode, Node, NodeBuilder, NodeExtent, NodeHandleBounds, NodeId, NodeInternals,
};
pub use types::position::{CoordinateExtent, Dimensions, NodeOrigin, Position, SnapGrid, Transform};
pub use types::viewport::{PanOnScrollMode, SelectionMode, Viewport};

// State
pub use state::flow_state::FlowState;

// Events
pub use events::FlowEvents;

// Render / widget
pub use render::canvas::{AllowAllConnections, ConnectionValidator, EdgeWidget, FlowCanvas};
pub use render::minimap::MinimapInfo;
pub use render::node_renderer::{DefaultNodeWidget, NodeWidget, UnitNodeWidget};

// Animation
pub use animation::easing;
pub use animation::viewport_animation::ViewportAnimation;

// State helpers
pub use state::node_lookup;

// Graph utilities
pub use graph::node_position::{flow_to_screen, screen_to_flow, snap_position};
pub use graph::utils::{
    get_connected_edges, get_incomers, get_nodes_bounds, get_outgoers, get_viewport_for_bounds,
};

// Edge path math
pub use edges::bezier::{get_bezier_path, sample_bezier};
pub use edges::positions::{get_edge_position, project_to_border};
pub use edges::smooth_step::{get_smooth_step_path, get_step_path};
pub use edges::straight::get_straight_path;

// Interaction
pub use interaction::resize::ResizeHandleKind;

// Physics is a distinct optional subsystem — callers opt in explicitly via
// `egui_xyflow::physics::*` rather than polluting the crate root / prelude.

// ── Prelude ───────────────────────────────────────────────────────────────────

/// Convenience glob import that brings the most commonly needed items into scope.
///
/// ```rust,no_run
/// use egui_xyflow::prelude::*;
/// ```
pub mod prelude {
    pub use crate::config::{BackgroundVariant, FlowConfig, ZIndexMode};

    pub use crate::types::changes::{EdgeChange, NodeChange};
    pub use crate::types::connection::{Connection, ConnectionMode, EdgeInfo};
    pub use crate::types::edge::{
        AnchorEndpoint, Edge, EdgeAnchor, EdgeGlow, EdgeId, EdgeStyle, EdgeType, MarkerType,
    };
    pub use crate::types::handle::{HandleType, NodeHandle};
    pub use crate::types::node::{Node, NodeExtent, NodeId};
    pub use crate::types::position::{CoordinateExtent, Dimensions, Position, Transform};
    pub use crate::types::viewport::{SelectionMode, Viewport};

    pub use crate::state::flow_state::FlowState;

    pub use crate::events::FlowEvents;

    pub use crate::render::canvas::{
        AllowAllConnections, ConnectionValidator, EdgeWidget, FlowCanvas,
    };
    pub use crate::render::minimap::MinimapInfo;
    pub use crate::render::node_renderer::{DefaultNodeWidget, NodeWidget, UnitNodeWidget};

    pub use crate::animation::easing::{ease_cubic, ease_linear};
    pub use crate::animation::viewport_animation::ViewportAnimation;

    pub use crate::interaction::resize::ResizeHandleKind;

    // Physics is NOT re-exported here. Opt in with `use egui_xyflow::physics::*;`.
}
