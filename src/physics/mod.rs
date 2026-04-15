//! Force-directed physics simulation.
//!
//! Provides a composable force simulation that integrates with
//! [`FlowState`]. Add built-in forces (charge, links, position, collision,
//! center) or implement the [`Force`] trait for custom behaviour.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use egui_xyflow::prelude::*;
//! use egui_xyflow::physics::*;
//!
//! let state: FlowState<MyData, ()> = /* … */;
//!
//! let mut sim = ForceSimulation::from_state(&state)
//!     .add_force("charge",   ManyBodyForce::new().strength(-30.0))
//!     .add_force("links",    LinkForce::from_state(&state).distance(30.0))
//!     .add_force("position", PositionForce::new().strength(0.1))
//!     .add_force("center",   CenterForce::new());
//!
//! // Each frame:
//! sim.step(&mut state);
//! ```
//!
//! All parameters default to D3.js values so a bare `from_state` + forces
//! produces familiar results. The many-body force uses a Barnes–Hut
//! quadtree (θ = 0.9) matching D3's `forceManyBody` approximation, giving
//! O(n log n) cost per tick for large graphs.

pub mod center;
pub mod collision;
pub mod force;
pub mod link;
pub mod many_body;
pub mod position;
pub mod quadtree;
pub mod sim_node;

pub use center::CenterForce;
pub use collision::CollisionForce;
pub use force::Force;
pub use link::{LinkForce, SimLink};
pub use many_body::ManyBodyForce;
pub use position::PositionForce;
pub use quadtree::QuadTree;
pub use sim_node::SimNode;

use crate::state::flow_state::FlowState;

// ── D3 defaults ──────────────────────────────────────────────────────────────

const DEFAULT_ALPHA: f32 = 1.0;
const DEFAULT_ALPHA_MIN: f32 = 0.001;
/// `1 - pow(0.001, 1/300)` — D3's default for 300-tick cool-down.
const DEFAULT_ALPHA_DECAY: f32 = 0.0228;
/// D3 internal velocity decay: `1 - apiDecay(0.4)`.
const DEFAULT_VELOCITY_DECAY: f32 = 0.6;
const DEFAULT_DRAG_ALPHA_TARGET: f32 = 0.3;

// ── ForceSimulation ──────────────────────────────────────────────────────────

/// Orchestrates forces and synchronises with [`FlowState`].
///
/// Create with [`ForceSimulation::new`] (raw nodes) or
/// [`ForceSimulation::from_state`] (copies positions from a `FlowState`).
/// Chain [`add_force`](Self::add_force) calls to compose behaviour, then call
/// [`step`](Self::step) each frame.
pub struct ForceSimulation {
    nodes: Vec<SimNode>,
    forces: Vec<(String, Box<dyn Force>)>,
    alpha: f32,
    alpha_target: f32,
    alpha_min: f32,
    alpha_decay: f32,
    velocity_decay: f32,
    drag_alpha_target: f32,
}

/// Options passed to [`ForceSimulation::from_state_with`] to derive
/// per-node simulation parameters from a `FlowState` node.
///
/// Use closures to compute radius / initial velocity / per-node charge
/// strength from your own domain data (rather than the node's bounding-box
/// size, which is the default).
pub struct FromStateOptions<'a, ND> {
    /// Compute the collision radius for this node. Default:
    /// `max(width, height) / 2`.
    pub radius: Box<dyn FnMut(&crate::types::node::Node<ND>) -> f32 + 'a>,
    /// Compute an optional per-node charge strength override for
    /// [`ManyBodyForce`]. Default: `None` (use the force's strength).
    pub strength: Box<dyn FnMut(&crate::types::node::Node<ND>) -> Option<f32> + 'a>,
}

impl<ND> Default for FromStateOptions<'_, ND> {
    fn default() -> Self {
        Self {
            radius: Box::new(|_| 0.0), // will be replaced by caller or default
            strength: Box::new(|_| None),
        }
    }
}

impl ForceSimulation {
    /// Create a simulation from pre-built [`SimNode`]s.
    pub fn new(nodes: Vec<SimNode>) -> Self {
        Self {
            nodes,
            forces: Vec::new(),
            alpha: DEFAULT_ALPHA,
            alpha_target: 0.0,
            alpha_min: DEFAULT_ALPHA_MIN,
            alpha_decay: DEFAULT_ALPHA_DECAY,
            velocity_decay: DEFAULT_VELOCITY_DECAY,
            drag_alpha_target: DEFAULT_DRAG_ALPHA_TARGET,
        }
    }

    /// Create a simulation with nodes initialised from a [`FlowState`].
    ///
    /// Each `SimNode` copies its position from the corresponding
    /// [`Node`](crate::Node) and derives its collision radius from the
    /// node's width/height (or config defaults), with the node's `NodeId`
    /// attached so drift can be detected during [`sync_from_state`](Self::sync_from_state).
    pub fn from_state<ND: Clone, ED: Clone>(state: &FlowState<ND, ED>) -> Self {
        let default_w = state.config.default_node_width;
        let default_h = state.config.default_node_height;
        let nodes = state
            .nodes
            .iter()
            .map(|n| {
                let w = n.width.unwrap_or(default_w);
                let h = n.height.unwrap_or(default_h);
                SimNode {
                    x: n.position.x,
                    y: n.position.y,
                    vx: 0.0,
                    vy: 0.0,
                    fx: None,
                    fy: None,
                    radius: w.max(h) / 2.0,
                    strength: None,
                    id: Some(n.id.clone()),
                }
            })
            .collect();
        Self::new(nodes)
    }

    /// Like [`from_state`](Self::from_state), but callers supply closures to
    /// derive the per-node radius and charge strength. Useful when your
    /// domain data (not the node's rendered size) drives simulation
    /// parameters — e.g. the disjoint-force example where node data carries
    /// a per-node radius from JSON.
    ///
    /// ```rust,ignore
    /// let sim = ForceSimulation::from_state_with(&state, FromStateOptions {
    ///     radius: Box::new(|n| 5.0 + n.data.as_ref().map_or(0.0, |d| d.r)),
    ///     strength: Box::new(|_| Some(-50.0)),
    /// });
    /// ```
    pub fn from_state_with<ND: Clone, ED: Clone>(
        state: &FlowState<ND, ED>,
        mut opts: FromStateOptions<'_, ND>,
    ) -> Self {
        let nodes = state
            .nodes
            .iter()
            .map(|n| SimNode {
                x: n.position.x,
                y: n.position.y,
                vx: 0.0,
                vy: 0.0,
                fx: None,
                fy: None,
                radius: (opts.radius)(n),
                strength: (opts.strength)(n),
                id: Some(n.id.clone()),
            })
            .collect();
        Self::new(nodes)
    }

    // ── Builder methods ──────────────────────────────────────────────────

    /// Register a named force. Forces are applied in insertion order each tick.
    pub fn add_force(mut self, name: impl Into<String>, force: impl Force + 'static) -> Self {
        self.forces.push((name.into(), Box::new(force)));
        self
    }

    /// Set the alpha decay rate. Default: 0.0228 (D3's 300-tick cool-down).
    pub fn alpha_decay(mut self, decay: f32) -> Self {
        self.alpha_decay = decay;
        self
    }

    /// Set the velocity decay (damping). Default: 0.6.
    /// Higher values preserve more momentum; lower values brake faster.
    pub fn velocity_decay(mut self, decay: f32) -> Self {
        self.velocity_decay = decay;
        self
    }

    /// Set the alpha floor. The simulation stops ticking below this value.
    /// Default: 0.001.
    pub fn alpha_min(mut self, min: f32) -> Self {
        self.alpha_min = min;
        self
    }

    /// Set the alpha target used while a node is being dragged. Default: 0.3.
    pub fn drag_alpha_target(mut self, target: f32) -> Self {
        self.drag_alpha_target = target;
        self
    }

    // ── Runtime mutation ─────────────────────────────────────────────────

    /// Add a force at runtime (non-builder version).
    pub fn insert_force(&mut self, name: impl Into<String>, force: impl Force + 'static) {
        self.forces.push((name.into(), Box::new(force)));
    }

    /// Remove a named force, returning it if found.
    pub fn remove_force(&mut self, name: &str) -> Option<Box<dyn Force>> {
        self.forces
            .iter()
            .position(|(n, _)| n == name)
            .map(|idx| self.forces.remove(idx).1)
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn nodes(&self) -> &[SimNode] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [SimNode] {
        &mut self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    pub fn alpha_target(&self) -> f32 {
        self.alpha_target
    }

    pub fn set_alpha_target(&mut self, target: f32) {
        self.alpha_target = target;
    }

    /// Returns `true` while the simulation is still cooling (alpha ≥ alpha_min).
    pub fn is_active(&self) -> bool {
        self.alpha >= self.alpha_min
    }

    // ── Simulation control ───────────────────────────────────────────────

    /// Advance the simulation by one tick: decay alpha, apply all forces,
    /// integrate velocities into positions.
    pub fn tick(&mut self) {
        self.alpha += (self.alpha_target - self.alpha) * self.alpha_decay;
        if self.alpha < self.alpha_min {
            return;
        }

        let alpha = self.alpha;

        let mut forces = std::mem::take(&mut self.forces);
        for (_, force) in &mut forces {
            force.apply(&mut self.nodes, alpha);
        }
        self.forces = forces;

        let velocity_decay = self.velocity_decay;
        for node in &mut self.nodes {
            if let Some(fx) = node.fx {
                node.x = fx;
                node.vx = 0.0;
            } else {
                node.vx *= velocity_decay;
                node.x += node.vx;
            }
            if let Some(fy) = node.fy {
                node.y = fy;
                node.vy = 0.0;
            } else {
                node.vy *= velocity_decay;
                node.y += node.vy;
            }
        }
    }

    /// Reset alpha to 1.0, restarting the cool-down.
    pub fn reheat(&mut self) {
        self.alpha = DEFAULT_ALPHA;
    }

    // ── FlowState synchronisation ────────────────────────────────────────

    /// Read drag state from `FlowState` into the simulation.
    ///
    /// Returns `false` and skips the sync when the simulation's `SimNode`
    /// ids no longer match the `FlowState` node order — i.e. the state was
    /// mutated (add/remove) since the simulation was constructed. In that
    /// case the caller should rebuild the simulation via
    /// [`from_state`](Self::from_state) before continuing.
    pub fn sync_from_state<ND: Clone, ED: Clone>(
        &mut self,
        state: &FlowState<ND, ED>,
    ) -> bool {
        if !self.ids_match(state) {
            return false;
        }

        let mut any_dragging = false;

        for (sim, flow_node) in self.nodes.iter_mut().zip(state.nodes.iter()) {
            if flow_node.dragging {
                any_dragging = true;
                sim.fx = Some(flow_node.position.x);
                sim.fy = Some(flow_node.position.y);
            } else {
                sim.fx = None;
                sim.fy = None;
            }
        }

        if any_dragging {
            self.alpha_target = self.drag_alpha_target;
            if self.alpha < self.alpha_target {
                self.alpha = self.alpha_target;
            }
        } else {
            self.alpha_target = 0.0;
        }

        true
    }

    /// Push simulation positions back into the `FlowState` and rebuild the
    /// internal lookup cache. Returns `false` on id mismatch (see
    /// [`sync_from_state`](Self::sync_from_state)).
    pub fn sync_to_state<ND: Clone, ED: Clone>(
        &self,
        state: &mut FlowState<ND, ED>,
    ) -> bool {
        if !self.ids_match(state) {
            return false;
        }
        for (sim, flow_node) in self.nodes.iter().zip(state.nodes.iter_mut()) {
            if !flow_node.dragging {
                flow_node.position = egui::pos2(sim.x, sim.y);
            }
        }
        state.rebuild_lookup();
        true
    }

    /// Convenience: [`sync_from_state`](Self::sync_from_state) →
    /// [`tick`](Self::tick) → [`sync_to_state`](Self::sync_to_state). Returns
    /// `false` if a sync was skipped due to id mismatch — caller should
    /// rebuild the simulation.
    pub fn step<ND: Clone, ED: Clone>(&mut self, state: &mut FlowState<ND, ED>) -> bool {
        if !self.sync_from_state(state) {
            return false;
        }
        self.tick();
        self.sync_to_state(state)
    }

    // ── Drift detection ──────────────────────────────────────────────────

    fn ids_match<ND: Clone, ED: Clone>(&self, state: &FlowState<ND, ED>) -> bool {
        if self.nodes.len() != state.nodes.len() {
            log::warn!(
                "ForceSimulation has {} SimNodes but FlowState has {} nodes — \
                 rebuild via ForceSimulation::from_state",
                self.nodes.len(),
                state.nodes.len()
            );
            return false;
        }
        // Only verify when the caller went through `from_state` (so ids are
        // populated). Hand-built SimNodes keep the old zip-by-index
        // behaviour.
        for (sim, flow_node) in self.nodes.iter().zip(state.nodes.iter()) {
            if let Some(ref sid) = sim.id {
                if sid != &flow_node.id {
                    log::warn!(
                        "ForceSimulation id drift: sim={} flow={} — rebuild",
                        sid, flow_node.id
                    );
                    return false;
                }
            }
        }
        true
    }
}

// ── Utility ──────────────────────────────────────────────────────────────────

/// Distribute nodes in a phyllotaxis (sunflower) spiral.
///
/// A good default initial layout for force-directed graphs — spreads nodes
/// evenly so the simulation doesn't start from a degenerate configuration.
/// D3 uses `scale = 10.0`.
pub fn phyllotaxis_layout(nodes: &mut [SimNode], scale: f32) {
    let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    for (i, node) in nodes.iter_mut().enumerate() {
        let r = scale * (0.5 + i as f32).sqrt();
        let angle = i as f32 * golden_angle;
        node.x = r * angle.cos();
        node.y = r * angle.sin();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_decays_alpha() {
        let mut sim = ForceSimulation::new(vec![SimNode::new(0.0, 0.0)]);
        let a0 = sim.alpha();
        sim.tick();
        assert!(sim.alpha() < a0);
    }

    #[test]
    fn velocity_decay_damps_motion() {
        let mut nodes = vec![SimNode::new(0.0, 0.0)];
        nodes[0].vx = 10.0;
        let mut sim = ForceSimulation::new(nodes);
        sim.tick();
        // After one tick: x += vx * velocity_decay ≈ 6.0
        assert!(sim.nodes()[0].x < 10.0);
        assert!(sim.nodes()[0].x > 0.0);
    }

    #[test]
    fn pinned_node_stays_put() {
        let mut n = SimNode::new(5.0, 7.0);
        n.vx = 100.0;
        n.vy = 100.0;
        n.fx = Some(5.0);
        n.fy = Some(7.0);
        let mut sim = ForceSimulation::new(vec![n]);
        sim.tick();
        assert_eq!(sim.nodes()[0].x, 5.0);
        assert_eq!(sim.nodes()[0].y, 7.0);
        assert_eq!(sim.nodes()[0].vx, 0.0);
        assert_eq!(sim.nodes()[0].vy, 0.0);
    }

    #[test]
    fn reheat_resets_alpha() {
        let mut sim = ForceSimulation::new(vec![SimNode::new(0.0, 0.0)]);
        for _ in 0..50 {
            sim.tick();
        }
        let cooled = sim.alpha();
        sim.reheat();
        assert!(sim.alpha() > cooled);
        assert!((sim.alpha() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn phyllotaxis_is_nondegenerate() {
        let mut nodes: Vec<SimNode> = (0..20).map(|_| SimNode::new(0.0, 0.0)).collect();
        phyllotaxis_layout(&mut nodes, 10.0);
        // No two nodes should share the same position.
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let d = (nodes[i].x - nodes[j].x).hypot(nodes[i].y - nodes[j].y);
                assert!(d > 0.1, "degenerate spiral: {i} vs {j}");
            }
        }
    }
}
