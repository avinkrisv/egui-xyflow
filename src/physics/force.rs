//! The [`Force`] trait — implement this to create custom forces.

use super::sim_node::SimNode;

/// A force that acts on simulation nodes each tick.
///
/// Forces modify node **velocities** (`vx`, `vy`) — they should not set
/// positions directly. The simulation integrates velocities into positions
/// after all forces have been applied.
///
/// # Implementing a custom force
///
/// ```rust,ignore
/// use egui_xyflow::physics::{Force, SimNode};
///
/// struct GravityWell {
///     center_x: f32,
///     center_y: f32,
///     strength: f32,
/// }
///
/// impl Force for GravityWell {
///     fn apply(&mut self, nodes: &mut [SimNode], alpha: f32) {
///         for node in nodes.iter_mut().filter(|n| n.fx.is_none()) {
///             node.vx += (self.center_x - node.x) * self.strength * alpha;
///             node.vy += (self.center_y - node.y) * self.strength * alpha;
///         }
///     }
/// }
/// ```
pub trait Force {
    /// Apply this force to the simulation nodes.
    ///
    /// `alpha` is the simulation's current cooling factor (starts at 1.0,
    /// decays toward 0). Multiply velocity deltas by `alpha` so forces
    /// weaken as the simulation settles.
    fn apply(&mut self, nodes: &mut [SimNode], alpha: f32);
}
