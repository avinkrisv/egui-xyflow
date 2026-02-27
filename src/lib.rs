//! # egui_xyflow
//!
//! A node graph editor widget for [egui](https://github.com/emilk/egui),
//! inspired by [xyflow](https://xyflow.com/) (React Flow / Svelte Flow).
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
//!         state.add_edge(Edge::new("e1-2", "1", "2"));
//!
//!         Self { state }
//!     }
//! }
//!
//! impl eframe::App for MyApp {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         egui::CentralPanel::default().show(ctx, |ui| {
//!             FlowCanvas::new(&mut self.state, &DefaultNodeWidget).show(ui);
//!         });
//!     }
//! }
//! ```

// ── Module tree ───────────────────────────────────────────────────────────────

pub mod animation;
pub mod config;
pub mod edges;
pub mod events;
pub mod graph;
pub mod interaction;
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
}
