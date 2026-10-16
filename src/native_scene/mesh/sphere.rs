use super::{DirectedEdge, NativeFace, NativeMesh, Points};
use std::f64::consts::PI;

// PROVISIONAL (#5): radius 1 and 20x20 are the polySphere node defaults; Maya 2026 is unverified.
const RADIUS: f64 = 1.0;
const AXIS_DIVISIONS: usize = 20;
const HEIGHT_DIVISIONS: usize = 20;

impl NativeMesh {
    /// Y-axis UV sphere: a vertex at each pole, `HEIGHT_DIVISIONS - 1` rings between them,
    /// triangle fans at the poles and outward-wound quads between rings.
    pub(super) fn polygon_sphere() -> Self {
        let mut points = vec![[0.0, RADIUS as f32, 0.0]];
        for ring in 1..HEIGHT_DIVISIONS {
            let polar = PI * ring as f64 / HEIGHT_DIVISIONS as f64;
            let y = if 2 * ring == HEIGHT_DIVISIONS {
                0.0
            } else {
                RADIUS * polar.cos()
            };
            for column in 0..AXIS_DIVISIONS {
                let azimuth = 2.0 * PI * column as f64 / AXIS_DIVISIONS as f64;
                let radial = RADIUS * polar.sin();
                points.push([
                    (radial * azimuth.cos()) as f32,
                    y as f32,
                    (radial * azimuth.sin()) as f32,
                ]);
            }
        }
        points.push([0.0, -RADIUS as f32, 0.0]);
        let top = 0;
        let bottom = points.len() - 1;
        let vertex =
            |ring: usize, column: usize| 1 + (ring - 1) * AXIS_DIVISIONS + column % AXIS_DIVISIONS;
        let mut loops: Vec<Vec<usize>> = Vec::new();
        for column in 0..AXIS_DIVISIONS {
            loops.push(vec![top, vertex(1, column + 1), vertex(1, column)]);
        }
        for ring in 1..HEIGHT_DIVISIONS - 1 {
            for column in 0..AXIS_DIVISIONS {
                loops.push(vec![
                    vertex(ring, column),
                    vertex(ring, column + 1),
                    vertex(ring + 1, column + 1),
                    vertex(ring + 1, column),
                ]);
            }
        }
        for column in 0..AXIS_DIVISIONS {
            loops.push(vec![
                bottom,
                vertex(HEIGHT_DIVISIONS - 1, column),
                vertex(HEIGHT_DIVISIONS - 1, column + 1),
            ]);
        }
        let mut mesh = Self {
            point_offsets: Vec::new(),
            points: Points::Float3(points),
            edges: Vec::new(),
            faces: Vec::new(),
            unsupported_channels: Vec::new(),
        };
        for corners in loops {
            let outer: Vec<DirectedEdge> = (0..corners.len())
                .map(|i| mesh.edge([corners[i], corners[(i + 1) % corners.len()]]))
                .collect();
            mesh.faces.push(NativeFace {
                outer,
                holes: Vec::new(),
            });
        }
        mesh
    }
}
