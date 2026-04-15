//! Global configuration for the flow canvas.
//!
//! [`FlowConfig`] controls viewport behaviour, node defaults, connection
//! handling, edge styling, background rendering, and more.  All fields have
//! sensible defaults via [`FlowConfig::default()`].

use crate::animation::easing;
use crate::types::connection::ConnectionMode;
use crate::types::edge::EdgeType;
use crate::types::position::{CoordinateExtent, NodeOrigin, Position, SnapGrid};
use crate::types::viewport::{PanOnScrollMode, SelectionMode};

/// The visual pattern drawn on the canvas background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BackgroundVariant {
    /// A grid of dots.
    Dots,
    /// Horizontal and vertical lines.
    Lines,
    /// Plus-shaped crosses at grid intersections.
    Cross,
}

/// How node z-index is determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ZIndexMode {
    /// Selected nodes are elevated automatically.
    Auto,
    /// Nodes render in insertion order.
    Basic,
    /// Nodes render in explicit `z_index` order.
    Manual,
}

/// Configuration for the flow canvas.
///
/// Controls viewport behaviour, node defaults, connection handling, edge
/// styling, background, grid snapping, handle appearance, animation,
/// and contact indicators.  All fields have sensible defaults.
#[derive(Clone)]
pub struct FlowConfig {
    /// Minimum allowed zoom level.
    pub min_zoom: f32,
    /// Maximum allowed zoom level.
    pub max_zoom: f32,
    /// Enable panning by dragging on the canvas background.
    pub pan_on_drag: bool,
    /// Enable panning via scroll wheel (instead of zooming).
    pub pan_on_scroll: bool,
    /// Enable zooming via scroll wheel.
    pub zoom_on_scroll: bool,
    /// Enable zooming via pinch gesture.
    pub zoom_on_pinch: bool,
    /// Enable zoom-to-fit on double-click.
    pub zoom_on_double_click: bool,
    /// Which axis responds to scroll-based panning.
    pub pan_on_scroll_mode: PanOnScrollMode,
    /// Bounding box that limits how far the viewport can pan.
    pub translate_extent: CoordinateExtent,
    /// Whether nodes can be dragged by default.
    pub nodes_draggable: bool,
    /// Whether nodes can start connections by default.
    pub nodes_connectable: bool,
    /// Whether nodes can be selected by default.
    pub nodes_selectable: bool,
    /// When `true`, a selected node shows 8 resize handles on its bounding
    /// rect.  Set to `false` for graph visualisations with custom-shaped
    /// (e.g. circular) nodes where rectangular resize handles are
    /// inappropriate.
    pub nodes_resizable: bool,
    /// Minimum drag distance (in pixels) before a drag starts.
    pub node_drag_threshold: f32,
    /// Bounding box that limits how far nodes can be dragged.
    pub node_extent: CoordinateExtent,
    /// Origin offset `[x, y]` applied to node positions (0.0--1.0).
    pub node_origin: NodeOrigin,
    /// Edge path algorithm used for new edges.
    pub default_edge_type: EdgeType,
    /// Default edge stroke colour.
    pub edge_color: egui::Color32,
    /// Edge stroke colour when selected.
    pub edge_selected_color: egui::Color32,
    /// Default edge stroke width in logical pixels.
    pub edge_stroke_width: f32,
    /// Raise connected edges above others when a node is selected.
    pub elevate_edges_on_select: bool,
    /// Snap node positions to the grid when dragging.
    pub snap_to_grid: bool,
    /// Grid cell size `[x, y]` used for snapping.
    pub snap_grid: SnapGrid,
    /// How much of a node must overlap the selection rectangle.
    pub selection_mode: SelectionMode,
    /// If true, hold Shift to add to an existing selection.
    pub multi_selection_shift: bool,
    /// Whether connections must land on a handle or can attach freely.
    pub connection_mode: ConnectionMode,
    /// Maximum distance (in pixels) to snap a connection to a handle.
    pub connection_radius: f32,
    /// Edge path algorithm used for the in-progress connection line.
    pub connection_line_type: EdgeType,
    /// Auto-pan the viewport when dragging a connection near the edge.
    pub auto_pan_on_connect: bool,
    /// Auto-pan the viewport when dragging a node near the edge.
    pub auto_pan_on_node_drag: bool,
    /// Speed of auto-panning in pixels per frame.
    pub auto_pan_speed: f32,
    /// Show the minimap overlay.
    pub show_minimap: bool,
    /// Show the background pattern.
    pub show_background: bool,
    /// Which background pattern to draw.
    pub background_variant: BackgroundVariant,
    /// Spacing between background pattern elements.
    pub background_gap: f32,
    /// Size of individual background pattern elements.
    pub background_size: f32,
    /// Colour of the background pattern.
    pub background_color: egui::Color32,
    /// How node z-index ordering is determined.
    pub z_index_mode: ZIndexMode,
    /// Dash length for animated edges (in logical pixels).
    pub animated_edge_dash_length: f32,
    /// Gap length between dashes for animated edges.
    pub animated_edge_gap_length: f32,
    /// Animation speed for animated edges (pixels per second).
    pub animated_edge_speed: f32,
    /// Default duration for viewport transitions (in seconds).
    pub default_transition_duration: f32,
    /// Default easing function for viewport transitions.
    pub default_transition_easing: fn(f32) -> f32,
    /// Animate the in-progress connection line.
    pub connection_line_animated: bool,
    /// Diameter of connection handles (in logical pixels).
    pub handle_size: f32,
    /// Default handle fill colour.
    pub handle_color: egui::Color32,
    /// Handle fill colour on hover.
    pub handle_hover_color: egui::Color32,
    /// Handle fill colour when a connection is attached.
    pub handle_connected_color: egui::Color32,
    /// Default node width when not explicitly set.
    pub default_node_width: f32,
    /// Default node height when not explicitly set.
    pub default_node_height: f32,
    /// Default node background fill colour.
    pub node_bg_color: egui::Color32,
    /// Node background fill colour when selected.
    pub node_selected_bg_color: egui::Color32,
    /// Default node border colour.
    pub node_border_color: egui::Color32,
    /// Node border colour when selected.
    pub node_selected_border_color: egui::Color32,
    /// Node border stroke width.
    pub node_border_width: f32,
    /// Node corner rounding radius.
    pub node_corner_radius: f32,
    /// Opacity multiplier (0.0–1.0) applied to node background fills in the
    /// default node widgets.  Values below 1.0 let edges underneath show
    /// through the node body.  Default: `1.0` (fully opaque).
    pub node_bg_opacity: f32,
    /// Default node text colour.
    pub node_text_color: egui::Color32,
    /// Default position where source edges connect to a node when no explicit
    /// handle is specified.  Set to [`Position::Center`] for force-directed or
    /// radial graph visualisations.  Default: `Position::Right`.
    pub default_source_position: Position,
    /// Default position where target edges connect to a node when no explicit
    /// handle is specified.  Set to [`Position::Center`] for force-directed or
    /// radial graph visualisations.  Default: `Position::Left`.
    pub default_target_position: Position,
    /// When `true`, users can drag edge endpoints to reposition them on the
    /// node border. The new position is stored as an `EdgeAnchor` on the edge.
    pub edge_anchors_draggable: bool,
    /// When `true`, small circles are drawn at edge source/target connection
    /// points.  These serve as visual indicators and grab handles for anchor
    /// dragging.
    pub show_edge_contact_indicators: bool,
    /// Radius of the edge contact indicator circle (in logical pixels, before
    /// zoom scaling).
    pub edge_contact_indicator_radius: f32,
    /// Fill colour of the edge contact indicator.
    pub edge_contact_indicator_color: egui::Color32,
    /// Fill colour when the pointer hovers over a contact indicator.
    pub edge_contact_indicator_hover_color: egui::Color32,
    /// Font size (in logical pixels) for edge labels set via `Edge::label`.
    pub edge_label_font_size: f32,
    /// Text colour for edge labels.
    pub edge_label_color: egui::Color32,
    /// Background colour painted behind edge labels for legibility. Set to
    /// `Color32::TRANSPARENT` to disable.
    pub edge_label_bg_color: egui::Color32,
    /// Padding (in logical pixels) applied around edge label text when drawing
    /// the background rectangle.
    pub edge_label_padding: f32,
    /// When `true`, edges whose bounding box falls entirely outside the
    /// visible canvas rect are skipped during rendering. Dramatically reduces
    /// per-frame cost for large graphs (especially `SmoothStep` edges) at the
    /// price of an AABB test per edge.
    pub cull_offscreen_edges: bool,
    /// When `true`, each frame runs the edge hit-test against the pointer
    /// position (whether or not the primary button is pressed) and exposes
    /// the match via
    /// [`FlowEvents::edge_hovered`](crate::events::FlowEvents::edge_hovered).
    /// Cost is O(edges) per frame with a polyline sample per edge; disable
    /// on very large graphs if you don't need hover cues. Default: `true`.
    pub track_edge_hover: bool,
}

impl FlowConfig {
    /// Return [`node_corner_radius`](Self::node_corner_radius) as an
    /// [`egui::CornerRadius`] for use in custom [`NodeWidget`](crate::render::node_renderer::NodeWidget)
    /// implementations.
    ///
    /// The `f32` value is rounded and clamped to `u8` range (0–255).
    /// Modify individual fields on the returned value for per-corner control:
    ///
    /// ```rust,ignore
    /// let mut r = config.corner_radius();
    /// r.ne = 0; // sharp top-right
    /// r.se = 0; // sharp bottom-right
    /// painter.rect_filled(rect, r, color);
    /// ```
    pub fn corner_radius(&self) -> egui::CornerRadius {
        let r = self.node_corner_radius.round().clamp(0.0, 255.0) as u8;
        egui::CornerRadius { nw: r, ne: r, sw: r, se: r }
    }
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            min_zoom: 0.5,
            max_zoom: 2.0,
            pan_on_drag: true,
            pan_on_scroll: false,
            zoom_on_scroll: true,
            zoom_on_pinch: true,
            zoom_on_double_click: true,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            translate_extent: CoordinateExtent::INFINITE,
            nodes_draggable: true,
            nodes_connectable: true,
            nodes_selectable: true,
            nodes_resizable: true,
            node_drag_threshold: 1.0,
            node_extent: CoordinateExtent::INFINITE,
            node_origin: [0.0, 0.0],
            default_edge_type: EdgeType::Bezier,
            edge_color: egui::Color32::from_rgb(177, 177, 183),
            edge_selected_color: egui::Color32::from_rgb(59, 130, 246),
            edge_stroke_width: 1.5,
            elevate_edges_on_select: true,
            snap_to_grid: false,
            snap_grid: [15.0, 15.0],
            selection_mode: SelectionMode::Partial,
            multi_selection_shift: true,
            connection_mode: ConnectionMode::Strict,
            connection_radius: 20.0,
            connection_line_type: EdgeType::Bezier,
            auto_pan_on_connect: true,
            auto_pan_on_node_drag: true,
            auto_pan_speed: 15.0,
            show_minimap: false,
            show_background: true,
            background_variant: BackgroundVariant::Dots,
            background_gap: 20.0,
            background_size: 1.0,
            background_color: egui::Color32::from_rgb(100, 100, 100),
            z_index_mode: ZIndexMode::Auto,
            animated_edge_dash_length: 5.0,
            animated_edge_gap_length: 5.0,
            animated_edge_speed: 20.0,
            default_transition_duration: 0.3,
            default_transition_easing: easing::ease_cubic,
            connection_line_animated: false,
            handle_size: 8.0,
            handle_color: egui::Color32::from_rgb(177, 177, 183),
            handle_hover_color: egui::Color32::from_rgb(59, 130, 246),
            handle_connected_color: egui::Color32::from_rgb(59, 130, 246),
            default_node_width: 150.0,
            default_node_height: 40.0,
            node_bg_color: egui::Color32::WHITE,
            node_selected_bg_color: egui::Color32::WHITE,
            node_border_color: egui::Color32::from_rgb(177, 177, 183),
            node_selected_border_color: egui::Color32::from_rgb(59, 130, 246),
            node_border_width: 1.0,
            node_corner_radius: 5.0,
            node_bg_opacity: 1.0,
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            default_source_position: Position::Right,
            default_target_position: Position::Left,
            edge_anchors_draggable: false,
            show_edge_contact_indicators: true,
            edge_contact_indicator_radius: 4.0,
            edge_contact_indicator_color: egui::Color32::from_rgb(177, 177, 183),
            edge_contact_indicator_hover_color: egui::Color32::from_rgb(59, 130, 246),
            edge_label_font_size: 11.0,
            edge_label_color: egui::Color32::from_rgb(50, 50, 50),
            edge_label_bg_color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 230),
            edge_label_padding: 3.0,
            cull_offscreen_edges: true,
            track_edge_hover: true,
        }
    }
}
