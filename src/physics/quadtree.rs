//! A 2D quadtree with per-point payload, used by [`ManyBodyForce`](super::ManyBodyForce)
//! for Barnes–Hut approximation.
//!
//! The tree stores the original point index and a `value` per point
//! (typically a charge strength). Each internal node accumulates:
//!
//! * `value` — the sum of child `value`s,
//! * `(cx, cy)` — the value-weighted centre of mass of the subtree.
//!
//! During traversal ([`visit_approx`](QuadTree::visit_approx)), a subtree is
//! treated as a single body whenever its bounding width over distance is
//! below `theta`, giving O(n log n) average cost versus the O(n²) of naïve
//! pairwise iteration.

/// A point in the quadtree: original index + position + value (strength).
#[derive(Debug, Clone, Copy)]
struct Point {
    idx: usize,
    x: f32,
    y: f32,
    value: f32,
}

/// One quadtree cell. Either a leaf (with one or more coincident points) or
/// an internal node with up to four children.
#[derive(Debug, Clone)]
enum Cell {
    Empty,
    Leaf(Vec<Point>),
    Internal {
        children: [Box<Cell>; 4],
    },
}

/// Aggregate summary computed after insertion: per-cell bounding box, total
/// value, and centre of mass.
#[derive(Debug, Clone)]
pub struct CellSummary {
    /// Left edge of the cell's bounding box.
    pub x1: f32,
    /// Top edge of the cell's bounding box.
    pub y1: f32,
    /// Right edge of the cell's bounding box.
    pub x2: f32,
    /// Bottom edge of the cell's bounding box.
    pub y2: f32,
    /// x-coordinate of the value-weighted centre of mass of this subtree.
    pub cx: f32,
    /// y-coordinate of the value-weighted centre of mass of this subtree.
    pub cy: f32,
    /// Sum of child `value`s in this subtree (typically total charge strength).
    pub value: f32,
    /// `true` if this cell is a leaf (holds zero or more coincident points
    /// and no children).
    pub is_leaf: bool,
    /// Indices of points contained in this cell (only populated for leaves).
    pub points: Vec<(usize, f32, f32, f32)>,
    /// Per-quadrant child summaries (NW, NE, SW, SE). `None` for empty quadrants.
    pub children: [Option<Box<CellSummary>>; 4],
}

/// Quadtree over a set of 2D points with scalar values.
#[derive(Debug, Clone)]
pub struct QuadTree {
    root: CellSummary,
    count: usize,
}

impl QuadTree {
    /// Build a quadtree from `(x, y, value)` triples.  `value` is typically
    /// a charge strength (negative for repulsion).
    pub fn new(points: &[(f32, f32, f32)]) -> Self {
        if points.is_empty() {
            return Self {
                root: CellSummary {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 0.0,
                    cx: 0.0,
                    cy: 0.0,
                    value: 0.0,
                    is_leaf: true,
                    points: Vec::new(),
                    children: [None, None, None, None],
                },
                count: 0,
            };
        }

        // Compute the overall bounding box, expanded to a square so the
        // subdivision is symmetric — matches D3's quadtree cover behaviour.
        let (mut x1, mut y1) = (f32::INFINITY, f32::INFINITY);
        let (mut x2, mut y2) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for &(x, y, _) in points {
            if x < x1 {
                x1 = x;
            }
            if y < y1 {
                y1 = y;
            }
            if x > x2 {
                x2 = x;
            }
            if y > y2 {
                y2 = y;
            }
        }
        // Guarantee a non-zero extent so recursion terminates even for
        // degenerate (all-coincident) inputs.
        let mut w = (x2 - x1).max(y2 - y1);
        if w == 0.0 {
            w = 1.0;
        }
        // Pad slightly so boundary points end up on the right side of
        // bisections.
        w *= 1.0 + 1e-6;
        let (x1, y1, x2, y2) = (x1, y1, x1 + w, y1 + w);

        let mut root_cell = Cell::Empty;
        for (i, &(x, y, v)) in points.iter().enumerate() {
            insert(&mut root_cell, Point { idx: i, x, y, value: v }, x1, y1, x2, y2);
        }

        let root = summarise(&root_cell, x1, y1, x2, y2);
        Self { root, count: points.len() }
    }

    /// Returns `true` if the tree contains no points.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Number of points inserted into the tree.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Root [`CellSummary`], covering the full bounding square.
    pub fn root(&self) -> &CellSummary {
        &self.root
    }

    /// Walk the tree and call `f` either on a single leaf point or on an
    /// aggregated internal cell (treated as a single body). `theta2` is the
    /// squared Barnes–Hut threshold: smaller = more accurate, slower.
    /// D3's default is `0.81` (θ = 0.9).
    ///
    /// `f` receives `(x, y, value, is_leaf, leaf_idx)`. When `is_leaf`
    /// is `true` and `leaf_idx == Some(i)`, the point is the same as query
    /// point `query_idx`; the caller decides whether to skip it.
    pub fn visit_approx<F>(
        &self,
        query_x: f32,
        query_y: f32,
        query_idx: usize,
        theta2: f32,
        mut f: F,
    ) where
        F: FnMut(f32, f32, f32, bool, Option<usize>),
    {
        visit(&self.root, query_x, query_y, query_idx, theta2, &mut f);
    }
}

fn insert(cell: &mut Cell, p: Point, x1: f32, y1: f32, x2: f32, y2: f32) {
    match cell {
        Cell::Empty => {
            *cell = Cell::Leaf(vec![p]);
        }
        Cell::Leaf(points) => {
            // If all existing leaf points coincide with `p`, keep them in
            // the same leaf — otherwise further subdivision would never
            // terminate. D3 behaves the same way.
            let coincident = points.iter().all(|q| q.x == p.x && q.y == p.y);
            if coincident {
                points.push(p);
                return;
            }
            // Otherwise convert leaf to internal, re-insert existing
            // points, then insert the new one.
            let existing = std::mem::take(points);
            let mut internal = Cell::Internal {
                children: [
                    Box::new(Cell::Empty),
                    Box::new(Cell::Empty),
                    Box::new(Cell::Empty),
                    Box::new(Cell::Empty),
                ],
            };
            for q in existing {
                insert_into_internal(&mut internal, q, x1, y1, x2, y2);
            }
            insert_into_internal(&mut internal, p, x1, y1, x2, y2);
            *cell = internal;
        }
        Cell::Internal { .. } => {
            insert_into_internal(cell, p, x1, y1, x2, y2);
        }
    }
}

fn insert_into_internal(cell: &mut Cell, p: Point, x1: f32, y1: f32, x2: f32, y2: f32) {
    let xm = (x1 + x2) * 0.5;
    let ym = (y1 + y2) * 0.5;
    let right = p.x >= xm;
    let bottom = p.y >= ym;
    let quad = match (right, bottom) {
        (false, false) => 0, // NW
        (true, false) => 1,  // NE
        (false, true) => 2,  // SW
        (true, true) => 3,   // SE
    };
    let (cx1, cy1, cx2, cy2) = match quad {
        0 => (x1, y1, xm, ym),
        1 => (xm, y1, x2, ym),
        2 => (x1, ym, xm, y2),
        _ => (xm, ym, x2, y2),
    };
    if let Cell::Internal { children } = cell {
        insert(&mut *children[quad], p, cx1, cy1, cx2, cy2);
    }
}

fn summarise(cell: &Cell, x1: f32, y1: f32, x2: f32, y2: f32) -> CellSummary {
    match cell {
        Cell::Empty => CellSummary {
            x1,
            y1,
            x2,
            y2,
            cx: 0.0,
            cy: 0.0,
            value: 0.0,
            is_leaf: true,
            points: Vec::new(),
            children: [None, None, None, None],
        },
        Cell::Leaf(points) => {
            let mut value = 0.0;
            let mut wsx = 0.0;
            let mut wsy = 0.0;
            let mut total_w = 0.0;
            for p in points {
                value += p.value;
                let w = p.value.abs();
                wsx += p.x * w;
                wsy += p.y * w;
                total_w += w;
            }
            let (cx, cy) = if total_w > 0.0 {
                (wsx / total_w, wsy / total_w)
            } else {
                // zero-value points: fall back to plain centroid
                let n = points.len() as f32;
                let (sx, sy) = points.iter().fold((0.0, 0.0), |(sx, sy), p| (sx + p.x, sy + p.y));
                (sx / n, sy / n)
            };
            CellSummary {
                x1,
                y1,
                x2,
                y2,
                cx,
                cy,
                value,
                is_leaf: true,
                points: points.iter().map(|p| (p.idx, p.x, p.y, p.value)).collect(),
                children: [None, None, None, None],
            }
        }
        Cell::Internal { children } => {
            let xm = (x1 + x2) * 0.5;
            let ym = (y1 + y2) * 0.5;
            let boxes = [
                (x1, y1, xm, ym),
                (xm, y1, x2, ym),
                (x1, ym, xm, y2),
                (xm, ym, x2, y2),
            ];
            let mut summaries: [Option<Box<CellSummary>>; 4] = [None, None, None, None];
            let mut value = 0.0;
            let mut wsx = 0.0;
            let mut wsy = 0.0;
            let mut total_w = 0.0;
            for i in 0..4 {
                let s = summarise(&children[i], boxes[i].0, boxes[i].1, boxes[i].2, boxes[i].3);
                value += s.value;
                let w = s.value.abs();
                wsx += s.cx * w;
                wsy += s.cy * w;
                total_w += w;
                // Skip truly empty subtrees from the output to save walk cost.
                if !(s.is_leaf && s.points.is_empty()) {
                    summaries[i] = Some(Box::new(s));
                }
            }
            let (cx, cy) = if total_w > 0.0 {
                (wsx / total_w, wsy / total_w)
            } else {
                ((x1 + x2) * 0.5, (y1 + y2) * 0.5)
            };
            CellSummary {
                x1,
                y1,
                x2,
                y2,
                cx,
                cy,
                value,
                is_leaf: false,
                points: Vec::new(),
                children: summaries,
            }
        }
    }
}

fn visit<F>(cell: &CellSummary, qx: f32, qy: f32, qidx: usize, theta2: f32, f: &mut F)
where
    F: FnMut(f32, f32, f32, bool, Option<usize>),
{
    let dx = cell.cx - qx;
    let dy = cell.cy - qy;
    let l = dx * dx + dy * dy;
    let w = cell.x2 - cell.x1;

    if !cell.is_leaf && (w * w) / theta2 < l {
        // Far enough: treat this whole subtree as a single body.
        f(cell.cx, cell.cy, cell.value, false, None);
        return;
    }

    if cell.is_leaf {
        for &(idx, px, py, v) in &cell.points {
            f(px, py, v, true, Some(idx));
            let _ = qidx; // idx-equality handled by caller
        }
        return;
    }

    for child in cell.children.iter().flatten() {
        visit(child, qx, qy, qidx, theta2, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree() {
        let qt = QuadTree::new(&[]);
        assert!(qt.is_empty());
        assert_eq!(qt.len(), 0);
    }

    #[test]
    fn single_point_is_leaf() {
        let qt = QuadTree::new(&[(1.0, 2.0, -30.0)]);
        assert_eq!(qt.len(), 1);
        assert!(qt.root.is_leaf);
        assert_eq!(qt.root.cx, 1.0);
        assert_eq!(qt.root.cy, 2.0);
        assert_eq!(qt.root.value, -30.0);
    }

    #[test]
    fn total_value_equals_input_sum() {
        let pts = [
            (0.0, 0.0, -1.0),
            (10.0, 0.0, -2.0),
            (0.0, 10.0, -3.0),
            (10.0, 10.0, -4.0),
            (5.0, 5.0, -5.0),
        ];
        let qt = QuadTree::new(&pts);
        let expected: f32 = pts.iter().map(|p| p.2).sum();
        assert!((qt.root.value - expected).abs() < 1e-4);
    }

    #[test]
    fn coincident_points_dont_infinite_recurse() {
        let pts: Vec<(f32, f32, f32)> = (0..16).map(|i| (0.0, 0.0, -1.0 * i as f32)).collect();
        let qt = QuadTree::new(&pts);
        assert_eq!(qt.len(), 16);
        // coincident points → single leaf
        assert!(qt.root.is_leaf);
    }

    #[test]
    fn visit_covers_all_points_with_small_theta() {
        // theta² = 0 forces full recursion into leaves. Every point must be
        // visited exactly once.
        let pts = [
            (0.0, 0.0, -1.0),
            (10.0, 0.0, -1.0),
            (0.0, 10.0, -1.0),
            (10.0, 10.0, -1.0),
        ];
        let qt = QuadTree::new(&pts);
        let mut visited: Vec<usize> = Vec::new();
        qt.visit_approx(
            100.0,
            100.0,
            usize::MAX,
            0.0,
            |_x, _y, _v, is_leaf, idx| {
                if is_leaf {
                    visited.push(idx.unwrap());
                }
            },
        );
        visited.sort();
        assert_eq!(visited, vec![0, 1, 2, 3]);
    }

    #[test]
    fn visit_aggregates_when_theta_permits() {
        // Far away query + large theta → tree visits as few aggregates
        // rather than every leaf.
        let pts: Vec<(f32, f32, f32)> =
            (0..64).map(|i| ((i % 8) as f32, (i / 8) as f32, -1.0)).collect();
        let qt = QuadTree::new(&pts);
        let mut leaf_visits = 0;
        let mut aggregate_visits = 0;
        qt.visit_approx(
            10_000.0,
            10_000.0,
            usize::MAX,
            0.81,
            |_x, _y, _v, is_leaf, _idx| {
                if is_leaf {
                    leaf_visits += 1;
                } else {
                    aggregate_visits += 1;
                }
            },
        );
        // With 64 points at distance ~10k, a single root aggregate should
        // win.
        assert_eq!(aggregate_visits, 1);
        assert_eq!(leaf_visits, 0);
    }
}
