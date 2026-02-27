//! `FlowCanvas` — the main egui widget that ties together all rendering and
//! interaction for a node-graph.
//!
//! Rendering order each frame
//! ──────────────────────────
//! 1. Background grid
//! 2. Edges (below nodes)
//! 3. In-progress connection line
//! 4. Nodes + handles (low-z first, high-z on top)
//! 5. Resize handles (selected node only)
//! 6. Selection rectangle overlay
//! 7. Minimap
//!
//! `show` returns a [`FlowEvents`] value that summarises everything that
//! happened this frame (connections made, nodes dragged, etc.).

use std::collections::HashMap;

use crate::config::FlowConfig;
use crate::events::FlowEvents;
use crate::graph::node_position::{flow_to_screen, screen_to_flow};
use crate::interaction::connection_drag::get_closest_handle;
use crate::interaction::drag::{
    handle_multi_node_drag, handle_multi_node_drag_end, handle_node_drag,
};
use crate::interaction::pan_zoom::{calc_auto_pan, clamp_translate, handle_pan_zoom, zoom_toward};
use crate::interaction::resize::{render_and_handle_resize, should_show_resize_handles};
use crate::interaction::selection::get_nodes_inside;
use crate::render::background::render_background;
use crate::render::connection_renderer::render_connection_line;
use crate::render::edge_renderer::{render_edge_contact_indicators, render_edges, EdgeEndpoints};
use crate::render::handle_renderer::render_handles;
use crate::render::minimap::render_minimap;
use crate::render::node_renderer::NodeWidget;
use crate::render::selection_renderer::render_selection_rect;
use crate::state::flow_state::FlowState;
use crate::types::changes::{EdgeChange, NodeChange};
use crate::types::connection::{Connection, ConnectionMode, ConnectionState, EdgeInfo};
use crate::types::edge::{Edge, EdgeId, EdgePosition};
use crate::types::handle::Handle;
use crate::types::node::NodeId;
use crate::types::position::Transform;

// ─────────────────────────────────────────────────────────────────────────────
// Public traits
// ─────────────────────────────────────────────────────────────────────────────

/// Validates whether a prospective connection between two handles should be
/// allowed.  Implement this to enforce domain-specific rules (e.g. type
/// compatibility, no cycles, maximum fan-out).
///
/// The `existing_edges` slice provides a type-erased view of every edge
/// currently in the graph, so validators can reason about the graph topology
/// (e.g. cycle detection, fan-in limits) without needing a separate snapshot.
///
/// Pass an implementor to [`FlowCanvas::connection_validator`].
pub trait ConnectionValidator {
    fn is_valid_connection(&self, connection: &Connection, existing_edges: &[EdgeInfo<'_>]) -> bool;
}

/// A [`ConnectionValidator`] that permits every connection.
pub struct AllowAllConnections;
impl ConnectionValidator for AllowAllConnections {
    fn is_valid_connection(&self, _: &Connection, _existing_edges: &[EdgeInfo<'_>]) -> bool {
        true
    }
}

/// Custom edge renderer.  Implement this to replace the built-in bezier/step
/// painter with your own visuals.
///
/// Pass an implementor to [`FlowCanvas::edge_widget`].
///
/// The `pos` parameter contains screen-space source/target connection points
/// (accounting for handle positions and `Position::Center`).  The `transform`
/// parameter allows converting arbitrary flow-space coordinates to screen
/// space via `x * transform.scale + transform.x`.
pub trait EdgeWidget<ED> {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &crate::types::edge::Edge<ED>,
        pos: &EdgePosition,
        config: &FlowConfig,
        time: f64,
        transform: &Transform,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-canvas persistent memory (stored in egui's temp data map)
// ─────────────────────────────────────────────────────────────────────────────

/// Transient state that lives across frames but is not part of the user-visible
/// `FlowState`.  Stored in `egui::Memory` keyed by the canvas `Id`.
#[derive(Clone, Default)]
struct CanvasMemory {
    /// Where the user started dragging to draw a selection rect.
    selection_start: Option<egui::Pos2>,
    /// Node IDs selected during the current in-progress selection drag.
    pending_selection: Vec<NodeId>,
    /// Active node resize state.
    resize_state: Option<crate::interaction::resize::ResizeState>,
    /// Whether we were dragging a node last frame (to detect drag-start).
    was_dragging_node: bool,
    /// Active edge anchor drag state.
    anchor_drag: Option<AnchorDragState>,
}

/// State for an in-progress edge anchor drag.
#[derive(Clone)]
struct AnchorDragState {
    edge_id: EdgeId,
    endpoint: crate::types::edge::AnchorEndpoint,
    node_id: NodeId,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public widget
// ─────────────────────────────────────────────────────────────────────────────

/// The main FlowCanvas widget.
///
/// ```rust,ignore
/// let events = FlowCanvas::new(&mut self.state, &DefaultNodeWidget).show(ui);
///
/// // With a custom connection validator:
/// let events = FlowCanvas::new(&mut self.state, &DefaultNodeWidget)
///     .connection_validator(&MyValidator)
///     .show(ui);
/// ```
pub struct FlowCanvas<'a, ND, ED, NW>
where
    NW: NodeWidget<ND>,
{
    state: &'a mut FlowState<ND, ED>,
    node_widget: &'a NW,
    /// Optional connection validator — defaults to [`AllowAllConnections`].
    validator: Option<&'a dyn ConnectionValidator>,
    /// Optional custom edge renderer — if `None` the built-in renderer is used.
    edge_widget: Option<&'a dyn EdgeWidget<ED>>,
}

impl<'a, ND, ED, NW> FlowCanvas<'a, ND, ED, NW>
where
    ND: Clone,
    ED: Clone + Default,
    NW: NodeWidget<ND>,
{
    pub fn new(state: &'a mut FlowState<ND, ED>, node_widget: &'a NW) -> Self {
        Self {
            state,
            node_widget,
            validator: None,
            edge_widget: None,
        }
    }

    /// Attach a [`ConnectionValidator`].  Called once per proposed connection
    /// before an edge is created; returning `false` cancels the connection.
    pub fn connection_validator(mut self, v: &'a dyn ConnectionValidator) -> Self {
        self.validator = Some(v);
        self
    }

    /// Attach a custom [`EdgeWidget`].  When set, every edge is rendered by
    /// `widget.show()` instead of the built-in path painter.
    pub fn edge_widget(mut self, w: &'a dyn EdgeWidget<ED>) -> Self {
        self.edge_widget = Some(w);
        self
    }

    /// Show the canvas and return all events that occurred this frame.
    pub fn show(mut self, ui: &mut egui::Ui) -> FlowEvents {
        let mut events = FlowEvents::default();

        // ── Allocate the full canvas area ────────────────────────────────────
        let available = ui.available_size_before_wrap();
        let (canvas_response, mut painter) =
            ui.allocate_painter(available, egui::Sense::click_and_drag());
        let canvas_rect = canvas_response.rect;
        painter.set_clip_rect(canvas_rect);

        let canvas_id = canvas_response.id;

        // ── Per-frame inputs ─────────────────────────────────────────────────
        let time = ui.input(|i| i.time);
        let pointer_pos: Option<egui::Pos2> = ui.input(|i| i.pointer.hover_pos());
        let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
        let primary_released = ui.input(|i| i.pointer.primary_released());

        // ── Tick viewport animation ──────────────────────────────────────────
        let animating = self.state.tick_animation(time);
        if animating || self.state.has_animated_edges {
            ui.ctx().request_repaint();
            events.set_viewport_changed();
        }

        let transform = self.state.viewport.to_transform();

        // ── 1. Background ────────────────────────────────────────────────────
        render_background(
            &painter,
            canvas_rect,
            &self.state.viewport,
            &self.state.config,
        );

        // ── 2. Edges ─────────────────────────────────────────────────────────
        let edge_endpoints: Vec<EdgeEndpoints>;
        if let Some(ew) = self.edge_widget {
            render_edges_custom(
                &painter,
                &self.state.edges,
                &self.state.node_lookup,
                &transform,
                &self.state.config,
                time,
                ew,
            );
            edge_endpoints = Vec::new(); // custom renderer handles its own indicators
        } else {
            edge_endpoints = render_edges(
                &painter,
                &self.state.edges,
                &self.state.node_lookup,
                &transform,
                &self.state.config,
                time,
            );
        }

        // ── Edge hit-testing / selection ─────────────────────────────────────
        let edge_clicked_this_frame = !events.edges_clicked.is_empty();
        let edge_changes = process_edge_clicks(
            ui,
            canvas_id,
            &self.state.edges,
            &self.state.node_lookup,
            &transform,
            &self.state.config,
            pointer_pos,
            primary_pressed,
            &mut events,
        );
        let edge_clicked_this_frame =
            edge_clicked_this_frame || !events.edges_clicked.is_empty();
        if !edge_changes.is_empty() {
            self.state.apply_edge_changes(&edge_changes);
        }

        // ── Edge anchor dragging ─────────────────────────────────────────────
        // Read persistent memory early — we need anchor_drag state here.
        let mut mem: CanvasMemory =
            ui.data(|d| d.get_temp::<CanvasMemory>(canvas_id).unwrap_or_default());

        {
            let anchor_edge_changes = handle_anchor_drag(
                &painter,
                ui,
                &self.state.edges,
                &self.state.node_lookup,
                &transform,
                &self.state.config,
                pointer_pos,
                primary_pressed,
                primary_released,
                &mut mem.anchor_drag,
                &mut events,
            );
            if !anchor_edge_changes.is_empty() {
                self.state.apply_edge_changes(&anchor_edge_changes);
            }
        }

        // ── 3. In-progress connection line ───────────────────────────────────
        render_connection_line(
            &painter,
            &self.state.connection_state,
            &transform,
            &self.state.config,
            time,
        );

        // ── 4. Nodes: render + collect interactions ──────────────────────────
        // Nodes are iterated low-z → high-z so that:
        //   • Painting: lower-z nodes appear underneath.
        //   • egui input: the last-registered rect wins, so higher-z gets
        //     pointer priority automatically.
        let sorted_ids = self.state.sorted_node_ids();

        let mut node_changes: Vec<NodeChange<ND>> = Vec::new();
        let mut any_node_dragging = false;
        let mut hovered_node: bool = false;

        // Track the node being actively dragged this frame (for multi-drag).
        let mut active_drag_id: Option<NodeId> = None;
        let mut active_drag_delta: egui::Vec2 = egui::Vec2::ZERO;
        let mut active_drag_ended = false;

        // Handle that was just pressed this frame → starts a connection drag.
        let mut newly_pressed_handle: Option<(Handle, egui::Pos2)> = None;

        // Whether a resize is in progress (suppress node drag / selection while resizing)
        let resize_in_progress = mem.resize_state.is_some();

        // Pre-compute resize handle hit-rects so the node loop can avoid
        // claiming a drag that should go to a resize handle.
        let resize_handle_rects: Vec<egui::Rect> = {
            use crate::interaction::resize::HANDLE_SIZE as RH_SIZE;
            if let Some(rid) = should_show_resize_handles(&self.state.node_lookup) {
                if let Some(rnode) = self.state.node_lookup.get(&rid) {
                    let origin = self.state.config.node_origin;
                    let raw = flow_to_screen(rnode.internals.position_absolute, &transform);
                    let nw = rnode.width() * transform.scale;
                    let nh = rnode.height() * transform.scale;
                    let adj = egui::pos2(raw.x - nw * origin[0], raw.y - nh * origin[1]);
                    let nr = egui::Rect::from_min_size(adj, egui::vec2(nw, nh));
                    use crate::interaction::resize::ResizeHandleKind;
                    ResizeHandleKind::ALL
                        .iter()
                        .map(|kind| {
                            let (ax, ay) = kind.anchor();
                            let center = egui::pos2(
                                nr.min.x + nr.width() * ax,
                                nr.min.y + nr.height() * ay,
                            );
                            egui::Rect::from_center_size(
                                center,
                                egui::vec2(RH_SIZE, RH_SIZE),
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };
        let pointer_on_resize_handle = pointer_pos
            .map(|pp| resize_handle_rects.iter().any(|r| r.contains(pp)))
            .unwrap_or(false);

        for id in &sorted_ids {
            // ── Read node data immutably ─────────────────────────────────────
            let (flow_pos, node_w, node_h, draggable_flag, connectable_flag, selectable_flag) = {
                let node = match self.state.node_lookup.get(id) {
                    Some(n) => n,
                    None => continue,
                };
                if node.node.hidden {
                    continue;
                }
                (
                    node.internals.position_absolute,
                    node.width(),
                    node.height(),
                    node.node.draggable,
                    node.node.connectable,
                    node.node.selectable,
                )
            };

            // Apply NodeOrigin offset: position represents the origin-fraction
            // of the node, defaulting to top-left [0.0, 0.0].
            let origin = self.state.config.node_origin;
            let screen_origin_raw = flow_to_screen(flow_pos, &transform);
            let screen_origin = egui::pos2(
                screen_origin_raw.x - node_w * transform.scale * origin[0],
                screen_origin_raw.y - node_h * transform.scale * origin[1],
            );
            let screen_size = egui::vec2(node_w * transform.scale, node_h * transform.scale);
            let screen_rect = egui::Rect::from_min_size(screen_origin, screen_size);

            // Skip off-screen nodes (culling)
            if !canvas_rect.intersects(screen_rect) {
                continue;
            }

            // Allocate an interaction region for this node
            let node_egui_id = canvas_id.with(id.as_str());
            let node_resp = ui.interact(screen_rect, node_egui_id, egui::Sense::click_and_drag());

            let is_hovered = node_resp.hovered() && !pointer_on_resize_handle;
            if is_hovered {
                hovered_node = true;
                events.set_node_hovered(id.clone());
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }

            // ── Node click → select ──────────────────────────────────────────
            if node_resp.clicked() && !resize_in_progress {
                let can_select = selectable_flag.unwrap_or(self.state.config.nodes_selectable);
                if can_select {
                    let shift = ui.input(|i| i.modifiers.shift);
                    let currently_selected = self
                        .state
                        .node_lookup
                        .get(id)
                        .map(|n| n.node.selected)
                        .unwrap_or(false);

                    if shift {
                        // Toggle this node in the selection.
                        node_changes.push(NodeChange::Select {
                            id: id.clone(),
                            selected: !currently_selected,
                        });
                    } else {
                        // Deselect all, then select this one.
                        for other_id in self.state.node_lookup.keys() {
                            let sel = other_id == id;
                            node_changes.push(NodeChange::Select {
                                id: other_id.clone(),
                                selected: sel,
                            });
                        }
                    }
                    events.push_node_click(id.clone());
                }
            }

            // ── Node dragging ────────────────────────────────────────────────
            let can_drag = draggable_flag.unwrap_or(self.state.config.nodes_draggable);
            if can_drag
                && node_resp.dragged_by(egui::PointerButton::Primary)
                && !resize_in_progress
                && !pointer_on_resize_handle
            {
                any_node_dragging = true;
                let delta = node_resp.drag_delta();

                // Record this as the active drag.
                active_drag_id = Some(id.clone());
                active_drag_delta = delta;

                // Auto-pan when near the canvas edge
                if self.state.config.auto_pan_on_node_drag {
                    if let Some(pp) = pointer_pos {
                        let pan =
                            calc_auto_pan(pp, canvas_rect, self.state.config.auto_pan_speed, 40.0);
                        if pan != egui::Vec2::ZERO {
                            self.state.viewport.x -= pan.x;
                            self.state.viewport.y -= pan.y;
                            clamp_translate(
                                &mut self.state.viewport,
                                &self.state.config.translate_extent,
                                canvas_rect,
                            );
                        }
                    }
                }
            }

            if can_drag && node_resp.drag_stopped() && !resize_in_progress && !pointer_on_resize_handle {
                active_drag_ended = true;
            }

            // ── Render node ──────────────────────────────────────────────────
            {
                let node = self.state.node_lookup.get(id).unwrap();
                self.node_widget.show(
                    &painter,
                    &node.node,
                    screen_rect,
                    &self.state.config,
                    is_hovered,
                    &transform,
                );
            }

            // ── Handles ──────────────────────────────────────────────────────
            let connecting = !matches!(self.state.connection_state, ConnectionState::None);
            let can_connect = connectable_flag.unwrap_or(self.state.config.nodes_connectable);
            let show_handles = (is_hovered || connecting) && can_connect;

            if show_handles {
                let node = self.state.node_lookup.get(id).unwrap();
                let hit_rects = render_handles(
                    &painter,
                    node,
                    &transform,
                    &self.state.config,
                    is_hovered,
                    pointer_pos,
                );

                // Check if a handle was just pressed this frame
                if can_connect && primary_pressed && newly_pressed_handle.is_none() {
                    if let Some(pp) = pointer_pos {
                        for hr in &hit_rects {
                            if hr.contains(pp) {
                                let from_flow = screen_to_flow(hr.screen_center, &transform);
                                newly_pressed_handle = Some((hr.handle.clone(), from_flow));
                                break;
                            }
                        }
                    }
                }
            }
        }

        // ── Resize handles ────────────────────────────────────────────────────
        // Processed BEFORE multi-node drag so that resize can cancel a
        // conflicting node drag when the resize handle overlaps the node rect.
        // We intentionally do NOT gate on `any_node_dragging` — the resize
        // handle rects overlap with the node's interaction rect, so the node
        // loop may have already set `any_node_dragging`.  If we skipped this
        // block the resize handles would never get registered with egui.
        let is_connecting_now = !matches!(self.state.connection_state, ConnectionState::None);
        if !is_connecting_now && self.state.config.nodes_resizable {
            if let Some(resize_node_id) = should_show_resize_handles(&self.state.node_lookup) {
                if let Some(node) = self.state.node_lookup.get(&resize_node_id) {
                    let origin = self.state.config.node_origin;
                    let raw_origin = flow_to_screen(node.internals.position_absolute, &transform);
                    let nw = node.width() * transform.scale;
                    let nh = node.height() * transform.scale;
                    let adjusted_origin =
                        egui::pos2(raw_origin.x - nw * origin[0], raw_origin.y - nh * origin[1]);
                    let node_screen_rect =
                        egui::Rect::from_min_size(adjusted_origin, egui::vec2(nw, nh));

                    let (new_resize_state, resize_changes, cursor) = render_and_handle_resize(
                        ui,
                        &painter,
                        &resize_node_id,
                        node_screen_rect,
                        node,
                        &transform,
                        &self.state.config,
                        mem.resize_state.take(),
                    );
                    mem.resize_state = new_resize_state;

                    // If a resize is active (just started or ongoing), cancel
                    // any conflicting node drag that was detected earlier in
                    // this frame — the resize handle takes priority.
                    if mem.resize_state.is_some() {
                        node_changes.retain(|c| !matches!(c, NodeChange::Position { id, .. } if *id == resize_node_id));
                        if active_drag_id.as_ref() == Some(&resize_node_id) {
                            active_drag_id = None;
                            active_drag_ended = false;
                            any_node_dragging = false;
                        }
                    }

                    for change in &resize_changes {
                        if let NodeChange::Dimensions {
                            id,
                            dimensions: Some(d),
                        } = change
                        {
                            events.push_resized(id.clone(), d.width, d.height);
                        }
                    }

                    if !resize_changes.is_empty() {
                        node_changes.extend(resize_changes);
                    }

                    if let Some(c) = cursor {
                        ui.ctx().set_cursor_icon(c);
                    }
                }
            } else {
                // More than one selected → clear any lingering resize state.
                mem.resize_state = None;
            }
        }

        // ── Multi-node drag ───────────────────────────────────────────────────
        // If the actively dragged node is selected, move all selected nodes together.
        if let Some(ref drag_id) = active_drag_id {
            let is_selected = self
                .state
                .node_lookup
                .get(drag_id)
                .map(|n| n.node.selected)
                .unwrap_or(false);

            if is_selected {
                // Multi-drag: apply same delta to all selected nodes.
                let multi_changes = handle_multi_node_drag(
                    drag_id,
                    active_drag_delta,
                    &transform,
                    self.state.config.snap_to_grid,
                    &self.state.config.snap_grid,
                    &self.state.node_lookup,
                );

                for change in &multi_changes {
                    if let NodeChange::Position {
                        id,
                        position: Some(pos),
                        ..
                    } = change
                    {
                        events.push_dragged(id.clone(), *pos);
                    }
                }

                // Emit drag-start events for all nodes that weren't dragging before.
                if !mem.was_dragging_node {
                    for change in &multi_changes {
                        if let NodeChange::Position { id, .. } = change {
                            events.push_drag_start(id.clone());
                        }
                    }
                }

                node_changes.extend(multi_changes);
            } else {
                // Single-node drag.
                let flow_pos = self
                    .state
                    .node_lookup
                    .get(drag_id)
                    .map(|n| n.internals.position_absolute)
                    .unwrap_or_default();

                if let Some(change) = handle_node_drag(
                    drag_id,
                    active_drag_delta,
                    &transform,
                    self.state.config.snap_to_grid,
                    &self.state.config.snap_grid,
                    flow_pos,
                ) {
                    if let NodeChange::Position {
                        position: Some(pos),
                        ..
                    } = &change
                    {
                        events.push_dragged(drag_id.clone(), *pos);
                        if !mem.was_dragging_node {
                            events.push_drag_start(drag_id.clone());
                        }
                    }
                    node_changes.push(change);
                }
            }
        }

        // ── Drag end ──────────────────────────────────────────────────────────
        if active_drag_ended {
            let end_changes = handle_multi_node_drag_end(&self.state.node_lookup);
            for change in &end_changes {
                if let NodeChange::Position { id, .. } = change {
                    events.push_drag_stop(id.clone());
                }
            }
            node_changes.extend(end_changes);
        }

        mem.was_dragging_node = active_drag_id.is_some();

        // ── Apply node changes ───────────────────────────────────────────────
        if !node_changes.is_empty() {
            self.state.apply_node_changes(&node_changes);
        }

        // ── Edge contact indicators (rendered on top of nodes) ───────────────
        render_edge_contact_indicators(
            &painter,
            &edge_endpoints,
            &transform,
            &self.state.config,
        );

        // ── Connection state machine ─────────────────────────────────────────
        // Start a new connection drag
        if let Some((handle, from_flow)) = newly_pressed_handle {
            let from_pos = handle.position;
            events.set_connection_started(NodeId(handle.node_id.clone()));
            self.state.connection_state = ConnectionState::InProgress {
                is_valid: None,
                from: from_flow,
                from_handle: handle.clone(),
                from_position: from_pos,
                from_node_id: NodeId(handle.node_id.clone()),
                to: from_flow,
                to_handle: Box::new(None),
                to_position: from_pos.opposite(),
                to_node_id: None,
            };
        }

        // Update "to" position of an in-progress connection
        update_connection_state(
            &mut self.state.connection_state,
            pointer_pos,
            &self.state.node_lookup,
            self.state.config.connection_radius,
            &transform,
            self.state.config.connection_mode,
        );

        // Resolve or cancel on pointer release
        if primary_released {
            let resolved = try_resolve_connection(&self.state.connection_state);
            if let Some(conn) = resolved {
                // Run through validator (default: allow all)
                let edge_infos: Vec<EdgeInfo<'_>> = self
                    .state
                    .edges
                    .iter()
                    .map(|e| EdgeInfo {
                        source: &e.source,
                        target: &e.target,
                        source_handle: e.source_handle.as_deref(),
                        target_handle: e.target_handle.as_deref(),
                    })
                    .collect();
                let allowed = self
                    .validator
                    .map(|v| v.is_valid_connection(&conn, &edge_infos))
                    .unwrap_or(true);

                if allowed {
                    // Avoid duplicate edges
                    let already_exists = self.state.edges.iter().any(|e| {
                        e.source == conn.source
                            && e.target == conn.target
                            && e.source_handle == conn.source_handle
                            && e.target_handle == conn.target_handle
                    });
                    if !already_exists {
                        let new_edge: Edge<ED> = Edge {
                            id: EdgeId::new(format!("e{}-{}", conn.source, conn.target)),
                            source: conn.source.clone(),
                            target: conn.target.clone(),
                            source_handle: conn.source_handle.clone(),
                            target_handle: conn.target_handle.clone(),
                            edge_type: Some(self.state.config.default_edge_type),
                            animated: false,
                            hidden: false,
                            deletable: None,
                            selectable: None,
                            data: None,
                            selected: false,
                            marker_start: None,
                            marker_end: None,
                            z_index: None,
                            interaction_width: 20.0,
                            source_anchor: None,
                            target_anchor: None,
                            anchors_draggable: None,
                            style: None,
                        };
                        self.state.add_edge(new_edge);
                        events.push_connection(conn);
                    }
                }
            }

            if !matches!(self.state.connection_state, ConnectionState::None) {
                events.set_connection_ended();
            }
            self.state.connection_state = ConnectionState::None;
        }

        // ── Canvas pan/zoom (background only) ────────────────────────────────
        let is_connecting = !matches!(self.state.connection_state, ConnectionState::None);
        let resize_active = mem.resize_state.is_some();
        let anchor_drag_active = mem.anchor_drag.is_some();
        // Suppress left-button panning when a selection drag is active or
        // starting (Shift held with pan_on_drag enabled).
        let shift_held = ui.input(|i| i.modifiers.shift);
        let selection_active = mem.selection_start.is_some()
            || (self.state.config.pan_on_drag && shift_held && !hovered_node);
        if !any_node_dragging && !is_connecting && !resize_active && !anchor_drag_active {
            let pz = handle_pan_zoom(
                ui,
                &canvas_response,
                &mut self.state.viewport,
                &self.state.config,
                canvas_rect,
                selection_active,
            );
            if pz.changed {
                events.set_viewport_changed();
            }
            // Double-click zoom: start an animated zoom instead of instant
            if let Some((screen_pos, factor)) = pz.animate_zoom {
                let mut target = self.state.viewport;
                zoom_toward(
                    &mut target,
                    screen_pos,
                    factor,
                    self.state.config.min_zoom,
                    self.state.config.max_zoom,
                );
                self.state.animate_viewport(target, time);
                events.set_viewport_changed();
            }
        }

        // ── Selection rectangle ───────────────────────────────────────────────
        let (new_selection_rect, new_mem, selection_node_changes, selection_edge_changes) =
            process_selection(
                ui,
                &canvas_response,
                canvas_rect,
                &mut mem,
                self.state.selection_rect,
                &self.state.node_lookup,
                &self.state.edges,
                &transform,
                &self.state.config,
                hovered_node,
                any_node_dragging,
                is_connecting,
                pointer_pos,
                resize_active,
                edge_clicked_this_frame,
                anchor_drag_active,
            );

        // Detect selection change
        let had_selection_change =
            !selection_node_changes.is_empty() || !selection_edge_changes.is_empty();
        if had_selection_change {
            // Apply first so selected_nodes snapshot is accurate.
            self.state.apply_node_changes(&selection_node_changes);
            if !selection_edge_changes.is_empty() {
                self.state.apply_edge_changes(&selection_edge_changes);
            }

            let selected_nodes: Vec<NodeId> = self
                .state
                .nodes
                .iter()
                .filter(|n| n.selected)
                .map(|n| n.id.clone())
                .collect();
            let selected_edges: Vec<EdgeId> = self
                .state
                .edges
                .iter()
                .filter(|e| e.selected)
                .map(|e| e.id.clone())
                .collect();
            events.set_selection_changed(selected_nodes, selected_edges);
        }

        // Write memory back
        ui.data_mut(|d| d.insert_temp(canvas_id, new_mem));
        self.state.selection_rect = new_selection_rect;

        // ── 5. Selection rect overlay ─────────────────────────────────────────
        if let Some(sel) = self.state.selection_rect {
            render_selection_rect(&painter, sel);
        }

        // ── 6. Minimap ────────────────────────────────────────────────────────
        let minimap_info = render_minimap(
            &painter,
            canvas_rect,
            &self.state.viewport,
            &self.state.node_lookup,
            &self.state.config,
        );

        // Minimap click-to-pan
        if let Some(mm_info) = minimap_info {
            let mm_id = canvas_id.with("minimap");
            let mm_resp = ui.interact(mm_info.mm_rect, mm_id, egui::Sense::click_and_drag());

            // Click anywhere in the minimap → pan the viewport so that the
            // clicked flow-space point is centred in the canvas.
            let click_pos = if mm_resp.clicked() {
                ui.input(|i| i.pointer.interact_pos())
            } else if mm_resp.dragged() {
                ui.input(|i| i.pointer.hover_pos())
            } else {
                None
            };

            if let Some(screen_pos) = click_pos {
                if let Some(flow_pos) = mm_info.screen_to_flow(screen_pos) {
                    // Animate the viewport so that flow_pos is centred.
                    let target_zoom = self.state.viewport.zoom; // keep current zoom
                    let target = crate::types::viewport::Viewport {
                        x: canvas_rect.center().x - flow_pos.x * target_zoom,
                        y: canvas_rect.center().y - flow_pos.y * target_zoom,
                        zoom: target_zoom,
                    };
                    self.state.animate_viewport(target, time);
                    events.set_viewport_changed();
                }
            }

            // Cursor hint when hovering the minimap
            if mm_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }

        // ── Keyboard shortcuts ────────────────────────────────────────────────
        self.process_keyboard(ui, canvas_rect, time, &mut events);

        events
    }

    // ── Keyboard ──────────────────────────────────────────────────────────────

    fn process_keyboard(
        &mut self,
        ui: &mut egui::Ui,
        canvas_rect: egui::Rect,
        time: f64,
        events: &mut FlowEvents,
    ) {
        let delete_pressed =
            ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
        let select_all = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A));
        let fit_view = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F));
        let fit_selected =
            ui.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::F));

        if delete_pressed {
            let node_removes: Vec<NodeChange<ND>> = self
                .state
                .nodes
                .iter()
                .filter(|n| n.selected && n.deletable.unwrap_or(true))
                .map(|n| NodeChange::Remove { id: n.id.clone() })
                .collect();

            let edge_removes: Vec<EdgeChange<ED>> = self
                .state
                .edges
                .iter()
                .filter(|e| e.selected && e.deletable.unwrap_or(true))
                .map(|e| EdgeChange::Remove { id: e.id.clone() })
                .collect();

            let deleted_node_ids: Vec<NodeId> = node_removes
                .iter()
                .filter_map(|c| {
                    if let NodeChange::Remove { id } = c {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();
            let deleted_edge_ids: Vec<EdgeId> = edge_removes
                .iter()
                .filter_map(|c| {
                    if let EdgeChange::Remove { id } = c {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if !node_removes.is_empty() {
                events.push_nodes_deleted(deleted_node_ids);
                self.state.apply_node_changes(&node_removes);
            }
            if !edge_removes.is_empty() {
                events.push_edges_deleted(deleted_edge_ids);
                self.state.apply_edge_changes(&edge_removes);
            }
        }

        if select_all {
            let changes: Vec<NodeChange<ND>> = self
                .state
                .nodes
                .iter()
                .filter(|n| n.selectable.unwrap_or(true) && !n.hidden)
                .map(|n| NodeChange::Select {
                    id: n.id.clone(),
                    selected: true,
                })
                .collect();
            if !changes.is_empty() {
                self.state.apply_node_changes(&changes);
                let selected_nodes: Vec<NodeId> = self
                    .state
                    .nodes
                    .iter()
                    .filter(|n| n.selected)
                    .map(|n| n.id.clone())
                    .collect();
                events.set_selection_changed(selected_nodes, vec![]);
            }
        }

        if fit_view {
            self.state.fit_view(canvas_rect, 20.0, time);
            events.set_viewport_changed();
        }

        if fit_selected {
            self.state.fit_selected_nodes(canvas_rect, 20.0, time);
            events.set_viewport_changed();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge hit-testing and click-to-select
// ─────────────────────────────────────────────────────────────────────────────

/// Check if any edge was clicked this frame and return select/deselect changes.
#[allow(clippy::too_many_arguments)]
fn process_edge_clicks<ND, ED>(
    ui: &egui::Ui,
    _canvas_id: egui::Id,
    edges: &[Edge<ED>],
    node_lookup: &HashMap<NodeId, crate::types::node::InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    pointer_pos: Option<egui::Pos2>,
    primary_pressed: bool,
    events: &mut FlowEvents,
) -> Vec<EdgeChange<ED>> {
    use crate::edges::bezier::{get_bezier_path, sample_bezier};
    use crate::edges::positions::get_edge_position;
    use crate::edges::smooth_step::{get_smooth_step_path, get_step_path};
    use crate::edges::straight::get_straight_path;
    use crate::types::edge::EdgeType;

    if !primary_pressed {
        return Vec::new();
    }
    let pp = match pointer_pos {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut clicked_edge: Option<EdgeId> = None;
    let mut best_dist = f32::INFINITY;

    for edge in edges {
        if edge.hidden {
            continue;
        }
        if edge.selectable == Some(false) {
            continue;
        }

        let ep = match get_edge_position(
            &edge.source,
            &edge.target,
            edge.source_handle.as_deref(),
            edge.target_handle.as_deref(),
            node_lookup,
            config.default_source_position,
            config.default_target_position,
            edge.source_anchor.as_ref(),
            edge.target_anchor.as_ref(),
        ) {
            Some(p) => p,
            None => continue,
        };

        let edge_type = edge.edge_type.unwrap_or(config.default_edge_type);
        let hit_width = edge.interaction_width * transform.scale;

        // Build screen-space polyline for hit testing
        let screen_points: Vec<egui::Pos2> = match edge_type {
            EdgeType::Bezier | EdgeType::SimpleBezier => {
                let result = get_bezier_path(&ep, None);
                if result.points.len() == 4 {
                    let p0 = flow_to_screen(result.points[0], transform);
                    let p1 = flow_to_screen(result.points[1], transform);
                    let p2 = flow_to_screen(result.points[2], transform);
                    let p3 = flow_to_screen(result.points[3], transform);
                    sample_bezier(p0, p1, p2, p3, 32)
                } else {
                    continue;
                }
            }
            EdgeType::Straight => {
                let result = get_straight_path(&ep);
                result
                    .points
                    .iter()
                    .map(|p| flow_to_screen(*p, transform))
                    .collect()
            }
            EdgeType::SmoothStep => {
                let result = get_smooth_step_path(&ep, None, None);
                result
                    .points
                    .iter()
                    .map(|p| flow_to_screen(*p, transform))
                    .collect()
            }
            EdgeType::Step => {
                let result = get_step_path(&ep, None);
                result
                    .points
                    .iter()
                    .map(|p| flow_to_screen(*p, transform))
                    .collect()
            }
        };

        // Minimum distance from pointer to any segment of the polyline
        let dist = min_dist_to_polyline(pp, &screen_points);
        if dist < hit_width / 2.0 && dist < best_dist {
            best_dist = dist;
            clicked_edge = Some(edge.id.clone());
        }
    }

    if let Some(ref edge_id) = clicked_edge {
        events.push_edge_click(edge_id.clone());

        // Toggle selection: select the clicked edge, deselect all others
        let shift = ui.input(|i| i.modifiers.shift);
        return edges
            .iter()
            .map(|e| {
                let select = if e.id == *edge_id {
                    if shift {
                        !e.selected
                    } else {
                        true
                    }
                } else if !shift {
                    false
                } else {
                    e.selected
                };
                EdgeChange::Select {
                    id: e.id.clone(),
                    selected: select,
                }
            })
            .collect();
    }

    Vec::new()
}

/// Minimum distance from point `p` to the closest segment in `polyline`.
fn min_dist_to_polyline(p: egui::Pos2, polyline: &[egui::Pos2]) -> f32 {
    let mut min_dist = f32::INFINITY;
    for i in 0..polyline.len().saturating_sub(1) {
        let a = polyline[i];
        let b = polyline[i + 1];
        let dist = dist_point_to_segment(p, a, b);
        if dist < min_dist {
            min_dist = dist;
        }
    }
    min_dist
}

/// Distance from point `p` to segment `[a, b]`.
fn dist_point_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let len_sq = ab.length_sq();
    if len_sq < f32::EPSILON {
        return ap.length();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

// ─────────────────────────────────────────────────────────────────────────────
// Custom edge rendering helper
// ─────────────────────────────────────────────────────────────────────────────

/// Render all edges using a user-supplied [`EdgeWidget`].
fn render_edges_custom<ND, ED>(
    painter: &egui::Painter,
    edges: &[Edge<ED>],
    node_lookup: &HashMap<NodeId, crate::types::node::InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    time: f64,
    widget: &dyn EdgeWidget<ED>,
) {
    use crate::edges::positions::get_edge_position;

    for edge in edges {
        if edge.hidden {
            continue;
        }
        if let Some(pos) = get_edge_position(
            &edge.source,
            &edge.target,
            edge.source_handle.as_deref(),
            edge.target_handle.as_deref(),
            node_lookup,
            config.default_source_position,
            config.default_target_position,
            edge.source_anchor.as_ref(),
            edge.target_anchor.as_ref(),
        ) {
            // Convert edge position to screen space
            let screen_pos = EdgePosition {
                source_x: pos.source_x * transform.scale + transform.x,
                source_y: pos.source_y * transform.scale + transform.y,
                target_x: pos.target_x * transform.scale + transform.x,
                target_y: pos.target_y * transform.scale + transform.y,
                source_pos: pos.source_pos,
                target_pos: pos.target_pos,
            };
            widget.show(painter, edge, &screen_pos, config, time, transform);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Connection helpers
// ─────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn update_connection_state<D>(
    connection_state: &mut ConnectionState,
    pointer_pos: Option<egui::Pos2>,
    node_lookup: &HashMap<NodeId, crate::types::node::InternalNode<D>>,
    connection_radius: f32,
    transform: &Transform,
    mode: ConnectionMode,
) {
    let (from_handle, from_pos) = match connection_state {
        ConnectionState::InProgress {
            from_handle,
            from_position,
            ..
        } => (from_handle.clone(), *from_position),
        ConnectionState::None => return,
    };

    let pp = match pointer_pos {
        Some(p) => p,
        None => return,
    };

    let flow_pp = screen_to_flow(pp, transform);

    // Find the closest valid target handle
    let closest = get_closest_handle(pp, connection_radius, node_lookup, &from_handle, mode);

    if let ConnectionState::InProgress {
        to,
        to_handle,
        to_position,
        to_node_id,
        is_valid,
        ..
    } = connection_state
    {
        *to = flow_pp;
        if let Some(h) = closest {
            **to_handle = Some(h.clone());
            *to_position = h.position;
            *to_node_id = Some(NodeId::new(h.node_id.clone()));
            *is_valid = Some(true);
        } else {
            **to_handle = None;
            *to_node_id = None;
            *is_valid = None;
            // Restore default opposite direction for the line endpoint
            *to_position = from_pos.opposite();
        }
    }
}

fn try_resolve_connection(state: &ConnectionState) -> Option<Connection> {
    match state {
        ConnectionState::InProgress {
            from_node_id,
            from_handle,
            to_node_id: Some(to_node_id),
            to_handle,
            is_valid: Some(true),
            ..
        } => {
            let to_handle = to_handle.as_ref().as_ref()?;
            // Direction: source handle → target handle
            let (source, target, sh, th) =
                if from_handle.handle_type == crate::types::handle::HandleType::Source {
                    (
                        from_node_id.clone(),
                        to_node_id.clone(),
                        from_handle.id.clone(),
                        to_handle.id.clone(),
                    )
                } else {
                    (
                        to_node_id.clone(),
                        from_node_id.clone(),
                        to_handle.id.clone(),
                        from_handle.id.clone(),
                    )
                };
            Some(Connection {
                source,
                target,
                source_handle: sh,
                target_handle: th,
            })
        }
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Selection rectangle helper (pure function — no mutation of FlowState)
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `(new_selection_rect, updated_memory, node_changes_to_apply)`.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn process_selection<ND, ED>(
    ui: &egui::Ui,
    canvas_response: &egui::Response,
    canvas_rect: egui::Rect,
    mem: &mut CanvasMemory,
    current_rect: Option<egui::Rect>,
    node_lookup: &HashMap<NodeId, crate::types::node::InternalNode<ND>>,
    edges: &[Edge<ED>],
    transform: &Transform,
    config: &FlowConfig,
    over_node: bool,
    node_dragging: bool,
    is_connecting: bool,
    pointer_pos: Option<egui::Pos2>,
    resize_active: bool,
    edge_clicked: bool,
    anchor_drag_active: bool,
) -> (
    Option<egui::Rect>,
    CanvasMemory,
    Vec<NodeChange<ND>>,
    Vec<EdgeChange<ED>>,
) {
    let mut new_mem = mem.clone();
    let mut node_changes: Vec<NodeChange<ND>> = Vec::new();
    let mut edge_changes: Vec<EdgeChange<ED>> = Vec::new();

    // If pointer is over a node / dragging a node / connecting / resizing /
    // edge was clicked / anchor drag active, don't do selection
    if over_node || node_dragging || is_connecting || resize_active || edge_clicked || anchor_drag_active {
        new_mem.selection_start = None;
        return (None, new_mem, node_changes, edge_changes);
    }

    // ── Single click on background → deselect all nodes and edges ────────────
    if canvas_response.clicked() && !over_node {
        for id in node_lookup.keys() {
            node_changes.push(NodeChange::Select {
                id: id.clone(),
                selected: false,
            });
        }
        for edge in edges {
            if edge.selected {
                edge_changes.push(EdgeChange::Select {
                    id: edge.id.clone(),
                    selected: false,
                });
            }
        }
        return (None, new_mem, node_changes, edge_changes);
    }

    // When pan_on_drag is enabled, require Shift to start a selection drag
    // (unmodified left-drag is used for panning instead).
    if config.pan_on_drag && new_mem.selection_start.is_none() {
        let shift_held = ui.input(|i| i.modifiers.shift);
        if !shift_held {
            return (current_rect, new_mem, node_changes, edge_changes);
        }
    }

    // ── Start drag ───────────────────────────────────────────────────────────
    if canvas_response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(pp) = pointer_pos {
            if canvas_rect.contains(pp) {
                new_mem.selection_start = Some(pp);
                new_mem.pending_selection.clear();

                // Deselect everything unless Shift is held
                let shift_held = ui.input(|i| i.modifiers.shift);
                if !shift_held {
                    node_changes.extend(node_lookup.keys().map(|id| NodeChange::Select {
                        id: id.clone(),
                        selected: false,
                    }));
                    for edge in edges {
                        if edge.selected {
                            edge_changes.push(EdgeChange::Select {
                                id: edge.id.clone(),
                                selected: false,
                            });
                        }
                    }
                }
                return (
                    Some(egui::Rect::from_min_size(pp, egui::Vec2::ZERO)),
                    new_mem,
                    node_changes,
                    edge_changes,
                );
            }
        }
    }

    // ── Update drag ───────────────────────────────────────────────────────────
    if canvas_response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(start), Some(pp)) = (new_mem.selection_start, pointer_pos) {
            let sel = egui::Rect::from_two_pos(start, pp);
            return (Some(sel), new_mem, node_changes, edge_changes);
        }
    }

    // ── Finish drag ───────────────────────────────────────────────────────────
    if canvas_response.drag_stopped() {
        if let Some(sel) = current_rect {
            if sel.width() > 4.0 || sel.height() > 4.0 {
                let selected_ids =
                    get_nodes_inside(node_lookup, sel, transform, config.selection_mode);
                let shift_held = ui.input(|i| i.modifiers.shift);

                if !shift_held {
                    for id in node_lookup.keys() {
                        let is_selected = selected_ids.contains(id);
                        node_changes.push(NodeChange::Select {
                            id: id.clone(),
                            selected: is_selected,
                        });
                    }
                } else {
                    for id in &selected_ids {
                        node_changes.push(NodeChange::Select {
                            id: id.clone(),
                            selected: true,
                        });
                    }
                }
            }
        }
        new_mem.selection_start = None;
        new_mem.pending_selection.clear();
        return (None, new_mem, node_changes, edge_changes);
    }

    // No change
    (current_rect, new_mem, node_changes, edge_changes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge anchor drag interaction
// ─────────────────────────────────────────────────────────────────────────────

/// Handle edge anchor dragging and draw hover highlights on contact
/// indicators.  Returns edge changes to apply.
#[allow(clippy::too_many_arguments)]
fn handle_anchor_drag<ND, ED>(
    painter: &egui::Painter,
    ui: &egui::Ui,
    edges: &[Edge<ED>],
    node_lookup: &HashMap<NodeId, crate::types::node::InternalNode<ND>>,
    transform: &Transform,
    config: &FlowConfig,
    pointer_pos: Option<egui::Pos2>,
    primary_pressed: bool,
    primary_released: bool,
    anchor_drag: &mut Option<AnchorDragState>,
    events: &mut FlowEvents,
) -> Vec<EdgeChange<ED>> {
    use crate::edges::positions::{get_edge_position, project_to_border};
    use crate::types::edge::AnchorEndpoint;

    let mut changes: Vec<EdgeChange<ED>> = Vec::new();

    let pp = match pointer_pos {
        Some(p) => p,
        None => return changes,
    };

    let indicator_r = config.edge_contact_indicator_radius * transform.scale;
    let hit_radius = (indicator_r * 2.5).max(config.handle_size * transform.scale);
    let mut hovered_any = false;

    // ── Hover highlights + start drag ────────────────────────────────────────
    for edge in edges {
        if edge.hidden {
            continue;
        }
        let draggable = edge
            .anchors_draggable
            .unwrap_or(config.edge_anchors_draggable);
        if !draggable {
            continue;
        }

        let ep = match get_edge_position(
            &edge.source,
            &edge.target,
            edge.source_handle.as_deref(),
            edge.target_handle.as_deref(),
            node_lookup,
            config.default_source_position,
            config.default_target_position,
            edge.source_anchor.as_ref(),
            edge.target_anchor.as_ref(),
        ) {
            Some(p) => p,
            None => continue,
        };

        let src_screen = flow_to_screen(egui::pos2(ep.source_x, ep.source_y), transform);
        let tgt_screen = flow_to_screen(egui::pos2(ep.target_x, ep.target_y), transform);

        let src_hovered = pp.distance(src_screen) < hit_radius;
        let tgt_hovered = pp.distance(tgt_screen) < hit_radius;

        // Draw hover highlight over the contact indicator
        if config.show_edge_contact_indicators {
            if src_hovered {
                let r = indicator_r * 1.5;
                painter.circle_filled(src_screen, r, config.edge_contact_indicator_hover_color);
                painter.circle_stroke(
                    src_screen,
                    r,
                    egui::Stroke::new(1.5 * transform.scale, egui::Color32::WHITE),
                );
                hovered_any = true;
            }
            if tgt_hovered {
                let r = indicator_r * 1.5;
                painter.circle_filled(tgt_screen, r, config.edge_contact_indicator_hover_color);
                painter.circle_stroke(
                    tgt_screen,
                    r,
                    egui::Stroke::new(1.5 * transform.scale, egui::Color32::WHITE),
                );
                hovered_any = true;
            }
        }

        // Start drag on press
        if primary_pressed && anchor_drag.is_none() {
            if src_hovered {
                *anchor_drag = Some(AnchorDragState {
                    edge_id: edge.id.clone(),
                    endpoint: AnchorEndpoint::Source,
                    node_id: edge.source.clone(),
                });
            } else if tgt_hovered {
                *anchor_drag = Some(AnchorDragState {
                    edge_id: edge.id.clone(),
                    endpoint: AnchorEndpoint::Target,
                    node_id: edge.target.clone(),
                });
            }
        }
    }

    if hovered_any {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    // ── During drag: draw a preview dot on the node border ───────────────────
    if let Some(drag) = anchor_drag {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let flow_pp = screen_to_flow(pp, transform);
        if let Some(node) = node_lookup.get(&drag.node_id) {
            let preview_anchor = project_to_border(flow_pp, node.rect());
            let preview_pt = preview_anchor.resolve(node.rect());
            let preview_screen = flow_to_screen(preview_pt, transform);
            let r = indicator_r * 1.8;
            painter.circle_filled(preview_screen, r, config.edge_contact_indicator_hover_color);
            painter.circle_stroke(
                preview_screen,
                r,
                egui::Stroke::new(1.5 * transform.scale, egui::Color32::WHITE),
            );
        }
        ui.ctx().request_repaint();
    }

    // ── Release: commit the anchor ───────────────────────────────────────────
    if primary_released {
        if let Some(drag) = anchor_drag.take() {
            let flow_pp = screen_to_flow(pp, transform);
            if let Some(node) = node_lookup.get(&drag.node_id) {
                let anchor = project_to_border(flow_pp, node.rect());
                let (sa, ta) = match drag.endpoint {
                    AnchorEndpoint::Source => (Some(Some(anchor)), None),
                    AnchorEndpoint::Target => (None, Some(Some(anchor))),
                };
                changes.push(EdgeChange::Anchor {
                    id: drag.edge_id.clone(),
                    source_anchor: sa,
                    target_anchor: ta,
                });
                events.push_anchor_changed(
                    drag.edge_id,
                    sa.and_then(|v| v),
                    ta.and_then(|v| v),
                );
            }
        }
    }

    changes
}
