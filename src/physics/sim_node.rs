//! Simulation node state used by the force simulation.

use crate::types::node::NodeId;

/// A node in the force simulation, holding position, velocity, and optional
/// fixed coordinates.
///
/// When created via [`ForceSimulation::from_state`](super::ForceSimulation::from_state)
/// the `id` field carries the corresponding [`NodeId`] so the simulation can
/// detect index drift if the `FlowState` has nodes added or removed between
/// ticks. Nodes built manually via [`SimNode::new`] have `id = None`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimNode {
    /// Position x in flow space.
    pub x: f32,
    /// Position y in flow space.
    pub y: f32,
    /// Velocity x (pixels per tick).
    pub vx: f32,
    /// Velocity y (pixels per tick).
    pub vy: f32,
    /// When `Some`, the node is pinned at this x (e.g. during a drag).
    /// Velocity integration is skipped and `vx` is zeroed each tick.
    pub fx: Option<f32>,
    /// When `Some`, the node is pinned at this y (e.g. during a drag).
    /// Velocity integration is skipped and `vy` is zeroed each tick.
    pub fy: Option<f32>,
    /// Collision radius for [`CollisionForce`](super::CollisionForce) and
    /// default size hint for other forces that care about size.
    pub radius: f32,
    /// Per-node charge strength override for [`ManyBodyForce`](super::ManyBodyForce).
    /// When `None`, the many-body strength configured on the force is used.
    pub strength: Option<f32>,
    /// Optional identifier linking this `SimNode` back to a `FlowState` node.
    /// Populated by `ForceSimulation::from_state`; `None` for hand-built nodes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub id: Option<NodeId>,
}

impl SimNode {
    /// Create a node at the given position with zero velocity, radius 1, no id.
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            fx: None,
            fy: None,
            radius: 1.0,
            strength: None,
            id: None,
        }
    }

    /// Set the collision radius (builder style).
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Attach a [`NodeId`] (builder style).
    pub fn with_id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set a per-node many-body strength override (builder style).
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = Some(strength);
        self
    }
}

impl Default for SimNode {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}
