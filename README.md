# egui_xyflow

An interactive node-graph editor widget for [egui](https://github.com/emilk/egui), inspired by [xyflow](https://xyflow.com/) (React Flow / Svelte Flow).

[![Crates.io](https://img.shields.io/crates/v/egui_xyflow.svg)](https://crates.io/crates/egui_xyflow)
[![docs.rs](https://docs.rs/egui_xyflow/badge.svg)](https://docs.rs/egui_xyflow)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- **Node graph editing** — drag nodes, create edges by dragging between handles, box-select, multi-select
- **Multiple edge types** — Bezier, SmoothStep, Step, Straight, SimpleBezier
- **Per-edge styling** — custom colours, stroke widths, glow effects
- **Animated edges** — dashed animation with configurable speed
- **Pan & zoom** — scroll wheel, pinch, double-click zoom, keyboard shortcuts
- **Minimap** — click/drag to navigate
- **Resize handles** — select a node to resize it
- **Edge anchors** — drag edge endpoints to reposition them on the node border
- **`Position::Closest`** — edge endpoints auto-track the nearest node side
- **Snap to grid** — optional grid snapping for node placement
- **Background patterns** — dots, lines, or cross patterns
- **Animated viewport transitions** — smooth pan/zoom with easing functions
- **Fully configurable** — 60+ options in `FlowConfig`
- **Serde support** — serialize/deserialize graph state (enabled by default)
- **Custom rendering** — implement `NodeWidget` or `EdgeWidget` for full control

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
egui_xyflow = "0.1"
eframe = "0.31"
```

Minimal example:

```rust,no_run
use eframe::egui;
use egui_xyflow::prelude::*;

struct MyApp {
    state: FlowState<String, ()>,
}

impl MyApp {
    fn new() -> Self {
        let mut state = FlowState::new(FlowConfig::default());

        state.add_node(
            Node::builder("1")
                .position(egui::pos2(100.0, 100.0))
                .data("Input".to_string())
                .handle(NodeHandle::source(Position::Right))
                .build(),
        );
        state.add_node(
            Node::builder("2")
                .position(egui::pos2(400.0, 100.0))
                .data("Output".to_string())
                .handle(NodeHandle::target(Position::Left))
                .build(),
        );
        state.add_edge(Edge::new("e1-2", "1", "2"));

        Self { state }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let events = FlowCanvas::new(&mut self.state, &DefaultNodeWidget).show(ui);
            // Inspect `events` for connections made, nodes clicked, etc.
            let _ = events;
        });
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "egui_xyflow",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}
```

## Architecture

Each frame follows: **State -> Canvas::show() -> Events -> Apply Changes -> State**

1. `FlowState<ND, ED>` holds all graph state (nodes, edges, viewport)
2. `FlowCanvas::new(&mut state, &widget).show(ui)` renders everything and returns `FlowEvents`
3. Inspect events for what happened (clicks, drags, connections made, etc.)
4. Optionally produce `NodeChange` / `EdgeChange` and call `apply_node_changes()` / `apply_edge_changes()`

## Examples

Run any of the 14 included examples:

```bash
cargo run --example basic_flow
cargo run --example data_pipeline
cargo run --example neural_network
cargo run --example dependency_graph
cargo run --example state_machine
cargo run --example logic_gates
cargo run --example radial_tree
cargo run --example collapsible_tree
cargo run --example sankey_diagram
cargo run --example chord_diagram
cargo run --example arc_diagram
cargo run --example temporal_force_graph
cargo run --example disjoint_force_graph
cargo run --example hierarchical_edge_bundling
```

## Per-Edge Styling & Glow

```rust,ignore
Edge::new("e1", "a", "b")
    .color(egui::Color32::from_rgb(59, 130, 246))
    .glow(egui::Color32::from_rgba_unmultiplied(59, 130, 246, 60), 12.0)
    .animated(true)
    .marker_end_arrow()
```

## Custom Node Rendering

Implement the `NodeWidget<D>` trait:

```rust,ignore
impl NodeWidget<MyData> for MyNodeRenderer {
    fn size(&self, node: &Node<MyData>, config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(200.0, 80.0)
    }

    fn show(&self, painter: &egui::Painter, node: &Node<MyData>,
            screen_rect: egui::Rect, config: &FlowConfig,
            hovered: bool, transform: &Transform) {
        // Custom rendering with the egui Painter
    }
}
```

## Connection Validation

```rust,ignore
struct MyValidator;

impl ConnectionValidator for MyValidator {
    fn is_valid_connection(&self, connection: &Connection,
                           existing_edges: &[EdgeInfo<'_>]) -> bool {
        // Enforce type compatibility, prevent cycles, etc.
        true
    }
}

FlowCanvas::new(&mut state, &widget)
    .connection_validator(&MyValidator)
    .show(ui);
```

## Compatibility

| egui_xyflow | egui  |
|-------------|-------|
| 0.1         | 0.31  |

## License

MIT
