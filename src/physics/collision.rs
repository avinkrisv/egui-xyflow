//! Collision force — prevents node overlap using circle radii.
//!
//! Inspired by D3's `forceCollide()`. Each node is treated as a circle
//! with its [`SimNode::radius`](super::SimNode::radius); overlapping pairs
//! are pushed apart.

use super::force::Force;
use super::sim_node::SimNode;

/// Collision avoidance force. Pushes overlapping nodes apart based on their
/// radii. Default: strength 1.0, 1 iteration, uses per-node `radius`.
///
/// # Examples
///
/// ```rust,ignore
/// use egui_xyflow::physics::CollisionForce;
///
/// // Use each node's radius
/// let collision = CollisionForce::new();
///
/// // Override all radii to a fixed value
/// let uniform = CollisionForce::new().radius(10.0);
///
/// // Higher iterations = more accurate separation per tick
/// let precise = CollisionForce::new().iterations(3);
/// ```
pub struct CollisionForce {
    strength: f32,
    iterations: usize,
    radius_override: Option<f32>,
}

impl Default for CollisionForce {
    fn default() -> Self {
        Self {
            strength: 1.0,
            iterations: 1,
            radius_override: None,
        }
    }
}

impl CollisionForce {
    /// Create with defaults: strength 1.0, 1 iteration, per-node radius.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the separation strength (0.0–1.0). Default: 1.0.
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }

    /// Set the number of relaxation iterations per tick. Default: 1.
    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Override every node's collision radius with a fixed value.
    /// When `None` (default), each node's own `radius` field is used.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius_override = Some(radius);
        self
    }

    /// Mutator for [`Self::strength`].
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength;
    }
}

impl Force for CollisionForce {
    fn apply(&mut self, nodes: &mut [SimNode], _alpha: f32) {
        let n = nodes.len();
        let strength = self.strength;
        let radius_override = self.radius_override;

        for _ in 0..self.iterations {
            let state: Vec<(f32, f32, f32)> = nodes
                .iter()
                .map(|nd| {
                    let r = radius_override.unwrap_or(nd.radius);
                    (nd.x, nd.y, r)
                })
                .collect();

            for i in 0..n {
                let (xi, yi, ri) = state[i];

                for j in (i + 1)..n {
                    let (xj, yj, rj) = state[j];
                    let dx = xj - xi;
                    let dy = yj - yi;
                    let dist_sq = dx * dx + dy * dy;
                    let min_dist = ri + rj;
                    let min_dist_sq = min_dist * min_dist;

                    if dist_sq >= min_dist_sq || dist_sq == 0.0 {
                        continue;
                    }

                    let dist = dist_sq.sqrt();
                    let overlap = (min_dist - dist) / dist * strength * 0.5;
                    let ox = dx * overlap;
                    let oy = dy * overlap;

                    if nodes[i].fx.is_none() {
                        nodes[i].vx -= ox;
                        nodes[i].vy -= oy;
                    }
                    if nodes[j].fx.is_none() {
                        nodes[j].vx += ox;
                        nodes[j].vy += oy;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_pair_is_pushed_apart() {
        let mut nodes = vec![
            SimNode::new(0.0, 0.0).with_radius(5.0),
            SimNode::new(3.0, 0.0).with_radius(5.0),
        ];
        let mut f = CollisionForce::new();
        f.apply(&mut nodes, 1.0);
        assert!(nodes[0].vx < 0.0);
        assert!(nodes[1].vx > 0.0);
    }

    #[test]
    fn non_overlapping_pair_is_left_alone() {
        let mut nodes = vec![
            SimNode::new(0.0, 0.0).with_radius(1.0),
            SimNode::new(100.0, 0.0).with_radius(1.0),
        ];
        let mut f = CollisionForce::new();
        f.apply(&mut nodes, 1.0);
        assert_eq!(nodes[0].vx, 0.0);
        assert_eq!(nodes[1].vx, 0.0);
    }

    #[test]
    fn radius_override_wins() {
        let mut nodes = vec![
            SimNode::new(0.0, 0.0).with_radius(0.1),
            SimNode::new(3.0, 0.0).with_radius(0.1),
        ];
        let mut f = CollisionForce::new().radius(5.0);
        f.apply(&mut nodes, 1.0);
        // At distance 3 with override radius 5, they overlap and push apart.
        assert!(nodes[0].vx < 0.0);
        assert!(nodes[1].vx > 0.0);
    }
}
