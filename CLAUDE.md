# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**egui_xyflow** is a Rust library providing an interactive node graph editor widget for [egui](https://github.com/emilk/egui), inspired by [xyflow](https://xyflow.com/) (React Flow). It supports node dragging, edge connections, pan/zoom, selection, minimap, and animated edges.

## Build & Run Commands

```bash
cargo build                        # Build the library
cargo test                         # Run tests
cargo run --example basic_flow     # Run the demo application
cargo doc --open                   # Generate and open documentation
```

## Architecture

### Generic Parameters

The library is parameterized over two types used throughout:
- `ND` — custom user data attached to nodes (defaults to `()`)
- `ED` — custom user data attached to edges (defaults to `()`)

### Core Data Flow

Each frame follows: **State → Canvas::show() → Events → Apply Changes → State**

1. `FlowState<ND, ED>` holds all graph state (nodes, edges, viewport, connection state)
2. `FlowCanvas::show(&mut ui)` renders everything and returns `FlowEvents`
3. Events contain what happened (clicks, drags, connections made, etc.)
4. User code inspects events and produces `NodeChange<D>` / `EdgeChange<D>` enums
5. `apply_node_changes()` / `apply_edge_changes()` mutate state atomically

### Module Layout

- **`types/`** — Core data types: `Node<D>`, `Edge<D>`, `Handle`, `Position`, `Viewport`, `Connection`, `NodeChange`, `EdgeChange`
- **`state/`** — `FlowState` central container, change application logic, internal node lookup cache
- **`render/`** — All rendering: `FlowCanvas` (orchestrator), node/edge/handle/background/minimap/selection/marker renderers
- **`interaction/`** — Input handling: node drag, pan/zoom, connection drag, resize, box selection
- **`edges/`** — Edge path algorithms: Bezier, SmoothStep, Straight, Step
- **`graph/`** — Utilities: bounds calculation, coordinate transforms (screen↔flow space), neighbor queries
- **`animation/`** — Viewport animation with easing functions

### Key Traits for Customization

- `NodeWidget<D>` (`render/node_renderer.rs`) — custom node rendering via `size()` and `show()`
- `ConnectionValidator` (`render/canvas.rs`) — validate prospective connections with `is_valid_connection()`
- `EdgeWidget<ED>` (`render/canvas.rs`) — custom edge path rendering

### Render Order (bottom to top)

Background → Edges → Connection drag line → Nodes (z-ordered) → Handles → Resize handles → Selection rectangle → Minimap

### Coordinate System

Two coordinate spaces with conversions in `graph/node_position.rs`:
- **Screen space** — UI pixel coordinates
- **Flow space** — graph coordinates, transformed by `Transform { x, y, scale }`

### Configuration

`FlowConfig` (`config.rs`) contains 60+ options controlling viewport behavior, node defaults, connection handling, edge styling, background, grid snapping, handle appearance, and animation parameters.

### Change System

All mutations are represented as enum variants (`NodeChange::Position`, `NodeChange::Dimensions`, `NodeChange::Select`, `NodeChange::Remove`, `NodeChange::Add`, `NodeChange::Replace`, and similarly for `EdgeChange`), applied via centralized functions in `state/changes.rs`.

## Dependencies

- `egui 0.31`, `emath 0.31`, `epaint 0.31` — UI framework
- `serde` (optional, default-enabled) — serialization
- `eframe` (dev only) — for running examples

## Mandatory Coding Cnstructions

The following instructions must be followed. If there is a need that these instructions might not hold true, ask the user what to do.

1. Use iterators and closures — they're zero-cost and often faster than manual loops.
2. Pre-allocate collections with with_capacity() when you know sizes.
3. Use SmallVec for frequently-created short collections.
4. Use Cow<str> when a function might or might not need to allocate.
5. Use the newtype pattern for type safety without runtime cost.
6. Use the typestate pattern for state machines and builders that need compile-time correctness.
7. Prefer &str and &[T] in function parameters; own data in structs.
8. Enable LTO and single codegen-unit in release profiles.
9. Profile before optimizing — use flamegraphs and DHAT.
10. Don't fight the borrow checker — restructure data rather than adding Rc<RefCell<T>> or .clone() everywhere.
