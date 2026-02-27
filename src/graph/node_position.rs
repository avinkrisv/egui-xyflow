use crate::types::position::{SnapGrid, Transform};

/// Convert screen point to flow coordinates.
pub fn screen_to_flow(screen_pos: egui::Pos2, transform: &Transform) -> egui::Pos2 {
    egui::pos2(
        (screen_pos.x - transform.x) / transform.scale,
        (screen_pos.y - transform.y) / transform.scale,
    )
}

/// Convert flow coordinates to screen coordinates.
pub fn flow_to_screen(flow_pos: egui::Pos2, transform: &Transform) -> egui::Pos2 {
    egui::pos2(
        flow_pos.x * transform.scale + transform.x,
        flow_pos.y * transform.scale + transform.y,
    )
}

/// Snap a position to the nearest grid point.
pub fn snap_position(pos: egui::Pos2, grid: &SnapGrid) -> egui::Pos2 {
    egui::pos2(
        grid[0] * (pos.x / grid[0]).round(),
        grid[1] * (pos.y / grid[1]).round(),
    )
}

/// Clamp a position within a coordinate extent.
pub fn clamp_position(pos: egui::Pos2, min: egui::Pos2, max: egui::Pos2) -> egui::Pos2 {
    egui::pos2(pos.x.clamp(min.x, max.x), pos.y.clamp(min.y, max.y))
}
