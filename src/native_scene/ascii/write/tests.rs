use super::*;
use crate::{
    native_scene::{NativeId, NativeMesh, test_fixtures as fixtures},
    scene::Primitive,
};
use std::sync::Arc;
#[path = "../../../../tests/support/maya_ascii.rs"]
mod ascii_fixture;
mod binary_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_binary.rs"
    ));
}

fn scene(names: &[&str]) -> NativeScene {
    let mut scene = NativeScene::default();
    for name in names {
        scene.create(Primitive::Cube, name).unwrap();
    }
    scene
}
fn prefix(scene: &NativeScene) -> Result<Vec<u8>, AsciiWriteRefusal> {
    let original = scene.clone();
    let result = emit_transform_declarations_prefix(scene);
    assert_eq!(scene, &original);
    assert_eq!(
        (scene.next_id, scene.next_native_id),
        (original.next_id, original.next_native_id)
    );
    result
}

#[test]
fn preflight_refusals_preserve_the_scene_and_canonical_causes() {
    use AsciiWriteRefusal as R;
    type Case = (fn(&mut NativeScene), R);
    let cases: [Case; 11] = [
        (|s| s.nodes[0].name = "3bad".into(), R::InvalidName),
        (|s| s.shapes[0].name = "bad name".into(), R::InvalidName),
        (|s| s.nodes[0].name = "persp".into(), R::ReservedName),
        (|s| s.nodes[0].name = "time1".into(), R::ReservedName),
        (
            |s| s.shapes[0].name = s.shapes[1].name.clone(),
            R::NameCollision,
        ),
        (|s| s.units.time = "pal".into(), R::UnsupportedUnits),
        (
            |s| s.nodes[0].channels[0].value = f64::NAN,
            R::InvalidScene(NativeError::NonFinite),
        ),
        (
            |s| s.nodes[0].transform_id = NativeId(0),
            R::InvalidScene(NativeError::InvalidIdentity),
        ),
        (
            |s| s.dag[0].child = NativeId(0),
            R::InvalidScene(NativeError::InvalidIdentity),
        ),
        (
            |s| fixtures::add_loose_edge_unvalidated(s, [0, usize::MAX]),
            R::InvalidScene(NativeError::InvalidTopology),
        ),
        (
            |s| s.shapes[0].mesh = NativeMesh::polygon_fixture(vec![], &[]),
            R::EmptyMesh,
        ),
    ];
    for (mutate, expected) in cases {
        let mut scene = scene(&["cube", "neighbor"]);
        scene.validate().unwrap();
        prefix(&scene).unwrap();
        mutate(&mut scene);
        if expected == R::EmptyMesh {
            scene.validate().unwrap();
        }
        let error = prefix(&scene).unwrap_err();
        if let R::InvalidScene(cause) = &expected {
            assert_eq!(
                error.source().unwrap().downcast_ref::<NativeError>(),
                Some(cause)
            );
        } else {
            assert!(error.source().is_none());
        }
        assert_eq!(error, expected);
    }
    assert_eq!(
        prefix(&scene(&["cube", "cubeShape"])),
        Err(R::NameCollision)
    );
    for scene in [fixtures::shared_shape().0, fixtures::grouped_shape().0] {
        scene.validate().unwrap();
        assert_eq!(prefix(&scene), Err(R::NonAuthoredHierarchy));
    }
    let mut no_faces = scene(&["cube"]);
    no_faces.shapes[0].mesh = NativeMesh::polygon_fixture(vec![[0.0; 3]], &[]);
    no_faces.validate().unwrap();
    assert_eq!(prefix(&no_faces), Err(R::EmptyMesh));
}

#[test]
fn sourced_formats_and_source_free_curves_refuse_without_touching_retention() {
    let ascii = super::super::decode(Arc::from(ascii_fixture::specimen())).unwrap();
    let binary =
        crate::native_scene::binary::decode(Arc::from(binary_fixture::specimen())).unwrap();
    for scene in [ascii.clone(), binary] {
        let source = scene.source.as_ref().unwrap().bytes.clone();
        assert_eq!(prefix(&scene), Err(AsciiWriteRefusal::SourcedScene));
        assert!(Arc::ptr_eq(&source, &scene.source.as_ref().unwrap().bytes));
    }
    let mut curve = ascii;
    curve.source = None;
    assert_eq!(prefix(&curve), Err(AsciiWriteRefusal::Curves));
}

#[test]
fn declarations_have_literal_order_and_exact_size_without_claiming_value_output() {
    let mut scene = scene(&["second", "first"]);
    scene.nodes[0].channels[0].value = -0.0;
    scene.nodes[0].channels[3].value = 90.0;
    scene.nodes[0].channels[6].value = -2.0;
    scene.nodes[0].rotate_pivot[0].value = 1.0;
    scene.nodes[0].restrictions.push("locked".into());
    scene.nodes[0]
        .unsupported_transform
        .push("unemitted".into());
    let golden = b"//Maya ASCII 2026 scene\nrequires maya \"2026\";\ncurrentUnit -l centimeter -a degree -t film;\nfileInfo \"AztecCraft\" \"1\";\ncreateNode transform -n \"second\";\ncreateNode transform -n \"first\";\n";
    let actual = prefix(&scene).unwrap();
    assert_eq!(actual, golden);
    assert_eq!(
        actual.len(),
        declaration_bytes(2, "secondfirst".len()).unwrap()
    );
    assert_eq!(prefix(&NativeScene::default()).unwrap(), HEADER.as_bytes());
    let mut legacy = crate::scene::Scene::default();
    legacy.create(Primitive::Cube, "legacy").unwrap();
    prefix(&NativeScene::from_legacy(legacy).unwrap()).unwrap();
}

#[test]
fn hole_container_and_checked_resource_boundaries_precede_canonical_work() {
    use crate::maya_ascii::ErrorKind;
    let maximum_holes = 50_000_000 / 32;
    charge_face_counts(&mut Work::new(), 0, maximum_holes).unwrap();
    for (outer, holes) in [(0, maximum_holes + 1), (0, usize::MAX), (usize::MAX, 1)] {
        assert_eq!(
            charge_face_counts(&mut Work::new(), outer, holes)
                .unwrap_err()
                .kind(),
            ErrorKind::ResourceLimit
        );
    }
    let mut invalid_hole = scene(&["cube"]);
    invalid_hole.shapes[0].mesh.add_hole_fixture(&[]);
    assert_eq!(
        prefix(&invalid_hole),
        Err(AsciiWriteRefusal::InvalidScene(
            NativeError::InvalidTopology
        ))
    );
    let mut over_work = NativeScene::default();
    for index in 0..100 {
        over_work
            .create(Primitive::Cube, &format!("{}{index}", "a".repeat(2500)))
            .unwrap();
    }
    over_work.validate().unwrap();
    crate::native_scene::FULL_VALIDATIONS.with(|count| count.set(0));
    assert_eq!(prefix(&over_work), Err(AsciiWriteRefusal::ResourceLimit));
    crate::native_scene::FULL_VALIDATIONS.with(|count| assert_eq!(count.get(), 0));
}

#[test]
fn declaration_size_checks_exact_cap_and_overflow_without_an_output_buffer() {
    let fixed = declaration_bytes(1, 0).unwrap();
    assert_eq!(declaration_bytes(1, MAX_BYTES - fixed), Ok(MAX_BYTES));
    for counts in [(1, MAX_BYTES - fixed + 1), (usize::MAX, 0), (0, usize::MAX)] {
        assert_eq!(
            declaration_bytes(counts.0, counts.1),
            Err(AsciiWriteRefusal::TooLarge)
        );
    }
}
