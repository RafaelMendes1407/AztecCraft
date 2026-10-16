use super::*;

fn frame(pointer: Vec2, pixels: [u32; 2], ppp: f32) -> Result<PickFrame, ViewError> {
    Camera::default().pick_frame(pointer, pixels, ppp)
}

fn refused(pointer: Vec2, pixels: [u32; 2], ppp: f32) -> bool {
    matches!(
        frame(pointer, pixels, ppp),
        Err(ViewError::Camera(InvalidCameraInput))
    )
}

#[test]
fn pick_frame_accepts_the_viewport_rectangle_edges_inclusively() {
    let accepted = frame(Vec2::new(200.0, 100.0), [200, 100], 1.0).unwrap();
    assert_eq!(accepted.size, Vec2::new(200.0, 100.0));
    assert_eq!(accepted.pointer, Vec2::new(200.0, 100.0));
    assert_eq!(accepted.aspect, 2.0);
    let origin = frame(Vec2::ZERO, [200, 100], 1.0).unwrap();
    assert_eq!(origin.pointer, Vec2::ZERO);
    let scaled = frame(Vec2::new(100.0, 50.0), [200, 100], 2.0).unwrap();
    assert_eq!(scaled.pointer, Vec2::new(200.0, 100.0));
}

#[test]
fn pick_frame_rejects_pointers_outside_the_viewport() {
    let past = 200.0_f32.next_up();
    let below = -f32::MIN_POSITIVE;
    assert!(refused(Vec2::new(past, 50.0), [200, 100], 1.0));
    assert!(refused(
        Vec2::new(100.0, 100.0_f32.next_up()),
        [200, 100],
        1.0
    ));
    assert!(refused(Vec2::new(below, 50.0), [200, 100], 1.0));
    assert!(refused(Vec2::new(100.0, below), [200, 100], 1.0));
    assert!(refused(Vec2::new(-1.0, 50.0), [200, 100], 1.0));
    assert!(refused(Vec2::new(100.0, -1.0), [200, 100], 1.0));
    assert!(refused(Vec2::new(101.0, 50.0), [200, 100], 2.0));
}

#[test]
fn pick_frame_rejects_empty_and_non_finite_input() {
    let pointer = Vec2::new(10.0, 10.0);
    assert!(refused(pointer, [0, 100], 1.0));
    assert!(refused(pointer, [200, 0], 1.0));
    assert!(refused(Vec2::ZERO, [200, 0], 1.0));
    assert!(refused(Vec2::ZERO, [0, 100], 1.0));
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(refused(Vec2::new(bad, 10.0), [200, 100], 1.0));
        assert!(refused(Vec2::new(10.0, bad), [200, 100], 1.0));
        assert!(refused(pointer, [200, 100], bad));
    }
    assert!(refused(pointer, [200, 100], 0.0));
    assert!(refused(pointer, [200, 100], -1.0));
}
