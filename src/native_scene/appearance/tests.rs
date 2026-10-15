use crate::native_scene::{DagEdge, test_fixtures};

#[test]
fn summaries_use_all_direct_shapes_without_subtree_or_id_aliasing() {
    let (mut scene, root, child, child_shape) = test_fixtures::grouped_shape();
    assert_eq!(scene.appearance_readiness().unbound, 1);
    assert_eq!(scene.transform_appearance_readiness(root).unwrap(), None);
    assert_eq!(
        scene
            .transform_appearance_readiness(child)
            .unwrap()
            .unwrap()
            .unbound,
        1
    );
    assert!(
        scene
            .transform_appearance_readiness(crate::scene::NodeId::new(9999))
            .is_err()
    );
    assert_eq!(
        scene
            .shape_appearance_readiness(child_shape)
            .unwrap()
            .shapes,
        1
    );
    let extra = scene
        .create(crate::scene::Primitive::Plane, "extra")
        .unwrap();
    let shape = scene.shapes.last().unwrap().id;
    let transform = scene.node(extra).unwrap().transform_id;
    scene.dag.retain(|edge| edge.child != shape);
    scene.dag.push(DagEdge {
        parent: Some(scene.node(child).unwrap().transform_id),
        child: shape,
    });
    scene.validate().unwrap();
    assert_eq!(
        scene
            .transform_appearance_readiness(child)
            .unwrap()
            .unwrap()
            .shapes,
        2
    );
    assert_eq!(scene.transform_appearance_readiness(extra).unwrap(), None);
    assert_ne!(transform, child_shape);
}

#[test]
fn shared_shape_paths_report_the_same_source_free_state() {
    let (scene, first, second, shape) = test_fixtures::shared_shape();
    let summary = scene.shape_appearance_readiness(shape).unwrap();
    assert_eq!(
        scene.transform_appearance_readiness(first).unwrap(),
        Some(summary)
    );
    assert_eq!(
        scene.transform_appearance_readiness(second).unwrap(),
        Some(summary)
    );
    assert!(summary.issues.evaluation_unavailable);
    assert!(summary.issues.quadratic_filtering_unproved);
    assert!(summary.issues.color_processor_unavailable);
}
