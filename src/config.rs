use crate::animation::easing;
use crate::types::connection::ConnectionMode;
use crate::types::edge::EdgeType;
use crate::types::position::{CoordinateExtent, NodeOrigin, Position, SnapGrid};
use crate::types::viewport::{PanOnScrollMode, SelectionMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BackgroundVariant {
    Dots,
    Lines,
    Cross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ZIndexMode {
    Auto,
    Basic,
    Manual,
}

#[derive(Clone)]
pub struct FlowConfig {
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub pan_on_drag: bool,
    pub pan_on_scroll: bool,
    pub zoom_on_scroll: bool,
    pub zoom_on_pinch: bool,
    pub zoom_on_double_click: bool,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub translate_extent: CoordinateExtent,
    pub nodes_draggable: bool,
    pub nodes_connectable: bool,
    pub nodes_selectable: bool,
    /// When `true`, a selected node shows 8 resize handles on its bounding
    /// rect.  Set to `false` for graph visualisations with custom-shaped
    /// (e.g. circular) nodes where rectangular resize handles are
    /// inappropriate.
    pub nodes_resizable: bool,
    pub node_drag_threshold: f32,
    pub node_extent: CoordinateExtent,
    pub node_origin: NodeOrigin,
    pub default_edge_type: EdgeType,
    pub edge_color: egui::Color32,
    pub edge_selected_color: egui::Color32,
    pub edge_stroke_width: f32,
    pub elevate_edges_on_select: bool,
    pub snap_to_grid: bool,
    pub snap_grid: SnapGrid,
    pub selection_mode: SelectionMode,
    /// If true, hold Shift to add to an existing selection.
    pub multi_selection_shift: bool,
    pub connection_mode: ConnectionMode,
    pub connection_radius: f32,
    pub connection_line_type: EdgeType,
    pub auto_pan_on_connect: bool,
    pub auto_pan_on_node_drag: bool,
    pub auto_pan_speed: f32,
    pub show_minimap: bool,
    pub show_background: bool,
    pub background_variant: BackgroundVariant,
    pub background_gap: f32,
    pub background_size: f32,
    pub background_color: egui::Color32,
    pub z_index_mode: ZIndexMode,
    pub animated_edge_dash_length: f32,
    pub animated_edge_gap_length: f32,
    pub animated_edge_speed: f32,
    pub default_transition_duration: f32,
    pub default_transition_easing: fn(f32) -> f32,
    pub connection_line_animated: bool,
    pub handle_size: f32,
    pub handle_color: egui::Color32,
    pub handle_hover_color: egui::Color32,
    pub handle_connected_color: egui::Color32,
    pub default_node_width: f32,
    pub default_node_height: f32,
    pub node_bg_color: egui::Color32,
    pub node_selected_bg_color: egui::Color32,
    pub node_border_color: egui::Color32,
    pub node_selected_border_color: egui::Color32,
    pub node_border_width: f32,
    pub node_corner_radius: f32,
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
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            default_source_position: Position::Right,
            default_target_position: Position::Left,
            edge_anchors_draggable: false,
        }
    }
}
