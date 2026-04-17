//! Node types: [`Node`], [`NodeId`], [`NodeBuilder`], and internal representations.

use std::sync::Arc;

use super::handle::{Handle, NodeHandle};
use super::position::{CoordinateExtent, Dimensions, NodeOrigin, NodeShape, Position};

/// Unique identifier for a node in the graph.
///
/// Internally backed by `Arc<str>` for O(1) cloning.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeId(pub Arc<str>);

impl NodeId {
    /// Create a new node identifier.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Return the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(Arc::from(s))
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

impl From<&String> for NodeId {
    fn from(s: &String) -> Self {
        Self(Arc::from(s.as_str()))
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
    /// Unique identifier within the graph.
    pub id: NodeId,
    /// Position in flow space, relative to the parent when [`parent_id`](Self::parent_id) is set.
    pub position: egui::Pos2,
    /// Arbitrary user payload attached to this node.
    pub data: D,
    /// Custom node type key, used to route to a registered [`crate::render::node_renderer::NodeWidget`].
    pub node_type: Option<String>,
    /// Default side from which outgoing edges leave; falls back to [`FlowConfig`](crate::config::FlowConfig).
    pub source_position: Option<Position>,
    /// Default side at which incoming edges arrive; falls back to [`FlowConfig`](crate::config::FlowConfig).
    pub target_position: Option<Position>,
    /// When `true`, the node and its handles/edges skip rendering and hit-testing.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: bool,
    /// Selection state, maintained by [`NodeChange::Select`](crate::types::changes::NodeChange::Select).
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: bool,
    /// `true` while the user is actively dragging the node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub dragging: bool,
    /// Per-node drag override; `None` inherits [`FlowConfig::nodes_draggable`](crate::config::FlowConfig::nodes_draggable).
    pub draggable: Option<bool>,
    /// Per-node selection override; `None` inherits the global config.
    pub selectable: Option<bool>,
    /// Per-node connection override; `None` inherits the global config.
    pub connectable: Option<bool>,
    /// Per-node deletion override; `None` inherits the global config.
    pub deletable: Option<bool>,
    /// Explicit width in flow-space pixels. When `None`, [`measured`](Self::measured) is used.
    pub width: Option<f32>,
    /// Explicit height in flow-space pixels. When `None`, [`measured`](Self::measured) is used.
    pub height: Option<f32>,
    /// Optional parent node; this node's [`position`](Self::position) becomes relative to the parent.
    pub parent_id: Option<NodeId>,
    /// Explicit render order. Higher values draw on top; `None` uses a default derived from selection/drag state.
    pub z_index: Option<i32>,
    /// Constrains how far the node can be dragged.
    pub extent: Option<NodeExtent>,
    /// When `true`, the parent's bounds grow to keep this child visible.
    #[cfg_attr(feature = "serde", serde(default))]
    pub expand_parent: bool,
    /// Normalised origin offset `[x, y]` in `0.0..=1.0`, controlling which point on the node is anchored to [`position`](Self::position).
    pub origin: Option<NodeOrigin>,
    /// Connection handles attached to this node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub handles: Vec<NodeHandle>,
    /// Dimensions measured during rendering; populated by the widget, not the user.
    pub measured: Option<Dimensions>,
}

impl<D: Default> Node<D> {
    /// Create a [`NodeBuilder`] for constructing a node with a fluent API.
    ///
    /// Requires `D: Default` — the node data is initialised to `D::default()`
    /// and can be overridden with [`.data()`](NodeBuilder::data).  For types
    /// that do not implement `Default`, use [`Node::builder_with_data`].
    pub fn builder(id: impl Into<Arc<str>>) -> NodeBuilder<D> {
        NodeBuilder::new(id)
    }
}

impl<D> Node<D> {
    /// Create a [`NodeBuilder`] with explicit data, without requiring `D: Default`.
    ///
    /// ```rust,ignore
    /// let node = Node::builder_with_data("n1", MyData { label: "hello".into() })
    ///     .position(egui::pos2(100.0, 50.0))
    ///     .build();
    /// ```
    pub fn builder_with_data(id: impl Into<Arc<str>>, data: D) -> NodeBuilder<D> {
        NodeBuilder::with_data(id, data)
    }
}

/// Builder for constructing [`Node`] instances with a fluent API.
///
/// Created via [`Node::builder`] (requires `D: Default`) or
/// [`Node::builder_with_data`] (works for any `D`).
pub struct NodeBuilder<D = ()> {
    id: NodeId,
    position: egui::Pos2,
    data: Option<D>,
    node_type: Option<String>,
    source_position: Option<Position>,
    target_position: Option<Position>,
    hidden: bool,
    draggable: Option<bool>,
    selectable: Option<bool>,
    connectable: Option<bool>,
    deletable: Option<bool>,
    width: Option<f32>,
    height: Option<f32>,
    parent_id: Option<NodeId>,
    z_index: Option<i32>,
    extent: Option<NodeExtent>,
    expand_parent: bool,
    origin: Option<NodeOrigin>,
    handles: Vec<NodeHandle>,
    measured: Option<Dimensions>,
}

impl<D: Default> NodeBuilder<D> {
    /// Create a new builder with the given node ID and default data.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self::with_data(id, D::default())
    }
}

impl<D> NodeBuilder<D> {
    /// Create a new builder with explicit data (no `Default` bound required).
    pub fn with_data(id: impl Into<Arc<str>>, data: D) -> Self {
        Self {
            id: NodeId::new(id),
            position: egui::Pos2::ZERO,
            data: Some(data),
            node_type: None,
            source_position: None,
            target_position: None,
            hidden: false,
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
        }
    }

    /// Set the node's initial position in flow space.
    pub fn position(mut self, pos: egui::Pos2) -> Self {
        self.position = pos;
        self
    }

    /// Set the user data attached to this node.
    pub fn data(mut self, data: D) -> Self {
        self.data = Some(data);
        self
    }

    /// Add a connection handle to this node.
    pub fn handle(mut self, handle: NodeHandle) -> Self {
        self.handles.push(handle);
        self
    }

    /// Set the explicit z-index for rendering order.
    pub fn z_index(mut self, z: i32) -> Self {
        self.z_index = Some(z);
        self
    }

    /// Set whether the node is hidden.
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Set the parent node ID for nested node groups.
    pub fn parent(mut self, parent_id: impl Into<Arc<str>>) -> Self {
        self.parent_id = Some(NodeId::new(parent_id));
        self
    }

    /// Set the explicit width and height of this node.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Consume the builder and return the constructed [`Node`].
    ///
    /// # Panics
    ///
    /// Panics if data was not provided. This cannot happen through the public
    /// API — both [`NodeBuilder::new`] and [`NodeBuilder::with_data`]
    /// guarantee data is set.
    pub fn build(self) -> Node<D> {
        Node {
            id: self.id,
            position: self.position,
            data: self.data.expect("NodeBuilder: data must be set"),
            node_type: self.node_type,
            source_position: self.source_position,
            target_position: self.target_position,
            hidden: self.hidden,
            selected: false,
            dragging: false,
            draggable: self.draggable,
            selectable: self.selectable,
            connectable: self.connectable,
            deletable: self.deletable,
            width: self.width,
            height: self.height,
            parent_id: self.parent_id,
            z_index: self.z_index,
            extent: self.extent,
            expand_parent: self.expand_parent,
            origin: self.origin,
            handles: self.handles,
            measured: self.measured,
        }
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
    /// Resolved geometric shape (from [`crate::render::node_renderer::NodeWidget::shape`]),
    /// used by edge routing to compute perimeter-intersection anchor points
    /// when no explicit handle is defined.
    pub shape: NodeShape,
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
    /// Return the bounding rectangle in flow space.
    pub fn rect(&self) -> egui::Rect {
        let w = self.node.width.or(self.node.measured.map(|d| d.width)).unwrap_or(150.0);
        let h = self.node.height.or(self.node.measured.map(|d| d.height)).unwrap_or(40.0);
        egui::Rect::from_min_size(self.internals.position_absolute, egui::vec2(w, h))
    }

    /// Return the effective width (explicit or measured or default).
    pub fn width(&self) -> f32 {
        self.node.width.or(self.node.measured.map(|d| d.width)).unwrap_or(150.0)
    }

    /// Return the effective height (explicit or measured or default).
    pub fn height(&self) -> f32 {
        self.node.height.or(self.node.measured.map(|d| d.height)).unwrap_or(40.0)
    }
}
