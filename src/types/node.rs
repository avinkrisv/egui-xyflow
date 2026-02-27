use super::handle::{Handle, NodeHandle};
use super::position::{CoordinateExtent, Dimensions, NodeOrigin, Position};

/// Unique identifier for a node in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Constraint on how far a node can be dragged.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeExtent {
    /// Constrain to the parent node's bounding box.
    Parent,
    /// Constrain to an explicit coordinate rectangle.
    Coordinates(CoordinateExtent),
}

/// A node in the graph, parameterised over user data `D`.
///
/// Create nodes with [`Node::builder`] for a fluent API, or construct
/// the struct directly for full control.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Node<D = ()> {
    pub id: NodeId,
    pub position: egui::Pos2,
    pub data: D,
    pub node_type: Option<String>,
    pub source_position: Option<Position>,
    pub target_position: Option<Position>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub dragging: bool,
    pub draggable: Option<bool>,
    pub selectable: Option<bool>,
    pub connectable: Option<bool>,
    pub deletable: Option<bool>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub parent_id: Option<NodeId>,
    pub z_index: Option<i32>,
    pub extent: Option<NodeExtent>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub expand_parent: bool,
    pub origin: Option<NodeOrigin>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub handles: Vec<NodeHandle>,
    pub measured: Option<Dimensions>,
}

impl<D: Default> Node<D> {
    pub fn builder(id: impl Into<String>) -> NodeBuilder<D> {
        NodeBuilder::new(id)
    }
}

/// Builder for constructing [`Node`] instances with a fluent API.
pub struct NodeBuilder<D = ()> {
    node: Node<D>,
}

impl<D: Default> NodeBuilder<D> {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            node: Node {
                id: NodeId::new(id),
                position: egui::Pos2::ZERO,
                data: D::default(),
                node_type: None,
                source_position: None,
                target_position: None,
                hidden: false,
                selected: false,
                dragging: false,
                draggable: None,
                selectable: None,
                connectable: None,
                deletable: None,
                width: None,
                height: None,
                parent_id: None,
                z_index: None,
                extent: None,
                expand_parent: false,
                origin: None,
                handles: Vec::new(),
                measured: None,
            },
        }
    }

    pub fn position(mut self, pos: egui::Pos2) -> Self {
        self.node.position = pos;
        self
    }

    pub fn data(mut self, data: D) -> Self {
        self.node.data = data;
        self
    }

    pub fn handle(mut self, handle: NodeHandle) -> Self {
        self.node.handles.push(handle);
        self
    }

    pub fn z_index(mut self, z: i32) -> Self {
        self.node.z_index = Some(z);
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.node.hidden = hidden;
        self
    }

    pub fn parent(mut self, parent_id: impl Into<String>) -> Self {
        self.node.parent_id = Some(NodeId::new(parent_id));
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.node.width = Some(width);
        self.node.height = Some(height);
        self
    }

    pub fn build(self) -> Node<D> {
        self.node
    }
}

/// Resolved handle positions for a node, split by source and target.
#[derive(Debug, Clone, Default)]
pub struct NodeHandleBounds {
    /// Source (output) handles.
    pub source: Vec<Handle>,
    /// Target (input) handles.
    pub target: Vec<Handle>,
}

/// Internal computed state for a node (absolute position, z-order, handles).
#[derive(Debug, Clone)]
pub struct NodeInternals {
    /// Absolute position in flow space (after parent offsets).
    pub position_absolute: egui::Pos2,
    /// Resolved z-index for rendering order.
    pub z: i32,
    /// Resolved handle geometry.
    pub handle_bounds: NodeHandleBounds,
}

/// A node paired with its computed internal state.
///
/// This is the type stored in the node lookup map and passed to renderers.
#[derive(Debug, Clone)]
pub struct InternalNode<D = ()> {
    /// The user-facing node data.
    pub node: Node<D>,
    /// Computed internal state.
    pub internals: NodeInternals,
}

impl<D> InternalNode<D> {
    pub fn rect(&self) -> egui::Rect {
        let w = self.node.width.or(self.node.measured.map(|d| d.width)).unwrap_or(150.0);
        let h = self.node.height.or(self.node.measured.map(|d| d.height)).unwrap_or(40.0);
        egui::Rect::from_min_size(self.internals.position_absolute, egui::vec2(w, h))
    }

    pub fn width(&self) -> f32 {
        self.node.width.or(self.node.measured.map(|d| d.width)).unwrap_or(150.0)
    }

    pub fn height(&self) -> f32 {
        self.node.height.or(self.node.measured.map(|d| d.height)).unwrap_or(40.0)
    }
}
