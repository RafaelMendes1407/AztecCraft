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
            scene.extrude_face(id, 1, 1.0),
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
    scene.dag.push(DagEdge {
        parent: None,
        child: scene.nodes[0].shape_id,
    });
    scene.validate().unwrap();
    assert!(scene.supported_view().is_err());
    assert_eq!(scene.remove(id), Err(NativeError::UnsupportedEdit));
    let (mut scene, parent) = cube();
    let child = scene.create(Primitive::Plane, "child").unwrap();
    scene.dag.push(DagEdge {
        parent: Some(scene.node(parent).unwrap().transform_id),
        child: scene.node(child).unwrap().transform_id,
    });
    scene.validate().unwrap();
    let before = scene.clone();
    assert_eq!(
        scene.set_channel(parent, Channel::TranslateX, 1.0),
        Err(NativeError::UnsupportedEdit)
    );
    assert_eq!(scene, before);
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
    assert_eq!(view.node(id).unwrap().mesh().vertices()[0].x, -50.0);
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
