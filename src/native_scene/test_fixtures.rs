use super::*;

pub(crate) fn cube() -> (NativeScene, NodeId) {
    let mut scene = NativeScene::default();
    let id = scene.create(Primitive::Cube, "pCube1").unwrap();
    scene.validate().unwrap();
    (scene, id)
}

pub(crate) fn shared_shape() -> (NativeScene, NodeId, NodeId, NativeId) {
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

pub(crate) fn grouped_shape() -> (NativeScene, NodeId, NodeId, NativeId) {
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

pub(crate) fn pivoted_shape() -> (NativeScene, NodeId) {
    let (mut scene, id) = cube();
    scene.nodes[0].rotate_pivot = [1.0, 0.25, -2.0].map(Scalar::new);
    scene.nodes[0].scale_pivot = [-0.5, 1.0, 0.0].map(Scalar::new);
    scene.validate().unwrap();
    (scene, id)
}

pub(crate) fn hidden_group() -> (NativeScene, NodeId, NodeId, NativeId) {
    let (mut scene, root, child, shape) = grouped_shape();
    scene.nodes[0].view.local_visibility = Some(false);
    scene.validate().unwrap();
    (scene, root, child, shape)
}

pub(crate) fn hidden_shape() -> (NativeScene, NodeId) {
    let (mut scene, id) = cube();
    scene.shapes[0].view.local_visibility = Some(false);
    scene.validate().unwrap();
    (scene, id)
}

pub(crate) fn unproved_group() -> (NativeScene, NodeId, NodeId, NativeId) {
    let (mut scene, root, child, shape) = grouped_shape();
    scene.nodes[0].view.geometry = None;
    scene.validate().unwrap();
    (scene, root, child, shape)
}
