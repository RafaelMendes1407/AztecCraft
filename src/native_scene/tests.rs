use super::*;

fn cube() -> (NativeScene, NodeId) {
    let mut scene = NativeScene::default();
    let id = scene.create(Primitive::Cube, "pCube1").unwrap();
    (scene, id)
}

fn retain(scene: &mut NativeScene, bytes: &[u8], records: Vec<Range<usize>>) {
    scene.source = Some(SourceRetention {
        bytes: bytes.into(),
        records,
    });
    scene.validate().unwrap();
}

#[test]
fn retained_source_and_lexemes_have_one_effective_value_and_survive_restore() {
    let (mut scene, id) = cube();
    retain(
        &mut scene,
        b"setAttr .tx +0.000;",
        std::iter::once(0..18).collect(),
    );
    scene.nodes[0].channels[0].source_lexeme = Some("+0.000".into());
    let before = scene.clone();
    scene.set_channel(id, Channel::TranslateX, 0.0).unwrap();
    assert_ne!(scene, before);
    assert_eq!(scene.source, before.source);
    assert_eq!(
        scene.nodes[0].channels[0].source_lexeme,
        before.nodes[0].channels[0].source_lexeme
    );
    let plus_zero = scene.clone();
    scene.set_channel(id, Channel::TranslateX, -0.0).unwrap();
    assert_ne!(scene, plus_zero);
    assert_eq!(
        scene.nodes[0].channels[0].replaced,
        plus_zero.nodes[0].channels[0].replaced
    );
    assert_eq!(
        scene.node(id).unwrap().transform().translation[0].to_bits(),
        (-0.0f64).to_bits()
    );
    scene.set_channel(id, Channel::TranslateX, 2.5).unwrap();
    assert_eq!(
        scene
            .supported_view()
            .unwrap()
            .node(id)
            .unwrap()
            .transform()
            .translation
            .x,
        2.5
    );
    scene.restore_authoring(before.clone());
    assert_eq!(scene, before);
    let mut deleted = before.clone();
    deleted.source = None;
    assert_ne!(deleted, before);
    deleted = before.clone();
    deleted.nodes[0].channels[0].source_lexeme = Some("0".into());
    assert_ne!(deleted, before);
}

#[test]
fn source_free_identical_bits_are_noops_but_signed_zero_remains_meaningful() {
    let (mut scene, id) = cube();
    let before = scene.clone();
    scene.set_channel(id, Channel::TranslateX, 0.0).unwrap();
    scene.set_transform(id, NativeTransform::default()).unwrap();
    assert_eq!(scene, before);
    scene.set_channel(id, Channel::TranslateX, -0.0).unwrap();
    assert_ne!(scene, before);
    let changed = scene.clone();
    scene
        .set_transform(id, scene.node(id).unwrap().transform())
        .unwrap();
    assert_eq!(scene, changed);
    let mut legacy = Scene::default();
    let id = legacy.create(Primitive::Cube, "legacy name").unwrap();
    let mut promoted = NativeScene::from_legacy(legacy).unwrap();
    let before = promoted.clone();
    promoted
        .set_transform(id, promoted.node(id).unwrap().transform())
        .unwrap();
    assert_eq!(promoted, before);
}

#[test]
fn dependency_unknown_transform_and_dag_effects_refuse_before_mutation() {
    for restriction in ["connection", "lock", "history", "unmapped owner"] {
        let (mut scene, id) = cube();
        retain(
            &mut scene,
            restriction.as_bytes(),
            std::iter::once(0..restriction.len()).collect(),
        );
        scene.nodes[0].restrictions.push(restriction.into());
        let before = scene.clone();
        assert_eq!(
            scene.rename(id, "renamed"),
            Err(NativeError::UnsupportedEdit)
        );
        assert_eq!(scene.remove(id), Err(NativeError::UnsupportedEdit));
        assert_eq!(
            scene.set_channel(id, Channel::TranslateY, 1.0),
            Err(NativeError::UnsupportedEdit)
        );
        assert_eq!(
            scene.extrude_face(scene.shapes[0].id, 1, 1.0),
            Err(NativeError::UnsupportedEdit)
        );
        assert_eq!(scene, before);
    }
    let (mut scene, id) = cube();
    scene.nodes[0].unsupported_transform = vec!["rotationOrder YZX".into(), "shear .5".into()];
    let before = scene.clone();
    assert!(scene.supported_view().is_err());
    assert_eq!(
        scene.set_channel(id, Channel::ScaleX, 2.0),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(scene, before);
    scene.nodes[0].unsupported_transform.clear();
    let other = scene.create(Primitive::Plane, "other").unwrap();
    scene.dag.push(DagEdge {
        parent: Some(scene.node(other).unwrap().transform_id),
        child: scene.shapes[0].id,
    });
    scene.validate().unwrap();
    assert!(scene.supported_view().is_err());
    let shared = scene.shapes[0].id;
    scene.remove(id).unwrap();
    assert!(scene.shape(shared).is_ok());
    let (mut scene, parent) = cube();
    let child = scene.create(Primitive::Plane, "child").unwrap();
    scene.dag.push(DagEdge {
        parent: Some(scene.node(parent).unwrap().transform_id),
        child: scene.node(child).unwrap().transform_id,
    });
    scene.validate().unwrap();
    let before = scene.clone();
    scene.set_channel(parent, Channel::TranslateX, 1.0).unwrap();
    assert_eq!(scene.dag, before.dag);
    assert_eq!(scene.shapes, before.shapes);
}

#[test]
fn unit_conversion_only_changes_derived_view() {
    let (mut scene, id) = cube();
    scene.units = Units {
        linear: LinearUnit::Meter,
        angular: AngularUnit::Radian,
        time: "pal".into(),
    };
    scene
        .set_transform(
            id,
            NativeTransform {
                translation: [1.0, 0.0, 0.0],
                rotation: [std::f64::consts::PI, 0.0, 0.0],
                ..NativeTransform::default()
            },
        )
        .unwrap();
    let before = scene.clone();
    let view = scene.supported_view().unwrap();
    assert_eq!(view.node(id).unwrap().transform().translation.x, 100.0);
    assert_eq!(view.node(id).unwrap().transform().rotation_degrees.x, 180.0);
    assert_eq!(view.node(id).unwrap().mesh().vertices()[0].x, -0.5);
    assert_eq!(scene, before);
    scene.units.linear = LinearUnit::Unsupported("native-token".into());
    scene.validate().unwrap();
    assert_eq!(
        scene.supported_view(),
        Err(NativeError::UnsupportedView("linear unit"))
    );
    assert_eq!(
        scene.units.linear,
        LinearUnit::Unsupported("native-token".into())
    );
}

#[test]
fn retained_ranges_and_exhausted_allocators_are_checked() {
    let (mut scene, _) = cube();
    scene.source = Some(SourceRetention {
        bytes: vec![0; 4],
        records: vec![0..3, 2..4],
    });
    assert_eq!(scene.validate(), Err(NativeError::InvalidRetention));
    scene.source = Some(SourceRetention {
        bytes: vec![0; 4],
        records: std::iter::once(0..5).collect(),
    });
    assert_eq!(scene.validate(), Err(NativeError::InvalidRetention));
    scene.source = None;
    scene.next_id = u64::MAX;
    scene.validate().unwrap();
    let before = scene.clone();
    assert_eq!(
        scene.create(Primitive::Cube, "another"),
        Err(NativeError::Legacy(SceneError::IdsExhausted))
    );
    assert_eq!(scene, before);
    assert_eq!(scene.next_id, before.next_id);
    scene.next_id = 10;
    scene.next_native_id = u64::MAX - 1;
    let before = scene.clone();
    assert_eq!(
        scene.create(Primitive::Plane, "another"),
        Err(NativeError::Legacy(SceneError::IdsExhausted))
    );
    assert_eq!(scene, before);
    assert_eq!(scene.next_native_id, before.next_native_id);
}

fn shared_shape() -> (NativeScene, NodeId, NodeId, NativeId) {
    let (mut scene, first) = cube();
    let second = scene.create(Primitive::Plane, "instance").unwrap();
    let original = scene.shapes[0].id;
    let discarded = scene.shapes.pop().unwrap().id;
    scene.dag.retain(|edge| edge.child != discarded);
    scene.dag.push(DagEdge {
        parent: Some(scene.node(second).unwrap().transform_id),
        child: original,
    });
    scene.validate().unwrap();
    (scene, first, second, original)
}

fn grouped_shape() -> (NativeScene, NodeId, NodeId, NativeId) {
    let (mut scene, root) = cube();
    let child = scene.create(Primitive::Cube, "child").unwrap();
    let discarded = scene.shapes.remove(0).id;
    let root_native = scene.node(root).unwrap().transform_id;
    let child_native = scene.node(child).unwrap().transform_id;
    scene.dag.retain(|edge| {
        edge.child != discarded && !(edge.child == child_native && edge.parent.is_none())
    });
    scene.dag.push(DagEdge {
        parent: Some(root_native),
        child: child_native,
    });
    scene.validate().unwrap();
    let shape = scene.shapes[0].id;
    (scene, root, child, shape)
}

#[test]
fn grouped_transform_and_pivots_preserve_native_precision_without_baking() {
    let (mut scene, root, child, shape) = grouped_shape();
    scene.nodes[1].rotate_pivot = [1.000000000001, -0.0, 2.0].map(Scalar::new);
    scene.nodes[1].scale_pivot = [-2.000000000001, 3.0, 4.0].map(Scalar::new);
    retain(
        &mut scene,
        b"known appearance metadata",
        std::iter::once(0..25).collect(),
    );
    let before = scene.clone();
    scene
        .set_channel(root, Channel::TranslateY, 4.000000000001)
        .unwrap();
    scene
        .set_channel(child, Channel::RotateZ, 17.000000000001)
        .unwrap();
    assert_eq!(scene.nodes[1].rotate_pivot, before.nodes[1].rotate_pivot);
    assert_eq!(scene.nodes[1].scale_pivot, before.nodes[1].scale_pivot);
    assert_eq!(scene.shape(shape), before.shape(shape));
    assert_eq!(scene.dag, before.dag);
    assert_eq!(scene.source, before.source);
    assert!(matches!(
        scene.supported_view(),
        Err(NativeError::UnsupportedView(_))
    ));
    scene.restore_authoring(before.clone());
    assert_eq!(scene, before);
    scene.nodes[1].rotate_pivot[1].value = 0.0;
    assert_ne!(scene, before);
    scene.nodes[1].scale_pivot[0].value = f64::INFINITY;
    assert_eq!(scene.validate(), Err(NativeError::NonFinite));
}

#[test]
fn shared_shape_extrusion_changes_one_owner_and_checks_the_other_instance() {
    let (mut scene, first, second, shape) = shared_shape();
    let links = scene.dag.clone();
    let shape_uuid = scene.shape(shape).unwrap().uuid();
    let points = scene.shape(shape).unwrap().mesh().points().clone();
    assert_eq!(scene.shapes.len(), 1);
    assert!(scene.supported_view().is_err());
    scene.extrude_face(shape, 1, 0.0).unwrap();
    assert_eq!(scene.shapes.len(), 1);
    assert_eq!(scene.dag, links);
    assert_eq!(scene.shape(shape).unwrap().uuid(), shape_uuid);
    assert_eq!(scene.shape(shape).unwrap().mesh().faces().len(), 10);
    let (Points::Float3(old), Points::Float3(new)) =
        (&points, scene.shape(shape).unwrap().mesh().points())
    else {
        panic!()
    };
    assert_eq!(Points::Float3(new[..old.len()].into()), points);
    let second_index = scene
        .nodes
        .iter()
        .position(|node| node.id == second)
        .unwrap();
    scene.nodes[second_index]
        .unsupported_transform
        .push("unproved ancestor effect".into());
    let target = FaceTarget {
        transform: first,
        shape,
        path: vec![scene.node(first).unwrap().transform_id, shape],
        face_index: 1,
    };
    scene.resolve_face_target(&target).unwrap();
    let before = scene.clone();
    assert_eq!(
        scene.extrude_face(shape, 1, 1.0),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(scene, before);
}

#[test]
fn root_leaf_removal_preserves_shared_shapes_and_refuses_groups_or_source_rewrites() {
    let (mut scene, first, second, shape) = shared_shape();
    let preserved = scene.shape(shape).unwrap().clone();
    scene.remove(first).unwrap();
    assert_eq!(scene.shape(shape).unwrap(), &preserved);
    assert_eq!(scene.dag.len(), 2);
    scene.validate().unwrap();
    scene.remove(second).unwrap();
    assert!(scene.shapes.is_empty());
    assert!(scene.dag.is_empty());
    scene.validate().unwrap();
    let (mut scene, root, child, _) = grouped_shape();
    let before = scene.clone();
    assert_eq!(scene.remove(root), Err(NativeError::UnsupportedEdit));
    assert_eq!(scene.remove(child), Err(NativeError::UnsupportedEdit));
    assert_eq!(scene, before);
    let (mut scene, id) = cube();
    retain(&mut scene, b"original", std::iter::once(0..8).collect());
    let before = scene.clone();
    assert_eq!(scene.remove(id), Err(NativeError::UnsupportedEdit));
    assert_eq!(
        scene.rename(id, "replacement"),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(
        scene.create(Primitive::Cube, "new"),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(
        scene.extrude_face(scene.shapes[0].id, 1, 1.0),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(scene, before);
    assert_eq!(scene.next_id, before.next_id);
    assert_eq!(scene.next_native_id, before.next_native_id);
}

#[test]
fn face_target_validates_full_native_identity_path_and_original_face_bounds() {
    let (scene, root, child, shape) = grouped_shape();
    let target = FaceTarget {
        transform: child,
        shape,
        path: vec![
            scene.node(root).unwrap().transform_id,
            scene.node(child).unwrap().transform_id,
            shape,
        ],
        face_index: 1,
    };
    assert_eq!(scene.resolve_face_target(&target).unwrap().id(), shape);
    let decimal = shape.to_decimal();
    assert_eq!(NativeId::from_decimal(&decimal).unwrap(), shape);
    for path in [
        vec![],
        vec![shape],
        vec![scene.node(child).unwrap().transform_id, shape],
        vec![scene.node(root).unwrap().transform_id, shape],
        vec![target.path[0], target.path[1], target.path[1], shape],
    ] {
        let bad = FaceTarget {
            path,
            ..target.clone()
        };
        assert_eq!(
            scene.resolve_face_target(&bad),
            Err(NativeError::InvalidInstancePath)
        );
    }
    let bad_transform = FaceTarget {
        transform: root,
        ..target.clone()
    };
    assert_eq!(
        scene.resolve_face_target(&bad_transform),
        Err(NativeError::InvalidInstancePath)
    );
    let bad_face = FaceTarget {
        face_index: 6,
        ..target
    };
    assert_eq!(
        scene.resolve_face_target(&bad_face),
        Err(SceneError::MissingFace(6).into())
    );
}

#[test]
fn invalid_dag_cycles_duplicates_roles_and_dangling_ids_are_rejected() {
    let (original, root, child, shape) = grouped_shape();
    let root_native = original.node(root).unwrap().transform_id;
    let child_native = original.node(child).unwrap().transform_id;
    for edge in [
        DagEdge {
            parent: Some(child_native),
            child: root_native,
        },
        DagEdge {
            parent: None,
            child: root_native,
        },
        DagEdge {
            parent: None,
            child: shape,
        },
        DagEdge {
            parent: Some(shape),
            child: root_native,
        },
        DagEdge {
            parent: Some(NativeId(u64::MAX)),
            child: child_native,
        },
        DagEdge {
            parent: Some(root_native),
            child: NativeId(u64::MAX),
        },
    ] {
        let mut scene = original.clone();
        scene.dag.push(edge);
        assert_eq!(scene.validate(), Err(NativeError::InvalidIdentity));
    }
    let mut scene = original;
    scene.dag.retain(|edge| edge.child != child_native);
    assert_eq!(scene.validate(), Err(NativeError::InvalidIdentity));
}

#[test]
fn unproved_ancestor_descendant_and_shape_effects_refuse_numeric_edits() {
    let (original, root, child, _) = grouped_shape();
    for (index, target) in [(0, child), (1, root)] {
        let mut scene = original.clone();
        scene.nodes[index]
            .unsupported_transform
            .push("unproved transform effect".into());
        let before = scene.clone();
        assert_eq!(
            scene.set_channel(target, Channel::TranslateX, 2.0),
            Err(NativeError::UnsupportedEdit)
        );
        assert_eq!(scene, before);
    }
    let mut scene = original;
    scene.shapes[0]
        .restrictions
        .push("unmapped geometry dependency".into());
    let before = scene.clone();
    assert_eq!(
        scene.set_transform(
            root,
            NativeTransform {
                scale: [2.0; 3],
                ..NativeTransform::default()
            }
        ),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(scene, before);
}

#[test]
fn declared_extrusion_distance_converts_once_without_rescaling_stored_geometry() {
    let (mut scene, _) = cube();
    scene.units.linear = LinearUnit::Meter;
    let shape = scene.shapes[0].id;
    let old_points = scene.shapes[0].mesh.points().clone();
    scene.extrude_face(shape, 1, 1.0).unwrap();
    let Points::Float3(points) = scene.shapes[0].mesh.points() else {
        panic!()
    };
    assert_eq!(points[8][2], 100.5);
    assert_eq!(Points::Float3(points[..8].into()), old_points);
    scene.units.linear = LinearUnit::Unsupported("unproved".into());
    let before = scene.clone();
    assert_eq!(
        scene.extrude_face(shape, 1, 1.0),
        Err(NativeError::UnsupportedLinearUnit)
    );
    assert_eq!(scene, before);
}

#[test]
fn each_pivot_independently_refuses_the_restricted_view_and_validates_finite_bits() {
    for rotate in [true, false] {
        let (mut scene, _) = cube();
        scene.supported_view().unwrap();
        let pivot = if rotate {
            &mut scene.nodes[0].rotate_pivot
        } else {
            &mut scene.nodes[0].scale_pivot
        };
        pivot[0].value = 0.125;
        scene.validate().unwrap();
        assert_eq!(
            scene.supported_view(),
            Err(NativeError::UnsupportedView(
                "DAG, transform or dependencies"
            ))
        );
        let pivot = if rotate {
            &mut scene.nodes[0].rotate_pivot
        } else {
            &mut scene.nodes[0].scale_pivot
        };
        pivot[0].value = -0.0;
        let signed_zero = scene.clone();
        let pivot = if rotate {
            &mut scene.nodes[0].rotate_pivot
        } else {
            &mut scene.nodes[0].scale_pivot
        };
        pivot[0].value = 0.0;
        assert_ne!(scene, signed_zero);
        scene.supported_view().unwrap();
        for value in [f64::INFINITY, f64::NAN] {
            let pivot = if rotate {
                &mut scene.nodes[0].rotate_pivot
            } else {
                &mut scene.nodes[0].scale_pivot
            };
            pivot[2].value = value;
            assert_eq!(scene.validate(), Err(NativeError::NonFinite));
        }
    }
}
