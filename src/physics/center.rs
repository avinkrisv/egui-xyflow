//! Center force — rigidly translates the system so the mean stays at a target.
//!
//! Matches D3's `forceCenter()` semantics. Unlike
//! [`PositionForce`](super::PositionForce), this does not apply a per-node
//! spring: it computes the centroid each tick and
//! shifts every (un-pinned) node by the same offset. No effect on relative
//! layout — only on where the whole graph sits on the canvas.

use super::force::Force;
use super::sim_node::SimNode;

/// Rigidly recentres the simulation so its centroid approaches a target.
///
/// # Examples
///
/// ```rust,ignore
/// use egui_xyflow::physics::CenterForce;
///
/// // Keep the graph centred at the origin (D3 default).
/// let center = CenterForce::new();
///
/// // Anchor the centroid at (500, 300) with weaker strength.
/// let c = CenterForce::new().target(500.0, 300.0).strength(0.5);
/// ```
pub struct CenterForce {
    target_x: f32,
    target_y: f32,
    strength: f32,
}

impl Default for CenterForce {
    fn default() -> Self {
        Self { target_x: 0.0, target_y: 0.0, strength: 1.0 }
    }
}

impl CenterForce {
    /// Create with D3 defaults: target `(0, 0)`, strength 1.0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target point the centroid is rigidly shifted toward.
    pub fn target(mut self, x: f32, y: f32) -> Self {
        self.target_x = x;
        self.target_y = y;
        self
    }

    /// Set the fraction of the centroid offset applied per tick. Default: 1.0
    /// (full snap, matching D3). Values below 1.0 ease toward the target over
    /// multiple ticks.
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
}

impl Force for CenterForce {
    fn apply(&mut self, nodes: &mut [SimNode], _alpha: f32) {
        let n = nodes.len();
        if n == 0 {
            return;
        }
        let (mut sx, mut sy) = (0.0_f32, 0.0_f32);
        for node in nodes.iter() {
            sx += node.x;
            sy += node.y;
        }
        let inv_n = 1.0 / n as f32;
        let shift_x = (self.target_x - sx * inv_n) * self.strength;
        let shift_y = (self.target_y - sy * inv_n) * self.strength;
        for node in nodes.iter_mut() {
            // Rigid translation of positions directly (matches D3 forceCenter).
            // Pinned nodes follow too — D3's behaviour; the pin is reapplied
            // during integration from `fx`/`fy`.
            node.x += shift_x;
            node.y += shift_y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centroid_moves_toward_target_with_unit_strength() {
        let mut nodes = vec![
            SimNode::new(10.0, 10.0),
            SimNode::new(20.0, 30.0),
            SimNode::new(30.0, 50.0),
        ];
        let mut f = CenterForce::new().target(0.0, 0.0).strength(1.0);
        f.apply(&mut nodes, 1.0);
        let cx: f32 = nodes.iter().map(|n| n.x).sum::<f32>() / nodes.len() as f32;
        let cy: f32 = nodes.iter().map(|n| n.y).sum::<f32>() / nodes.len() as f32;
        assert!(cx.abs() < 1e-4, "cx={cx}");
        assert!(cy.abs() < 1e-4, "cy={cy}");
    }

    #[test]
    fn relative_distances_preserved() {
        let mut nodes = vec![SimNode::new(0.0, 0.0), SimNode::new(10.0, 0.0)];
        let before = nodes[1].x - nodes[0].x;
        let mut f = CenterForce::new().target(100.0, 100.0);
        f.apply(&mut nodes, 1.0);
        let after = nodes[1].x - nodes[0].x;
        assert!((before - after).abs() < 1e-4);
    }
}
