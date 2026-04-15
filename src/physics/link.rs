//! Link (spring) force — pulls connected nodes toward a target distance.
//!
//! Matches D3's `forceLink()` semantics: degree-based strength and bias,
//! with velocity-aware displacement.

use std::collections::HashMap;

use super::force::Force;
use super::sim_node::SimNode;
use crate::state::flow_state::FlowState;
use crate::types::node::NodeId;

/// A single link in the simulation, connecting two nodes by index.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimLink {
    /// Source node index.
    pub source: usize,
    /// Target node index.
    pub target: usize,
    /// Spring strength for this link (D3: `1 / min(degree_source, degree_target)`).
    pub strength: f32,
    /// Bias toward moving the target vs. the source (D3: `degree_source / (degree_source + degree_target)`).
    pub bias: f32,
}

/// Spring force between linked node pairs. Default distance: 30.
///
/// # Examples
///
/// ```rust,ignore
/// use egui_xyflow::physics::LinkForce;
///
/// // From FlowState edges (degree-based strength computed automatically)
/// let links = LinkForce::from_state(&state).distance(50.0);
///
/// // From raw index pairs
/// let links = LinkForce::from_pairs(&[(0, 1), (1, 2)], 3);
/// ```
pub struct LinkForce {
    links: Vec<SimLink>,
    distance: f32,
}

impl LinkForce {
    /// Create from pre-built [`SimLink`]s. Default distance: 30.
    pub fn new(links: Vec<SimLink>) -> Self {
        Self {
            links,
            distance: 30.0,
        }
    }

    /// Create from raw `(source_idx, target_idx)` pairs, computing D3-style
    /// degree-based strength and bias automatically.
    pub fn from_pairs(pairs: &[(usize, usize)], node_count: usize) -> Self {
        let mut degree = vec![0_usize; node_count];
        for &(s, t) in pairs {
            degree[s] += 1;
            degree[t] += 1;
        }

        let links = pairs
            .iter()
            .map(|&(s, t)| {
                let ds = degree[s].max(1);
                let dt = degree[t].max(1);
                SimLink {
                    source: s,
                    target: t,
                    strength: 1.0 / ds.min(dt) as f32,
                    bias: ds as f32 / (ds + dt) as f32,
                }
            })
            .collect();

        Self {
            links,
            distance: 30.0,
        }
    }

    /// Create from a [`FlowState`], extracting edge connectivity and computing
    /// D3-style degree-based strength and bias.
    pub fn from_state<ND: Clone, ED: Clone>(state: &FlowState<ND, ED>) -> Self {
        let id_to_idx: HashMap<&NodeId, usize> = state
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (&n.id, i))
            .collect();

        let n = state.nodes.len();
        let mut degree = vec![0_usize; n];
        let mut pairs = Vec::with_capacity(state.edges.len());

        for edge in &state.edges {
            if let (Some(&si), Some(&ti)) =
                (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target))
            {
                degree[si] += 1;
                degree[ti] += 1;
                pairs.push((si, ti));
            }
        }

        let links = pairs
            .iter()
            .map(|&(s, t)| {
                let ds = degree[s].max(1);
                let dt = degree[t].max(1);
                SimLink {
                    source: s,
                    target: t,
                    strength: 1.0 / ds.min(dt) as f32,
                    bias: ds as f32 / (ds + dt) as f32,
                }
            })
            .collect();

        Self {
            links,
            distance: 30.0,
        }
    }

    /// Set the target rest-length for all links. Default: 30.
    pub fn distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance;
    }

    pub fn get_distance(&self) -> f32 {
        self.distance
    }

    /// Read-only access to the computed links.
    pub fn links(&self) -> &[SimLink] {
        &self.links
    }
}

impl Force for LinkForce {
    fn apply(&mut self, nodes: &mut [SimNode], alpha: f32) {
        // Snapshot to avoid aliasing: each link reads source+target state and
        // writes velocity to both ends.
        let state: Vec<(f32, f32, f32, f32)> =
            nodes.iter().map(|n| (n.x, n.y, n.vx, n.vy)).collect();

        let distance = self.distance;

        for link in &self.links {
            let (sx, sy, svx, svy) = state[link.source];
            let (tx, ty, tvx, tvy) = state[link.target];

            let mut dx = tx + tvx - sx - svx;
            let mut dy = ty + tvy - sy - svy;

            if dx == 0.0 {
                dx = 1.0e-6;
            }
            if dy == 0.0 {
                dy = 1.0e-6;
            }

            let l = (dx * dx + dy * dy).sqrt();
            let f = (l - distance) / l * alpha * link.strength;
            let fx = dx * f;
            let fy = dy * f;

            let b = link.bias;
            if nodes[link.target].fx.is_none() {
                nodes[link.target].vx -= fx * b;
                nodes[link.target].vy -= fy * b;
            }
            if nodes[link.source].fx.is_none() {
                nodes[link.source].vx += fx * (1.0 - b);
                nodes[link.source].vy += fy * (1.0 - b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::ForceSimulation;

    #[test]
    fn spring_pulls_far_apart_nodes_closer() {
        let mut nodes = vec![SimNode::new(0.0, 0.0), SimNode::new(100.0, 0.0)];
        let mut f = LinkForce::from_pairs(&[(0, 1)], 2).distance(30.0);
        f.apply(&mut nodes, 1.0);
        // At distance 100 vs. target 30, source accelerates +x, target -x.
        assert!(nodes[0].vx > 0.0);
        assert!(nodes[1].vx < 0.0);
    }

    #[test]
    fn spring_pushes_too_close_nodes_apart() {
        let mut nodes = vec![SimNode::new(0.0, 0.0), SimNode::new(5.0, 0.0)];
        let mut f = LinkForce::from_pairs(&[(0, 1)], 2).distance(30.0);
        f.apply(&mut nodes, 1.0);
        // At distance 5 vs. target 30, the spring pushes them apart.
        assert!(nodes[0].vx < 0.0);
        assert!(nodes[1].vx > 0.0);
    }

    #[test]
    fn repeated_ticks_converge_to_target_distance() {
        let mut nodes = vec![SimNode::new(0.0, 0.0), SimNode::new(200.0, 0.0)];
        let mut sim = ForceSimulation::new(std::mem::take(&mut nodes))
            .add_force("links", LinkForce::from_pairs(&[(0, 1)], 2).distance(50.0));
        for _ in 0..400 {
            sim.tick();
        }
        let n = sim.nodes();
        let d = (n[1].x - n[0].x).hypot(n[1].y - n[0].y);
        assert!((d - 50.0).abs() < 2.0, "converged distance {d}");
    }

    #[test]
    fn degree_based_strength() {
        // Star graph: node 0 connected to 1, 2, 3. Degree of 0 is 3, others 1.
        // D3 strength = 1 / min(ds, dt). For (0,1) = 1 / min(3,1) = 1.
        let f = LinkForce::from_pairs(&[(0, 1), (0, 2), (0, 3)], 4);
        for l in f.links() {
            assert_eq!(l.strength, 1.0);
        }
    }
}
