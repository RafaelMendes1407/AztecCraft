use azteccraft::{
    obj::{ObjError, export_new},
    scene::{Primitive, Scene, Transform},
};
use glam::Vec3;
use std::{fs, io};

#[derive(Default)]
struct Parsed {
    names: Vec<String>,
    vertices: Vec<Vec3>,
    faces: Vec<Vec<usize>>,
}

fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let mut fields = line.split_whitespace();
        match fields.next().unwrap() {
            "o" => {
                parsed.names.push(fields.next().unwrap().to_string());
                assert!(fields.next().is_none());
            }
            "v" => {
                let xyz: Vec<f32> = fields.map(|s| s.parse().unwrap()).collect();
                assert_eq!(xyz.len(), 3);
                assert!(xyz.iter().all(|n| n.is_finite()));
                parsed.vertices.push(Vec3::new(xyz[0], xyz[1], xyz[2]));
            }
            "f" => {
                let indices: Vec<usize> = fields.map(|s| s.parse().unwrap()).collect();
                assert_eq!(indices.len(), 4);
                assert!(
                    indices
                        .iter()
                        .all(|i| *i > 0 && *i <= parsed.vertices.len())
                );
                parsed.faces.push(indices);
            }
            other => panic!("unexpected directive {other}"),
        }
    }
    parsed
}

#[test]
fn multi_mesh_export_reconstructs_world_geometry_with_global_indices() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.obj");
    let mut scene = Scene::default();
    let cube = scene.create(Primitive::Cube, "cube").unwrap();
    let plane = scene.create(Primitive::Plane, "plane").unwrap();
    scene
        .set_transform(
            cube,
            Transform {
                translation: Vec3::new(4.0, 5.0, 6.0),
                rotation_degrees: Vec3::new(0.0, 0.0, 90.0),
                scale: Vec3::new(-2.0, 3.0, 0.5),
            },
        )
        .unwrap();
    scene
        .set_transform(
            plane,
            Transform {
                translation: Vec3::new(0.12345679, -3.0, 7.0),
                scale: Vec3::new(0.25, 1.0, 2.0),
                ..Transform::default()
            },
        )
        .unwrap();
    let before = scene.clone();
    export_new(&path, &scene).unwrap();
    assert_eq!(scene, before);
    let output = parse(&fs::read_to_string(&path).unwrap());
    assert_eq!(output.names, ["cube", "plane"]);
    assert_eq!(output.vertices.len(), 12);
    assert_eq!(output.faces.len(), 7);
    for (local, actual) in scene
        .node(cube)
        .unwrap()
        .mesh()
        .vertices()
        .iter()
        .zip(&output.vertices[..8])
    {
        let expected = Vec3::new(
            4.0 - 3.0 * local.y,
            5.0 - 2.0 * local.x,
            6.0 + 0.5 * local.z,
        );
        assert!(actual.distance(expected) < 0.000001);
    }
    assert_eq!(
        &output.vertices[8..],
        scene.node(plane).unwrap().world_vertices()
    );
    assert_eq!(output.faces[6], [9, 10, 11, 12]);
    for (face, exported) in scene
        .node(cube)
        .unwrap()
        .mesh()
        .faces()
        .iter()
        .zip(&output.faces[..6])
    {
        let reconstructed: Vec<Vec3> = exported
            .iter()
            .map(|index| output.vertices[index - 1])
            .collect();
        let expected: Vec<Vec3> = face
            .iter()
            .rev()
            .map(|index| output.vertices[*index])
            .collect();
        assert_eq!(reconstructed, expected);
    }
}

#[test]
fn object_names_are_distinct_single_tokens_with_no_directive_injection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("names.obj");
    let mut scene = Scene::default();
    for name in [
        "two words # f 1 2 3",
        "two%20words",
        "café\\",
        "plain-_.123",
    ] {
        scene.create(Primitive::Plane, name).unwrap();
    }
    assert!(scene.create(Primitive::Cube, "bad\nf 1 2 3 4").is_err());
    export_new(&path, &scene).unwrap();
    let parsed = parse(&fs::read_to_string(path).unwrap());
    assert_eq!(
        parsed.names,
        [
            "two%20words%20%23%20f%201%202%203",
            "two%2520words",
            "caf%C3%A9%5C",
            "plain-_.123"
        ]
    );
    assert_eq!(parsed.faces.len(), 4);
}

#[test]
fn empty_scene_exports_valid_empty_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.obj");
    export_new(&path, &Scene::default()).unwrap();
    let output = parse(&fs::read_to_string(path).unwrap());
    assert!(output.vertices.is_empty() && output.faces.is_empty() && output.names.is_empty());
}

#[test]
fn occupied_file_and_failed_creation_preserve_data_and_leave_no_staging_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("occupied.obj");
    fs::write(&path, b"original bytes").unwrap();
    let mut scene = Scene::default();
    scene.create(Primitive::Cube, "cube").unwrap();
    let before = scene.clone();
    assert!(
        matches!(export_new(&path, &scene), Err(ObjError::Io { source, .. }) if source.kind() == io::ErrorKind::AlreadyExists)
    );
    assert_eq!(fs::read(&path).unwrap(), b"original bytes");
    assert!(export_new(&dir.path().join("absent/scene.obj"), &scene).is_err());
    assert!(export_new(dir.path(), &scene).is_err());
    assert_eq!(scene, before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn dangling_symlink_is_not_replaced_or_followed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.obj");
    let target = dir.path().join("missing.obj");
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(export_new(&path, &Scene::default()).is_err());
    assert_eq!(fs::read_link(&path).unwrap(), target);
    assert!(!target.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn transformed_cubes_keep_outward_facing_with_odd_and_even_reflections() {
    let dir = tempfile::tempdir().unwrap();
    let center = Vec3::new(4.0, 5.0, 6.0);
    for (index, scale) in [Vec3::new(-2.0, 3.0, 0.5), Vec3::new(-2.0, -3.0, 0.5)]
        .into_iter()
        .enumerate()
    {
        let mut scene = Scene::default();
        let cube = scene.create(Primitive::Cube, "cube").unwrap();
        scene
            .set_transform(
                cube,
                Transform {
                    translation: center,
                    rotation_degrees: Vec3::new(25.0, 50.0, 90.0),
                    scale,
                },
            )
            .unwrap();
        let before = scene.clone();
        let path = dir.path().join(format!("reflection-{index}.obj"));
        export_new(&path, &scene).unwrap();
        let parsed = parse(&fs::read_to_string(path).unwrap());
        for face in parsed.faces {
            let points: Vec<Vec3> = face.iter().map(|i| parsed.vertices[i - 1]).collect();
            let normal = (points[1] - points[0]).cross(points[2] - points[0]);
            let face_center = points.iter().copied().sum::<Vec3>() / 4.0;
            assert!(
                normal.dot(face_center - center) > 0.0,
                "inward face for {scale:?}"
            );
        }
        assert_eq!(scene, before);
    }
}

#[test]
fn zero_scale_exports_finite_degenerate_geometry_with_original_topology() {
    let dir = tempfile::tempdir().unwrap();
    let mut scene = Scene::default();
    let cube = scene.create(Primitive::Cube, "collapsed").unwrap();
    scene
        .set_transform(
            cube,
            Transform {
                scale: Vec3::new(-2.0, 0.0, 0.5),
                ..Transform::default()
            },
        )
        .unwrap();
    let path = dir.path().join("collapsed.obj");
    export_new(&path, &scene).unwrap();
    let parsed = parse(&fs::read_to_string(path).unwrap());
    assert_eq!(parsed.vertices.len(), 8);
    for (original, exported) in scene
        .node(cube)
        .unwrap()
        .mesh()
        .faces()
        .iter()
        .zip(parsed.faces)
    {
        assert_eq!(exported, original.map(|i| i + 1));
    }
}
