//! Edge types: [`Edge`], [`EdgeId`], path algorithms, markers, anchors, and styling.

use std::sync::Arc;

use smallvec::SmallVec;

use super::node::NodeId;
use super::position::Position;

/// Per-edge visual style overrides. When set on an [`Edge`], these take
/// priority over the global `FlowConfig` edge colour / stroke settings.
/// Per-edge visual style overrides. When set on an [`Edge`], these take
/// priority over the global `FlowConfig` edge colour / stroke settings.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeStyle {
    /// Override edge colour (when not selected).
    pub color: Option<egui::Color32>,
    /// Override edge colour when selected.
    pub selected_color: Option<egui::Color32>,
    /// Override stroke width (before the 2× selected multiplier).
    pub stroke_width: Option<f32>,
    /// When `Some`, a glow effect is drawn behind the edge.
    pub glow: Option<EdgeGlow>,
}

/// Configuration for an edge glow effect — a wider, semi-transparent stroke
/// painted underneath the main edge stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeGlow {
    /// Colour of the glow.  A semi-transparent colour works best
    /// (e.g. `Color32::from_rgba_unmultiplied(59, 130, 246, 80)`).
    pub color: egui::Color32,
    /// Total width of the glow stroke.  The glow is drawn first, so the main
    /// edge stroke paints on top.  Typical values: 8.0–20.0.
    pub width: f32,
}

impl EdgeGlow {
    /// Create a new glow configuration.
    pub fn new(color: egui::Color32, width: f32) -> Self {
        Self { color, width }
    }
}

/// A user-defined edge endpoint position on a node's border.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeAnchor {
    /// Which side of the node the endpoint is on (Top/Right/Bottom/Left only).
    pub side: Position,
    /// 0.0–1.0 normalized position along that side.
    /// For Top/Bottom: 0.0 = left edge, 1.0 = right edge.
    /// For Left/Right: 0.0 = top edge, 1.0 = bottom edge.
    pub t: f32,
}

impl EdgeAnchor {
    /// Create a new anchor on the given side at normalized position `t`.
    pub fn new(side: Position, t: f32) -> Self {
        Self { side, t: t.clamp(0.0, 1.0) }
    }

    /// Convert this anchor into an absolute flow-space point given the node rect.
    pub fn resolve(&self, rect: egui::Rect) -> egui::Pos2 {
        match self.side {
            Position::Top => egui::pos2(rect.min.x + self.t * rect.width(), rect.min.y),
            Position::Bottom => egui::pos2(rect.min.x + self.t * rect.width(), rect.max.y),
            Position::Left => egui::pos2(rect.min.x, rect.min.y + self.t * rect.height()),
            Position::Right => egui::pos2(rect.max.x, rect.min.y + self.t * rect.height()),
            _ => rect.center(),
        }
    }
}

/// Which endpoint of an edge an anchor drag is acting on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AnchorEndpoint {
    Source,
    Target,
}

/// Unique identifier for an edge in the graph.
///
/// Internally backed by `Arc<str>` for O(1) cloning.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeId(pub Arc<str>);

impl EdgeId {
    /// Create a new edge identifier.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Return the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EdgeId {
    fn from(s: &str) -> Self {
        Self(Arc::from(s))
    }
}

impl From<String> for EdgeId {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

impl From<&String> for EdgeId {
    fn from(s: &String) -> Self {
        Self(Arc::from(s.as_str()))
    }
}

/// The path algorithm used to draw an edge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EdgeType {
    /// A single straight line segment.
    Straight,
    /// A cubic bezier curve (default).
    #[default]
    Bezier,
    /// An orthogonal path with rounded corners.
    SmoothStep,
    /// A simplified bezier with fewer control points.
    SimpleBezier,
    /// An orthogonal path with sharp 90-degree corners.
    Step,
}

/// The shape of an arrow marker at an edge endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarkerType {
    /// Open arrow head (two lines).
    Arrow,
    /// Closed (filled) arrow head.
    ArrowClosed,
}

/// Arrow marker configuration for an edge endpoint.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeMarker {
    pub marker_type: MarkerType,
    pub color: Option<egui::Color32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub stroke_width: Option<f32>,
}

/// An edge connecting two nodes, parameterised over user data `D`.
///
/// Create edges with [`Edge::new`] and chain builder methods for style,
/// animation, markers, anchors, and glow effects.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Edge<D = ()> {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub edge_type: Option<EdgeType>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub animated: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: bool,
    pub deletable: Option<bool>,
    pub selectable: Option<bool>,
    pub data: Option<D>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: bool,
    pub marker_start: Option<EdgeMarker>,
    pub marker_end: Option<EdgeMarker>,
    pub z_index: Option<i32>,
    #[cfg_attr(feature = "serde", serde(default = "default_interaction_width"))]
    pub interaction_width: f32,
    /// User-defined source endpoint anchor. When set, overrides handle position.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_anchor: Option<EdgeAnchor>,
    /// User-defined target endpoint anchor. When set, overrides handle position.
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_anchor: Option<EdgeAnchor>,
    /// Per-edge override for anchor dragging. When `Some(true)`, the user can
    /// drag this edge's endpoints regardless of the global
    /// `FlowConfig::edge_anchors_draggable` setting. When `Some(false)`,
    /// dragging is disabled for this edge. `None` falls back to the global
    /// config.
    pub anchors_draggable: Option<bool>,
    /// Per-edge visual style overrides (colour, stroke width, glow).
    #[cfg_attr(feature = "serde", serde(default))]
    pub style: Option<EdgeStyle>,
    /// Optional text label drawn at the edge's computed `label_pos`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub label: Option<String>,
}

fn default_interaction_width() -> f32 {
    20.0
}

impl<D> Edge<D> {
    /// Create a new edge connecting two nodes.
    pub fn new(id: impl Into<Arc<str>>, source: impl Into<Arc<str>>, target: impl Into<Arc<str>>) -> Self {
        Self {
            id: EdgeId::new(id),
            source: NodeId::new(source),
            target: NodeId::new(target),
            source_handle: None,
            target_handle: None,
            edge_type: None,
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
            label: None,
        }
    }

    /// Alias for [`Edge::new`] mirroring [`Node::builder`](crate::types::node::Node::builder) naming.
    pub fn builder(
        id: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
        target: impl Into<Arc<str>>,
    ) -> Self {
        Self::new(id, source, target)
    }

    /// Set the path algorithm for this edge.
    pub fn edge_type(mut self, t: EdgeType) -> Self {
        self.edge_type = Some(t);
        self
    }

    /// Enable or disable dash animation on this edge.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Add a closed arrow marker at the target end.
    pub fn marker_end_arrow(mut self) -> Self {
        self.marker_end = Some(EdgeMarker {
            marker_type: MarkerType::ArrowClosed,
            color: None,
            width: None,
            height: None,
            stroke_width: None,
        });
        self
    }

    /// Set a custom source endpoint anchor position.
    pub fn source_anchor(mut self, anchor: EdgeAnchor) -> Self {
        self.source_anchor = Some(anchor);
        self
    }

    /// Set a custom target endpoint anchor position.
    pub fn target_anchor(mut self, anchor: EdgeAnchor) -> Self {
        self.target_anchor = Some(anchor);
        self
    }

    /// Override the global anchor-dragging setting for this edge.
    pub fn anchors_draggable(mut self, draggable: bool) -> Self {
        self.anchors_draggable = Some(draggable);
        self
    }

    /// Set a per-edge visual style override.
    pub fn style(mut self, style: EdgeStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set the edge colour (when not selected).
    pub fn color(mut self, color: egui::Color32) -> Self {
        self.style.get_or_insert_with(EdgeStyle::default).color = Some(color);
        self
    }

    /// Set the edge colour when selected.
    pub fn selected_color(mut self, color: egui::Color32) -> Self {
        self.style.get_or_insert_with(EdgeStyle::default).selected_color = Some(color);
        self
    }

    /// Set the edge stroke width.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.style.get_or_insert_with(EdgeStyle::default).stroke_width = Some(width);
        self
    }

    /// Add a glow effect behind the edge.
    pub fn glow(mut self, color: egui::Color32, width: f32) -> Self {
        self.style.get_or_insert_with(EdgeStyle::default).glow = Some(EdgeGlow::new(color, width));
        self
    }

    /// Set a text label drawn at the edge's midpoint.
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }
}

/// Resolved positions for rendering an edge.
#[derive(Debug, Clone, Copy)]
pub struct EdgePosition {
    pub source_x: f32,
    pub source_y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub source_pos: Position,
    pub target_pos: Position,
}

/// Result of edge path calculation.
///
/// `points` uses `SmallVec<[Pos2; 8]>` to avoid heap allocation for the
/// common case (2–7 control points).
#[derive(Debug, Clone)]
pub struct EdgePathResult {
    pub points: SmallVec<[egui::Pos2; 8]>,
    pub label_pos: egui::Pos2,
    pub center_x: f32,
    pub center_y: f32,
}
