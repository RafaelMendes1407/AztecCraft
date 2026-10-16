use super::NativeMesh;
use std::f64::consts::PI;

// PROVISIONAL (#5): radius 1, height 2 and 20x1x0 follow the polyCone node page; Maya 2026 is
// unverified.
const RADIUS: f64 = 1.0;
const HALF_HEIGHT: f64 = 1.0;
const AXIS_DIVISIONS: usize = 20;

impl NativeMesh {
    /// Y-axis cone: the apex (vertex 0) at `(0, 1, 0)`, a base ring (`1..=20`) at `y = -1`,
    /// 20 outward side triangles and one true 20-gon base cap, with no cap centre vertex.
    pub(super) fn polygon_cone() -> Self {
        let mut points = vec![[0.0, HALF_HEIGHT as f32, 0.0]];
        for column in 0..AXIS_DIVISIONS {
            let azimuth = 2.0 * PI * column as f64 / AXIS_DIVISIONS as f64;
            points.push([
                (RADIUS * azimuth.cos()) as f32,
                -HALF_HEIGHT as f32,
                (RADIUS * azimuth.sin()) as f32,
            ]);
        }
        let base = |column: usize| 1 + column % AXIS_DIVISIONS;
        let mut loops: Vec<Vec<usize>> = Vec::new();
        for column in 0..AXIS_DIVISIONS {
            loops.push(vec![0, base(column + 1), base(column)]);
        }
        loops.push((0..AXIS_DIVISIONS).map(base).collect());
        Self::from_loops(points, loops)
    }
}
