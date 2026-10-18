use super::*;
use crate::{
    native_scene::{AngularUnit, LinearUnit, NativeId, NativeMesh, test_fixtures as fixtures},
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
    let result = emit_mesh_declarations_prefix(scene);
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
    let cases: [Case; 34] = [
        (
            |s| s.nodes[0].rotate_pivot[0].value = 1.0,
            R::NonDefaultPivot,
        ),
        (
            |s| s.nodes[0].rotate_pivot[0].value = -0.0,
            R::NonDefaultPivot,
        ),
        (
            |s| s.nodes[0].scale_pivot[0].value = 1.0,
            R::NonDefaultPivot,
        ),
        (
            |s| s.nodes[0].scale_pivot[0].value = -0.0,
            R::NonDefaultPivot,
        ),
        (
            |s| s.nodes[0].extras.rotate_order = Some(0),
            R::TransformExtras,
        ),
        (
            |s| s.nodes[0].extras.rotate_axis = Some([0.0; 3]),
            R::TransformExtras,
        ),
        (
            |s| s.nodes[0].extras.shear = Some([0.0; 3]),
            R::TransformExtras,
        ),
        (
            |s| s.nodes[0].extras.rotate_pivot_translate = Some([0.0; 3]),
            R::TransformExtras,
        ),
        (
            |s| s.nodes[0].extras.scale_pivot_translate = Some([0.0; 3]),
            R::TransformExtras,
        ),
        (
            |s| s.nodes[0].restrictions.push("locked".into()),
            R::TransformRestriction,
        ),
        (
            |s| s.nodes[0].unsupported_transform.push("effect".into()),
            R::UnsupportedTransformEffect,
        ),
        (
            |s| s.nodes[0].view.local_visibility = Some(false),
            R::TransformVisibility,
        ),
        (
            |s| s.nodes[0].view.local_visibility = None,
            R::TransformVisibility,
        ),
        (|s| s.nodes[0].view.geometry = None, R::TransformVisibility),
        (
            |s| s.shapes[0].restrictions.push("locked".into()),
            R::ShapeRestriction,
        ),
        (
            |s| s.shapes[0].view.local_visibility = Some(false),
            R::ShapeVisibility,
        ),
        (
            |s| s.shapes[0].view.local_visibility = None,
            R::ShapeVisibility,
        ),
        (|s| s.shapes[0].view.geometry = None, R::ShapeVisibility),
        (
            |s| s.shapes[0].mesh = NativeMesh::triangle_double_fixture(),
            R::Double3Points,
        ),
        (
            |s| s.shapes[0].mesh.add_hole_fixture(&[4, 5, 6]),
            R::FaceHoles,
        ),
        (|s| s.nodes[0].name = "3bad".into(), R::InvalidName),
        (|s| s.shapes[0].name = "bad name".into(), R::InvalidName),
        (|s| s.nodes[0].name = "persp".into(), R::ReservedName),
        (|s| s.nodes[0].name = "time1".into(), R::ReservedName),
        (
            |s| s.shapes[0].name = s.shapes[1].name.clone(),
            R::NameCollision,
        ),
        (|s| s.units.time = "pal".into(), R::UnsupportedUnits),
        (|s| s.units.linear = LinearUnit::Meter, R::UnsupportedUnits),
        (|s| s.units.linear = LinearUnit::Inch, R::UnsupportedUnits),
        (
            |s| s.units.angular = AngularUnit::Radian,
            R::UnsupportedUnits,
        ),
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
        if !matches!(expected, R::InvalidScene(_)) {
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
fn transform_prefix_keeps_uuid_values_and_relative_statements_with_their_owner() {
    let mut scene = scene(&["second", "first"]);
    scene.nodes[0].transform_uuid = Some(uuid::Uuid::from_u128(0x6c1b2d4e0a3f4b5c8d7e9f0a1b2c3d4e));
    scene.nodes[1].transform_uuid = Some(uuid::Uuid::from_u128(0x11111111222233334444555555555555));
    scene.shapes[0].name = "storedSecond".into();
    scene.shapes[1].name = "storedFirst".into();
    scene.shapes[0].uuid = Some(uuid::Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa));
    scene.shapes[1].uuid = Some(uuid::Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb));
    scene.shapes.swap(0, 1);
    scene.nodes[0].channels[0].value = -0.0;
    scene.nodes[0].channels[3].value = 90.0;
    scene.nodes[0].channels[6].value = -2.0;
    let values = [
        2.54,
        1e-9,
        1e15,
        0.1,
        1.0 / 3.0,
        std::f64::consts::PI,
        1.5,
        -2.25,
        3.0,
    ];
    for (channel, value) in scene.nodes[1].channels.iter_mut().zip(values) {
        channel.value = value;
    }
    let golden = b"//Maya ASCII 2026 scene\n\
requires maya \"2026\";\n\
currentUnit -l centimeter -a degree -t film;\n\
fileInfo \"AztecCraft\" \"1\";\n\
createNode transform -n \"second\";\n\
\trename -uid \"6c1b2d4e-0a3f-4b5c-8d7e-9f0a1b2c3d4e\";\n\
\tsetAttr \".t\" -type \"double3\" -0 0 0;\n\
\tsetAttr \".r\" -type \"double3\" 90 0 0;\n\
\tsetAttr \".s\" -type \"double3\" -2 1 1;\n\
createNode mesh -n \"storedSecond\" -p \"second\";\n\
\trename -uid \"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\";\n\
createNode transform -n \"first\";\n\
\trename -uid \"11111111-2222-3333-4444-555555555555\";\n\
\tsetAttr \".t\" -type \"double3\" 2.54 0.000000001 1000000000000000;\n\
\tsetAttr \".r\" -type \"double3\" 0.1 0.3333333333333333 3.141592653589793;\n\
\tsetAttr \".s\" -type \"double3\" 1.5 -2.25 3;\n\
createNode mesh -n \"storedFirst\" -p \"first\";\n\
\trename -uid \"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb\";\n";
    let actual = prefix(&scene).unwrap();
    assert_eq!(actual, golden);
    assert_eq!(actual.len(), prefix_bytes(&scene).unwrap());
    let text = std::str::from_utf8(&actual).unwrap();
    let uuids: Vec<_> = text
        .lines()
        .filter_map(|line| line.strip_prefix("\trename -uid \""))
        .map(|tail| uuid::Uuid::parse_str(tail.strip_suffix("\";").unwrap()).unwrap())
        .collect();
    assert_eq!(
        uuids,
        [
            scene.nodes[0].transform_uuid(),
            scene.shapes[1].uuid(),
            scene.nodes[1].transform_uuid(),
            scene.shapes[0].uuid()
        ]
        .map(Option::unwrap)
    );
    assert_eq!(
        scene
            .internal_transform(scene.nodes[0].id())
            .unwrap()
            .rotation_radians[0]
            .to_bits(),
        std::f64::consts::FRAC_PI_2.to_bits()
    );
    scene.nodes[1].transform_uuid = None;
    assert_eq!(prefix(&scene).unwrap().len(), golden.len() - 53);
    scene.shapes[0].uuid = None;
    let omitted = prefix(&scene).unwrap();
    assert_eq!(
        std::str::from_utf8(&omitted)
            .unwrap()
            .matches("rename -uid")
            .count(),
        2
    );
    assert_eq!(rows(&omitted).len(), 6);
    assert_eq!(prefix(&NativeScene::default()).unwrap(), HEADER.as_bytes());
    let mut legacy = crate::scene::Scene::default();
    legacy.create(Primitive::Cube, "legacy").unwrap();
    prefix(&NativeScene::from_legacy(legacy).unwrap()).unwrap();
    for primitive in [
        Primitive::Cube,
        Primitive::Plane,
        Primitive::Sphere,
        Primitive::Cylinder,
        Primitive::Cone,
    ] {
        let mut scene = NativeScene::default();
        scene.create(primitive, "primitive").unwrap();
        scene.validate().unwrap();
        assert_eq!(prefix(&scene).unwrap().len(), prefix_bytes(&scene).unwrap());
    }
}

#[test]
fn child_work_counts_are_checked_before_traversal() {
    use crate::maya_ascii::ErrorKind;
    assert_eq!(mesh_declaration_work(0, 0, 0).unwrap(), 0);
    assert_eq!(mesh_declaration_work(100, 100, 200).unwrap(), 76_400);
    for (nodes, shapes, dag) in [(usize::MAX, 0, 0), (1, usize::MAX, 1), (1, 0, usize::MAX)] {
        assert_eq!(
            mesh_declaration_work(nodes, shapes, dag)
                .unwrap_err()
                .kind(),
            ErrorKind::ResourceLimit
        );
    }
}

fn rows(bytes: &[u8]) -> Vec<(&str, &str, [f64; 3])> {
    let mut owner = None;
    let mut rows = Vec::new();
    for line in std::str::from_utf8(bytes).unwrap().lines() {
        if let Some(name) = line.strip_prefix("createNode transform -n \"") {
            owner = Some(name.strip_suffix("\";").unwrap());
        }
        if let Some(row) = line.strip_prefix("\tsetAttr \"") {
            let (attribute, values) = row.split_once("\" -type \"double3\" ").unwrap();
            assert!([".t", ".r", ".s"].contains(&attribute));
            let values = values
                .strip_suffix(';')
                .unwrap()
                .split(' ')
                .map(|value| value.parse::<f64>().unwrap())
                .collect::<Vec<_>>();
            rows.push((owner.unwrap(), attribute, values.try_into().unwrap()));
        }
    }
    rows
}

#[test]
fn every_slot_round_trips_defaults_signed_zero_and_finite_extremes() {
    let values = [
        0.0,
        -0.0,
        1.0,
        0.1,
        1.0 / 3.0,
        std::f64::consts::PI,
        f64::from_bits(1.0f64.to_bits() + 1),
        1e-9,
        1e15,
        -2.0,
        f64::MAX,
        -f64::MAX,
        f64::from_bits(f64::MAX.to_bits() - 1),
        -f64::from_bits(f64::MAX.to_bits() - 1),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(f64::MIN_POSITIVE.to_bits() + 1),
        -f64::from_bits(f64::MIN_POSITIVE.to_bits() + 1),
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::from_bits(2),
        -f64::from_bits(2),
    ];
    for slot in 0..9 {
        for value in values {
            let mut scene = scene(&["value"]);
            scene.nodes[0].transform_uuid = None;
            scene.nodes[0].channels[slot].value = value;
            let mut expected = if slot < 6 { [0.0f64; 3] } else { [1.0f64; 3] };
            let changed = value.to_bits() != expected[slot % 3].to_bits();
            expected[slot % 3] = value;
            let bytes = prefix(&scene).unwrap();
            let actual = rows(&bytes);
            assert_eq!(actual.len(), usize::from(changed));
            if changed {
                assert_eq!(
                    (actual[0].0, actual[0].1),
                    ("value", [".t", ".r", ".s"][slot / 3])
                );
                assert_eq!(actual[0].2.map(f64::to_bits), expected.map(f64::to_bits));
            }
        }
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut scene = scene(&["invalid"]);
        scene.nodes[0].channels[0].value = value;
        assert_eq!(
            prefix(&scene),
            Err(AsciiWriteRefusal::InvalidScene(NativeError::NonFinite))
        );
    }
}

#[test]
fn uuid_and_channel_bytes_are_in_the_admitted_size() {
    let mut scene = scene(&["plain"]);
    scene.nodes[0].transform_uuid = None;
    scene.shapes[0].uuid = None;
    assert_eq!(prefix(&scene).unwrap().len(), 195);
    scene.nodes[0].transform_uuid = Some(uuid::Uuid::nil());
    let bytes = prefix(&scene).unwrap();
    assert_eq!(bytes.len(), 248);
    assert_eq!(bytes.len(), prefix_bytes(&scene).unwrap());
    assert!(rows(&bytes).is_empty());
    for contribution in [53, 37, MESH_SYNTAX_BYTES] {
        assert_eq!(
            add_prefix_bytes(MAX_BYTES - contribution, contribution),
            Ok(MAX_BYTES)
        );
        assert_eq!(
            add_prefix_bytes(MAX_BYTES - contribution + 1, contribution),
            Err(AsciiWriteRefusal::TooLarge)
        );
    }
    assert_eq!(
        add_prefix_bytes(usize::MAX, 1),
        Err(AsciiWriteRefusal::TooLarge)
    );
}

#[test]
fn formatting_workspace_and_admitted_output_refuse_one_extra_byte() {
    let mut line = ChannelLine {
        bytes: [0; CHANNEL_BYTES],
        used: 0,
    };
    line.format(format_args!("{:width$}", "", width = CHANNEL_BYTES))
        .unwrap();
    assert_eq!(line.used, CHANNEL_BYTES);
    let before = line.bytes;
    assert_eq!(
        line.format(format_args!("x")),
        Err(AsciiWriteRefusal::ResourceLimit)
    );
    assert_eq!((line.used, line.bytes), (CHANNEL_BYTES, before));
    assert!(fmt::Write::write_str(&mut line, "x").is_err());
    assert_eq!((line.used, line.bytes), (CHANNEL_BYTES, before));
    let mut out = Vec::with_capacity(32);
    append(&mut out, 2, b"ab").unwrap();
    let capacity = out.capacity();
    assert_eq!(append(&mut out, 2, b"c"), Err(AsciiWriteRefusal::TooLarge));
    assert_eq!(out, b"ab");
    assert_eq!(out.capacity(), capacity);
}

#[test]
fn added_formatting_work_refuses_a_previously_admissible_small_scene() {
    let mut scene = NativeScene::default();
    for index in 0..100 {
        scene
            .create(Primitive::Cube, &format!("{}{index}", "a".repeat(1225)))
            .unwrap();
    }
    let (n, s, d) = (scene.nodes.len(), scene.shapes.len(), scene.dag.len());
    let b: usize = names(&scene).map(|(_, name)| name.len()).sum();
    assert_eq!((n, s, d, b), (100, 100, 200, 245880));
    let mesh = scene.shapes[0].mesh();
    let corners: usize = mesh.faces().iter().map(|face| face.outer().len()).sum();
    let holes: usize = mesh.faces().iter().map(|face| face.holes().len()).sum();
    assert_eq!(
        (
            mesh.point_count(),
            mesh.edges().len(),
            mesh.faces().len(),
            corners,
            holes
        ),
        (8, 12, 6, 24, 0)
    );
    let linear = n
        + s
        + d
        + n * (8 + 12 + 6 + 24)
        + scene.session_materials.assignments.len()
        + scene.session_materials.materials.len()
        + scene.session_materials.groups.len()
        + 1;
    let prior = 2 * (n + s) * d + n * (d + s) + (n + s) * b + 32 * linear;
    assert_eq!(prior, 49_458_896);
    assert_eq!((CHANNEL_BYTES, FORMATTING_WORK * n), (1063, 645000));
    assert_eq!(prior + FORMATTING_WORK * n, 50_103_896);
    scene.validate().unwrap();
    assert_eq!(prefix(&scene), Err(AsciiWriteRefusal::ResourceLimit));
    for (index, (node, shape)) in scene.nodes.iter_mut().zip(&mut scene.shapes).enumerate() {
        node.name = format!("{}{index}", "a".repeat(1209));
        shape.name = format!("{}Shape", node.name);
    }
    let shorter_names: usize = names(&scene).map(|(_, name)| name.len()).sum();
    assert_eq!(shorter_names, 242680);
    let parent = prior - (n + s) * (b - shorter_names) + FORMATTING_WORK * n;
    let lookup = 2 * n * (d + s);
    let declarations = 2 * (n * (MESH_SYNTAX_BYTES + UUID_STATEMENT_BYTES) + shorter_names);
    assert_eq!((MESH_SYNTAX_BYTES, UUID_STATEMENT_BYTES), (29, 53));
    assert_eq!(
        (parent, lookup, declarations),
        (49_463_896, 60_000, 501_760)
    );
    assert_eq!(parent + lookup + declarations, 50_025_656);
    assert!(parent + lookup < 50_000_000 && parent + declarations < 50_000_000);
    scene.validate().unwrap();
    assert_eq!(prefix(&scene), Err(AsciiWriteRefusal::ResourceLimit));
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
