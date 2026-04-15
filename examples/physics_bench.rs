//! Simple timing harness for the physics subsystem.
//!
//! Measures per-tick cost of a full charge + link + position simulation at
//! increasing N, so you can see the shape of the cost curve. Barnes–Hut
//! should give roughly O(n log n) scaling for the charge force; the link
//! force is linear in the edge count; collision is O(n²) in the current
//! implementation and will dominate at very large N.
//!
//! This is a deliberately low-ceremony bench — we use `std::time::Instant`
//! so the crate doesn't take on a criterion dev-dep. Numbers will vary
//! across machines; use it for relative comparisons.
//!
//! Run with: `cargo run --release --example physics_bench`

use std::time::Instant;

use egui_xyflow::physics::{
    phyllotaxis_layout, CollisionForce, ForceSimulation, LinkForce, ManyBodyForce, PositionForce,
    SimNode,
};

fn build_sim(n: usize, edges: &[(usize, usize)]) -> ForceSimulation {
    let mut nodes: Vec<SimNode> = (0..n).map(|_| SimNode::new(0.0, 0.0).with_radius(5.0)).collect();
    phyllotaxis_layout(&mut nodes, 10.0);
    ForceSimulation::new(nodes)
        .add_force("charge", ManyBodyForce::new().strength(-30.0))
        .add_force("links", LinkForce::from_pairs(edges, n).distance(30.0))
        .add_force("collide", CollisionForce::new())
        .add_force("pos", PositionForce::new().strength(0.1))
}

fn ring_edges(n: usize) -> Vec<(usize, usize)> {
    (0..n).map(|i| (i, (i + 1) % n)).collect()
}

fn bench(n: usize, ticks: usize) {
    let edges = ring_edges(n);
    let mut sim = build_sim(n, &edges);
    // Warm-up tick (first tick includes quadtree allocation heat).
    sim.tick();
    let start = Instant::now();
    for _ in 0..ticks {
        sim.tick();
    }
    let elapsed = start.elapsed();
    let per_tick_us = elapsed.as_micros() as f64 / ticks as f64;
    let per_node_ns = per_tick_us * 1000.0 / n as f64;
    println!(
        "  N={n:>5}  ticks={ticks:>4}  total={:>8.2?}  per-tick={per_tick_us:>8.1} µs  per-tick-per-node={per_node_ns:>6.1} ns",
        elapsed
    );
}

fn main() {
    println!("egui_xyflow physics bench (release mode strongly recommended)");
    println!("Stack: ManyBody (Barnes–Hut θ=0.9) + Link + Collision + Position\n");

    for &(n, ticks) in &[
        (100_usize, 200_usize),
        (500, 200),
        (1_000, 100),
        (2_000, 50),
        (5_000, 20),
    ] {
        bench(n, ticks);
    }
}
