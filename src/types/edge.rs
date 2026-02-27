use super::node::NodeId;
use super::position::Position;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeId(pub String);

impl EdgeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EdgeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EdgeType {
    Straight,
    Bezier,
    SmoothStep,
    SimpleBezier,
    Step,
}

impl Default for EdgeType {
    fn default() -> Self {
        EdgeType::Bezier
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarkerType {
    Arrow,
    ArrowClosed,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeMarker {
    pub marker_type: MarkerType,
    pub color: Option<egui::Color32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub stroke_width: Option<f32>,
}

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
}

fn default_interaction_width() -> f32 {
    20.0
}

impl<D: Default> Edge<D> {
    pub fn new(id: impl Into<String>, source: impl Into<String>, target: impl Into<String>) -> Self {
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
        }
    }
}

impl<D> Edge<D> {
    pub fn edge_type(mut self, t: EdgeType) -> Self {
        self.edge_type = Some(t);
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

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

    pub fn source_anchor(mut self, anchor: EdgeAnchor) -> Self {
        self.source_anchor = Some(anchor);
        self
    }

    pub fn target_anchor(mut self, anchor: EdgeAnchor) -> Self {
        self.target_anchor = Some(anchor);
        self
    }

    pub fn anchors_draggable(mut self, draggable: bool) -> Self {
        self.anchors_draggable = Some(draggable);
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
#[derive(Debug, Clone)]
pub struct EdgePathResult {
    pub points: Vec<egui::Pos2>,
    pub label_pos: egui::Pos2,
    pub center_x: f32,
    pub center_y: f32,
}
