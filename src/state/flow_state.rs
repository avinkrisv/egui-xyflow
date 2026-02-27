use crate::types::position::CoordinateExtent;
use std::collections::HashMap;

use crate::animation::viewport_animation::ViewportAnimation;
use crate::config::FlowConfig;
use crate::graph::utils::get_nodes_bounds;
use crate::types::changes::{EdgeChange, NodeChange};
use crate::types::connection::ConnectionState;
use crate::types::edge::Edge;
use crate::types::handle::{Handle, HandleType};
use crate::types::node::{InternalNode, Node, NodeHandleBounds, NodeId, NodeInternals};
use crate::types::position::Position;
use crate::types::viewport::Viewport;

use super::changes::{apply_edge_changes, apply_node_changes};

/// Central state for the flow graph.
pub struct FlowState<ND = (), ED = ()> {
    pub nodes: Vec<Node<ND>>,
    pub edges: Vec<Edge<ED>>,
    pub node_lookup: HashMap<NodeId, InternalNode<ND>>,
    pub viewport: Viewport,
    pub connection_state: ConnectionState,
    pub selection_rect: Option<egui::Rect>,
    pub config: FlowConfig,
    pub viewport_animation: Option<ViewportAnimation>,
    /// Tracks whether any edge is animated (for repaint requests).
    pub has_animated_edges: bool,
    /// Cached z-sorted node IDs. Invalidated on structural changes.
    sorted_ids_cache: Option<Vec<NodeId>>,
}

impl<ND: Clone, ED: Clone> FlowState<ND, ED> {
    pub fn new(config: FlowConfig) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: HashMap::new(),
            viewport: Viewport::default(),
            connection_state: ConnectionState::None,
            selection_rect: None,
            config,
            viewport_animation: None,
            has_animated_edges: false,
            sorted_ids_cache: None,
        }
    }

    pub fn add_node(&mut self, node: Node<ND>) {
        self.nodes.push(node);
        self.rebuild_lookup();
    }

    pub fn add_edge(&mut self, edge: Edge<ED>) {
        if edge.animated {
            self.has_animated_edges = true;
        }
        self.edges.push(edge);
    }

    pub fn apply_node_changes(&mut self, changes: &[NodeChange<ND>]) {
        let has_structural = changes.iter().any(|c| {
            matches!(
                c,
                NodeChange::Add { .. } | NodeChange::Remove { .. } | NodeChange::Replace { .. }
            )
        });

        if has_structural {
            // Structural changes require a full rebuild.
            apply_node_changes(changes, &mut self.nodes);
            self.rebuild_lookup();
        } else {
            // Non-structural (Position, Dimensions, Select): update nodes vec
            // and patch the lookup in place — avoids cloning all N nodes.
            apply_node_changes(changes, &mut self.nodes);
            self.apply_incremental_lookup_updates(changes);
        }
    }

    /// Apply non-structural changes to the lookup without a full rebuild.
    fn apply_incremental_lookup_updates(&mut self, changes: &[NodeChange<ND>]) {
        let mut needs_parent_update = false;

        for change in changes {
            match change {
                NodeChange::Position { id, position, dragging } => {
                    if let Some(internal) = self.node_lookup.get_mut(id) {
                        if let Some(pos) = position {
                            internal.node.position = *pos;
                            internal.internals.position_absolute = *pos;
                            needs_parent_update = true;
                        }
                        if let Some(d) = dragging {
                            internal.node.dragging = *d;
                        }
                    }
                }
                NodeChange::Dimensions { id, dimensions } => {
                    if let Some(internal) = self.node_lookup.get_mut(id) {
                        internal.node.measured = *dimensions;
                        if let Some(d) = dimensions {
                            internal.node.width = Some(d.width);
                            internal.node.height = Some(d.height);
                        }
                        // Rebuild handle bounds since dimensions changed.
                        internal.internals.handle_bounds =
                            build_handle_bounds(&internal.node, &self.config);
                    }
                }
                NodeChange::Select { id, selected } => {
                    if let Some(internal) = self.node_lookup.get_mut(id) {
                        internal.node.selected = *selected;
                    }
                }
                _ => {} // Structural changes handled by full rebuild path
            }
        }

        if needs_parent_update {
            self.update_absolute_positions();
        }
    }

    pub fn apply_edge_changes(&mut self, changes: &[EdgeChange<ED>]) {
        apply_edge_changes(changes, &mut self.edges);
        self.has_animated_edges = self.edges.iter().any(|e| e.animated);
    }

    /// Rebuild internal node lookup from user nodes.
    pub fn rebuild_lookup(&mut self) {
        self.sorted_ids_cache = None; // invalidate cache
        self.node_lookup.clear();
        for node in &self.nodes {
            let handle_bounds = build_handle_bounds(node, &self.config);
            let internal = InternalNode {
                internals: NodeInternals {
                    position_absolute: node.position,
                    z: node.z_index.unwrap_or(0),
                    handle_bounds,
                },
                node: node.clone(),
            };
            self.node_lookup.insert(node.id.clone(), internal);
        }
        self.update_absolute_positions();
    }

    /// Compute absolute positions for child nodes.
    fn update_absolute_positions(&mut self) {
        // Fast path: skip if no node has a parent.
        if !self.nodes.iter().any(|n| n.parent_id.is_some()) {
            return;
        }

        // Collect parent relationships.
        let parent_map: Vec<(NodeId, NodeId)> = self
            .nodes
            .iter()
            .filter_map(|n| n.parent_id.as_ref().map(|pid| (n.id.clone(), pid.clone())))
            .collect();

        // Pre-collect parent data (position_absolute + z) to avoid simultaneous
        // mutable + immutable borrows of node_lookup inside the loop.
        let parent_data: HashMap<&NodeId, (egui::Pos2, i32)> = parent_map
            .iter()
            .filter_map(|(_, pid)| {
                self.node_lookup
                    .get(pid)
                    .map(|p| (pid, (p.internals.position_absolute, p.internals.z)))
            })
            .collect();

        // Resolve absolute positions (simple single-level parent support).
        for (child_id, parent_id) in &parent_map {
            if let Some(&(parent_pos, parent_z)) = parent_data.get(parent_id) {
                if let Some(child) = self.node_lookup.get_mut(child_id) {
                    child.internals.position_absolute = egui::pos2(
                        parent_pos.x + child.node.position.x,
                        parent_pos.y + child.node.position.y,
                    );
                    if child.internals.z <= parent_z {
                        child.internals.z = parent_z + 1;
                    }
                }
            }
        }
    }

    /// Animate viewport to fit all nodes.
    pub fn fit_view(&mut self, canvas_rect: egui::Rect, padding: f32, current_time: f64) {
        let bounds = get_nodes_bounds(&self.node_lookup);
        if bounds == egui::Rect::NOTHING {
            return;
        }
        let target = crate::graph::utils::get_viewport_for_bounds(
            bounds,
            canvas_rect.width(),
            canvas_rect.height(),
            self.config.min_zoom,
            self.config.max_zoom,
            padding,
        );
        self.animate_viewport(target, current_time);
    }

    /// Animate the viewport to fit an arbitrary **flow-space** bounding box
    /// into the canvas area.
    ///
    /// This is the Rust equivalent of xyflow's `fitBounds` helper.  Use it
    /// when you want to frame a specific region of the graph rather than all
    /// nodes (e.g. fitting the selected nodes, or an imported sub-graph).
    ///
    /// `padding` is extra space in canvas pixels added around the bounds.
    ///
    /// ```rust,ignore
    /// // Frame just the first two nodes:
    /// let bounds = CoordinateExtent {
    ///     min: egui::pos2(80.0, 100.0),
    ///     max: egui::pos2(480.0, 200.0),
    /// };
    /// state.fit_bounds(bounds, canvas_rect, 20.0, current_time);
    /// ```
    pub fn fit_bounds(
        &mut self,
        bounds: CoordinateExtent,
        canvas_rect: egui::Rect,
        padding: f32,
        current_time: f64,
    ) {
        let flow_rect = egui::Rect::from_min_max(bounds.min, bounds.max);

        if flow_rect.width() <= 0.0 || flow_rect.height() <= 0.0 {
            return;
        }

        let target = crate::graph::utils::get_viewport_for_bounds(
            flow_rect,
            canvas_rect.width(),
            canvas_rect.height(),
            self.config.min_zoom,
            self.config.max_zoom,
            padding,
        );
        self.animate_viewport(target, current_time);
    }

    /// Animate the viewport to fit only the currently **selected** nodes.
    ///
    /// Does nothing when no nodes are selected.
    pub fn fit_selected_nodes(&mut self, canvas_rect: egui::Rect, padding: f32, current_time: f64) {
        // Compute bounds directly without cloning into a temporary HashMap.
        let bounds = self
            .node_lookup
            .values()
            .filter(|n| n.node.selected && !n.node.hidden)
            .fold(egui::Rect::NOTHING, |acc, n| acc.union(n.rect()));

        if bounds == egui::Rect::NOTHING {
            return;
        }

        let target = crate::graph::utils::get_viewport_for_bounds(
            bounds,
            canvas_rect.width(),
            canvas_rect.height(),
            self.config.min_zoom,
            self.config.max_zoom,
            padding,
        );
        self.animate_viewport(target, current_time);
    }

    /// Animate viewport zoom in.
    pub fn zoom_in(&mut self, current_time: f64) {
        let target = Viewport {
            zoom: (self.viewport.zoom * 1.2).min(self.config.max_zoom),
            ..self.viewport
        };
        self.animate_viewport(target, current_time);
    }

    /// Animate viewport zoom out.
    pub fn zoom_out(&mut self, current_time: f64) {
        let target = Viewport {
            zoom: (self.viewport.zoom / 1.2).max(self.config.min_zoom),
            ..self.viewport
        };
        self.animate_viewport(target, current_time);
    }

    /// Animate to center the view on a specific flow-space point with an
    /// optional zoom level.  If `zoom` is `None` the current zoom is kept.
    ///
    /// ```rust,ignore
    /// state.set_center(400.0, 200.0, Some(1.0), current_time);
    /// ```
    pub fn set_center(
        &mut self,
        x: f32,
        y: f32,
        zoom: Option<f32>,
        canvas_rect: egui::Rect,
        current_time: f64,
    ) {
        let target_zoom = zoom
            .unwrap_or(self.viewport.zoom)
            .clamp(self.config.min_zoom, self.config.max_zoom);
        // viewport offset so that (x, y) appears at the canvas centre
        let target = Viewport {
            x: canvas_rect.center().x - x * target_zoom,
            y: canvas_rect.center().y - y * target_zoom,
            zoom: target_zoom,
        };
        self.animate_viewport(target, current_time);
    }

    /// Animate to an arbitrary viewport with a custom duration and easing
    /// function.
    ///
    /// ```rust,ignore
    /// use egui_xyflow::animation::easing::ease_linear;
    /// state.set_viewport(Viewport { x: 0.0, y: 0.0, zoom: 1.0 }, 0.5, ease_linear, current_time);
    /// ```
    pub fn set_viewport(
        &mut self,
        target: Viewport,
        duration: f32,
        easing: fn(f32) -> f32,
        current_time: f64,
    ) {
        self.viewport_animation = Some(ViewportAnimation::new(
            self.viewport,
            target,
            duration,
            current_time,
            easing,
        ));
    }

    /// Animate to a specific viewport using the default duration and easing.
    pub fn animate_viewport(&mut self, target: Viewport, current_time: f64) {
        self.viewport_animation = Some(ViewportAnimation::new(
            self.viewport,
            target,
            self.config.default_transition_duration,
            current_time,
            self.config.default_transition_easing,
        ));
    }

    /// Tick viewport animation. Returns true if animation is active.
    pub fn tick_animation(&mut self, current_time: f64) -> bool {
        if let Some(ref mut anim) = self.viewport_animation {
            self.viewport = anim.tick(current_time);
            if !anim.active {
                self.viewport_animation = None;
                return false;
            }
            return true;
        }
        false
    }

    /// Get sorted node IDs by z-index (lowest first).
    ///
    /// Uses an internal cache invalidated on structural changes
    /// (add/remove/replace). Non-structural changes (position, select)
    /// reuse the cached ordering.
    pub fn sorted_node_ids(&mut self) -> Vec<NodeId> {
        if let Some(ref cached) = self.sorted_ids_cache {
            return cached.clone();
        }
        let mut ids = Vec::with_capacity(self.node_lookup.len());
        ids.extend(self.node_lookup.keys().cloned());
        ids.sort_by_key(|id| self.node_lookup.get(id).map(|n| n.internals.z).unwrap_or(0));
        self.sorted_ids_cache = Some(ids.clone());
        ids
    }
}

/// Build handle bounds from user-specified handles.
fn build_handle_bounds<D>(node: &Node<D>, config: &FlowConfig) -> NodeHandleBounds {
    let node_w = node.width.unwrap_or(config.default_node_width);
    let node_h = node.height.unwrap_or(config.default_node_height);
    let handle_size = config.handle_size;

    // Count handles per position for even spacing
    let source_handles: Vec<_> = node
        .handles
        .iter()
        .filter(|h| h.handle_type == HandleType::Source)
        .collect();
    let target_handles: Vec<_> = node
        .handles
        .iter()
        .filter(|h| h.handle_type == HandleType::Target)
        .collect();

    let mut source = Vec::with_capacity(source_handles.len());
    let mut target = Vec::with_capacity(target_handles.len());

    for nh in source_handles.iter() {
        let count = source_handles
            .iter()
            .filter(|h| h.position == nh.position)
            .count();
        let idx = source_handles
            .iter()
            .filter(|h| h.position == nh.position)
            .position(|h| std::ptr::eq(*h, *nh))
            .unwrap_or(0);
        let (x, y) = compute_handle_offset(nh.position, node_w, node_h, handle_size, count, idx);
        source.push(Handle {
            id: nh.id.clone(),
            node_id: node.id.0.clone(), // Arc<str> clone — O(1)
            x,
            y,
            position: nh.position,
            handle_type: HandleType::Source,
            width: handle_size,
            height: handle_size,
        });
    }

    for nh in target_handles.iter() {
        let count = target_handles
            .iter()
            .filter(|h| h.position == nh.position)
            .count();
        let idx = target_handles
            .iter()
            .filter(|h| h.position == nh.position)
            .position(|h| std::ptr::eq(*h, *nh))
            .unwrap_or(0);
        let (x, y) = compute_handle_offset(nh.position, node_w, node_h, handle_size, count, idx);
        target.push(Handle {
            id: nh.id.clone(),
            node_id: node.id.0.clone(), // Arc<str> clone — O(1)
            x,
            y,
            position: nh.position,
            handle_type: HandleType::Target,
            width: handle_size,
            height: handle_size,
        });
    }

    NodeHandleBounds { source, target }
}

/// Compute handle offset within node bounds.
fn compute_handle_offset(
    position: Position,
    node_w: f32,
    node_h: f32,
    handle_size: f32,
    count: usize,
    index: usize,
) -> (f32, f32) {
    let half = handle_size / 2.0;
    match position {
        Position::Top => {
            let spacing = node_w / (count as f32 + 1.0);
            let x = spacing * (index as f32 + 1.0) - half;
            (x, -half)
        }
        Position::Bottom => {
            let spacing = node_w / (count as f32 + 1.0);
            let x = spacing * (index as f32 + 1.0) - half;
            (x, node_h - half)
        }
        Position::Left => {
            let spacing = node_h / (count as f32 + 1.0);
            let y = spacing * (index as f32 + 1.0) - half;
            (-half, y)
        }
        Position::Right => {
            let spacing = node_h / (count as f32 + 1.0);
            let y = spacing * (index as f32 + 1.0) - half;
            (node_w - half, y)
        }
        Position::Center => {
            // Handle sits at the center of the node
            (node_w / 2.0 - half, node_h / 2.0 - half)
        }
        Position::Closest => {
            // Resolved dynamically at render time; place at center for now
            (node_w / 2.0 - half, node_h / 2.0 - half)
        }
    }
}
