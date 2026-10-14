use azteccraft::scene::{Primitive, Scene, SceneError, Transform};
use glam::Vec3;
use std::collections::HashMap;

fn near(actual: Vec3, expected: Vec3) {
    assert!(
        (actual - expected).length() < 0.00001,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn cube_is_closed_and_faces_point_outward() {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Cube, "cube").unwrap();
    let mesh: &azteccraft::scene::Mesh = scene.node(id).unwrap().mesh();
    assert_eq!(mesh.vertices().len(), 8);
    assert_eq!(mesh.faces().len(), 6);
    let mut edges = HashMap::new();
    for face in mesh.faces() {
        let [a, b, c, d] = face.map(|i| mesh.vertices()[i]);
        assert!((b - a).cross(c - a).dot((a + b + c + d) / 4.0) > 0.0);
        for i in 0..4 {
            let edge = (face[i], face[(i + 1) % 4]);
            *edges.entry(edge).or_insert(0) += 1;
        }
    }
    assert_eq!(edges.len(), 24);
    for ((a, b), count) in &edges {
        assert_eq!(*count, 1);
        assert_eq!(edges.get(&(*b, *a)), Some(&1));
    }
}

#[test]
fn plane_is_a_unit_quad_facing_positive_y() {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Plane, "ground").unwrap();
    let mesh = scene.node(id).unwrap().mesh();
    assert_eq!(mesh.vertices().len(), 4);
    assert_eq!(mesh.faces(), &[[0, 1, 2, 3]]);
    let [a, b, c, _] = mesh.faces()[0].map(|i| mesh.vertices()[i]);
    near((b - a).cross(c - a), Vec3::Y);
    assert!(mesh.vertices().iter().all(|v| v.y == 0.0));
}

#[test]
fn ids_survive_edits_and_are_never_reused_after_removal_or_failed_creation() {
    let mut scene = Scene::default();
    let a = scene.create(Primitive::Cube, "a").unwrap();
    let before = scene.clone();
    assert_eq!(
        scene.create(Primitive::Cube, "\n"),
        Err(SceneError::InvalidName)
    );
    assert_eq!(scene, before);
    let b = scene.create(Primitive::Plane, "b").unwrap();
    scene.rename(b, "renamed").unwrap();
    scene.remove(a).unwrap();
    let before = scene.clone();
    assert_eq!(scene.remove(a), Err(SceneError::MissingNode(a)));
    assert_eq!(scene, before);
    let c = scene.create(Primitive::Cube, "c").unwrap();
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
    assert_eq!(scene.node(b).unwrap().name(), "renamed");
    assert_eq!(scene.node(b).unwrap().id(), b);
}

#[test]
fn scale_then_rotate_then_translate_in_world_space() {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Cube, "cube").unwrap();
    scene
        .set_transform(
            id,
            Transform {
                translation: Vec3::new(10.0, 20.0, 30.0),
                rotation_degrees: Vec3::new(0.0, 0.0, 90.0),
                scale: Vec3::new(-2.0, 4.0, 6.0),
            },
        )
        .unwrap();
    near(
        scene.node(id).unwrap().world_vertices()[0],
        Vec3::new(12.0, 21.0, 27.0),
    );
    near(
        scene.node(id).unwrap().mesh().vertices()[0],
        Vec3::splat(-0.5),
    );
}

#[test]
fn maya_xyz_rotates_about_x_then_y_and_zero_scale_is_supported() {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Plane, "plane").unwrap();
    scene
        .set_transform(
            id,
            Transform {
                rotation_degrees: Vec3::new(90.0, 90.0, 0.0),
                ..Transform::default()
            },
        )
        .unwrap();
    near(
        scene.node(id).unwrap().world_vertices()[0],
        Vec3::new(0.0, 0.5, 0.5),
    );
    scene
        .set_transform(
            id,
            Transform {
                scale: Vec3::ZERO,
                ..Transform::default()
            },
        )
        .unwrap();
    assert!(
        scene
            .node(id)
            .unwrap()
            .world_vertices()
            .iter()
            .all(|v| *v == Vec3::ZERO)
    );
}

#[test]
fn invalid_edits_leave_the_complete_scene_unchanged() {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Cube, "cube").unwrap();
    let before = scene.clone();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for transform in [
            Transform {
                translation: Vec3::splat(value),
                ..Transform::default()
            },
            Transform {
                rotation_degrees: Vec3::splat(value),
                ..Transform::default()
            },
            Transform {
                scale: Vec3::splat(value),
                ..Transform::default()
            },
        ] {
            assert_eq!(
                scene.set_transform(id, transform),
                Err(SceneError::InvalidTransform)
            );
            assert_eq!(scene, before);
        }
    }
    let overflowing = Transform {
        scale: Vec3::splat(f32::MAX),
        translation: Vec3::splat(f32::MAX),
        ..Transform::default()
    };
    assert_eq!(
        scene.set_transform(id, overflowing),
        Err(SceneError::InvalidTransform)
    );
    assert_eq!(scene, before);
    for name in ["", "   ", "bad\nname", "bad\0name"] {
        assert_eq!(scene.rename(id, name), Err(SceneError::InvalidName));
        assert_eq!(scene, before);
    }
    scene.remove(id).unwrap();
    let empty = scene.clone();
    assert_eq!(scene.rename(id, "gone"), Err(SceneError::MissingNode(id)));
    assert_eq!(
        scene.set_transform(id, Transform::default()),
        Err(SceneError::MissingNode(id))
    );
    assert_eq!(scene, empty);
}
