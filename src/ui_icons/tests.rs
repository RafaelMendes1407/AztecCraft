use super::*;
use eframe::egui::epaint::{ClippedShape, Primitive, TessellationOptions, Tessellator};

const PANEL: [f32; 3] = [43.0, 43.0, 43.0];
const MARGIN: f32 = 4.0;

fn index(icon: UiIcon) -> usize {
    match icon {
        UiIcon::Select => 0,
    }
}
const ICON_COUNT: usize = 1;

struct Raster {
    w: usize,
    h: usize,
    /// Premultiplied RGBA in 0..=255, composited over transparent black.
    px: Vec<[f32; 4]>,
}

/// Mimics egui's disabled painter: fill alpha multiplied by `Visuals::disabled_alpha`.
fn dimmed(mut shapes: Vec<Shape>, opacity: f32) -> Vec<Shape> {
    for shape in &mut shapes {
        if let Shape::Path(path) = shape {
            path.fill = path.fill.gamma_multiply(opacity);
        }
    }
    shapes
}

fn rasterize(shapes: Vec<Shape>, canvas_pts: f32, ppp: f32) -> Raster {
    let mut tess = Tessellator::new(ppp, TessellationOptions::default(), [1, 1], vec![]);
    let mut prims = Vec::new();
    for shape in shapes {
        tess.tessellate_clipped_shape(
            ClippedShape {
                clip_rect: Rect::EVERYTHING,
                shape,
            },
            &mut prims,
        );
    }
    let n = (canvas_pts * ppp).ceil() as usize;
    let mut px = vec![[0.0f32; 4]; n * n];
    for prim in prims {
        let Primitive::Mesh(mesh) = prim.primitive else {
            continue;
        };
        for tri in mesh.indices.as_chunks::<3>().0 {
            let v: Vec<_> = tri.iter().map(|&i| mesh.vertices[i as usize]).collect();
            let p: Vec<(f32, f32)> = v.iter().map(|v| (v.pos.x * ppp, v.pos.y * ppp)).collect();
            let area =
                (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
            if area.abs() < 1e-9 {
                continue;
            }
            // Half-open fill rule: a pixel centre on an edge belongs to the triangle
            // whose (orientation-normalised) edge points downwards, or rightwards if level.
            let owns = |w: f32, j: usize, k: usize| {
                if w > 1e-5 {
                    return true;
                }
                if w < -1e-5 {
                    return false;
                }
                let sign = area.signum();
                let (dx, dy) = (sign * (p[k].0 - p[j].0), sign * (p[k].1 - p[j].1));
                dy > 0.0 || (dy == 0.0 && dx > 0.0)
            };
            for y in 0..n {
                for x in 0..n {
                    let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = ((p[1].0 - cx) * (p[2].1 - cy) - (p[2].0 - cx) * (p[1].1 - cy)) / area;
                    let w1 = ((p[2].0 - cx) * (p[0].1 - cy) - (p[0].0 - cx) * (p[2].1 - cy)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if !(owns(w0, 1, 2) && owns(w1, 2, 0) && owns(w2, 0, 1)) {
                        continue;
                    }
                    let mut src = [0.0f32; 4];
                    for (c, s) in src.iter_mut().enumerate() {
                        *s = w0 * f32::from(v[0].color.to_array()[c])
                            + w1 * f32::from(v[1].color.to_array()[c])
                            + w2 * f32::from(v[2].color.to_array()[c]);
                    }
                    let dst = &mut px[y * n + x];
                    let keep = 1.0 - src[3] / 255.0;
                    for c in 0..4 {
                        dst[c] = src[c] + dst[c] * keep;
                    }
                }
            }
        }
    }
    Raster { w: n, h: n, px }
}

struct Measure {
    coverage: f32,
    bbox_fraction: f32,
    outside_ink: f32,
    contrast: f32,
}

fn measure(icon: UiIcon, opacity: f32, size: f32, ppp: f32) -> Measure {
    let rect = Rect::from_min_size(Pos2::new(MARGIN, MARGIN), egui::vec2(size, size));
    let r = rasterize(
        dimmed(shapes(icon, rect), opacity),
        size + 2.0 * MARGIN,
        ppp,
    );
    // Feathering reaches half a physical px past an edge.
    let tol = 0.5 / ppp;
    let outer = rect.expand(tol);
    let (mut sum_a, mut outside, mut n_inside) = (0.0f32, 0.0f32, 0usize);
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
    let (mut contrast_sum, mut contrast_n) = (0.0f32, 0usize);
    for y in 0..r.h {
        for x in 0..r.w {
            let a = r.px[y * r.w + x][3] / 255.0;
            let c = Pos2::new((x as f32 + 0.5) / ppp, (y as f32 + 0.5) / ppp);
            if rect.contains(c) {
                n_inside += 1;
                sum_a += a;
            }
            if !outer.contains(c) && a > 0.01 {
                outside += a;
            }
            if a >= 0.5 && rect.contains(c) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                let p = r.px[y * r.w + x];
                let lum =
                    |ch: &dyn Fn(usize) -> f32| 0.2126 * ch(0) + 0.7152 * ch(1) + 0.0722 * ch(2);
                let over = lum(&|i| p[i] + PANEL[i] * (1.0 - a));
                let bg = lum(&|i| PANEL[i]);
                contrast_sum += (over - bg).abs();
                contrast_n += 1;
            }
        }
    }
    let bbox = if x1 >= x0 && y1 >= y0 && x0 != usize::MAX {
        ((x1 - x0 + 1) * (y1 - y0 + 1)) as f32
    } else {
        0.0
    };
    Measure {
        coverage: sum_a / n_inside as f32,
        bbox_fraction: bbox / n_inside as f32,
        outside_ink: outside,
        contrast: contrast_sum / contrast_n.max(1) as f32,
    }
}

fn disabled_alpha() -> f32 {
    egui::Visuals::dark().disabled_alpha()
}

const SIZES: [f32; 3] = [16.0, 24.0, 32.0];
const SCALES: [f32; 2] = [1.0, 2.0];

#[test]
fn all_is_exhaustive_and_unique() {
    let mut seen = [false; ICON_COUNT];
    for &icon in ALL {
        let i = index(icon);
        assert!(!seen[i], "duplicate {icon:?}");
        seen[i] = true;
    }
    assert_eq!(ALL.len(), ICON_COUNT);
}

#[test]
fn ink_coverage_is_in_range() {
    for &icon in ALL {
        for opacity in [1.0, disabled_alpha()] {
            for size in SIZES {
                for ppp in SCALES {
                    let c = measure(icon, opacity, size, ppp).coverage;
                    assert!(
                        (0.08..=0.60).contains(&c),
                        "{icon:?} opacity {opacity} {size}px@{ppp}x coverage {c}"
                    );
                }
            }
        }
    }
}

#[test]
fn ink_fills_the_rect_and_stays_inside() {
    for &icon in ALL {
        for size in SIZES {
            for ppp in SCALES {
                let m = measure(icon, 1.0, size, ppp);
                assert!(
                    m.bbox_fraction >= 0.60,
                    "{icon:?} {size}px@{ppp}x bbox {}",
                    m.bbox_fraction
                );
                assert!(
                    m.outside_ink == 0.0,
                    "{icon:?} {size}px@{ppp}x ink outside rect: {}",
                    m.outside_ink
                );
            }
        }
    }
}

#[test]
fn disabled_is_dimmer_but_readable() {
    const READABLE_FLOOR: f32 = 40.0;
    for &icon in ALL {
        for size in SIZES {
            for ppp in SCALES {
                let n = measure(icon, 1.0, size, ppp).contrast;
                let d = measure(icon, disabled_alpha(), size, ppp).contrast;
                assert!(d < n, "{icon:?} {size}px@{ppp}x disabled {d} >= normal {n}");
                assert!(d > READABLE_FLOOR, "{icon:?} {size}px@{ppp}x disabled {d}");
            }
        }
    }
}

fn alpha_grid(icon: UiIcon, size: f32, ppp: f32) -> (Vec<f32>, usize) {
    alpha_grid_at(icon, 1.0, size, ppp)
}

fn alpha_grid_at(icon: UiIcon, opacity: f32, size: f32, ppp: f32) -> (Vec<f32>, usize) {
    let rect = Rect::from_min_size(Pos2::new(MARGIN, MARGIN), egui::vec2(size, size));
    let r = rasterize(
        dimmed(shapes(icon, rect), opacity),
        size + 2.0 * MARGIN,
        ppp,
    );
    (r.px.iter().map(|p| p[3] / 255.0).collect(), r.w)
}

#[test]
fn interior_is_solid_without_seams() {
    for &icon in ALL {
        for size in SIZES {
            for ppp in SCALES {
                let (a, w) = alpha_grid(icon, size, ppp);
                for y in 1..w - 1 {
                    for x in 1..w - 1 {
                        let eroded =
                            (y - 1..=y + 1).all(|j| (x - 1..=x + 1).all(|i| a[j * w + i] >= 0.5));
                        assert!(
                            !eroded || a[y * w + x] >= 0.98,
                            "{icon:?} {size}px@{ppp}x seam at ({x},{y}): alpha {}",
                            a[y * w + x]
                        );
                    }
                }
            }
        }
    }
}

/// Intentional change detector: 12x12 occupancy at 24px@1x. Update on purpose.
fn golden(icon: UiIcon) -> [&'static str; 12] {
    match icon {
        UiIcon::Select => GOLDEN_SELECT,
    }
}
const GOLDEN_SELECT: [&str; 12] = [
    "............",
    ".#..........",
    ".###........",
    ".####.......",
    ".######.....",
    ".########...",
    ".#########..",
    ".#####......",
    ".##.##......",
    ".....#......",
    ".....##.....",
    "............",
];

#[test]
fn occupancy_matches_golden() {
    let m = MARGIN as usize;
    for &icon in ALL {
        let (a, w) = alpha_grid(icon, 24.0, 1.0);
        let rows: Vec<String> = (0..12)
            .map(|cy| {
                (0..12)
                    .map(|cx| {
                        let sum: f32 = (0..2)
                            .flat_map(|j| (0..2).map(move |i| (j, i)))
                            .map(|(j, i)| a[(m + cy * 2 + j) * w + m + cx * 2 + i])
                            .sum();
                        if sum / 4.0 >= 0.5 { '#' } else { '.' }
                    })
                    .collect()
            })
            .collect();
        assert_eq!(
            rows,
            golden(icon),
            "{icon:?} occupancy changed:\n{}",
            rows.join("\n")
        );
    }
}

#[test]
fn uniform_interior_under_disabled_opacity() {
    for &icon in ALL {
        for size in SIZES {
            for ppp in SCALES {
                let (a, w) = alpha_grid_at(icon, disabled_alpha(), size, ppp);
                let mut interior = Vec::new();
                for y in 1..w - 1 {
                    for x in 1..w - 1 {
                        let solid = (y - 1..=y + 1).all(|j| {
                            (x - 1..=x + 1).all(|i| a[j * w + i] >= 0.5 * disabled_alpha())
                        });
                        if solid {
                            interior.push((x, y, a[y * w + x]));
                        }
                    }
                }
                let mut sorted: Vec<f32> = interior.iter().map(|p| p.2).collect();
                sorted.sort_by(f32::total_cmp);
                let median = sorted[sorted.len() / 2];
                for (x, y, v) in interior {
                    assert!(
                        (v - median).abs() <= 0.02,
                        "{icon:?} {size}px@{ppp}x alpha {v} at ({x},{y}) vs median {median}"
                    );
                }
            }
        }
    }
}

#[test]
fn outlines_fan_fill_clockwise_without_fold_over() {
    let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(24.0, 24.0));
    for &icon in ALL {
        for shape in shapes(icon, rect) {
            let Shape::Path(path) = shape else {
                panic!("{icon:?}: expected a single filled path");
            };
            let v = &path.points;
            for i in 2..v.len() {
                let (a, b) = (v[i - 1] - v[0], v[i] - v[0]);
                let area = 0.5 * (a.x * b.y - a.y * b.x);
                assert!(area > 0.0, "{icon:?} fan triangle {i} area {area}");
            }
        }
    }
}

#[test]
fn shared_edge_pixels_are_filled_once() {
    use eframe::egui::epaint::Mesh;
    let half = Color32::from_rgba_premultiplied(128, 128, 128, 128);
    let mut mesh = Mesh::default();
    for p in [(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)] {
        mesh.colored_vertex(Pos2::new(p.0, p.1), half);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    let r = rasterize(vec![Shape::mesh(mesh)], 8.0, 1.0);
    let max = r.px.iter().map(|p| p[3] / 255.0).fold(0.0, f32::max);
    assert!(max <= 128.0 / 255.0 + 1e-3, "shared-edge pixel alpha {max}");
}
