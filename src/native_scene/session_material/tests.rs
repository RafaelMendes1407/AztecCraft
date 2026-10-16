use super::*;
use crate::native_scene::test_fixtures::cube;
use crate::shading::SURFACE_BASE_RGB;

fn assigned_cube() -> (NativeScene, NativeId) {
    let (mut scene, _) = cube();
    let shape = scene.shapes[0].id;
    scene.session_materials.assignments.insert(shape, 0);
    scene.validate().unwrap();
    (scene, shape)
}

#[test]
fn authored_shapes_start_in_the_initial_group_with_the_viewport_base_fill() {
    let (scene, _) = cube();
    let shape = scene.shapes[0].id;
    assert_eq!(
        scene.session_shading_groups()[0].name(),
        "initialShadingGroup"
    );
    assert_eq!(
        scene.session_material(shape).unwrap().color(),
        SURFACE_BASE_RGB.map(|channel| f64::from(channel) / 255.0)
    );
}

#[test]
fn validate_refuses_each_malformed_table() {
    type Break = fn(&mut NativeScene, NativeId);
    let cases: [(&str, Break, NativeError); 8] = [
        (
            "dangling material",
            |s, _| s.session_materials.groups[0].material = SessionMaterialId(9),
            NativeError::InvalidSessionMaterials,
        ),
        (
            "dangling group",
            |s, shape| {
                s.session_materials.assignments.insert(shape, 9);
            },
            NativeError::InvalidSessionMaterials,
        ),
        (
            "unknown shape",
            |s, _| {
                s.session_materials.assignments.insert(NativeId(9999), 0);
            },
            NativeError::InvalidSessionMaterials,
        ),
        (
            "no materials",
            |s, _| s.session_materials.materials.clear(),
            NativeError::InvalidSessionMaterials,
        ),
        (
            "no groups",
            |s, _| s.session_materials.groups.clear(),
            NativeError::InvalidSessionMaterials,
        ),
        (
            "non-finite",
            |s, _| s.session_materials.materials[0].color[1] = f64::NAN,
            NativeError::NonFinite,
        ),
        (
            "above range",
            |s, _| s.session_materials.materials[0].color[0] = 1.5,
            NativeError::SessionColorOutOfRange,
        ),
        (
            "below range",
            |s, _| s.session_materials.materials[0].color[2] = -0.1,
            NativeError::SessionColorOutOfRange,
        ),
    ];
    for (label, apply, expected) in cases {
        let (mut scene, shape) = assigned_cube();
        apply(&mut scene, shape);
        assert_eq!(scene.validate(), Err(expected), "{label}");
    }
}

#[test]
fn validate_refuses_an_over_cap_or_misaligned_table() {
    let (mut scene, _) = cube();
    let lambert = scene.session_materials.materials[0].clone();
    let group = scene.session_materials.groups[0].clone();
    for index in 1..=MAX_SESSION_MATERIALS {
        scene.session_materials.materials.push(lambert.clone());
        let mut group = group.clone();
        group.material = SessionMaterialId(index);
        scene.session_materials.groups.push(group);
    }
    assert_eq!(scene.validate(), Err(NativeError::InvalidSessionMaterials));
    let (mut scene, _) = cube();
    scene.session_materials.materials.push(lambert);
    assert_eq!(scene.validate(), Err(NativeError::InvalidSessionMaterials));
    scene.session_materials.groups.push(group.clone());
    assert_eq!(scene.validate(), Err(NativeError::InvalidSessionMaterials));
    scene.session_materials.groups[1].material = SessionMaterialId(1);
    scene.validate().unwrap();
    scene.session_materials.groups[0].material = SessionMaterialId(1);
    assert_eq!(scene.validate(), Err(NativeError::InvalidSessionMaterials));
}

#[test]
fn the_table_participates_in_equality() {
    let (scene, shape) = assigned_cube();
    let mut other = scene.clone();
    assert_eq!(scene, other);
    other.session_materials.materials[0].color[0] = 0.25;
    assert_ne!(scene, other);
    other = scene.clone();
    other.session_materials.assignments.remove(&shape);
    assert_ne!(scene, other);
}

#[test]
fn removing_an_object_prunes_its_shape_assignment() {
    let (mut scene, shape) = assigned_cube();
    let node = scene.nodes()[0].id();
    scene.remove(node).unwrap();
    assert!(!scene.session_materials.assignments.contains_key(&shape));
    scene.validate().unwrap();
}
