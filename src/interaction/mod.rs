//! Input handling: node dragging, pan/zoom, connection dragging, resize, and box selection.

/// Drag-to-connect: starting from a handle, tracking the pointer, and emitting
/// a [`Connection`](crate::types::connection::Connection) when released over a
/// compatible target handle.
pub mod connection_drag;
/// Node dragging: single-node and multi-selection drag, including grid
/// snapping and parent-relative position updates.
pub mod drag;
/// Viewport pan and zoom: wheel/trackpad zoom, middle/right-button pan, and
/// auto-pan when the pointer approaches the canvas edge during a drag.
pub mod pan_zoom;
pub mod resize;
/// Box-select: click-drag on empty canvas to draw a selection rectangle and
/// select every node whose bounds intersect it.
pub mod selection;
