#![allow(clippy::needless_range_loop)]
//! Hierarchical Edge Bundling example for egui_xyflow.
//!
//! Implements a D3-style hierarchical edge bundling visualization: leaf nodes
//! are arranged in a circle, and dependency edges between them are drawn as
//! cubic Bezier curves bundled through the hierarchy's internal nodes toward
//! the centre. Edge colour indicates the source module group; hovering a node
//! highlights its connected edges.
//!
//! Inspired by <https://observablehq.com/@d3/hierarchical-edge-bundling>.
//!
//! Run with: `cargo run --example hierarchical_edge_bundling`

use std::cell::Cell;
use std::collections::HashSet;

use eframe::egui;
use egui_xyflow::prelude::*;
use egui_xyflow::EdgePosition;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NODE_SIZE: f32 = 8.0;
const CIRCLE_RADIUS: f32 = 350.0; // flow-space radius of the leaf ring
const DOT_RADIUS: f32 = 3.0;
const LABEL_OFFSET: f32 = 8.0;
const BETA: f32 = 0.85; // bundling tension (0 = straight, 1 = fully bundled)
const EDGE_ALPHA: u8 = 60;
const HIGHLIGHT_ALPHA: u8 = 200;

// ---------------------------------------------------------------------------
// Colour palette for module groups
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

// ---------------------------------------------------------------------------
// Hierarchy builder
// ---------------------------------------------------------------------------

struct HierarchyBuilder {
    labels: Vec<String>,       // full dotted path
    short_labels: Vec<String>, // leaf-only short name
    parents: Vec<Option<usize>>,
    group_indices: Vec<usize>, // top-level group index for colouring
}

impl HierarchyBuilder {
    fn new(root: &str) -> Self {
        Self {
            labels: vec![root.to_string()],
            short_labels: vec![root.to_string()],
            parents: vec![None],
            group_indices: vec![0],
        }
    }

    fn add(&mut self, parent: usize, label: &str, group: usize) -> usize {
        let id = self.labels.len();
        self.labels.push(label.to_string());
        self.short_labels.push(
            label
                .rsplit('.')
                .next()
                .unwrap_or(label)
                .to_string(),
        );
        self.parents.push(Some(parent));
        self.group_indices.push(group);
        id
    }
}

/// Build a software-package hierarchy with ~90 leaf nodes and ~160 import edges.
fn build_hierarchy() -> (HierarchyBuilder, Vec<(usize, usize)>) {
    let mut h = HierarchyBuilder::new("app");

    // Top-level modules (group indices 0..7)
    let ui = h.add(0, "app.ui", 0);
    let net = h.add(0, "app.net", 1);
    let data = h.add(0, "app.data", 2);
    let core = h.add(0, "app.core", 3);
    let auth = h.add(0, "app.auth", 4);
    let util = h.add(0, "app.util", 5);
    let render = h.add(0, "app.render", 6);
    let plugin = h.add(0, "app.plugin", 7);

    // -- ui subtree --
    let ui_widget = h.add(ui, "app.ui.widget", 0);
    let ui_btn = h.add(ui_widget, "app.ui.widget.button", 0);
    let ui_input = h.add(ui_widget, "app.ui.widget.input", 0);
    let ui_slider = h.add(ui_widget, "app.ui.widget.slider", 0);
    let ui_dropdown = h.add(ui_widget, "app.ui.widget.dropdown", 0);
    let ui_checkbox = h.add(ui_widget, "app.ui.widget.checkbox", 0);
    let ui_toggle = h.add(ui_widget, "app.ui.widget.toggle", 0);
    let ui_layout = h.add(ui, "app.ui.layout", 0);
    let ui_flex = h.add(ui_layout, "app.ui.layout.flex", 0);
    let ui_grid = h.add(ui_layout, "app.ui.layout.grid", 0);
    let ui_stack = h.add(ui_layout, "app.ui.layout.stack", 0);
    let ui_theme = h.add(ui, "app.ui.theme", 0);
    let ui_colors = h.add(ui_theme, "app.ui.theme.colors", 0);
    let ui_fonts = h.add(ui_theme, "app.ui.theme.fonts", 0);
    let ui_icons = h.add(ui_theme, "app.ui.theme.icons", 0);

    // -- net subtree --
    let net_http = h.add(net, "app.net.http", 1);
    let net_client = h.add(net_http, "app.net.http.client", 1);
    let net_server = h.add(net_http, "app.net.http.server", 1);
    let net_middleware = h.add(net_http, "app.net.http.middleware", 1);
    let net_router = h.add(net_http, "app.net.http.router", 1);
    let net_ws = h.add(net, "app.net.ws", 1);
    let net_conn = h.add(net_ws, "app.net.ws.connection", 1);
    let net_proto = h.add(net_ws, "app.net.ws.protocol", 1);
    let net_frame = h.add(net_ws, "app.net.ws.frame", 1);
    let net_rpc = h.add(net, "app.net.rpc", 1);
    let net_codec = h.add(net_rpc, "app.net.rpc.codec", 1);
    let net_transport = h.add(net_rpc, "app.net.rpc.transport", 1);
    let net_registry = h.add(net_rpc, "app.net.rpc.registry", 1);

    // -- data subtree --
    let data_store = h.add(data, "app.data.store", 2);
    let data_sql = h.add(data_store, "app.data.store.sql", 2);
    let data_nosql = h.add(data_store, "app.data.store.nosql", 2);
    let data_cache = h.add(data_store, "app.data.store.cache", 2);
    let data_blob = h.add(data_store, "app.data.store.blob", 2);
    let data_query = h.add(data, "app.data.query", 2);
    let data_builder = h.add(data_query, "app.data.query.builder", 2);
    let data_executor = h.add(data_query, "app.data.query.executor", 2);
    let data_planner = h.add(data_query, "app.data.query.planner", 2);
    let data_migrate = h.add(data, "app.data.migrate", 2);
    let data_schema = h.add(data_migrate, "app.data.migrate.schema", 2);
    let data_seed = h.add(data_migrate, "app.data.migrate.seed", 2);
    let data_rollback = h.add(data_migrate, "app.data.migrate.rollback", 2);

    // -- core subtree --
    let core_parser = h.add(core, "app.core.parser", 3);
    let core_lexer = h.add(core_parser, "app.core.parser.lexer", 3);
    let core_ast = h.add(core_parser, "app.core.parser.ast", 3);
    let core_validator = h.add(core_parser, "app.core.parser.validator", 3);
    let core_compiler = h.add(core, "app.core.compiler", 3);
    let core_optimizer = h.add(core_compiler, "app.core.compiler.optimizer", 3);
    let core_codegen = h.add(core_compiler, "app.core.compiler.codegen", 3);
    let core_linker = h.add(core_compiler, "app.core.compiler.linker", 3);
    let core_runtime = h.add(core, "app.core.runtime", 3);
    let core_gc = h.add(core_runtime, "app.core.runtime.gc", 3);
    let core_jit = h.add(core_runtime, "app.core.runtime.jit", 3);
    let core_debugger = h.add(core_runtime, "app.core.runtime.debugger", 3);

    // -- auth subtree --
    let auth_token = h.add(auth, "app.auth.token", 4);
    let auth_jwt = h.add(auth_token, "app.auth.token.jwt", 4);
    let auth_oauth = h.add(auth_token, "app.auth.token.oauth", 4);
    let auth_refresh = h.add(auth_token, "app.auth.token.refresh", 4);
    let auth_session = h.add(auth, "app.auth.session", 4);
    let auth_cookie = h.add(auth_session, "app.auth.session.cookie", 4);
    let auth_store = h.add(auth_session, "app.auth.session.store", 4);
    let auth_perm = h.add(auth, "app.auth.permission", 4);
    let auth_role = h.add(auth_perm, "app.auth.permission.role", 4);
    let auth_acl = h.add(auth_perm, "app.auth.permission.acl", 4);
    let auth_policy = h.add(auth_perm, "app.auth.permission.policy", 4);

    // -- util subtree --
    let util_log = h.add(util, "app.util.log", 5);
    let util_format = h.add(util_log, "app.util.log.format", 5);
    let util_rotate = h.add(util_log, "app.util.log.rotate", 5);
    let util_filter = h.add(util_log, "app.util.log.filter", 5);
    let util_config = h.add(util, "app.util.config", 5);
    let util_loader = h.add(util_config, "app.util.config.loader", 5);
    let util_merge = h.add(util_config, "app.util.config.merge", 5);
    let util_crypto = h.add(util, "app.util.crypto", 5);
    let util_hash = h.add(util_crypto, "app.util.crypto.hash", 5);
    let util_cipher = h.add(util_crypto, "app.util.crypto.cipher", 5);
    let util_rand = h.add(util_crypto, "app.util.crypto.rand", 5);

    // -- render subtree --
    let render_scene = h.add(render, "app.render.scene", 6);
    let render_camera = h.add(render_scene, "app.render.scene.camera", 6);
    let render_light = h.add(render_scene, "app.render.scene.light", 6);
    let render_mesh = h.add(render_scene, "app.render.scene.mesh", 6);
    let render_shader = h.add(render, "app.render.shader", 6);
    let render_vertex = h.add(render_shader, "app.render.shader.vertex", 6);
    let render_fragment = h.add(render_shader, "app.render.shader.fragment", 6);
    let render_compute = h.add(render_shader, "app.render.shader.compute", 6);
    let render_pipe = h.add(render, "app.render.pipeline", 6);
    let render_forward = h.add(render_pipe, "app.render.pipeline.forward", 6);
    let render_deferred = h.add(render_pipe, "app.render.pipeline.deferred", 6);

    // -- plugin subtree --
    let plugin_api = h.add(plugin, "app.plugin.api", 7);
    let plugin_hook = h.add(plugin_api, "app.plugin.api.hook", 7);
    let plugin_event = h.add(plugin_api, "app.plugin.api.event", 7);
    let plugin_ctx = h.add(plugin_api, "app.plugin.api.context", 7);
    let plugin_loader = h.add(plugin, "app.plugin.loader", 7);
    let plugin_scan = h.add(plugin_loader, "app.plugin.loader.scanner", 7);
    let plugin_resolve = h.add(plugin_loader, "app.plugin.loader.resolver", 7);
    let plugin_sandbox = h.add(plugin_loader, "app.plugin.loader.sandbox", 7);

    // -----------------------------------------------------------------------
    // Dependency edges (leaf → leaf imports)
    // -----------------------------------------------------------------------
    let edges = vec![
        // ui widgets depend on theme
        (ui_btn, ui_colors),
        (ui_btn, ui_fonts),
        (ui_input, ui_colors),
        (ui_input, ui_fonts),
        (ui_slider, ui_colors),
        (ui_dropdown, ui_colors),
        (ui_dropdown, ui_icons),
        (ui_checkbox, ui_colors),
        (ui_toggle, ui_colors),
        // layout depends on widgets
        (ui_flex, ui_btn),
        (ui_flex, ui_input),
        (ui_grid, ui_btn),
        (ui_grid, ui_slider),
        (ui_stack, ui_dropdown),
        // net http depends on auth and util
        (net_client, auth_jwt),
        (net_client, auth_oauth),
        (net_client, util_format),
        (net_server, auth_cookie),
        (net_server, util_filter),
        (net_server, util_rotate),
        (net_middleware, auth_role),
        (net_middleware, auth_acl),
        (net_router, util_loader),
        // net ws depends on net http and util
        (net_conn, net_client),
        (net_conn, util_hash),
        (net_proto, net_codec),
        (net_proto, util_cipher),
        (net_frame, net_transport),
        // net rpc depends on data and util
        (net_codec, util_hash),
        (net_transport, util_cipher),
        (net_registry, data_cache),
        // data depends on core and util
        (data_sql, core_lexer),
        (data_sql, core_ast),
        (data_nosql, core_validator),
        (data_nosql, util_hash),
        (data_cache, util_cipher),
        (data_blob, util_rand),
        (data_builder, core_ast),
        (data_builder, core_lexer),
        (data_executor, data_sql),
        (data_executor, data_nosql),
        (data_planner, core_optimizer),
        (data_schema, core_validator),
        (data_seed, util_rand),
        (data_rollback, data_schema),
        // core internal
        (core_ast, core_lexer),
        (core_validator, core_ast),
        (core_optimizer, core_ast),
        (core_codegen, core_optimizer),
        (core_codegen, core_ast),
        (core_linker, core_codegen),
        (core_jit, core_codegen),
        (core_jit, core_optimizer),
        (core_gc, util_format),
        (core_debugger, core_ast),
        (core_debugger, util_format),
        // auth depends on util and data
        (auth_jwt, util_hash),
        (auth_jwt, util_cipher),
        (auth_oauth, net_client),
        (auth_oauth, util_hash),
        (auth_refresh, auth_jwt),
        (auth_cookie, util_cipher),
        (auth_store, data_cache),
        (auth_store, data_nosql),
        (auth_role, data_sql),
        (auth_acl, auth_role),
        (auth_policy, auth_acl),
        (auth_policy, auth_role),
        // util internal
        (util_format, util_filter),
        (util_rotate, util_format),
        (util_loader, util_merge),
        (util_hash, util_rand),
        (util_cipher, util_rand),
        // render depends on core and util
        (render_camera, util_format),
        (render_light, render_camera),
        (render_mesh, render_vertex),
        (render_mesh, util_hash),
        (render_vertex, core_codegen),
        (render_fragment, core_codegen),
        (render_fragment, render_vertex),
        (render_compute, core_jit),
        (render_forward, render_fragment),
        (render_forward, render_light),
        (render_forward, render_camera),
        (render_deferred, render_fragment),
        (render_deferred, render_mesh),
        // plugin depends on core, util, auth
        (plugin_hook, core_ast),
        (plugin_hook, util_format),
        (plugin_event, plugin_hook),
        (plugin_event, util_filter),
        (plugin_ctx, auth_policy),
        (plugin_ctx, data_cache),
        (plugin_scan, util_loader),
        (plugin_resolve, plugin_scan),
        (plugin_resolve, net_registry),
        (plugin_sandbox, auth_policy),
        (plugin_sandbox, core_gc),
        // cross-module edges for visual density
        (ui_btn, plugin_event),
        (ui_input, plugin_event),
        (net_server, render_forward),
        (data_executor, util_format),
        (core_linker, util_hash),
        (render_deferred, util_cipher),
        (plugin_sandbox, util_rand),
        (net_middleware, util_format),
        (auth_refresh, util_cipher),
        (data_planner, util_format),
        (render_compute, util_rand),
        (plugin_resolve, util_hash),
        (net_frame, util_hash),
        (data_cache, util_format),
        (core_debugger, util_filter),
        (auth_cookie, util_hash),
        (render_camera, core_optimizer),
        (plugin_ctx, util_format),
        (net_router, auth_acl),
        (data_blob, util_format),
    ];

    (h, edges)
}

// ---------------------------------------------------------------------------
// Layout: arrange leaves in a circle, compute ancestor paths for bundling
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct LayoutResult {
    /// (x, y) positions in flow space for every node (internal + leaf).
    positions: Vec<egui::Pos2>,
    /// Angular position of each node on the circle.
    angles: Vec<f32>,
    /// Whether each node is a leaf.
    is_leaf: Vec<bool>,
    /// Children of each node.
    children: Vec<Vec<usize>>,
    /// Depth of each node.
    depths: Vec<usize>,
    /// For each node, the path from root to that node (inclusive).
    ancestor_paths: Vec<Vec<usize>>,
}

fn compute_layout(parents: &[Option<usize>]) -> LayoutResult {
    let n = parents.len();

    // Build children lists and depths
    let mut children = vec![Vec::<usize>::new(); n];
    let mut depths = vec![0_usize; n];
    for i in 1..n {
        if let Some(p) = parents[i] {
            children[p].push(i);
            depths[i] = depths[p] + 1;
        }
    }

    let is_leaf: Vec<bool> = children.iter().map(|c| c.is_empty()).collect();

    // Collect leaves in DFS order (preserves hierarchy grouping)
    let mut leaves_ordered = Vec::new();
    fn collect_leaves(node: usize, children: &[Vec<usize>], out: &mut Vec<usize>) {
        if children[node].is_empty() {
            out.push(node);
        } else {
            for &c in &children[node] {
                collect_leaves(c, children, out);
            }
        }
    }
    collect_leaves(0, &children, &mut leaves_ordered);

    let leaf_count = leaves_ordered.len();

    // Assign angular positions to leaves evenly around the circle
    let mut angles = vec![0.0_f32; n];
    for (i, &leaf) in leaves_ordered.iter().enumerate() {
        angles[leaf] = i as f32 / leaf_count as f32 * std::f32::consts::TAU;
    }

    // Internal nodes get the midpoint angle of their children
    fn compute_internal_angles(
        node: usize,
        children: &[Vec<usize>],
        is_leaf: &[bool],
        angles: &mut [f32],
    ) {
        if is_leaf[node] {
            return;
        }
        for &c in &children[node] {
            compute_internal_angles(c, children, is_leaf, angles);
        }
        let sum: f32 = children[node].iter().map(|&c| angles[c]).sum();
        angles[node] = sum / children[node].len() as f32;
    }
    compute_internal_angles(0, &children, &is_leaf, &mut angles);

    // Positions: leaves on the circle, internal nodes at scaled radius
    let max_depth = *depths.iter().max().unwrap_or(&1);
    let positions: Vec<egui::Pos2> = (0..n)
        .map(|i| {
            let r = if is_leaf[i] {
                CIRCLE_RADIUS
            } else if i == 0 {
                0.0
            } else {
                depths[i] as f32 / max_depth as f32 * CIRCLE_RADIUS * 0.8
            };
            let theta = angles[i];
            egui::pos2(r * theta.cos(), r * theta.sin())
        })
        .collect();

    // Ancestor paths: root to node
    let mut ancestor_paths = vec![Vec::new(); n];
    for i in 0..n {
        let mut path = Vec::new();
        let mut cur = i;
        path.push(cur);
        while let Some(p) = parents[cur] {
            path.push(p);
            cur = p;
        }
        path.reverse();
        ancestor_paths[i] = path;
    }

    LayoutResult {
        positions,
        angles,
        is_leaf,
        children,
        depths,
        ancestor_paths,
    }
}

/// Compute the bundled path between two leaf nodes through their hierarchy.
///
/// Returns a list of intermediate control positions in flow space that the
/// edge should pass through. The bundling tension (beta) interpolates between
/// the direct straight line (beta=0) and the fully bundled hierarchy path
/// (beta=1).
fn bundled_path(
    source: usize,
    target: usize,
    ancestor_paths: &[Vec<usize>],
    positions: &[egui::Pos2],
    beta: f32,
) -> Vec<egui::Pos2> {
    let path_s = &ancestor_paths[source];
    let path_t = &ancestor_paths[target];

    // Find lowest common ancestor
    let mut lca_depth = 0;
    for i in 0..path_s.len().min(path_t.len()) {
        if path_s[i] == path_t[i] {
            lca_depth = i;
        } else {
            break;
        }
    }

    // Build the path: source ancestors up to LCA, then down to target
    let mut via_points = Vec::new();
    // source up to (but not including) LCA
    for i in (lca_depth + 1..path_s.len()).rev() {
        via_points.push(positions[path_s[i]]);
    }
    // LCA itself
    via_points.push(positions[path_s[lca_depth]]);
    // LCA down to target
    for i in (lca_depth + 1)..path_t.len() {
        via_points.push(positions[path_t[i]]);
    }

    // Apply bundling tension: interpolate each via point toward the straight line
    let src = positions[source];
    let tgt = positions[target];
    let n = via_points.len();
    for (i, pt) in via_points.iter_mut().enumerate() {
        let t = if n <= 1 {
            0.5
        } else {
            i as f32 / (n - 1) as f32
        };
        // Point on straight line
        let straight = egui::pos2(
            src.x + (tgt.x - src.x) * t,
            src.y + (tgt.y - src.y) * t,
        );
        // Interpolate: beta=1 means fully hierarchy-bundled
        pt.x = straight.x + (pt.x - straight.x) * beta;
        pt.y = straight.y + (pt.y - straight.y) * beta;
    }

    via_points
}

// ---------------------------------------------------------------------------
// Edge data: stores precomputed Bezier control points in flow space
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct BundleEdgeData {
    /// Flow-space control points for the bundled cubic Bezier.
    /// Typically 4 points: [source, cp1, cp2, target].
    control_points: Vec<egui::Pos2>,
    /// Colour based on source group.
    color: egui::Color32,
    /// Source node index (for hover highlighting).
    source_idx: usize,
    /// Target node index (for hover highlighting).
    target_idx: usize,
}

// ---------------------------------------------------------------------------
// Node data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct NodeData {
    label: String,
    is_leaf: bool,
    group: usize,
    /// Original angle on the circle (for label placement).
    angle: f32,
}

// ---------------------------------------------------------------------------
// Custom NodeWidget
// ---------------------------------------------------------------------------

struct BundleNodeWidget;

impl NodeWidget<NodeData> for BundleNodeWidget {
    fn size(&self, _node: &Node<NodeData>, _config: &FlowConfig) -> egui::Vec2 {
        egui::vec2(NODE_SIZE, NODE_SIZE)
    }

    fn show(
        &self,
        painter: &egui::Painter,
        node: &Node<NodeData>,
        screen_rect: egui::Rect,
        config: &FlowConfig,
        hovered: bool,
        _transform: &Transform,
    ) {
        if !node.data.is_leaf {
            return; // only draw leaf nodes
        }

        let center = screen_rect.center();
        let fill = group_color(node.data.group);

        // Highlight ring on hover/select
        if node.selected || hovered {
            painter.circle_filled(
                center,
                DOT_RADIUS + 3.0,
                egui::Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), 80),
            );
        }
        painter.circle_filled(center, DOT_RADIUS, fill);

        // Label
        if node.data.label.is_empty() {
            return;
        }

        let angle = node.data.angle;
        let is_right = angle.abs() < std::f32::consts::FRAC_PI_2;

        let font = egui::FontId::proportional(9.0);
        let text_color = if node.selected || hovered {
            egui::Color32::from_rgb(30, 30, 30)
        } else {
            config.node_text_color
        };
        let galley = painter.layout_no_wrap(node.data.label.clone(), font, text_color);

        let text_pos = if is_right {
            egui::pos2(center.x + LABEL_OFFSET, center.y - galley.size().y / 2.0)
        } else {
            egui::pos2(
                center.x - LABEL_OFFSET - galley.size().x,
                center.y - galley.size().y / 2.0,
            )
        };

        // Halo for readability
        let halo_rect = egui::Rect::from_min_size(
            text_pos - egui::vec2(1.0, 0.0),
            galley.size() + egui::vec2(2.0, 0.0),
        );
        painter.rect_filled(
            halo_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220),
        );

        painter.galley(text_pos, galley, text_color);
    }
}

// ---------------------------------------------------------------------------
// Custom EdgeWidget -- bundled Bezier curves
// ---------------------------------------------------------------------------

struct BundleEdgeWidget {
    /// Set of node indices that are currently hovered (source or target).
    hovered_node_idx: Cell<Option<usize>>,
    /// Precomputed: for each leaf index, the set of leaf indices it connects
    /// to (as source or target).
    #[allow(dead_code)]
    connected_leaves: Vec<HashSet<usize>>,
}

impl BundleEdgeWidget {
    fn new(connected_leaves: Vec<HashSet<usize>>) -> Self {
        Self {
            hovered_node_idx: Cell::new(None),
            connected_leaves,
        }
    }
}

impl EdgeWidget<BundleEdgeData> for BundleEdgeWidget {
    fn show(
        &self,
        painter: &egui::Painter,
        edge: &Edge<BundleEdgeData>,
        _pos: &EdgePosition,
        _config: &FlowConfig,
        _time: f64,
        transform: &Transform,
    ) {
        let d = match edge.data.as_ref() {
            Some(d) => d,
            None => return,
        };

        if d.control_points.len() < 2 {
            return;
        }

        // Determine if this edge should be highlighted
        let hovered = self.hovered_node_idx.get();
        let is_highlighted = match hovered {
            Some(idx) => d.source_idx == idx || d.target_idx == idx,
            None => false,
        };

        let alpha = if hovered.is_some() {
            if is_highlighted {
                HIGHLIGHT_ALPHA
            } else {
                20 // dim non-connected edges
            }
        } else {
            EDGE_ALPHA
        };

        let color = egui::Color32::from_rgba_unmultiplied(
            d.color.r(),
            d.color.g(),
            d.color.b(),
            alpha,
        );

        let width = if is_highlighted { 2.0 } else { 1.0 };
        let stroke = egui::Stroke::new(width, color);

        let pts = &d.control_points;

        // If we have exactly 4 points, draw a single cubic Bezier
        if pts.len() == 4 {
            let screen_pts: Vec<egui::Pos2> = pts
                .iter()
                .map(|p| egui::pos2(p.x * transform.scale + transform.x, p.y * transform.scale + transform.y))
                .collect();
            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                [screen_pts[0], screen_pts[1], screen_pts[2], screen_pts[3]],
                false,
                egui::Color32::TRANSPARENT,
                stroke,
            );
            painter.add(bezier);
            return;
        }

        // For paths with more via-points, sample a composite Bezier through all points
        // using Catmull-Rom-like interpolation converted to cubic segments.
        let screen_pts: Vec<egui::Pos2> = pts
            .iter()
            .map(|p| egui::pos2(p.x * transform.scale + transform.x, p.y * transform.scale + transform.y))
            .collect();

        // Fit a smooth Catmull-Rom spline through the via-points
        let n = screen_pts.len();
        if n < 2 {
            return;
        }

        // Draw as a series of cubic Bezier segments between consecutive points,
        // using Catmull-Rom tangents for control points.
        for seg in 0..(n - 1) {
            let p_prev = if seg > 0 {
                screen_pts[seg - 1]
            } else {
                screen_pts[0]
            };
            let p0 = screen_pts[seg];
            let p1 = screen_pts[seg + 1];
            let p_next = if seg + 2 < n {
                screen_pts[seg + 2]
            } else {
                screen_pts[n - 1]
            };

            // Catmull-Rom tangents (alpha = 0.5 for centripetal; we use uniform here)
            let tension = 1.0 / 3.0;
            let cp1 = egui::pos2(
                p0.x + (p1.x - p_prev.x) * tension,
                p0.y + (p1.y - p_prev.y) * tension,
            );
            let cp2 = egui::pos2(
                p1.x - (p_next.x - p0.x) * tension,
                p1.y - (p_next.y - p0.y) * tension,
            );

            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                [p0, cp1, cp2, p1],
                false,
                egui::Color32::TRANSPARENT,
                stroke,
            );
            painter.add(bezier);
        }
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct HierarchicalEdgeBundlingApp {
    state: FlowState<NodeData, BundleEdgeData>,
    edge_widget: BundleEdgeWidget,
    first_frame: bool,
    leaf_count: usize,
    edge_count: usize,
    group_count: usize,
}

impl HierarchicalEdgeBundlingApp {
    fn new() -> Self {
        let (hierarchy, dep_edges) = build_hierarchy();
        let layout = compute_layout(&hierarchy.parents);
        let n = hierarchy.labels.len();

        // Count leaves and groups
        let leaf_count = layout.is_leaf.iter().filter(|&&l| l).count();
        let group_count = 8;

        // Build connected-leaves map for hover highlighting
        let mut connected_leaves: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        for &(src, tgt) in &dep_edges {
            connected_leaves[src].insert(tgt);
            connected_leaves[tgt].insert(src);
        }

        let config = FlowConfig {
            snap_to_grid: false,
            show_background: false,
            nodes_draggable: false,
            nodes_connectable: false,
            nodes_selectable: true,
            nodes_resizable: false,
            min_zoom: 0.1,
            max_zoom: 5.0,
            default_node_width: NODE_SIZE,
            default_node_height: NODE_SIZE,
            node_bg_color: egui::Color32::TRANSPARENT,
            node_border_width: 0.0,
            node_text_color: egui::Color32::from_rgb(100, 100, 100),
            edge_stroke_width: 1.0,
            default_source_position: Position::Center,
            default_target_position: Position::Center,
            ..FlowConfig::default()
        };

        let mut state = FlowState::new(config);

        // Add nodes (only leaves become visible).
        // Node position is top-left; offset by half size so the visual center
        // sits on the circle point (matching edge control points).
        let half = NODE_SIZE / 2.0;
        for i in 0..n {
            if !layout.is_leaf[i] {
                continue; // only add leaf nodes to the flow state
            }
            let nid = format!("n{}", i);
            state.add_node(
                Node::builder(nid)
                    .position(egui::pos2(
                        layout.positions[i].x - half,
                        layout.positions[i].y - half,
                    ))
                    .data(NodeData {
                        label: hierarchy.short_labels[i].clone(),
                        is_leaf: layout.is_leaf[i],
                        group: hierarchy.group_indices[i],
                        angle: layout.angles[i],
                    })
                    .size(NODE_SIZE, NODE_SIZE)
                    .build(),
            );
        }

        // Add edges with precomputed bundled control points
        let edge_count = dep_edges.len();
        for (ei, &(src, tgt)) in dep_edges.iter().enumerate() {
            // Both src and tgt must be leaves
            if !layout.is_leaf[src] || !layout.is_leaf[tgt] {
                continue;
            }

            // via already includes source (first) and target (last) positions.
            let via = bundled_path(
                src,
                tgt,
                &layout.ancestor_paths,
                &layout.positions,
                BETA,
            );

            // Use the via points directly as control points.
            // Pad short paths to exactly 4 for a cubic Bézier.
            let control_points = match via.len() {
                0 | 1 => {
                    // Degenerate: straight line
                    let s = layout.positions[src];
                    let t = layout.positions[tgt];
                    let mid = egui::pos2((s.x + t.x) / 2.0, (s.y + t.y) / 2.0);
                    vec![s, mid, mid, t]
                }
                2 => {
                    // Two points: use midpoint as control
                    let mid = egui::pos2(
                        (via[0].x + via[1].x) / 2.0,
                        (via[0].y + via[1].y) / 2.0,
                    );
                    vec![via[0], mid, mid, via[1]]
                }
                3 => {
                    // Three points: duplicate middle as both control points
                    vec![via[0], via[1], via[1], via[2]]
                }
                _ => via,
            };

            let src_group = hierarchy.group_indices[src];

            let mut edge = Edge::new(
                format!("e{}", ei),
                format!("n{}", src),
                format!("n{}", tgt),
            )
            .edge_type(EdgeType::Straight);

            edge.data = Some(BundleEdgeData {
                control_points,
                color: group_color(src_group),
                source_idx: src,
                target_idx: tgt,
            });

            state.add_edge(edge);
        }

        Self {
            state,
            edge_widget: BundleEdgeWidget::new(connected_leaves),
            first_frame: true,
            leaf_count,
            edge_count,
            group_count,
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for HierarchicalEdgeBundlingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Determine which node (if any) is hovered by checking selected nodes
        // (egui_xyflow highlights on hover through the node_hovered event).
        // We detect the hovered node by checking which nodes report as selected.
        // For hover highlighting, we scan nodes each frame.
        let mut hovered_idx: Option<usize> = None;

        // Top bar
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Hierarchical Edge Bundling");
                ui.separator();
                ui.label(format!(
                    "{} modules, {} imports, {} groups",
                    self.leaf_count, self.edge_count, self.group_count
                ));
                ui.separator();
                if ui.button("Fit View").clicked() {
                    let rect = ctx.screen_rect();
                    self.state
                        .fit_view(rect, 60.0, ctx.input(|i| i.time));
                }
                if ui.button("Zoom In").clicked() {
                    self.state.zoom_in(ctx.input(|i| i.time));
                }
                if ui.button("Zoom Out").clicked() {
                    self.state.zoom_out(ctx.input(|i| i.time));
                }
            });
        });

        // Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                // Fit view on first frame
                if self.first_frame {
                    let rect = ui.available_rect_before_wrap();
                    let t = ctx.input(|i| i.time);
                    self.state.fit_view(rect, 80.0, t);
                    self.first_frame = false;
                }

                let events = FlowCanvas::new(&mut self.state, &BundleNodeWidget)
                    .edge_widget(&self.edge_widget)
                    .show(ui);

                // Map hovered NodeId to hierarchy index for edge highlighting
                hovered_idx = events.node_hovered.as_ref().and_then(|nid| {
                    nid.0.strip_prefix('n').and_then(|s| s.parse::<usize>().ok())
                });
                self.edge_widget.hovered_node_idx.set(hovered_idx);

                // Request repaint when hovering for smooth highlighting
                if hovered_idx.is_some() {
                    ctx.request_repaint();
                }
            });
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("egui_xyflow -- Hierarchical Edge Bundling")
            .with_inner_size([1100.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hierarchical Edge Bundling",
        options,
        Box::new(|_cc| Ok(Box::new(HierarchicalEdgeBundlingApp::new()))),
    )
}
