//! Many-body (charge) force — pairwise repulsion or attraction, approximated
//! with a Barnes–Hut quadtree.
//!
//! Matches D3's `forceManyBody()` semantics:
//!
//! * Per-body strength (D3's `strength.accessor`), negative for repulsion.
//! * Inverse-distance force: `dv = dx * strength * alpha / l`.
//! * Barnes–Hut aggregation with threshold θ (default 0.9 ↔ θ² = 0.81).
//! * Distance clamping via `distance_min` / `distance_max`.
//!
//! Cost is O(n log n) average for well-distributed inputs; degrades to
//! O(n²) only for pathological configurations.

use super::force::Force;
use super::quadtree::QuadTree;
use super::sim_node::SimNode;

/// Pairwise many-body force. Default: repulsion with strength −30, θ = 0.9.
pub struct ManyBodyForce {
    strength: f32,
    theta2: f32,
    distance_min_sq: f32,
    distance_max_sq: f32,
}

impl Default for ManyBodyForce {
    fn default() -> Self {
        Self {
            strength: -30.0,
            theta2: 0.81,
            distance_min_sq: 1.0,
            distance_max_sq: f32::INFINITY,
        }
    }
}

impl ManyBodyForce {
    /// Create with D3 defaults: strength −30, θ = 0.9, `distance_min` = 1,
    /// `distance_max` = ∞.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default strength. Per-node overrides via
    /// [`SimNode::strength`](super::SimNode::strength) take precedence.
    pub fn strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }

    /// Set the Barnes–Hut threshold θ. Smaller = more accurate, slower.
    /// Default: 0.9 (D3). Values above ~1.0 trade significant accuracy for
    /// speed.
    pub fn theta(mut self, theta: f32) -> Self {
        self.theta2 = theta * theta;
        self
    }

    /// Set the minimum interaction distance (clamped from below). Default: 1.0.
    pub fn distance_min(mut self, dist: f32) -> Self {
        self.distance_min_sq = dist * dist;
        self
    }

    /// Set the maximum interaction distance. Pairs further apart are ignored.
    pub fn distance_max(mut self, dist: f32) -> Self {
        self.distance_max_sq = dist * dist;
        self
    }

    /// Mutator for [`Self::strength`].
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength;
    }

    /// Current default strength.
    pub fn get_strength(&self) -> f32 {
        self.strength
    }
}

impl Force for ManyBodyForce {
    fn apply(&mut self, nodes: &mut [SimNode], alpha: f32) {
        let n = nodes.len();
        if n < 2 {
            return;
        }

        // Effective per-node strength: per-node override if set, else force-level default.
        let points: Vec<(f32, f32, f32)> = nodes
            .iter()
            .map(|nd| (nd.x, nd.y, nd.strength.unwrap_or(self.strength)))
            .collect();

        let tree = QuadTree::new(&points);
        let theta2 = self.theta2;
        let dmin2 = self.distance_min_sq;
        let dmax2 = self.distance_max_sq;

        for i in 0..n {
            if nodes[i].fx.is_some() && nodes[i].fy.is_some() {
                continue;
            }
            let qx = nodes[i].x;
            let qy = nodes[i].y;

            let mut dvx = 0.0_f32;
            let mut dvy = 0.0_f32;

            tree.visit_approx(qx, qy, i, theta2, |px, py, value, is_leaf, leaf_idx| {
                // Skip self-interaction (leaf point = query point).
                if is_leaf && leaf_idx == Some(i) {
                    return;
                }
                let dx = px - qx;
                let dy = py - qy;
                let mut l = dx * dx + dy * dy;
                if l >= dmax2 {
                    return;
                }
                if l == 0.0 {
                    // Tiny deterministic jitter for coincident pairs.
                    let jx = 1.0e-6 * ((i * 131) % 1000) as f32 - 0.5e-3;
                    let jy = 1.0e-6 * ((i * 97) % 1000) as f32 - 0.5e-3;
                    dvx += jx * value * alpha;
                    dvy += jy * value * alpha;
                    return;
                }
                if l < dmin2 {
                    l = (dmin2 * l).sqrt();
                }
                let f = value * alpha / l;
                dvx += dx * f;
                dvy += dy * f;
            });

            nodes[i].vx += dvx;
            nodes[i].vy += dvy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact O(n²) reference implementation, used to validate the
    /// Barnes–Hut approximation.
    fn apply_exact(nodes: &mut [SimNode], strength: f32, alpha: f32) {
        let pos: Vec<(f32, f32, f32)> = nodes
            .iter()
            .map(|nd| (nd.x, nd.y, nd.strength.unwrap_or(strength)))
            .collect();
        let n = nodes.len();
        let dmin2 = 1.0_f32;
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let (jx, jy, jv) = pos[j];
                let (ix, iy, _) = pos[i];
                let dx = jx - ix;
                let dy = jy - iy;
                let mut l = dx * dx + dy * dy;
                if l == 0.0 {
                    continue;
                }
                if l < dmin2 {
                    l = (dmin2 * l).sqrt();
                }
                let f = jv * alpha / l;
                nodes[i].vx += dx * f;
                nodes[i].vy += dy * f;
            }
        }
    }

    #[test]
    fn two_body_repulsion_pushes_apart() {
        let mut nodes = vec![SimNode::new(-5.0, 0.0), SimNode::new(5.0, 0.0)];
        let mut f = ManyBodyForce::new().strength(-30.0);
        f.apply(&mut nodes, 1.0);
        // Left node accelerates left; right node accelerates right.
        assert!(nodes[0].vx < 0.0);
        assert!(nodes[1].vx > 0.0);
        // y-velocity unchanged.
        assert!(nodes[0].vy.abs() < 1e-4);
        assert!(nodes[1].vy.abs() < 1e-4);
    }

    #[test]
    fn two_body_attraction_pulls_together() {
        let mut nodes = vec![SimNode::new(-5.0, 0.0), SimNode::new(5.0, 0.0)];
        let mut f = ManyBodyForce::new().strength(30.0);
        f.apply(&mut nodes, 1.0);
        assert!(nodes[0].vx > 0.0);
        assert!(nodes[1].vx < 0.0);
    }

    #[test]
    fn barnes_hut_matches_exact_within_tolerance() {
        use std::f32::consts::PI;
        // 80 points on a Cartesian lattice.
        let mut a: Vec<SimNode> = Vec::new();
        let mut b: Vec<SimNode> = Vec::new();
        for i in 0..10 {
            for j in 0..8 {
                let x = -50.0 + 10.0 * i as f32 + (i as f32 * PI).sin();
                let y = -40.0 + 10.0 * j as f32 + (j as f32 * PI).cos();
                a.push(SimNode::new(x, y));
                b.push(SimNode::new(x, y));
            }
        }

        let mut bh = ManyBodyForce::new().strength(-30.0).theta(0.9);
        bh.apply(&mut a, 1.0);
        apply_exact(&mut b, -30.0, 1.0);

        // Compare per-node velocity deltas. Barnes–Hut at θ=0.9 should
        // track exact within a modest tolerance.
        let mut max_err_ratio = 0.0_f32;
        let mut mean_err_ratio = 0.0_f32;
        let mut count = 0_usize;
        for (ai, bi) in a.iter().zip(b.iter()) {
            let mag = (bi.vx * bi.vx + bi.vy * bi.vy).sqrt().max(1e-6);
            let dx = ai.vx - bi.vx;
            let dy = ai.vy - bi.vy;
            let err = (dx * dx + dy * dy).sqrt() / mag;
            max_err_ratio = max_err_ratio.max(err);
            mean_err_ratio += err;
            count += 1;
        }
        mean_err_ratio /= count as f32;
        // D3 uses θ=0.9; typical error ratio well under 15 % and mean
        // under a few percent for benign configurations like this one.
        assert!(max_err_ratio < 0.20, "max err ratio {max_err_ratio}");
        assert!(mean_err_ratio < 0.05, "mean err ratio {mean_err_ratio}");
    }

    #[test]
    fn pinned_node_is_unaffected() {
        let mut n = SimNode::new(0.0, 0.0);
        n.fx = Some(0.0);
        n.fy = Some(0.0);
        let mut nodes = vec![n, SimNode::new(10.0, 0.0)];
        let mut f = ManyBodyForce::new().strength(-30.0);
        f.apply(&mut nodes, 1.0);
        assert_eq!(nodes[0].vx, 0.0);
        assert_eq!(nodes[0].vy, 0.0);
    }

    #[test]
    fn per_node_strength_overrides_default() {
        // Node 1 is "heavier" (stronger repulsion), node 0 gets pushed more.
        let mut nodes = vec![SimNode::new(-5.0, 0.0), SimNode {
            strength: Some(-300.0),
            ..SimNode::new(5.0, 0.0)
        }];
        let mut f = ManyBodyForce::new().strength(-30.0);
        f.apply(&mut nodes, 1.0);

        let mut baseline = vec![SimNode::new(-5.0, 0.0), SimNode::new(5.0, 0.0)];
        let mut g = ManyBodyForce::new().strength(-30.0);
        g.apply(&mut baseline, 1.0);

        assert!(
            nodes[0].vx.abs() > baseline[0].vx.abs(),
            "heavier neighbour should push node 0 harder: {} vs {}",
            nodes[0].vx,
            baseline[0].vx
        );
    }

    #[test]
    fn distance_max_ignores_far_bodies() {
        let mut close = vec![SimNode::new(-1.0, 0.0), SimNode::new(1.0, 0.0)];
        let mut far = vec![SimNode::new(-1000.0, 0.0), SimNode::new(1000.0, 0.0)];
        let mut f = ManyBodyForce::new().strength(-30.0).distance_max(100.0);
        f.apply(&mut close, 1.0);
        f.apply(&mut far, 1.0);
        assert!(close[0].vx != 0.0);
        assert_eq!(far[0].vx, 0.0);
        assert_eq!(far[1].vx, 0.0);
    }
}
