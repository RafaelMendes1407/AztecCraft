//! `DefaultLightingApprox`: a headlight Lambert term over an ambient floor.
//!
//! This is an approximation for display only. It is not Maya's default lighting: the 2026
//! Lighting page was unreachable, so the light direction and whether it follows the camera are
//! unverified (see `known-issues.md`). It shades per face, not per pixel as Maya does.

/// Base surface fill, shared by the viewport and later material display work.
pub const SURFACE_BASE_RGB: [u8; 3] = [83; 3];

/// Chosen for legibility, not measured from Maya (provisional).
///
/// Share of the base colour kept when a face is edge-on, back-facing or has no usable normal.
pub const AMBIENT_FLOOR: f32 = 0.4;

/// Shades `base` for a face with `normal` seen along `view_dir` (surface toward the viewer).
/// `view_dir` is expected to be roughly unit length; vectors too small to normalise count as unusable.
///
/// A face looking straight at the viewer keeps `base`; the result falls to `AMBIENT_FLOOR * base`
/// at grazing angles and for back-facing, zero-length or non-finite input. Never returns NaN.
pub fn shade(normal: [f32; 3], view_dir: [f32; 3], base: [u8; 3]) -> [u8; 3] {
    let lambert = match (unit(normal), unit(view_dir)) {
        (Some(n), Some(v)) => (n[0] * v[0] + n[1] * v[1] + n[2] * v[2]).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let scale = AMBIENT_FLOOR + (1.0 - AMBIENT_FLOOR) * lambert;
    base.map(|channel| (f32::from(channel) * scale).round().clamp(0.0, 255.0) as u8)
}

fn unit(vector: [f32; 3]) -> Option<[f32; 3]> {
    let length = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    (vector.iter().all(|v| v.is_finite()) && length.is_finite() && length > 0.0)
        .then(|| vector.map(|v| v / length))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: [u8; 3] = [200, 100, 50];
    const FLOOR: [u8; 3] = [80, 40, 20];

    #[test]
    fn front_facing_keeps_the_base_colour() {
        assert_eq!(shade([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], BASE), BASE);
        assert_eq!(shade([0.0, 0.0, 5.0], [0.0, 0.0, 0.5], BASE), BASE);
    }

    #[test]
    fn back_facing_and_grazing_fall_to_the_ambient_floor() {
        assert_eq!(shade([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], BASE), FLOOR);
        assert_eq!(shade([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], BASE), FLOOR);
    }

    #[test]
    fn oblique_faces_sit_strictly_between_floor_and_base() {
        let lit = shade([1.0, 0.0, 1.0], [0.0, 0.0, 1.0], BASE);
        for channel in 0..3 {
            assert!(lit[channel] > FLOOR[channel] && lit[channel] < BASE[channel]);
        }
    }

    #[test]
    fn unusable_normals_or_views_give_the_ambient_floor_not_nan() {
        let bad = [
            [0.0; 3],
            [f32::NAN, 0.0, 1.0],
            [f32::INFINITY, 0.0, 0.0],
            [f32::MAX, f32::MAX, f32::MAX],
            [1e-30, 0.0, 1e-30],
        ];
        for vector in bad {
            assert_eq!(shade(vector, [0.0, 0.0, 1.0], BASE), FLOOR);
            assert_eq!(shade([0.0, 0.0, 1.0], vector, BASE), FLOOR);
        }
    }

    #[test]
    fn output_never_drops_below_the_floor() {
        for x in -4..=4 {
            for z in -4..=4 {
                let lit = shade([x as f32, 0.5, z as f32], [0.0, 0.0, 1.0], BASE);
                assert!((0..3).all(|c| lit[c] >= FLOOR[c]));
            }
        }
    }
}
