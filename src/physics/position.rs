//! Position force — pulls nodes toward a target point.
//!
//! Matches D3's `forceX()` / `forceY()` combined into a single force.
//! Useful for keeping disjoint components in view.

use super::force::Force;
use super::sim_node::SimNode;

/// Pulls all nodes toward a target point. Default: origin `(0, 0)`, strength 0.1.
///
/// # Examples
///
/// ```rust,ignore
/// use egui_xyflow::physics::PositionForce;
///
/// // Pull toward origin (D3 default)
/// let center = PositionForce::new();
///
/// // Pull toward a specific point with custom strength
/// let toward = PositionForce::new().target(200.0, 100.0).strength(0.05);
/// ```
pub struct PositionForce {
    target_x: f32,
    target_y: f32,
    strength: f32,
}

impl Default for PositionForce {
    fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            strength: 0.1,
        }
    }
}

impl PositionForce {
    /// Create with D3 defaults: target `(0, 0)`, strength 0.1.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target position that nodes are pulled toward.
    pub fn target(mut self, x: f32, y: f32) -> Self {
        self.target_x = x;
        self.target_y = y;
        self
    }

    /// Set the pull strength. Higher = faster convergence, lower = gentler.
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }

    /// Mutator for [`Self::target`].
    pub fn set_target(&mut self, x: f32, y: f32) {
        self.target_x = x;
        self.target_y = y;
    }

    /// Mutator for [`Self::strength`].
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength;
    }

    /// Current pull strength.
    pub fn get_strength(&self) -> f32 {
        self.strength
    }
}

impl Force for PositionForce {
    fn apply(&mut self, nodes: &mut [SimNode], alpha: f32) {
        let strength = self.strength;
        let tx = self.target_x;
        let ty = self.target_y;

        for node in nodes.iter_mut().filter(|n| n.fx.is_none()) {
            node.vx += (tx - node.x) * strength * alpha;
            node.vy += (ty - node.y) * strength * alpha;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::ForceSimulation;

    #[test]
    fn applies_pull_toward_target() {
        let mut nodes = vec![SimNode::new(100.0, -50.0)];
        let mut f = PositionForce::new().target(0.0, 0.0).strength(0.5);
        f.apply(&mut nodes, 1.0);
        // Node to the right of target → -x velocity. Above target → +y.
        assert!(nodes[0].vx < 0.0);
        assert!(nodes[0].vy > 0.0);
    }

    #[test]
    fn converges_to_target_under_repeated_ticks() {
        let sim = ForceSimulation::new(vec![SimNode::new(200.0, 100.0)])
            .add_force("pos", PositionForce::new().target(0.0, 0.0).strength(0.3));
        let mut sim = sim;
        for _ in 0..600 {
            sim.tick();
        }
        let n = &sim.nodes()[0];
        assert!(n.x.abs() < 5.0, "x={}", n.x);
        assert!(n.y.abs() < 5.0, "y={}", n.y);
    }

    #[test]
    fn does_not_affect_pinned_nodes() {
        let mut n = SimNode::new(100.0, 0.0);
        n.fx = Some(100.0);
        n.fy = Some(0.0);
        let mut nodes = vec![n];
        let mut f = PositionForce::new().target(0.0, 0.0).strength(1.0);
        f.apply(&mut nodes, 1.0);
        assert_eq!(nodes[0].vx, 0.0);
        assert_eq!(nodes[0].vy, 0.0);
    }
}
