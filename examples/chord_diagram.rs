#![allow(clippy::needless_range_loop, clippy::manual_clamp)]
//! Chord diagram example for egui_xyflow.
//!
//! Implements a D3-style chord diagram: groups are arranged as arcs around
//! a circle, with ribbon-shaped chords connecting groups that have
//! relationships. The width of each chord at its endpoint is proportional
//! to the flow value.
//!
//! Inspired by <https://observablehq.com/@d3/chord-diagram>.
//!
//! Run with: `cargo run --example chord_diagram`

use std::f32::consts::TAU;

use eframe::egui;
use egui_xyflow::prelude::*;
use egui_xyflow::EdgePosition;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const OUTER_RADIUS: f32 = 280.0;
const INNER_RADIUS: f32 = 260.0;
const GAP_ANGLE: f32 = 0.04; // radians between arcs
const NODE_SIZE: f32 = 12.0; // invisible hit-test square
const BEZIER_SAMPLES: usize = 48;
const CHORD_ALPHA: u8 = 153; // ~60% opacity for chords
const LABEL_OFFSET: f32 = 14.0; // px from outer arc to label

// ---------------------------------------------------------------------------
// Colour palette (Tableau 10)
// ---------------------------------------------------------------------------

fn group_color(index: usize) -> egui::Color32 {
    const PALETTE: [(u8, u8, u8); 10] = [
        (31, 119, 180),  // blue
        (255, 127, 14),  // orange
        (44, 160, 44),   // green
        (214, 39, 40),   // red
        (148, 103, 189), // purple
        (140, 86, 75),   // brown
        (227, 119, 194), // pink
        (127, 127, 127), // grey
        (188, 189, 34),  // olive
        (23, 190, 207),  // cyan
    ];
    let (r, g, b) = PALETTE[index % PALETTE.len()];
    egui::Color32::from_rgb(r, g, b)
}

fn chord_color(source_index: usize) -> egui::Color32 {
    let c = group_color(source_index);
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), CHORD_ALPHA)
}

// ---------------------------------------------------------------------------
// Flow matrix data
// ---------------------------------------------------------------------------

const GROUP_NAMES: [&str; 7] = [
    "Asia",
    "Europe",
    "N. America",
    "S. America",
    "Africa",
    "Oceania",
    "Middle East",
];

/// Asymmetric flow matrix: matrix[i][j] = flow from group i to group j.
/// Values are in arbitrary units (billions of dollars of trade, for instance).
fn flow_matrix() -> Vec<Vec<f32>> {
    vec![
        //  Asia   Europe N.Am   S.Am   Africa Ocean  M.East
        vec![120.0, 80.0, 90.0, 30.0, 25.0, 40.0, 55.0], // Asia
        vec![70.0, 100.0, 60.0, 20.0, 35.0, 15.0, 45.0],  // Europe
        vec![85.0, 55.0, 80.0, 40.0, 15.0, 20.0, 30.0],   // N. America
        vec![25.0, 18.0, 35.0, 50.0, 10.0, 5.0, 8.0],     // S. America
        vec![20.0, 40.0, 12.0, 8.0, 45.0, 3.0, 15.0],     // Africa
        vec![35.0, 12.0, 18.0, 4.0, 2.0, 30.0, 6.0],      // Oceania
        vec![50.0, 42.0, 28.0, 6.0, 18.0, 5.0, 40.0],     // Middle East
    ]
}

// ---------------------------------------------------------------------------
// Chord layout computation
// ---------------------------------------------------------------------------

/// Per-group layout: the arc that this group occupies on the outer ring.
struct GroupArc {
    start_angle: f32,
    end_angle: f32,
}

/// Per-chord layout: angular extents on the source and target arcs.
struct ChordLayout {
    source_group: usize,
    target_group: usize,
    source_start: f32,
    source_end: f32,
    target_start: f32,
    target_end: f32,
    #[allow(dead_code)]
    value: f32,
}

struct LayoutResult {
    group_arcs: Vec<GroupArc>,
    chords: Vec<ChordLayout>,
    total_flow: f32,
}

fn compute_chord_layout(matrix: &[Vec<f32>]) -> LayoutResult {
    let n = matrix.len();

    // Total flow per group = sum of row + sum of column (both directions)
    let mut group_totals = vec![0.0_f32; n];
    let mut grand_total = 0.0_f32;
    for i in 0..n {
        for j in 0..n {
            group_totals[i] += matrix[i][j];
            grand_total += matrix[i][j];
        }
    }

    // Available angle after gaps
    let total_gap = GAP_ANGLE * n as f32;
    let available = TAU - total_gap;

    // Each group gets an angular span proportional to its total outflow
    let mut group_arcs = Vec::with_capacity(n);
    let mut cursor = 0.0_f32;
    for i in 0..n {
        let span = available * (group_totals[i] / grand_total);
        group_arcs.push(GroupArc {
            start_angle: cursor,
            end_angle: cursor + span,
        });
        cursor += span + GAP_ANGLE;
    }

    // For each group, track how far along its arc we have allocated chord
    // endpoints. We need separate trackers for outgoing (source side) and
    // incoming (target side). We split each group's arc: the outgoing half
    // comes first, then the incoming half, proportionally.
    //
    // Actually, in a classic D3 chord diagram, each group's arc is divided
    // among ALL matrix entries (both outgoing and incoming). The simplest
    // approach: for group i, the arc is divided into segments for each j:
    //   matrix[i][j] (outgoing to j) and matrix[j][i] (incoming from j).
    //
    // Simpler approach matching D3: iterate (i,j) pairs, allocate source
    // extent from group i's arc, and target extent from group j's arc.

    let mut group_cursors = vec![0.0_f32; n]; // offset within each group's arc

    let mut chords = Vec::new();

    // For each ordered pair (i, j) where i != j, create a chord.
    // We handle i==j (self-loops) as well to show internal flows.
    for i in 0..n {
        for j in 0..n {
            let val = matrix[i][j];
            if val <= 0.0 {
                continue;
            }

            // Source extent: portion of group i's arc
            let src_frac = val / group_totals[i];
            let src_span =
                (group_arcs[i].end_angle - group_arcs[i].start_angle) * src_frac;
            let src_start = group_arcs[i].start_angle + group_cursors[i];
            let src_end = src_start + src_span;
            group_cursors[i] += src_span;

            chords.push(ChordLayout {
                source_group: i,
                target_group: j,
                source_start: src_start,
                source_end: src_end,
                // Target angles will be filled in a second pass
                target_start: 0.0,
                target_end: 0.0,
                value: val,
            });
        }
    }

    // Second pass: allocate target extents on the target group's arc.
    // We need to track incoming cursors separately.
    // Actually, since each group's outgoing fills its arc completely, the
    // target side for group j is also allocated from group j's arc by the
    // matrix[j][*] entries (when j is source). So for chord (i->j), the
    // target on j's arc corresponds to the chord (j->i) source side IF
    // one exists. In D3, the target side of chord (i->j) is a separate
    // allocation on j's arc.
    //
    // For a clean D3-style diagram, let us treat each group's arc as
    // being divided among all outgoing entries matrix[g][*]. The chord
    // from i to j draws a ribbon from the matrix[i][j] segment of i's
    // arc to the matrix[j][i] segment of j's arc. When matrix[j][i]=0
    // there is no return ribbon, but we still draw the outgoing one.
    //
    // Simplification: we already allocated source extents sequentially.
    // For target side of chord (i->j), we look for the chord (j->i) and
    // use its source extent. For one-way flows (no return), we allocate a
    // tiny sliver.

    // Build a lookup: chord index by (source_group, target_group)
    let mut chord_lookup = std::collections::HashMap::new();
    for (idx, c) in chords.iter().enumerate() {
        chord_lookup.insert((c.source_group, c.target_group), idx);
    }

    // For each chord (i->j), set its target extents to the source extents
    // of the chord (j->i), creating the visual symmetry of a ribbon.
    // If (j->i) does not exist, we still want a small target ribbon.
    for idx in 0..chords.len() {
        let i = chords[idx].source_group;
        let j = chords[idx].target_group;
        if let Some(&reverse_idx) = chord_lookup.get(&(j, i)) {
            let src_start = chords[reverse_idx].source_start;
            let src_end = chords[reverse_idx].source_end;
            chords[idx].target_start = src_start;
            chords[idx].target_end = src_end;
        } else {
            // No reverse flow; place a zero-width target at group j's arc start
            let mid = (group_arcs[j].start_angle + group_arcs[j].end_angle) / 2.0;
            chords[idx].target_start = mid;
            chords[idx].target_end = mid;
        }
    }

    // Deduplicate: for pairs (i,j) and (j,i) where i < j, keep only the
    // one with the larger value to avoid drawing two overlapping ribbons.
    // For i==j (self-loops), always keep them.
    let mut keep = vec![true; chords.len()];
    for idx in 0..chords.len() {
        let i = chords[idx].source_group;
        let j = chords[idx].target_group;
        if i > j {
            // Check if the (j,i) chord exists and has >= value; if so, hide this one
            if let Some(&rev_idx) = chord_lookup.get(&(j, i)) {
                if keep[rev_idx] {
                    keep[idx] = false;
                }
            }
        }
    }

    let chords: Vec<ChordLayout> = chords
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| keep[*idx])
        .map(|(_, c)| c)
        .collect();

    LayoutResult {
        group_arcs,
        chords,
        total_flow: grand_total,
    }
}

// ---------------------------------------------------------------------------
// FlowState data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    color: egui::Color32,
    #[allow(dead_code)]
    group_index: usize,
    start_angle: f32,
    end_angle: f32,
}

#[derive(Debug, Clone, Default)]
struct ChordData {
    #[allow(dead_code)]
    source_group: usize,
    source_start: f32,
    source_end: f32,
    target_start: f32,
    target_end: f32,
    color: egui::Color32,
}

// ---------------------------------------------------------------------------
// Arc drawing helper
// ---------------------------------------------------------------------------

/// Generate points for a thick arc (annular sector) from angle_start to
/// angle_end, with inner and outer radii, centered at `center`.
fn arc_points(
    center: egui::Pos2,
    r_inner: f32,
    r_outer: f32,
    angle_start: f32,
    angle_end: f32,
    num_segments: usize,
) -> Vec<egui::Pos2> {
    let mut pts = Vec::with_capacity(num_segments * 2 + 2);

    // Outer arc: start to end
    for i in 0..=num_segments {
        let t = i as f32 / num_segments as f32;
        let a = angle_start + t * (angle_end - angle_start);
        pts.push(egui::pos2(
            center.x + r_outer * a.cos(),
            center.y + r_outer * a.sin(),
        ));
    }

    // Inner arc: end to start (reverse)
    for i in 0..=num_segments {
        let t = i as f32 / num_segments as f32;
        let a = angle_end - t * (angle_end - angle_start);
        pts.push(egui::pos2(
            center.x + r_inner * a.cos(),
            center.y + r_inner * a.sin(),
        ));
    }

    pts
}

// ---------------------------------------------------------------------------
// Custom NodeWidget -- draws the outer arc segment and label
// ---------------------------------------------------------------------------

struct ChordNodeWidget;

impl NodeWidget<NodeData> for ChordNodeWidget {
    fn size(&self, _node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(NODE_SIZE, NODE_SIZE)
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<NodeData>,
        _screen_rect: egui::Rect,
        _config: &FlowConfig,
        hovered: bool,
        transform: &Transform,
    ) {
        let scale = transform.scale;
        let center = egui::pos2(transform.x, transform.y);

        let r_inner = INNER_RADIUS * scale;
        let r_outer = OUTER_RADIUS * scale;

        let start = node.data.start_angle;
        let end = node.data.end_angle;

        if (end - start).abs() < 1e-6 {
            return;
        }

        // Draw the arc segment
        let color = if node.selected || hovered {
            let c = node.data.color;
            egui::Color32::from_rgb(
                (c.r() as u16 + 40).min(255) as u8,
                (c.g() as u16 + 40).min(255) as u8,
                (c.b() as u16 + 40).min(255) as u8,
            )
        } else {
            node.data.color
        };

        let num_segs = ((end - start).abs() * 60.0).max(8.0) as usize;
        let pts = arc_points(center, r_inner, r_outer, start, end, num_segs);
        painter.add(egui::epaint::PathShape {
            points: pts,
            closed: true,
            fill: color,
            stroke: egui::epaint::PathStroke::NONE,
        });

        // Selection outline
        if node.selected || hovered {
            let outline_pts =
                arc_points(center, r_inner - 1.0, r_outer + 1.0, start, end, num_segs);
            painter.add(egui::epaint::PathShape {
                points: outline_pts,
                closed: true,
                fill: egui::Color32::TRANSPARENT,
                stroke: egui::epaint::PathStroke::new(
                    2.0,
                    egui::Color32::from_rgb(50, 50, 50),
                ),
            });
        }

        // Label at the midpoint of the arc, outside the outer ring
        let mid_angle = (start + end) / 2.0;
        let label_r = r_outer + LABEL_OFFSET * scale;
        let label_center = egui::pos2(
            center.x + label_r * mid_angle.cos(),
            center.y + label_r * mid_angle.sin(),
        );

        let font = egui::FontId::proportional((12.0 * scale).max(8.0).min(14.0));
        let galley = painter.layout_no_wrap(
            node.data.label.clone(),
            font,
            egui::Color32::from_rgb(50, 50, 50),
        );

        // Rotate text alignment based on angle
        let is_right = mid_angle.cos() >= 0.0;
        let text_pos = if is_right {
            egui::pos2(
                label_center.x,
                label_center.y - galley.size().y / 2.0,
            )
        } else {
            egui::pos2(
                label_center.x - galley.size().x,
                label_center.y - galley.size().y / 2.0,
            )
        };

        painter.galley(
            text_pos,
            galley,
            egui::Color32::from_rgb(50, 50, 50),
        );
    }
}

// ---------------------------------------------------------------------------
// Custom EdgeWidget -- draws ribbon chords
// ---------------------------------------------------------------------------

struct ChordEdgeWidget;

impl EdgeWidget<ChordData> for ChordEdgeWidget {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &Edge<ChordData>,
        _pos: &EdgePosition,
        _config: &FlowConfig,
        _time: f64,
        transform: &Transform,
    ) {
        let d = match edge.data.as_ref() {
            Some(d) => d,
            None => return,
        };

        let scale = transform.scale;
        let center = egui::pos2(transform.x, transform.y);
        let r = INNER_RADIUS * scale;

        // The ribbon connects two arc segments on the inner circle.
        // Source arc: from source_start to source_end
        // Target arc: from target_start to target_end
        //
        // The ribbon is drawn as:
        //   1. Arc along source from source_start to source_end
        //   2. Bezier curve from source_end to target_start
        //   3. Arc along target from target_start to target_end
        //   4. Bezier curve from target_end back to source_start

        let color = if edge.selected {
            egui::Color32::from_rgb(59, 130, 246)
        } else {
            d.color
        };

        let angle_to_pt = |angle: f32| -> egui::Pos2 {
            egui::pos2(center.x + r * angle.cos(), center.y + r * angle.sin())
        };

        let mut points = Vec::with_capacity(BEZIER_SAMPLES * 4 + 4);

        // 1. Source arc: source_start -> source_end
        let src_segs = ((d.source_end - d.source_start).abs() * 40.0).max(4.0) as usize;
        for i in 0..=src_segs {
            let t = i as f32 / src_segs as f32;
            let a = d.source_start + t * (d.source_end - d.source_start);
            points.push(angle_to_pt(a));
        }

        // 2. Bezier from source_end to target_start (through near center)
        let p0 = angle_to_pt(d.source_end);
        let p3 = angle_to_pt(d.target_start);
        for i in 1..=BEZIER_SAMPLES {
            let t = i as f32 / BEZIER_SAMPLES as f32;
            points.push(cubic_bezier_point(p0, center, center, p3, t));
        }

        // 3. Target arc: target_start -> target_end
        let tgt_segs = ((d.target_end - d.target_start).abs() * 40.0).max(4.0) as usize;
        for i in 1..=tgt_segs {
            let t = i as f32 / tgt_segs as f32;
            let a = d.target_start + t * (d.target_end - d.target_start);
            points.push(angle_to_pt(a));
        }

        // 4. Bezier from target_end back to source_start (through near center)
        let q0 = angle_to_pt(d.target_end);
        let q3 = angle_to_pt(d.source_start);
        for i in 1..BEZIER_SAMPLES {
            let t = i as f32 / BEZIER_SAMPLES as f32;
            points.push(cubic_bezier_point(q0, center, center, q3, t));
        }

        if points.len() >= 3 {
            painter.add(egui::epaint::PathShape {
                points,
                closed: true,
                fill: color,
                stroke: egui::epaint::PathStroke::NONE,
            });
        }
    }
}

fn cubic_bezier_point(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    t: f32,
) -> egui::Pos2 {
    let u = 1.0 - t;
    let u2 = u * u;
    let t2 = t * t;
    egui::pos2(
        u2 * u * p0.x + 3.0 * u2 * t * p1.x + 3.0 * u * t2 * p2.x + t2 * t * p3.x,
        u2 * u * p0.y + 3.0 * u2 * t * p1.y + 3.0 * u * t2 * p2.y + t2 * t * p3.y,
    )
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct ChordDiagramApp {
    state: FlowState<NodeData, ChordData>,
    node_widget: ChordNodeWidget,
    edge_widget: ChordEdgeWidget,
    first_frame: bool,
    group_count: usize,
    chord_count: usize,
    total_flow: f32,
}

impl ChordDiagramApp {
    fn new() -> Self {
        let matrix = flow_matrix();
        let layout = compute_chord_layout(&matrix);
        let n = GROUP_NAMES.len();

        let config = FlowConfig {
            snap_to_grid: false,
            nodes_draggable: false,
            nodes_connectable: false,
            nodes_selectable: true,
            nodes_resizable: false,
            show_background: false,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_text_color: egui::Color32::from_rgb(50, 50, 50),
            edge_stroke_width: 1.0,
            default_source_position: Position::Center,
            default_target_position: Position::Center,
            min_zoom: 0.1,
            max_zoom: 5.0,
            ..FlowConfig::default()
        };

        let mut state = FlowState::new(config);

        // Add group nodes positioned at the midpoint of each arc on the outer ring.
        // The actual arc drawing happens in the custom NodeWidget.
        for i in 0..n {
            let arc = &layout.group_arcs[i];
            let mid_angle = (arc.start_angle + arc.end_angle) / 2.0;
            // Place node at the center of the circle (0,0) offset by arc midpoint
            // so that the flow graph bounding box is reasonable.
            let pos_r = (OUTER_RADIUS + INNER_RADIUS) / 2.0;
            let pos = egui::pos2(pos_r * mid_angle.cos(), pos_r * mid_angle.sin());

            state.add_node(
                Node::builder(format!("g{}", i))
                    .position(pos)
                    .data(NodeData {
                        label: GROUP_NAMES[i].to_string(),
                        color: group_color(i),
                        group_index: i,
                        start_angle: arc.start_angle,
                        end_angle: arc.end_angle,
                    })
                    .size(NODE_SIZE, NODE_SIZE)
                    .build(),
            );
        }

        // Add chord edges
        let chord_count = layout.chords.len();
        for (idx, chord) in layout.chords.iter().enumerate() {
            let mut edge = Edge::new(
                format!("c{}", idx),
                format!("g{}", chord.source_group),
                format!("g{}", chord.target_group),
            )
            .edge_type(EdgeType::Straight);

            edge.data = Some(ChordData {
                source_group: chord.source_group,
                source_start: chord.source_start,
                source_end: chord.source_end,
                target_start: chord.target_start,
                target_end: chord.target_end,
                color: chord_color(chord.source_group),
            });

            state.add_edge(edge);
        }

        Self {
            state,
            node_widget: ChordNodeWidget,
            edge_widget: ChordEdgeWidget,
            first_frame: true,
            group_count: n,
            chord_count,
            total_flow: layout.total_flow,
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for ChordDiagramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Chord Diagram -- Trade Between Regions");
                ui.separator();
                ui.label(format!(
                    "{} groups, {} chords, {:.0} total flow",
                    self.group_count, self.chord_count, self.total_flow,
                ));
                ui.separator();
                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    self.state.fit_view(rect, 80.0, ctx.input(|i| i.time));
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                if self.first_frame {
                    let rect = ui.available_rect_before_wrap();
                    self.state
                        .fit_view(rect, 80.0, ctx.input(|i| i.time));
                    self.first_frame = false;
                }

                let _events = FlowCanvas::new(&mut self.state, &self.node_widget)
                    .edge_widget(&self.edge_widget)
                    .show(ui);
            });
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- Chord Diagram")
            .with_inner_size([1000.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Chord Diagram",
        options,
        Box::new(|_cc| Ok(Box::new(ChordDiagramApp::new()))),
    )
}
