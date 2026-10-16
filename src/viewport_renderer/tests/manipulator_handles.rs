//! Pixel checks of the Rotate and Scale handles through the real egui renderer on a headless
//! GPU. They prove the drawn handles reach pixels and the hidden ring half does not; they are
//! not visual-parity evidence.

use super::*;
use azteccraft::{
    manipulator_handles::{
        HandleFrame, RotateHandles, ScaleHandles,
        paint::{paint_rotate, paint_scale},
    },
    move_handles::ViewportCapture,
    native_scene::{RotateAxis, ScaleHandle},
};
use eframe::egui::Pos2;
use glam::{DVec2, DVec3};

const BACKGROUND: u8 = 31;

fn handle_camera(yaw: f32, pitch: f32, fov_degrees: f32) -> Camera {
    Camera::new(
        Vec3::ZERO,
        yaw,
        pitch,
        40.0,
        fov_degrees.to_radians(),
        0.1,
        200.0,
    )
    .unwrap()
}

fn handle_capture(camera: &Camera) -> ViewportCapture {
    ViewportCapture::new(camera, [0, 0], [SIZE, SIZE], 1.0).unwrap()
}

fn rgb(image: &[u8], x: i64, y: i64) -> [i32; 3] {
    let size = i64::from(SIZE);
    assert!(
        (0..size).contains(&x) && (0..size).contains(&y),
        "({x}, {y})"
    );
    let pixel = pixel(image, x as u32, y as u32);
    [pixel[0], pixel[1], pixel[2]].map(i32::from)
}

// Whether any pixel within one pixel of `at` is dominated by channel `channel`.
fn near_dominant(image: &[u8], at: DVec2, channel: usize) -> bool {
    let (cx, cy) = (at.x.round() as i64, at.y.round() as i64);
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            let color = rgb(image, cx + dx, cy + dy);
            (0..3)
                .filter(|other| *other != channel)
                .all(|other| color[channel] > color[other] + 60)
        })
    })
}

fn near_background(image: &[u8], at: DVec2) -> bool {
    let (cx, cy) = (at.x.round() as i64, at.y.round() as i64);
    (-2..=2).all(|dy| {
        (-2..=2).all(|dx| {
            rgb(image, cx + dx, cy + dy)
                .iter()
                .all(|channel| (channel - i32::from(BACKGROUND)).abs() < 8)
        })
    })
}

fn render_rotate(camera: &Camera) -> (Vec<u8>, RotateHandles) {
    let capture = handle_capture(camera);
    let handles = RotateHandles::new(&capture, &HandleFrame::world(DVec3::ZERO).unwrap()).unwrap();
    let frame = ui_frame(1.0, |ui| {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, Color32::from_gray(BACKGROUND));
        paint_rotate(ui.painter(), Pos2::ZERO, 1.0, &handles, None, None);
    });
    (Offscreen::new().render(frame), handles)
}

fn pixel_of(camera: &Camera, world: DVec3) -> DVec2 {
    let ndc = camera.project(world.as_vec3(), 1.0).unwrap().unwrap();
    DVec2::new(
        (f64::from(ndc.x) + 1.0) * f64::from(SIZE) * 0.5,
        (1.0 - f64::from(ndc.y)) * f64::from(SIZE) * 0.5,
    )
}

#[test]
#[ignore = "requires a real headless GPU adapter"]
fn headless_gpu_rotate_ring_front_half_reaches_pixels_and_back_half_does_not() {
    let camera = // A narrow view keeps a 72 px ring well in front of the camera in this 128 px image.
    handle_camera(0.6, 0.4, 30.0);
    let (image, handles) = render_rotate(&camera);
    let eye = camera.eye().as_dvec3();
    let focal = f64::from(SIZE) * 0.5 / (f64::from(30_f32.to_radians()) * 0.5).tan();
    let radius = 72.0 * f64::from(camera.distance()) / focal;
    for (axis_index, axis, channel) in [(0, DVec3::X, 0), (1, DVec3::Y, 1), (2, DVec3::Z, 2)] {
        let toward = (eye - axis * eye.dot(axis)).normalize() * radius;
        let front = pixel_of(&camera, toward);
        let back = pixel_of(&camera, -toward);
        assert!(
            near_dominant(&image, front, channel),
            "axis {axis_index}: no ring pixel at {front}"
        );
        // Another ring may cross the hidden point, so test the ring's own color there.
        assert!(
            !near_dominant(&image, back, channel),
            "axis {axis_index}: hidden half is drawn at {back}"
        );
    }
    // The pixels the picker accepts are the pixels that were drawn.
    for ring in handles.rings() {
        let channel = match ring.axis {
            RotateAxis::X => 0,
            RotateAxis::Y => 1,
            RotateAxis::Z => 2,
        };
        let drawn = ring
            .segments()
            .iter()
            .filter(|segment| {
                let middle = (segment.start + segment.end) * 0.5;
                let inside = middle.cmpge(DVec2::splat(2.0)).all()
                    && middle.cmple(DVec2::splat(f64::from(SIZE) - 3.0)).all();
                inside && handles.hit_test(middle) == Some(ring.axis)
            })
            .filter(|segment| near_dominant(&image, (segment.start + segment.end) * 0.5, channel))
            .count();
        assert!(drawn >= 3, "{:?} drew {drawn} verified segments", ring.axis);
    }
}

#[test]
#[ignore = "requires a real headless GPU adapter"]
fn headless_gpu_scale_boxes_center_and_disabled_planes_reach_pixels() {
    let camera = handle_camera(0.0, 0.0, 90.0);
    let capture = handle_capture(&camera);
    let handles = ScaleHandles::new(&capture, &HandleFrame::world(DVec3::ZERO).unwrap()).unwrap();
    let frame = ui_frame(1.0, |ui| {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, Color32::from_gray(BACKGROUND));
        paint_scale(
            ui.painter(),
            Pos2::ZERO,
            1.0,
            &handles,
            Some(ScaleHandle::Uniform),
            None,
        );
    });
    let image = Offscreen::new().render(frame);
    // The viewport is 128 px wide, so the 72 px shafts run off the edge; check what remains.
    assert!(near_dominant(&image, DVec2::new(100.0, 64.0), 0));
    assert!(near_dominant(&image, DVec2::new(64.0, 28.0), 1));
    // Z points at the camera, so there is no Z handle anywhere on the X axis row below.
    assert!(near_background(&image, DVec2::new(64.0, 100.0)));
    // The hovered center box is white on its right edge; the plane outline is the dim grey.
    let edge = rgb(&image, 71, 64);
    assert!(edge.iter().all(|channel| *channel > 200), "{edge:?}");
    let plane = rgb(&image, 64 + 36, 64 - 30);
    assert!(
        plane.iter().all(|channel| (80..140).contains(channel)),
        "{plane:?}"
    );
    assert!(near_background(&image, DVec2::new(100.0, 100.0)));
}
