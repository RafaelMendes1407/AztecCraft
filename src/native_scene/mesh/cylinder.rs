use super::NativeMesh;
use std::f64::consts::PI;

// PROVISIONAL (#5): radius 1, height 2 and 20x1x0 follow the polyCylinder node page (sc=0 variant);
// the command page lists sz 1 and Maya 2026 is unverified.
const RADIUS: f64 = 1.0;
const HALF_HEIGHT: f64 = 1.0;
const AXIS_DIVISIONS: usize = 20;

impl NativeMesh {
    /// Y-axis cylinder: a top ring (vertices `0..20`) and a bottom ring (`20..40`), 20 outward
    /// side quads and two true 20-gon caps, with no cap centre vertices.
    pub(super) fn polygon_cylinder() -> Self {
        let mut points = Vec::with_capacity(2 * AXIS_DIVISIONS);
        for y in [HALF_HEIGHT, -HALF_HEIGHT] {
            for column in 0..AXIS_DIVISIONS {
                let azimuth = 2.0 * PI * column as f64 / AXIS_DIVISIONS as f64;
                points.push([
                    (RADIUS * azimuth.cos()) as f32,
                    y as f32,
                    (RADIUS * azimuth.sin()) as f32,
                ]);
            }
        }
        let vertex = |ring: usize, column: usize| ring * AXIS_DIVISIONS + column % AXIS_DIVISIONS;
        let mut loops: Vec<Vec<usize>> = Vec::new();
        for column in 0..AXIS_DIVISIONS {
            loops.push(vec![
                vertex(0, column),
                vertex(0, column + 1),
                vertex(1, column + 1),
                vertex(1, column),
            ]);
        }
        loops.push(
            (0..AXIS_DIVISIONS)
                .map(|column| vertex(0, AXIS_DIVISIONS - column))
                .collect(),
        );
        loops.push(
            (0..AXIS_DIVISIONS)
                .map(|column| vertex(1, column))
                .collect(),
        );
        Self::from_loops(points, loops)
    }
}
