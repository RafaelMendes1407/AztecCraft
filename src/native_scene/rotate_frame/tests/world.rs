use super::*;
use glam::{DQuat, DVec3, EulerRot};

fn quat(degrees: [f64; 3]) -> DQuat {
    let [x, y, z] = degrees.map(f64::to_radians);
    DQuat::from_euler(EulerRot::ZYX, z, y, x)
}

fn world_axis(axis: RotateAxis) -> DVec3 {
    match axis {
        RotateAxis::X => DVec3::X,
        RotateAxis::Y => DVec3::Y,
        RotateAxis::Z => DVec3::Z,
    }
}

fn same_orientation(a: DQuat, b: DQuat) -> bool {
    1.0 - a.dot(b).abs() < 1e-12
}

fn world_frame(scene: &NativeScene, node: NodeId, axis: RotateAxis) -> RotateFrame {
    scene
        .begin_rotation(&target(scene, node), axis, RotateSpace::World)
        .unwrap()
        .0
}

fn nested(parent: NativeTransform, child: [f64; 3]) -> (NativeScene, NodeId) {
    let (mut scene, root, node, _) = test_fixtures::grouped_shape();
    scene.set_transform(root, parent).unwrap();
    scene
        .set_transform(
            node,
            NativeTransform {
                rotation: child,
                ..NativeTransform::default()
            },
        )
        .unwrap();
    (scene, node)
}

#[test]
fn seeded_sweep_matches_an_independent_quaternion_world_orientation() {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let (mut checked, mut gimbal) = (0, 0);
    for round in 0..300 {
        let parent_rotation = [
            next() * 720.0 - 360.0,
            next() * 360.0 - 180.0,
            next() * 720.0 - 360.0,
        ];
        let uniform = 0.1 + next() * 9.9;
        let child = [
            next() * 720.0 - 360.0,
            next() * 170.0 - 85.0,
            next() * 720.0 - 360.0,
        ];
        let degrees = next() * 720.0 - 360.0;
        // Every fifth round rotates a root node, whose parent map is the identity.
        let (scene, node, parent_q) = if round % 5 == 0 {
            let (scene, node) = rotated(child);
            (scene, node, DQuat::IDENTITY)
        } else {
            let (scene, node) = nested(
                NativeTransform {
                    translation: [next() * 10.0, 0.0, -3.0],
                    rotation: parent_rotation,
                    scale: [uniform; 3],
                },
                child,
            );
            (scene, node, quat(parent_rotation))
        };
        for axis in [RotateAxis::X, RotateAxis::Y, RotateAxis::Z] {
            let frame = world_frame(&scene, node, axis);
            match frame.rotation(axis, degrees) {
                Ok(out) => {
                    let expected = DQuat::from_axis_angle(world_axis(axis), degrees.to_radians())
                        * (parent_q * quat(child));
                    assert!(
                        same_orientation(parent_q * quat(out), expected),
                        "round {round} {axis:?} {degrees}"
                    );
                    checked += 1;
                }
                Err(NativeError::UnsupportedRotate(RotateUnsupported::Gimbal)) => gimbal += 1,
                Err(error) => panic!("round {round} {axis:?}: {error}"),
            }
        }
    }
    assert!(checked > 800, "{checked} checked, {gimbal} gimbal");
}

#[test]
fn world_axis_moves_only_the_matching_channel_for_an_identity_orientation() {
    let (scene, node) = rotated([0.0; 3]);
    for (axis, index) in [(RotateAxis::X, 0), (RotateAxis::Y, 1), (RotateAxis::Z, 2)] {
        let out = world_frame(&scene, node, axis)
            .rotation(axis, 33.0)
            .unwrap();
        assert!((out[index] - 33.0).abs() < 1e-9, "{out:?}");
        for other in (0..3).filter(|i| *i != index) {
            assert_eq!(out[other].to_bits(), 0.0f64.to_bits());
        }
    }
    for (axis, index) in [(RotateAxis::X, 0), (RotateAxis::Y, 1), (RotateAxis::Z, 2)] {
        let out = world_frame(&scene, node, axis)
            .rotation(axis, 1e-11)
            .unwrap();
        assert!((out[index] - 1e-11).abs() < 1e-23, "{axis:?} {out:?}");
    }
}

#[test]
fn zero_angle_and_out_and_back_are_bit_exact() {
    let (mut scene, node) = nested(
        NativeTransform {
            rotation: [20.0, 35.0, -15.0],
            scale: [2.5; 3],
            ..NativeTransform::default()
        },
        [13.0, 120.0, -77.0],
    );
    let before = scene.clone();
    for axis in [RotateAxis::X, RotateAxis::Y, RotateAxis::Z] {
        let (frame, baseline) = scene
            .begin_rotation(&target(&scene, node), axis, RotateSpace::World)
            .unwrap();
        assert_eq!(
            bits(frame.rotation(axis, 0.0).unwrap()),
            bits(frame.declared)
        );
        assert_eq!(
            bits(frame.rotation(axis, -0.0).unwrap()),
            bits(frame.declared)
        );
        let out = frame.rotation(axis, 41.0).unwrap();
        assert!(scene.preview_channels(&baseline, out).unwrap().1);
        scene
            .preview_channels(&baseline, frame.rotation(axis, 0.0).unwrap())
            .unwrap();
        assert_eq!(scene, before);
    }
}

#[test]
fn parents_that_are_not_rotation_times_uniform_scale_are_refused_without_mutation() {
    let skewed = [
        [2.0, 3.0, 4.0],
        [1.0, 1.0, 1.0001],
        [1.0, -1.0, 1.0],
        [-1.0, -1.0, -1.0],
        [0.0, 1.0, 1.0],
    ];
    for scale in skewed {
        let (scene, node) = nested(
            NativeTransform {
                rotation: [30.0, 40.0, 50.0],
                scale,
                ..NativeTransform::default()
            },
            [5.0, 6.0, 7.0],
        );
        let before = scene.clone();
        assert!(
            matches!(
                scene.begin_rotation(&target(&scene, node), RotateAxis::Y, RotateSpace::World),
                Err(NativeError::UnsupportedRotate(
                    RotateUnsupported::ParentTransform
                ))
            ),
            "{scale:?}"
        );
        // The same scene rotates in object space: no parent condition applies there.
        assert!(
            scene
                .begin_rotation(&target(&scene, node), RotateAxis::Y, RotateSpace::Object)
                .is_ok()
        );
        assert_eq!(scene, before);
    }
    // Uniform scale within the stated tolerance is accepted.
    let (scene, node) = nested(
        NativeTransform {
            scale: [1.0, 1.0 + 1e-11, 1.0],
            ..NativeTransform::default()
        },
        [0.0; 3],
    );
    assert!(
        scene
            .begin_rotation(&target(&scene, node), RotateAxis::X, RotateSpace::World)
            .is_ok()
    );
}

#[test]
fn shear_composed_from_a_rotated_non_uniform_scale_is_refused() {
    let skew = DMat3::from_cols(DVec3::X, DVec3::new(0.5, 1.0, 0.0), DVec3::Z);
    assert!(!rotation_times_uniform_scale(skew));
    // Equal column lengths but not orthogonal: a pure shear of unit vectors.
    let (sin, cos) = 0.4f64.sin_cos();
    let unit_shear = DMat3::from_cols(DVec3::X, DVec3::new(cos, sin, 0.0), DVec3::Z);
    assert!(!rotation_times_uniform_scale(unit_shear));
    let nonuniform_after_rotation =
        DMat3::from_diagonal(DVec3::new(2.0, 1.0, 1.0)) * DMat3::from_rotation_z(0.7);
    assert!(!rotation_times_uniform_scale(nonuniform_after_rotation));
    assert!(rotation_times_uniform_scale(
        DMat3::from_rotation_y(0.3) * 4.0
    ));
    assert!(!rotation_times_uniform_scale(DMat3::from_cols(
        DVec3::splat(f64::NAN),
        DVec3::Y,
        DVec3::Z
    )));
    assert!(!rotation_times_uniform_scale(DMat3::ZERO));
}

#[test]
fn non_positive_scale_on_the_selected_node_is_refused() {
    for scale in [[-1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, -2.0]] {
        let (mut scene, node) = test_fixtures::cube();
        scene
            .set_transform(
                node,
                NativeTransform {
                    scale,
                    ..NativeTransform::default()
                },
            )
            .unwrap();
        let before = scene.clone();
        assert!(matches!(
            scene.begin_rotation(&target(&scene, node), RotateAxis::X, RotateSpace::World),
            Err(NativeError::UnsupportedRotate(
                RotateUnsupported::NonPositiveScale
            ))
        ));
        assert!(
            scene
                .begin_rotation(&target(&scene, node), RotateAxis::X, RotateSpace::Object)
                .is_ok()
        );
        assert_eq!(scene, before);
    }
}

#[test]
fn world_rings_need_all_three_rotate_leaves_and_inherit_the_other_refusals() {
    let (scene, node) = rotated([10.0, 20.0, 30.0]);
    let base = world_frame(&scene, node, RotateAxis::X);
    let only_x = RotateFrame {
        absent_leaf: [false, true, true],
        ..base
    };
    for axis in [RotateAxis::X, RotateAxis::Y, RotateAxis::Z] {
        assert!(matches!(
            only_x.require_leaves(axis),
            Err(NativeError::AbsentChannelLeaf {
                channel: ChannelLeaf::Rotate,
                component: 1
            })
        ));
    }
    let (mut scene, node) = test_fixtures::cube();
    scene.source = Some(SourceRetention {
        bytes: [1].into(),
        format: SourceFormat::Binary,
        records: std::iter::once(0..1).collect(),
        leaves: vec![],
        appearance: None,
    });
    for axis in [RotateAxis::X, RotateAxis::Y, RotateAxis::Z] {
        assert!(matches!(
            scene.begin_rotation(&target(&scene, node), axis, RotateSpace::World),
            Err(NativeError::AbsentChannelLeaf {
                channel: ChannelLeaf::Rotate,
                component: 0
            })
        ));
    }
    let (mut scene, node) = test_fixtures::cube();
    scene.nodes[0].rotate_pivot[1] = Scalar::new(0.5);
    assert!(matches!(
        scene.begin_rotation(&target(&scene, node), RotateAxis::X, RotateSpace::World),
        Err(NativeError::UnsupportedRotate(
            RotateUnsupported::SelectedPivot
        ))
    ));
}

#[test]
fn gimbal_and_huge_angles_are_refused_in_world_space() {
    let (scene, node) = rotated([0.0; 3]);
    let frame = world_frame(&scene, node, RotateAxis::Y);
    assert!(matches!(
        frame.rotation(RotateAxis::Y, 90.0),
        Err(NativeError::UnsupportedRotate(RotateUnsupported::Gimbal))
    ));
    for bad in [f64::NAN, f64::INFINITY] {
        assert!(matches!(
            frame.rotation(RotateAxis::Y, bad),
            Err(NativeError::NonFinite)
        ));
    }
    assert!(matches!(
        frame.rotation(RotateAxis::Y, 1e308),
        Err(NativeError::UnsupportedRotate(
            RotateUnsupported::NumericalAngle
        ))
    ));
}
