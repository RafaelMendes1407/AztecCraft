use azteccraft::history::{Command, EditOutcome, History};
use azteccraft::scene::{NodeId, Primitive, Scene, SceneError, Transform};
use glam::Vec3;

fn create(history: &mut History, name: &str) -> NodeId {
    match history
        .execute(Command::Create {
            primitive: Primitive::Cube,
            name: name.into(),
        })
        .unwrap()
    {
        EditOutcome::Created(id) => id,
        outcome => panic!("unexpected outcome: {outcome:?}"),
    }
}

#[test]
fn undo_and_redo_restore_every_authoring_state_in_order() {
    let mut history = History::default();
    let mut states = vec![history.scene().nodes().to_vec()];
    let cube = create(&mut history, "cube");
    states.push(history.scene().nodes().to_vec());
    let plane = match history
        .execute(Command::Create {
            primitive: Primitive::Plane,
            name: "ground".into(),
        })
        .unwrap()
    {
        EditOutcome::Created(id) => id,
        _ => panic!("expected created plane"),
    };
    states.push(history.scene().nodes().to_vec());
    history
        .execute(Command::SetTransform {
            id: cube,
            transform: Transform {
                translation: Vec3::new(2.0, 3.0, 4.0),
                rotation_degrees: Vec3::new(45.0, 30.0, 60.0),
                scale: Vec3::new(-2.0, 3.0, 4.0),
            },
        })
        .unwrap();
    states.push(history.scene().nodes().to_vec());
    history
        .execute(Command::Rename {
            id: plane,
            name: "floor".into(),
        })
        .unwrap();
    states.push(history.scene().nodes().to_vec());
    history.execute(Command::Remove(cube)).unwrap();
    states.push(history.scene().nodes().to_vec());
    assert!(history.scene().node(cube).is_err());
    for expected in states[..states.len() - 1].iter().rev() {
        assert!(history.undo());
        assert_eq!(history.scene().nodes(), expected);
    }
    assert!(!history.can_undo());
    assert!(!history.undo());
    for expected in &states[1..] {
        assert!(history.redo());
        assert_eq!(history.scene().nodes(), expected);
    }
    assert!(!history.can_redo());
    assert!(!history.redo());
}

#[test]
fn failed_commands_preserve_scene_and_both_history_stacks() {
    let mut history = History::default();
    let cube = create(&mut history, "cube");
    history
        .execute(Command::Rename {
            id: cube,
            name: "renamed".into(),
        })
        .unwrap();
    assert!(history.undo());
    let before = history.scene().clone();
    for command in [
        Command::Create {
            primitive: Primitive::Cube,
            name: "".into(),
        },
        Command::Rename {
            id: cube,
            name: "\n".into(),
        },
        Command::SetTransform {
            id: cube,
            transform: Transform {
                scale: Vec3::splat(f32::NAN),
                ..Transform::default()
            },
        },
    ] {
        assert!(history.execute(command).is_err());
        assert_eq!(history.scene(), &before);
        assert!(history.can_undo());
        assert!(history.can_redo());
    }
    assert!(history.redo());
    assert_eq!(history.scene().node(cube).unwrap().name(), "renamed");
    assert!(history.undo());
    assert!(history.undo());
    assert!(history.scene().nodes().is_empty());
    assert!(!history.undo());
}

#[test]
fn undo_create_then_branch_reserves_discarded_ids() {
    let mut history = History::default();
    let a = create(&mut history, "a");
    let b = create(&mut history, "b");
    assert!(history.undo());
    let c = create(&mut history, "c");
    assert_ne!(b, c);
    assert_ne!(a, c);
    assert!(!history.can_redo());
    assert!(!history.redo());
    assert_eq!(history.scene().node(b), Err(SceneError::MissingNode(b)));
    assert_eq!(
        history.execute(Command::Remove(b)),
        Err(SceneError::MissingNode(b))
    );
    assert!(history.undo());
    assert!(history.undo());
    let d = create(&mut history, "d");
    for reserved in [a, b, c] {
        assert_ne!(reserved, d);
    }
}

#[test]
fn no_op_edits_do_not_consume_undo_or_invalidate_redo() {
    let mut history = History::default();
    let id = create(&mut history, "cube");
    history
        .execute(Command::Rename {
            id,
            name: "renamed".into(),
        })
        .unwrap();
    assert!(history.undo());
    assert_eq!(
        history
            .execute(Command::Rename {
                id,
                name: "cube".into()
            })
            .unwrap(),
        EditOutcome::Unchanged
    );
    assert_eq!(
        history
            .execute(Command::SetTransform {
                id,
                transform: Transform::default()
            })
            .unwrap(),
        EditOutcome::Unchanged
    );
    assert!(history.can_redo());
    assert!(history.redo());
    assert!(history.undo());
    assert!(history.undo());
    assert!(!history.undo());
}

#[test]
fn loaded_scene_starts_without_history_and_missing_nodes_return_errors() {
    let mut scene = Scene::default();
    let id = scene.create(Primitive::Cube, "existing").unwrap();
    let mut history = History::new(scene.clone());
    assert_eq!(history.scene(), &scene);
    assert!(!history.undo());
    history.execute(Command::Remove(id)).unwrap();
    let before = history.scene().clone();
    assert_eq!(
        history.execute(Command::Rename {
            id,
            name: "missing".into()
        }),
        Err(SceneError::MissingNode(id))
    );
    assert_eq!(
        history.execute(Command::SetTransform {
            id,
            transform: Transform::default()
        }),
        Err(SceneError::MissingNode(id))
    );
    assert_eq!(history.scene(), &before);
    assert!(history.undo());
    assert_eq!(history.scene(), &scene);
    assert!(history.redo());
    assert!(history.scene().nodes().is_empty());
}

#[test]
fn zero_and_signed_extrusions_are_undoable_and_new_edits_replace_redo_without_reusing_ids() {
    let mut history = History::default();
    let cube = create(&mut history, "cube");
    let initial = history.scene().clone();
    assert_eq!(
        history
            .execute(Command::ExtrudeFace {
                id: cube,
                face_index: 0,
                distance: 0.0
            })
            .unwrap(),
        EditOutcome::Changed
    );
    let zero = history.scene().clone();
    history
        .execute(Command::ExtrudeFace {
            id: cube,
            face_index: 0,
            distance: -0.25,
        })
        .unwrap();
    let inward = history.scene().clone();
    assert!(history.undo());
    assert_eq!(history.scene(), &zero);
    assert!(history.undo());
    assert_eq!(history.scene(), &initial);
    assert!(history.redo());
    assert_eq!(history.scene(), &zero);
    assert!(history.redo());
    assert_eq!(history.scene(), &inward);
    let discarded = create(&mut history, "discarded");
    assert!(history.undo());
    history
        .execute(Command::ExtrudeFace {
            id: cube,
            face_index: 0,
            distance: 0.5,
        })
        .unwrap();
    assert!(!history.can_redo());
    let replacement = create(&mut history, "replacement");
    assert_ne!(replacement, discarded);
}
