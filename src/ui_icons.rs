//! Original vector UI icons, drawn in code on a 24-unit grid. Each glyph is
//! authored from a written spec; nothing is traced, copied or derived from any Maya asset.

use eframe::egui::{
    self, Color32, Pos2, Rect, Shape, Stroke, Vec2,
    epaint::{PathShape, PathStroke},
    pos2, vec2,
};
use std::f32::consts::{FRAC_1_SQRT_2, PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiIcon {
    Select,
    Lasso,
    PaintSelect,
    Move,
    Rotate,
    Scale,
    /// Static placeholder glyph; the dynamic Last Used behaviour is unsupported.
    LastUsed,
    SelectHierarchy,
    SelectObject,
    SelectComponent,
}

/// Tool Box order.
pub const ALL: &[UiIcon] = &[
    UiIcon::Select,
    UiIcon::Lasso,
    UiIcon::PaintSelect,
    UiIcon::Move,
    UiIcon::Rotate,
    UiIcon::Scale,
    UiIcon::LastUsed,
    UiIcon::SelectHierarchy,
    UiIcon::SelectObject,
    UiIcon::SelectComponent,
];

// Newly chosen values; not sampled from any Maya image.
const NEUTRAL_LIGHT: Color32 = Color32::from_rgb(214, 218, 222);
const AXIS_X: Color32 = Color32::from_rgb(242, 98, 98);
const AXIS_Y: Color32 = Color32::from_rgb(124, 208, 96);
const AXIS_Z: Color32 = Color32::from_rgb(104, 152, 244);
const NEUTRAL_MID: Color32 = Color32::from_rgb(150, 156, 164);
const NEUTRAL_DARK: Color32 = Color32::from_rgb(106, 112, 122);
const ACCENT: Color32 = Color32::from_rgb(238, 196, 92);

const GRID: f32 = 24.0;
/// Stroke width in grid units.
const STROKE: f32 = 1.6;
/// The lasso's thinner-looking loop gets a heavier line to stay legible at 16 px.
const LASSO_STROKE: f32 = 1.3 * STROKE;
/// Rotate ring break gap, as a multiple of the stroke width.
const RING_GAP_STROKES: f32 = 1.375;
const SHAFT_HALF: f32 = 0.9;
const HEAD_LEN: f32 = 6.5;
const HEAD_HALF: f32 = 4.0;
const HANDLE: f32 = 6.8;
const CENTER_HALF: f32 = 1.9;
const CENTER: Pos2 = pos2(10.0, 14.0);
/// Centre of the 24-unit grid.
const GRID_CENTER: Pos2 = pos2(12.0, 12.0);
const X_DIR: Vec2 = vec2(1.0, 0.0);
const Y_DIR: Vec2 = vec2(0.0, -1.0);
const Z_DIR: Vec2 = vec2(-FRAC_1_SQRT_2, FRAC_1_SQRT_2);

/// Maps 24-unit grid geometry into a rect. Every filled shape is one closed path
/// that egui fans from vertex 0 (clockwise, star-shaped from vertex 0); overlapping
/// shapes are avoided because they double-composite under disabled opacity.
struct Fit {
    origin: Pos2,
    scale: f32,
}

impl Fit {
    fn new(rect: Rect) -> Self {
        Self {
            origin: rect.min,
            scale: rect.width().min(rect.height()) / GRID,
        }
    }

    fn map(&self, grid: &[Pos2]) -> Vec<Pos2> {
        grid.iter()
            .map(|p| self.origin + p.to_vec2() * self.scale)
            .collect()
    }

    fn fill(&self, grid: &[Pos2], color: Color32) -> Shape {
        let mut path = PathShape::closed_line(self.map(grid), Stroke::NONE);
        path.fill = color;
        Shape::Path(path)
    }

    fn stroke(&self, grid: &[Pos2], closed: bool, color: Color32, width: f32) -> Shape {
        let stroke = PathStroke::new(width * self.scale, color);
        let points = self.map(grid);
        Shape::Path(if closed {
            PathShape::closed_line(points, stroke)
        } else {
            PathShape::line(points, stroke)
        })
    }
}

fn rot(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Points of an ellipse, increasing parameter (clockwise on screen).
fn ellipse(center: Pos2, radii: Vec2, tilt: f32, from: f32, sweep: f32, n: usize) -> Vec<Pos2> {
    (0..n)
        .map(|i| {
            let t = from + sweep * i as f32 / (n - 1) as f32;
            center + rot(vec2(radii.x * t.cos(), radii.y * t.sin()), tilt)
        })
        .collect()
}

fn disc(center: Pos2, r: f32) -> Vec<Pos2> {
    let mut pts = ellipse(center, vec2(r, r), 0.0, 0.0, TAU, 17);
    pts.pop();
    pts
}

/// Local (along, across) to grid; `across` is `dir` turned clockwise on screen.
fn along(origin: Pos2, dir: Vec2, pts: &[(f32, f32)]) -> Vec<Pos2> {
    let perp = vec2(-dir.y, dir.x);
    pts.iter()
        .map(|&(u, v)| origin + dir * u + perp * v)
        .collect()
}

/// Shaft and filled triangular head as one outline, starting at the tip.
fn arrow(origin: Pos2, dir: Vec2, from: f32, to: f32) -> Vec<Pos2> {
    let b = to - HEAD_LEN;
    let (w, h) = (SHAFT_HALF, HEAD_HALF);
    along(
        origin,
        dir,
        &[
            (to, 0.0),
            (b, h),
            (b, w),
            (from, w),
            (from, -w),
            (b, -w),
            (b, -h),
        ],
    )
}

/// Shaft ending in a square handle as one outline. Vertex 0 sits on the handle's
/// far edge inside the band that sees every vertex.
fn lollipop(origin: Pos2, dir: Vec2, from: f32, to: f32) -> Vec<Pos2> {
    let (w, c, a) = (SHAFT_HALF, HANDLE / 2.0, to - HANDLE);
    along(
        origin,
        dir,
        &[
            (to, 0.0),
            (to, c),
            (a, c),
            (a, w),
            (from, w),
            (from, -w),
            (a, -w),
            (a, -c),
            (to, -c),
        ],
    )
}

/// Point at `t` on the quadratic Bezier from `a` to `b` with control point `c`.
fn quad_bezier(a: Pos2, c: Pos2, b: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    (a.to_vec2() * (u * u) + c.to_vec2() * (2.0 * t * u) + b.to_vec2() * (t * t)).to_pos2()
}

fn dist_to_polyline(p: Pos2, line: &[Pos2]) -> f32 {
    line.windows(2)
        .map(|s| {
            let d = s[1] - s[0];
            let t = ((p - s[0]).dot(d) / d.length_sq()).clamp(0.0, 1.0);
            (p - (s[0] + d * t)).length()
        })
        .fold(f32::MAX, f32::min)
}

/// Splits a closed ring into open runs, dropping points near `fronts`.
fn break_ring(ring: &[Pos2], fronts: &[&[Pos2]], gap: f32) -> Vec<Vec<Pos2>> {
    let keep: Vec<bool> = ring
        .iter()
        .map(|&p| fronts.iter().all(|f| dist_to_polyline(p, f) >= gap))
        .collect();
    let Some(start) = keep.iter().position(|k| !k) else {
        return vec![ring.to_vec()];
    };
    let mut runs: Vec<Vec<Pos2>> = vec![];
    let mut cur = vec![];
    for i in 1..=ring.len() {
        let j = (start + i) % ring.len();
        if keep[j] {
            cur.push(ring[j]);
        } else if !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs.retain(|r| r.len() >= 2);
    runs
}

fn lasso(fit: &Fit) -> Vec<Shape> {
    let (c, radii, tilt) = (pos2(13.8, 9.8), vec2(9.0, 6.6), -0.4);
    let start = 3.3;
    let mut path = ellipse(c, radii, tilt, start, TAU - 1.0, 44);
    let p0 = path[0];
    // The cord leaves along the loop's tangent so the join stays smooth.
    let tangent = (path[1] - p0).normalized();
    let outward = (p0 - c).normalized();
    let ctrl = p0 - tangent * 4.0;
    let end = p0 - tangent * 5.5 + outward * 1.2;
    let knot = end + (end - ctrl).normalized() * 3.4;
    let cord: Vec<Pos2> = (0..8)
        .map(|i| quad_bezier(end, ctrl, p0, i as f32 / 7.0))
        .collect();
    path.splice(0..1, cord);
    vec![
        fit.stroke(&path, false, NEUTRAL_LIGHT, LASSO_STROKE),
        fit.fill(&disc(knot, 2.2), NEUTRAL_LIGHT),
    ]
}

fn paint_select(fit: &Fit) -> Vec<Shape> {
    let (b, r, half) = (pos2(16.5, 7.5), 4.4_f32, 1.5_f32);
    let d = vec2(FRAC_1_SQRT_2, -FRAC_1_SQRT_2);
    let junction = (half / r).asin();
    let far = 0.75 * PI; // opposite the handle
    let tip = -0.25 * PI;
    let mut pts = ellipse(b, vec2(r, r), 0.0, far, tip - junction + TAU - far, 12);
    pts.extend(along(b, d, &[(r + 2.4, -half), (r + 2.4, half)]));
    pts.extend(ellipse(
        b,
        vec2(r, r),
        0.0,
        tip + junction + TAU,
        far - tip - junction,
        12,
    ));
    pts.pop();
    let mut shapes = vec![fit.fill(&pts, ACCENT)];
    let (s, ctrl, e) = (b - d * 7.0, pos2(5.0, 12.0), pos2(3.2, 20.8));
    for i in 0..3 {
        let at = quad_bezier(s, ctrl, e, i as f32 / 2.0);
        shapes.push(fit.fill(&disc(at, 1.9), NEUTRAL_LIGHT));
    }
    shapes
}

fn axes(fit: &Fit, shape: fn(Pos2, Vec2, f32, f32) -> Vec<Pos2>, gaps: [f32; 3]) -> Vec<Shape> {
    [
        (X_DIR, AXIS_X, 13.0),
        (Y_DIR, AXIS_Y, 12.5),
        (Z_DIR, AXIS_Z, 10.9),
    ]
    .iter()
    .zip(gaps)
    .map(|(&(dir, color, to), from)| fit.fill(&shape(CENTER, dir, from, to), color))
    .collect()
}

fn rotate(fit: &Fit) -> Vec<Shape> {
    let c = GRID_CENTER;
    // 96 samples include the closing duplicate of point 0; keep the first 95.
    let ring = |radii: Vec2, tilt: f32| ellipse(c, radii, tilt, 0.0, TAU, 96)[..95].to_vec();
    let (z, y, x) = (
        ring(vec2(10.6, 10.6), 0.0),
        ring(vec2(8.2, 3.2), 0.0),
        ring(vec2(4.4, 8.2), 0.3),
    );
    let gap = RING_GAP_STROKES * STROKE;
    let mut shapes = vec![];
    for (pts, color, fronts) in [
        (&z, AXIS_Z, vec![&x[..], &y[..]]),
        (&x, AXIS_X, vec![&y[..]]),
        (&y, AXIS_Y, vec![]),
    ] {
        for run in break_ring(pts, &fronts, gap) {
            let closed = run.len() == pts.len();
            shapes.push(fit.stroke(&run, closed, color, STROKE));
        }
    }
    shapes
}

fn last_used(fit: &Fit) -> Vec<Shape> {
    let (c, r) = (pos2(17.6, 6.8), 4.7_f32);
    let cut = r + 3.2;
    let (lo, hi) = (2.5, 16.5);
    let (top, bottom) = (8.0, 22.0);
    // Rounded square, clockwise from the top-right corner, so the cut is one run.
    let rr = 2.6_f32;
    let corner =
        |cx: f32, cy: f32, from: f32| ellipse(pos2(cx, cy), vec2(rr, rr), 0.0, from, 0.5 * PI, 7);
    let edge = |a: Pos2, b: Pos2| (1..12).map(move |i| a + (b - a) * (i as f32 / 12.0));
    let ring: Vec<Pos2> = [
        corner(hi - rr, top + rr, -0.5 * PI),
        edge(pos2(hi, top + rr), pos2(hi, bottom - rr)).collect(),
        corner(hi - rr, bottom - rr, 0.0),
        edge(pos2(hi - rr, bottom), pos2(lo + rr, bottom)).collect(),
        corner(lo + rr, bottom - rr, 0.5 * PI),
        edge(pos2(lo, bottom - rr), pos2(lo, top + rr)).collect(),
        corner(lo + rr, top + rr, PI),
        edge(pos2(lo + rr, top), pos2(hi - rr, top)).collect(),
    ]
    .concat()
    .into_iter()
    .filter(|p| (*p - c).length() >= cut)
    .collect();
    let end = 0.75 * PI;
    let arc = ellipse(c, vec2(r, r), 0.0, 1.25 * PI, 1.5 * PI, 32);
    let tangent = vec2(-end.sin(), end.cos());
    let normal = vec2(end.cos(), end.sin());
    let base = c + normal * r + tangent * 1.3;
    let head = [
        base + tangent * 3.4,
        base - normal * 2.2,
        base + normal * 2.2,
    ];
    vec![
        fit.stroke(&ring, false, NEUTRAL_LIGHT, STROKE),
        fit.stroke(&arc, false, NEUTRAL_LIGHT, STROKE),
        fit.fill(&head, NEUTRAL_LIGHT),
    ]
}

/// Rectangle from `a` to `b` as one clockwise outline.
fn rect_path(a: Pos2, b: Pos2) -> [Pos2; 4] {
    [a, pos2(b.x, a.y), b, pos2(a.x, b.y)]
}

/// Gap between neighbouring regions, in grid units.
const REGION_GAP: f32 = 0.7;
/// Hierarchy node disc radius.
const NODE_RADIUS: f32 = 2.8;
/// Half the width of a hierarchy connector bar.
const BAR_HALF: f32 = 1.2;
/// Top edge of the hierarchy fork bar.
const FORK_BAR_TOP: f32 = 11.6;

fn select_hierarchy(fit: &Fit) -> Vec<Shape> {
    let (root, left, right) = (pos2(12.0, 4.4), pos2(5.0, 20.4), pos2(19.0, 20.4));
    let (r, half) = (NODE_RADIUS, BAR_HALF);
    let (stem_top, bar_top) = (root.y + r + REGION_GAP, FORK_BAR_TOP);
    let bar_bottom = bar_top + 2.0 * half;
    let drop_bottom = left.y - r - REGION_GAP;
    let (stem_bottom, drop_top) = (bar_top - REGION_GAP, bar_bottom + REGION_GAP);
    let mut shapes = vec![
        fit.fill(&disc(root, r), ACCENT),
        fit.fill(
            &rect_path(
                pos2(root.x - half, stem_top),
                pos2(root.x + half, stem_bottom),
            ),
            NEUTRAL_MID,
        ),
        fit.fill(
            &rect_path(
                pos2(left.x - half, bar_top),
                pos2(right.x + half, bar_bottom),
            ),
            NEUTRAL_MID,
        ),
    ];
    for node in [left, right] {
        let drop = rect_path(
            pos2(node.x - half, drop_top),
            pos2(node.x + half, drop_bottom),
        );
        shapes.push(fit.fill(&drop, NEUTRAL_MID));
    }
    shapes.push(fit.fill(&disc(left, r), NEUTRAL_LIGHT));
    shapes.push(fit.fill(&disc(right, r), NEUTRAL_LIGHT));
    shapes
}

/// Cube silhouette vertices: six hexagon corners clockwise from the top, then the centre.
fn cube_vertices(radius: f32) -> [Pos2; 7] {
    let c = pos2(12.0, 12.0);
    let corner = |k: usize| {
        let a = k as f32 * PI / 3.0;
        c + vec2(radius * a.sin(), -radius * a.cos())
    };
    [
        corner(0),
        corner(1),
        corner(2),
        corner(3),
        corner(4),
        corner(5),
        c,
    ]
}

/// Moves every edge of a clockwise convex polygon inward by `d`.
fn inset(poly: &[Pos2], d: f32) -> Vec<Pos2> {
    let n = poly.len();
    // Each edge as a point and direction, pushed along its inward normal.
    let lines: Vec<(Pos2, Vec2)> = (0..n)
        .map(|i| {
            let dir = (poly[(i + 1) % n] - poly[i]).normalized();
            (poly[i] + vec2(-dir.y, dir.x) * d, dir)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (p, u) = lines[(i + n - 1) % n];
            let (q, v) = lines[i];
            let t = ((q - p).x * v.y - (q - p).y * v.x) / (u.x * v.y - u.y * v.x);
            p + u * t
        })
        .collect()
}

fn select_object(fit: &Fit) -> Vec<Shape> {
    let [top, ur, lr, bottom, ll, ul, c] = cube_vertices(OBJECT_CUBE_RADIUS);
    let half_gap = REGION_GAP / 2.0;
    [
        ([top, ur, c, ul], NEUTRAL_LIGHT),
        ([c, ur, lr, bottom], NEUTRAL_MID),
        ([c, bottom, ll, ul], NEUTRAL_DARK),
    ]
    .iter()
    .map(|(face, color)| fit.fill(&inset(face, half_gap), *color))
    .collect()
}

/// Cube circumradii, in grid units.
const OBJECT_CUBE_RADIUS: f32 = 11.4;
const COMPONENT_CUBE_RADIUS: f32 = 10.0;
/// Half the width of a cube edge bar.
const EDGE_HALF: f32 = 0.6;
/// Half the side of a cube corner point.
const POINT_HALF: f32 = 1.4;

fn select_component(fit: &Fit) -> Vec<Shape> {
    let v = cube_vertices(COMPONENT_CUBE_RADIUS);
    let [top, ur, lr, bottom, ll, ul, c] = v;
    let edges = [
        (top, ur),
        (ur, lr),
        (lr, bottom),
        (bottom, ll),
        (ll, ul),
        (ul, top),
        (c, ur),
        (c, bottom),
        (c, ul),
    ];
    let mut shapes = vec![];
    for (a, b) in edges {
        let dir = (b - a).normalized();
        // Distance from a corner point's centre to its far side along the edge, plus the gap.
        let clear = POINT_HALF * (dir.x.abs() + dir.y.abs()) + REGION_GAP;
        let len = (b - a).length();
        shapes.push(fit.fill(
            &along(
                a,
                dir,
                &[
                    (clear, -EDGE_HALF),
                    (len - clear, -EDGE_HALF),
                    (len - clear, EDGE_HALF),
                    (clear, EDGE_HALF),
                ],
            ),
            NEUTRAL_LIGHT,
        ));
    }
    for p in v {
        let h = POINT_HALF;
        shapes.push(fit.fill(&rect_path(p - vec2(h, h), p + vec2(h, h)), ACCENT));
    }
    shapes
}

/// Geometry of `icon` fitted to `rect`, without needing a window. Disabled looks
/// come from the host `Ui` (egui's disabled opacity); icons have no tone.
pub fn shapes(icon: UiIcon, rect: Rect) -> Vec<Shape> {
    let fit = Fit::new(rect);
    match icon {
        // Solid arrow pointer, tip upper-left, angled down-right, short tail.
        UiIcon::Select => vec![fit.fill(
            &[
                pos2(2.5, 2.0),   // tip
                pos2(22.0, 13.5), // wing
                pos2(10.5, 13.5), // notch
                pos2(13.5, 20.5), // tail right
                pos2(12.0, 23.0), // tail end
                pos2(7.5, 14.5),  // tail left
                pos2(2.5, 19.5),  // heel
            ],
            NEUTRAL_LIGHT,
        )],
        UiIcon::Lasso => lasso(&fit),
        UiIcon::PaintSelect => paint_select(&fit),
        UiIcon::Move => axes(&fit, arrow, [1.5, 1.5, 1.5]),
        UiIcon::Rotate => rotate(&fit),
        UiIcon::Scale => {
            let mut shapes = axes(&fit, lollipop, [3.1, 3.1, 3.7]);
            let h = CENTER_HALF;
            let square = [(h, -h), (h, h), (-h, h), (-h, -h)];
            shapes.push(fit.fill(&along(CENTER, X_DIR, &square), NEUTRAL_LIGHT));
            shapes
        }
        UiIcon::LastUsed => last_used(&fit),
        UiIcon::SelectHierarchy => select_hierarchy(&fit),
        UiIcon::SelectObject => select_object(&fit),
        UiIcon::SelectComponent => select_component(&fit),
    }
}

pub fn paint(painter: &egui::Painter, rect: Rect, icon: UiIcon) {
    painter.extend(shapes(icon, rect));
}

#[cfg(test)]
mod tests;
