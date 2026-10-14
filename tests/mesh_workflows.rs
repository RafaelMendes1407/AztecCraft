use azteccraft::scene::{Primitive, Scene};
use serde_json::{Value, json};

fn scene_value(mesh: Value) -> Value {
    let mut scene = Scene::default();
    scene.create(Primitive::Plane, "editable").unwrap();
    let mut value = serde_json::to_value(scene).unwrap();
    value["nodes"][0]["mesh"] = mesh;
    value
}

#[test]
fn changed_quad_geometry_and_authoring_states_survive_scene_validation() {
    let points = json!([[0, 0, 0], [2, 0, 0], [2, 1, 0], [0, 1, 0], [99, 99, 99]]);
    for mesh in [
        json!({"vertices": points, "faces": [[0, 1, 2, 3]]}),
        json!({"vertices": points, "faces": [[3, 2, 1, 0]]}),
        json!({"vertices": [[0,0,0], [0,0,0], [0,0,0], [0,0,0]], "faces": [[0,1,2,3]]}),
        json!({"vertices": [[0,0,0], [2,0,0], [2,1,1], [0,1,0]], "faces": [[0,1,2,3]]}),
        json!({"vertices": points, "faces": [[0,1,2,3], [0,1,2,3], [0,1,2,3], [3,2,1,0]]}),
        json!({"vertices": points, "faces": [[0,2,1,3]]}),
        json!({"vertices": points, "faces": []}),
        json!({"vertices": [], "faces": []}),
    ] {
        let value = scene_value(mesh);
        let expected: azteccraft::mesh::Mesh =
            serde_json::from_value(value["nodes"][0]["mesh"].clone()).unwrap();
        let scene: Scene = serde_json::from_value(value.clone()).unwrap();
        scene.validate().unwrap();
        assert_eq!(scene.nodes()[0].mesh(), &expected);
    }
}

#[test]
fn malformed_quad_structure_is_rejected_at_scene_boundary() {
    let vertices = json!([[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]]);
    for faces in [
        json!([[0, 1, 2, 4]]),
        json!([[0, 1, 1, 3]]),
        json!([[0, 1, 2]]),
        json!([[0, 1, 2, 3, 0]]),
        json!([[-1, 1, 2, 3]]),
    ] {
        assert!(
            serde_json::from_value::<Scene>(scene_value(
                json!({"vertices": vertices, "faces": faces})
            ))
            .is_err()
        );
    }
    for vertices in [
        json!([[null, 0, 0]]),
        json!([[1e100, 0, 0]]),
        json!([[0, 0]]),
        json!([[0, 0, 0, 0]]),
    ] {
        assert!(
            serde_json::from_value::<Scene>(scene_value(
                json!({"vertices": vertices, "faces": []})
            ))
            .is_err()
        );
    }
    assert!(
        serde_json::from_value::<Scene>(scene_value(json!({"vertices": [], "faces": [[0,1,2,3]]})))
            .is_err()
    );
    let mut overflow = scene_value(json!({"vertices": [[f32::MAX,0,0]], "faces": []}));
    overflow["nodes"][0]["transform"]["scale"] = json!([2, 1, 1]);
    assert!(serde_json::from_value::<Scene>(overflow).is_err());
}

#[test]
fn primitive_mesh_arrays_and_public_inspector_path_remain_unchanged() {
    let mut scene = Scene::default();
    let cube = scene.create(Primitive::Cube, "cube").unwrap();
    let plane = scene.create(Primitive::Plane, "plane").unwrap();
    let mesh: &azteccraft::scene::Mesh = scene.node(cube).unwrap().mesh();
    assert_eq!(
        serde_json::to_value(mesh).unwrap(),
        json!({
            "vertices": [[-0.5,-0.5,-0.5],[0.5,-0.5,-0.5],[0.5,0.5,-0.5],[-0.5,0.5,-0.5],[-0.5,-0.5,0.5],[0.5,-0.5,0.5],[0.5,0.5,0.5],[-0.5,0.5,0.5]],
            "faces": [[0,3,2,1],[4,5,6,7],[0,4,7,3],[1,2,6,5],[0,1,5,4],[3,7,6,2]]
        })
    );
    assert_eq!(
        serde_json::to_value(scene.node(plane).unwrap().mesh()).unwrap(),
        json!({
            "vertices": [[-0.5,0.0,-0.5],[-0.5,0.0,0.5],[0.5,0.0,0.5],[0.5,0.0,-0.5]], "faces": [[0,1,2,3]]
        })
    );
}
