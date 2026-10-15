use super::*;
use crate::{native_scene::*, scene::Primitive};
use std::sync::Arc;

fn candidate() -> (NativeScene, crate::scene::NodeId) {
    let (mut scene, node) = crate::native_scene::test_fixtures::cube();
    let id = scene.shapes[0].id;
    scene.shapes.clear();
    let bytes: Arc<[u8]> = Arc::from(&b"setAttr geometry;"[..]);
    let end = bytes.len();
    scene.source = Some(SourceRetention {
        records: std::iter::once(0..bytes.len()).collect(),
        bytes,
        format: SourceFormat::Ascii,
        leaves: Vec::new(),
        appearance: None,
    });
    scene.curves.push(NativeCurveShape {
        id,
        uuid: Some(Uuid::nil()),
        name: "curveShape".into(),
        control_vertices: vec![
            [0.0, -0.0, 0.0],
            [1.0, 2.0, 0.0],
            [2.0, -1.0, 1.0],
            [3.0, 0.0, 2.0],
        ],
        knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        source: CurveSource {
            owner: id,
            statement: 0..end,
        },
        view: GeometryViewFacts::authored(),
    });
    scene.validate().unwrap();
    (scene, node)
}

#[test]
fn canonical_curve_is_not_a_mesh_and_never_silently_projects_or_encodes() {
    let (scene, node) = candidate();
    let curve = &scene.curves()[0];
    assert_eq!(curve.degree(), 3);
    assert_eq!(curve.form(), CurveForm::Open);
    assert_eq!(curve.domain().unwrap(), [0.0, 1.0]);
    assert_eq!(curve.control_vertices().len(), 4);
    assert_eq!(curve.knots().len(), 6);
    assert_eq!(curve.name(), "curveShape");
    assert_eq!(curve.uuid(), Some(Uuid::nil()));
    assert!(scene.shape(curve.id()).is_err());
    assert!(scene.shapes().is_empty());
    assert!(matches!(
        scene.supported_view(),
        Err(NativeError::UnsupportedView(_))
    ));
    assert_eq!(
        binary::encode(&scene).unwrap_err().kind(),
        binary::ErrorKind::Unsupported
    );
    assert!(
        scene
            .resolve_face_target(&FaceTarget {
                transform: node,
                shape: curve.id(),
                path: vec![scene.nodes[0].transform_id, curve.id()],
                face_index: 0,
            })
            .is_err()
    );
}

#[test]
fn malformed_curve_counts_knots_and_nonfinite_values_refuse() {
    let (original, _) = candidate();
    for knots in [
        vec![],
        vec![0.0; 6],
        vec![0.0, 0.0, 0.5, 1.0, 1.0, 1.0],
        vec![0.0, 0.0, 0.0, -1.0, -1.0, -1.0],
        vec![
            -f64::MAX,
            -f64::MAX,
            -f64::MAX,
            f64::MAX,
            f64::MAX,
            f64::MAX,
        ],
    ] {
        let mut scene = original.clone();
        scene.curves[0].knots = knots;
        assert_eq!(scene.validate(), Err(NativeError::InvalidCurve));
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut scene = original.clone();
        scene.curves[0].control_vertices[0][0] = value;
        assert_eq!(scene.validate(), Err(NativeError::NonFinite));
        let mut scene = original.clone();
        scene.curves[0].knots[0] = value;
        assert_eq!(scene.validate(), Err(NativeError::NonFinite));
    }
    let mut scene = original.clone();
    scene.curves[0].control_vertices.pop();
    assert_eq!(scene.validate(), Err(NativeError::InvalidCurve));
    let mut scene = original;
    scene.curves[0]
        .control_vertices
        .extend([[4.0; 3], [5.0; 3]]);
    scene.curves[0].knots = vec![0.0, 0.0, 0.0, 0.25, 0.75, 1.0, 1.0, 1.0];
    scene.validate().unwrap();
    scene.curves[0].knots[4] = 0.25;
    assert!(matches!(
        scene.validate(),
        Err(NativeError::UnsupportedView(_))
    ));
    scene.curves[0].knots[4] = 0.1;
    assert_eq!(scene.validate(), Err(NativeError::InvalidCurve));
}

#[test]
fn curve_ids_parents_and_source_owner_ranges_are_checked() {
    let (original, _) = candidate();
    for id in [
        NativeId(0),
        original.nodes[0].transform_id,
        NativeId(original.next_native_id),
    ] {
        let mut scene = original.clone();
        scene.curves[0].id = id;
        assert_eq!(scene.validate(), Err(NativeError::InvalidIdentity));
    }
    let mut scene = original.clone();
    scene.curves.push(scene.curves[0].clone());
    assert_eq!(scene.validate(), Err(NativeError::InvalidIdentity));
    for edge in [
        DagEdge {
            parent: None,
            child: original.curves[0].id,
        },
        DagEdge {
            parent: Some(original.curves[0].id),
            child: original.nodes[0].transform_id,
        },
        DagEdge {
            parent: Some(NativeId(999)),
            child: original.curves[0].id,
        },
        DagEdge {
            parent: Some(original.nodes[0].transform_id),
            child: original.nodes[0].transform_id,
        },
    ] {
        let mut scene = original.clone();
        scene.dag.push(edge);
        assert_eq!(scene.validate(), Err(NativeError::InvalidIdentity));
    }
    let mut scene = original.clone();
    scene.dag.retain(|e| e.child != scene.curves[0].id);
    assert_eq!(scene.validate(), Err(NativeError::InvalidIdentity));
    for range in [0..0, 1..4, 0..usize::MAX, Range { start: 5, end: 4 }] {
        let mut scene = original.clone();
        scene.curves[0].source.statement = range;
        assert_eq!(scene.validate(), Err(NativeError::InvalidRetention));
    }
    let mut scene = original.clone();
    scene.curves[0].source.owner = scene.nodes[0].transform_id;
    assert_eq!(scene.validate(), Err(NativeError::InvalidRetention));
    let mut scene = original.clone();
    scene.source = None;
    assert_eq!(scene.validate(), Err(NativeError::InvalidRetention));
    let mut scene = original;
    scene.source.as_mut().unwrap().format = SourceFormat::Binary;
    assert_eq!(scene.validate(), Err(NativeError::InvalidRetention));
}

#[test]
fn all_ascii_authoring_refuses_and_preserves_exact_curve_source() {
    let (mut scene, node) = candidate();
    let before = scene.clone();
    let id = scene.curves[0].id;
    assert_eq!(
        scene.create(Primitive::Cube, "new"),
        Err(NativeError::ReadOnlyFormat)
    );
    assert_eq!(scene.rename(node, "new"), Err(NativeError::ReadOnlyFormat));
    assert_eq!(scene.remove(node), Err(NativeError::ReadOnlyFormat));
    assert_eq!(
        scene.set_channel(node, Channel::TranslateX, 2.0),
        Err(NativeError::ReadOnlyFormat)
    );
    assert_eq!(
        scene.set_transform(node, NativeTransform::default()),
        Err(NativeError::ReadOnlyFormat)
    );
    assert_eq!(
        scene.edit_node(node, None, [None; 9]),
        Err(NativeError::ReadOnlyFormat)
    );
    assert_eq!(
        scene.extrude_face(id, 0, 1.0),
        Err(NativeError::ReadOnlyFormat)
    );
    assert_eq!(scene, before);
    assert_eq!(
        (scene.next_id, scene.next_native_id),
        (before.next_id, before.next_native_id)
    );
    assert!(Arc::ptr_eq(
        &scene.source.as_ref().unwrap().bytes,
        &before.source.as_ref().unwrap().bytes
    ));
    let mut history = crate::history::History::new(scene);
    assert_eq!(
        history.execute(crate::history::Command::SetTransform {
            id: node,
            transform: NativeTransform::default()
        }),
        Err(NativeError::ReadOnlyFormat)
    );
    assert!(!history.can_undo());
    assert!(!history.can_redo());
    assert_eq!(history.scene(), &before);
}

#[test]
fn signed_zero_curve_bits_and_retention_survive_authoring_restore() {
    let (mut scene, _) = candidate();
    let before = scene.clone();
    scene.curves[0].control_vertices[0][1] = 0.0;
    assert_ne!(scene, before);
    scene.restore_authoring(before.clone());
    assert_eq!(scene, before);
    scene.curves[0].knots[0] = -0.0;
    scene.validate().unwrap();
    assert_ne!(scene, before);
    scene.next_native_id = 100;
    scene.restore_authoring(before.clone());
    assert_eq!(scene, before);
    assert_eq!(scene.next_native_id, 100);
    assert!(Arc::ptr_eq(
        &scene.source.as_ref().unwrap().bytes,
        &before.source.as_ref().unwrap().bytes
    ));
}
