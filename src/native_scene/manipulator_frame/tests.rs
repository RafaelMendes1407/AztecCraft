use super::*;
use crate::native_scene::test_fixtures;

fn target_of(scene: &NativeScene, node: NodeId) -> TransformTarget {
    scene
        .transform_paths()
        .unwrap()
        .into_iter()
        .find(|path| path.node == node)
        .unwrap()
}

fn near(actual: DVec3, expected: DVec3) {
    assert!(
        (actual - expected).abs().max_element() < 1e-12,
        "{actual} != {expected}"
    );
}

#[test]
fn a_root_frame_sits_at_its_translation_with_its_own_rotation() {
    let (mut scene, node) = test_fixtures::cube();
    scene
        .set_transform(
            node,
            NativeTransform {
                translation: [1.0, 2.0, 3.0],
                rotation: [0.0, 0.0, 90.0],
                scale: [5.0, 6.0, 7.0],
            },
        )
        .unwrap();
    let frame = scene.manipulator_frame(&target_of(&scene, node)).unwrap();
    near(frame.origin_cm, DVec3::new(1.0, 2.0, 3.0));
    // Node scale never reaches the axes; a quarter turn about Z sends X to Y and Y to -X.
    near(frame.object_axes[0], DVec3::Y);
    near(frame.object_axes[1], -DVec3::X);
    near(frame.object_axes[2], DVec3::Z);
}

#[test]
fn parents_move_the_origin_and_turn_and_scale_the_axes() {
    let (mut scene, root, node, _) = test_fixtures::grouped_shape();
    scene
        .set_transform(
            root,
            NativeTransform {
                translation: [10.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 90.0],
                scale: [2.0, 2.0, 2.0],
            },
        )
        .unwrap();
    scene
        .set_transform(
            node,
            NativeTransform {
                translation: [1.0, 0.0, 0.0],
                ..NativeTransform::default()
            },
        )
        .unwrap();
    let frame = scene.manipulator_frame(&target_of(&scene, node)).unwrap();
    near(frame.origin_cm, DVec3::new(10.0, 2.0, 0.0));
    near(frame.object_axes[0], DVec3::new(0.0, 2.0, 0.0));
    near(frame.object_axes[1], DVec3::new(-2.0, 0.0, 0.0));
}

#[test]
fn a_negative_scale_parent_gives_left_handed_axes() {
    let (mut scene, root, node, _) = test_fixtures::grouped_shape();
    scene
        .set_transform(
            root,
            NativeTransform {
                scale: [-1.0, 1.0, 1.0],
                ..NativeTransform::default()
            },
        )
        .unwrap();
    let frame = scene.manipulator_frame(&target_of(&scene, node)).unwrap();
    let [x, y, z] = frame.object_axes;
    assert!(x.cross(y).dot(z) < 0.0);
}

#[test]
fn an_invalid_target_is_refused() {
    let (scene, node) = test_fixtures::cube();
    let mut bad = target_of(&scene, node);
    bad.path.clear();
    assert!(scene.manipulator_frame(&bad).is_err());
}

#[test]
fn refusals_report_what_begin_would_refuse() {
    let (scene, node) = test_fixtures::cube();
    let target = target_of(&scene, node);
    assert_eq!(
        scene.rotation_refusal(&target, RotateAxis::Y, RotateSpace::Object),
        None
    );
    assert_eq!(
        scene.scale_refusal(&target, ScaleHandle::Uniform, ScaleSpace::World),
        None
    );
    let (mut uneven, root, child, _) = test_fixtures::grouped_shape();
    uneven
        .set_transform(
            root,
            NativeTransform {
                scale: [1.0, 2.0, 1.0],
                ..NativeTransform::default()
            },
        )
        .unwrap();
    assert_eq!(
        uneven.scale_refusal(
            &target_of(&uneven, child),
            ScaleHandle::X,
            ScaleSpace::World
        ),
        Some(NativeError::UnsupportedScale(
            ScaleUnsupported::ParentTransform
        ))
    );
    let (pivoted, node) = test_fixtures::pivoted_shape();
    assert_eq!(
        pivoted.rotation_refusal(
            &target_of(&pivoted, node),
            RotateAxis::X,
            RotateSpace::Object
        ),
        Some(NativeError::UnsupportedRotate(
            RotateUnsupported::SelectedPivot
        ))
    );
}
