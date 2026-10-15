use azteccraft::scene::{NodeId, Primitive, Scene};

pub fn extruded_corner() -> (Scene, NodeId) {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Cube, "extruded_cube").unwrap();
    for face_index in [1, 7] {
        scene.extrude_face(id, face_index, 0.5).unwrap();
    }
    (scene, id)
}
