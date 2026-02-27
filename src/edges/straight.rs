use smallvec::smallvec;

use crate::types::edge::{EdgePathResult, EdgePosition};

pub fn get_straight_path(pos: &EdgePosition) -> EdgePathResult {
    let source = egui::pos2(pos.source_x, pos.source_y);
    let target = egui::pos2(pos.target_x, pos.target_y);
    let center_x = (pos.source_x + pos.target_x) / 2.0;
    let center_y = (pos.source_y + pos.target_y) / 2.0;

    EdgePathResult {
        points: smallvec![source, target],
        label_pos: egui::pos2(center_x, center_y),
        center_x,
        center_y,
    }
}
