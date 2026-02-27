//! Node resize interaction.
//!
//! When a single node is selected this module:
//!  1. Renders 8 resize handles (corners + edge-midpoints) around the node.
//!  2. Tracks which handle the user is dragging.
//!  3. Produces [`NodeChange`] values that update the node's position and
//!     size in flow-space.
//!
//! The resize handles are **only** shown when exactly one node is selected
//! (matching xyflow behaviour).  When multiple nodes are selected the handles
//! are hidden to avoid ambiguity.

use crate::config::FlowConfig;
use crate::graph::node_position::flow_to_screen;
use crate::types::changes::NodeChange;
use crate::types::node::{InternalNode, NodeId};
use crate::types::position::Transform;

// ─────────────────────────────────────────────────────────────────────────────
// Handle geometry
// ─────────────────────────────────────────────────────────────────────────────

/// One of the eight resize handles that surround a selected node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandleKind {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl ResizeHandleKind {
    pub const ALL: [ResizeHandleKind; 8] = [
        ResizeHandleKind::TopLeft,
        ResizeHandleKind::Top,
        ResizeHandleKind::TopRight,
        ResizeHandleKind::Right,
        ResizeHandleKind::BottomRight,
        ResizeHandleKind::Bottom,
        ResizeHandleKind::BottomLeft,
        ResizeHandleKind::Left,
    ];

    /// Normalised (0..=1, 0..=1) anchor point within the node for this handle.
    pub fn anchor(self) -> (f32, f32) {
        match self {
            ResizeHandleKind::TopLeft => (0.0, 0.0),
            ResizeHandleKind::Top => (0.5, 0.0),
            ResizeHandleKind::TopRight => (1.0, 0.0),
            ResizeHandleKind::Right => (1.0, 0.5),
            ResizeHandleKind::BottomRight => (1.0, 1.0),
            ResizeHandleKind::Bottom => (0.5, 1.0),
            ResizeHandleKind::BottomLeft => (0.0, 1.0),
            ResizeHandleKind::Left => (0.0, 0.5),
        }
    }

    /// egui cursor icon appropriate for this resize direction.
    pub fn cursor(self) -> egui::CursorIcon {
        match self {
            ResizeHandleKind::TopLeft | ResizeHandleKind::BottomRight => {
                egui::CursorIcon::ResizeNwSe
            }
            ResizeHandleKind::TopRight | ResizeHandleKind::BottomLeft => {
                egui::CursorIcon::ResizeNeSw
            }
            ResizeHandleKind::Top | ResizeHandleKind::Bottom => egui::CursorIcon::ResizeVertical,
            ResizeHandleKind::Left | ResizeHandleKind::Right => egui::CursorIcon::ResizeHorizontal,
        }
    }

    /// Returns `(moves_left_edge, moves_top_edge)`.
    pub fn affects_origin(self) -> (bool, bool) {
        let moves_left = matches!(
            self,
            ResizeHandleKind::TopLeft | ResizeHandleKind::Left | ResizeHandleKind::BottomLeft
        );
        let moves_top = matches!(
            self,
            ResizeHandleKind::TopLeft | ResizeHandleKind::Top | ResizeHandleKind::TopRight
        );
        (moves_left, moves_top)
    }

    /// Returns `(scales_width, scales_height)`.
    pub fn affects_size(self) -> (bool, bool) {
        let w = !matches!(self, ResizeHandleKind::Top | ResizeHandleKind::Bottom);
        let h = !matches!(self, ResizeHandleKind::Left | ResizeHandleKind::Right);
        (w, h)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-node resize state (stored in CanvasMemory)
// ─────────────────────────────────────────────────────────────────────────────

/// Transient state kept while a resize drag is in progress.
#[derive(Debug, Clone)]
pub struct ResizeState {
    /// Which node is being resized.
    pub node_id: NodeId,
    /// Which of the eight handles is being dragged.
    pub handle: ResizeHandleKind,
    /// Node position (flow space) at the start of the resize.
    pub initial_pos: egui::Pos2,
    /// Node dimensions (flow space) at the start of the resize.
    pub initial_size: egui::Vec2,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handle size constants
// ─────────────────────────────────────────────────────────────────────────────

pub const HANDLE_SIZE: f32 = 8.0;

const MIN_NODE_SIZE: f32 = 20.0;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Render all 8 resize handles around `node_rect` (screen space) and process
/// drag interactions.
///
/// Returns:
/// * An updated `Option<ResizeState>` (the caller should store this in
///   `CanvasMemory`).
/// * Zero or more [`NodeChange`] values to apply to `FlowState`.
/// * Whether the cursor should be overridden (non-`None` → cursor to use).
#[allow(clippy::too_many_arguments)]
pub fn render_and_handle_resize<D>(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    node_id: &NodeId,
    node_rect: egui::Rect, // screen space
    node: &InternalNode<D>,
    transform: &Transform,
    config: &FlowConfig,
    resize_state: Option<ResizeState>,
) -> (
    Option<ResizeState>,
    Vec<NodeChange<D>>,
    Option<egui::CursorIcon>,
) {
    let mut changes: Vec<NodeChange<D>> = Vec::new();
    let mut cursor_override: Option<egui::CursorIcon> = None;

    // ── Check if we're currently resizing this node ──────────────────────────
    if let Some(ref rs) = resize_state {
        if rs.node_id == *node_id {
            // Ongoing resize — update based on drag delta
            let delta = ui.input(|i| i.pointer.delta());
            let released = ui.input(|i| i.pointer.primary_released());
            cursor_override = Some(rs.handle.cursor());

            // On release, just clear the resize state.  The last drag
            // frame already applied the correct dimensions; re-computing
            // here would use press_origin() which returns None after
            // release, resetting the node to its initial size.
            if released {
                return (None, changes, cursor_override);
            }

            if delta != egui::Vec2::ZERO {
                // Accumulate total pointer movement since resize started
                // (we use the live pointer position vs the rect corner that was grabbed)
                let total_delta_screen = ui
                    .input(|i| i.pointer.press_origin().zip(i.pointer.hover_pos()))
                    .map(|(origin, hover)| hover - origin)
                    .unwrap_or(egui::Vec2::ZERO);

                let flow_delta = egui::vec2(
                    total_delta_screen.x / transform.scale,
                    total_delta_screen.y / transform.scale,
                );

                let (moves_left, moves_top) = rs.handle.affects_origin();
                let (scales_w, scales_h) = rs.handle.affects_size();

                let mut new_x = rs.initial_pos.x;
                let mut new_y = rs.initial_pos.y;
                let mut new_w = rs.initial_size.x;
                let mut new_h = rs.initial_size.y;

                if scales_w {
                    if moves_left {
                        let delta_w = -flow_delta.x;
                        new_w = (rs.initial_size.x + delta_w).max(MIN_NODE_SIZE);
                        new_x = rs.initial_pos.x + rs.initial_size.x - new_w;
                    } else {
                        new_w = (rs.initial_size.x + flow_delta.x).max(MIN_NODE_SIZE);
                    }
                }

                if scales_h {
                    if moves_top {
                        let delta_h = -flow_delta.y;
                        new_h = (rs.initial_size.y + delta_h).max(MIN_NODE_SIZE);
                        new_y = rs.initial_pos.y + rs.initial_size.y - new_h;
                    } else {
                        new_h = (rs.initial_size.y + flow_delta.y).max(MIN_NODE_SIZE);
                    }
                }

                // Position change (if origin moved)
                if (new_x - rs.initial_pos.x).abs() > 0.1 || (new_y - rs.initial_pos.y).abs() > 0.1
                {
                    changes.push(NodeChange::Position {
                        id: node_id.clone(),
                        position: Some(egui::pos2(new_x, new_y)),
                        dragging: Some(true),
                    });
                }

                // Size change
                changes.push(NodeChange::Dimensions {
                    id: node_id.clone(),
                    dimensions: Some(crate::types::position::Dimensions {
                        width: new_w,
                        height: new_h,
                    }),
                });

                return (resize_state, changes, cursor_override);
            }

            // No movement and not released — keep state.
            return (resize_state, changes, cursor_override);
        }
    }

    // ── No active resize — render handles and check for new press ─────────────
    let handle_color = config.node_selected_border_color;
    let handle_bg = egui::Color32::WHITE;

    let mut new_state: Option<ResizeState> = None;

    for kind in ResizeHandleKind::ALL {
        let (ax, ay) = kind.anchor();
        let center = egui::pos2(
            node_rect.min.x + node_rect.width() * ax,
            node_rect.min.y + node_rect.height() * ay,
        );
        let rect = egui::Rect::from_center_size(center, egui::vec2(HANDLE_SIZE, HANDLE_SIZE));

        // Render
        painter.rect_filled(rect, 2.0, handle_bg);
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.5, handle_color),
            egui::StrokeKind::Middle,
        );

        // Interaction
        let handle_id = ui.id().with(format!("resize_{:?}_{}", node_id, kind as u8));
        let resp = ui.interact(rect, handle_id, egui::Sense::drag());

        if resp.hovered() {
            cursor_override = Some(kind.cursor());
        }

        if resp.drag_started() && new_state.is_none() {
            // Capture initial state
            let flow_pos = node.internals.position_absolute;
            let flow_w = node.width();
            let flow_h = node.height();

            new_state = Some(ResizeState {
                node_id: node_id.clone(),
                handle: kind,
                initial_pos: flow_pos,
                initial_size: egui::vec2(flow_w, flow_h),
            });
            cursor_override = Some(kind.cursor());
        }
    }

    (new_state.or(resize_state), changes, cursor_override)
}

/// Returns `true` if exactly one node in the graph is currently selected and
/// is resizable (i.e. it has explicit or default `selectable = true`).
///
/// Used by the canvas to decide whether resize handles should be shown.
pub fn should_show_resize_handles<D>(
    node_lookup: &std::collections::HashMap<NodeId, InternalNode<D>>,
) -> Option<NodeId> {
    let selected: Vec<&NodeId> = node_lookup
        .iter()
        .filter(|(_, n)| n.node.selected && !n.node.hidden)
        .map(|(id, _)| id)
        .collect();

    if selected.len() == 1 {
        Some(selected[0].clone())
    } else {
        None
    }
}

/// Compute the screen-space rect for a node, taking the transform into account.
pub fn node_screen_rect(node: &InternalNode<impl Sized>, transform: &Transform) -> egui::Rect {
    let origin = flow_to_screen(node.internals.position_absolute, transform);
    let size = egui::vec2(
        node.width() * transform.scale,
        node.height() * transform.scale,
    );
    egui::Rect::from_min_size(origin, size)
}
