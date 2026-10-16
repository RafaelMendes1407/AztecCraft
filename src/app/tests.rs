use super::*;

mod material_wire {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_binary.rs"
    ));
}
mod material_fixture {
    use super::material_wire as wire;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_appearance.rs"
    ));
}

pub(in crate::app) fn material_editor() -> Editor {
    let scene = azteccraft::native_scene::binary::decode(
        material_fixture::appearance_specimen_with_users(2).into(),
    )
    .unwrap();
    let mut editor = Editor {
        history: History::new(scene),
        ..Editor::default()
    };
    editor.selected = Some(editor.history.scene().nodes()[0].id());
    editor.selected_path = vec![editor.history.scene().nodes()[0].transform_id()];
    editor.sync_fields();
    editor
}

fn independent_material_editor() -> Editor {
    let mut bytes = material_fixture::appearance_specimen_with_users(2);
    let link = b"shader.oc\0surface2.ss\0";
    let offset = bytes
        .windows(link.len())
        .position(|word| word == link)
        .unwrap();
    bytes[offset..offset + 6].copy_from_slice(b"shadeB");
    let container = azteccraft::maya_binary::container::inspect(bytes.clone().into()).unwrap();
    let shader = container
        .chunks()
        .iter()
        .find(|chunk| chunk.form() == Some(*b"RPHO"))
        .unwrap();
    let mut second = bytes[shader.header_range().start..shader.padding_range().end].to_vec();
    let offset = second
        .windows(7)
        .position(|word| word == b"shader\0")
        .unwrap();
    second[offset..offset + 6].copy_from_slice(b"shadeB");
    let mut children = Vec::new();
    for (index, chunk) in container
        .chunks()
        .iter()
        .enumerate()
        .filter(|(_, chunk)| chunk.parent() == Some(0))
    {
        if chunk.form() == Some(*b"CONS") {
            let connection = container
                .chunks()
                .iter()
                .find(|child| child.parent() == Some(index) && child.form() == Some(*b"CONN"))
                .unwrap();
            let mut links = bytes
                [connection.payload_range().start + 4..connection.payload_range().end]
                .to_vec();
            links.extend(material_wire::chunk(
                b"CWFL",
                b"\0texture.oc\0shadeB.c\0",
                8,
            ));
            children.extend(material_wire::group(
                b"CONS",
                &material_wire::group(b"CONN", &links, 4),
                4,
            ));
        } else {
            children
                .extend_from_slice(&bytes[chunk.header_range().start..chunk.padding_range().end]);
        }
    }
    children.extend(second);
    let scene = azteccraft::native_scene::binary::decode(
        material_wire::group(b"Maya", &children, 1).into(),
    )
    .unwrap();
    let mut editor = Editor {
        history: History::new(scene),
        ..Editor::default()
    };
    editor.selected = Some(editor.history.scene().nodes()[0].id());
    editor.sync_fields();
    editor
}

#[test]
fn material_selection_commits_only_original_independent_shader() {
    let mut editor = independent_material_editor();
    let first = editor.material_shape.unwrap();
    let second = editor.history.scene().shapes()[1].id();
    let other = editor.history.scene().nodes()[1].id();
    let untouched = editor
        .history
        .scene()
        .material_for_shape(second)
        .unwrap()
        .diffuse
        .to_bits();
    editor.material_draft.as_mut().unwrap().text = "0.125".into();
    editor.select(Some(other));
    assert_eq!(editor.material_shape, Some(second));
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(first)
            .unwrap()
            .diffuse,
        0.125
    );
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(second)
            .unwrap()
            .diffuse
            .to_bits(),
        untouched
    );
    editor.history_step(false);
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(first)
            .unwrap()
            .diffuse
            .to_bits(),
        untouched
    );
    editor.history_step(true);
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(first)
            .unwrap()
            .diffuse,
        0.125
    );
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(second)
            .unwrap()
            .diffuse
            .to_bits(),
        untouched
    );
}

#[test]
fn material_real_text_enter_and_escape_preserve_precision_in_one_history_entry() {
    let mut editor = material_editor();
    let shape = editor.material_shape.unwrap();
    let before = editor.history.scene().clone();
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(material_edit_id()));
    frame(&mut editor, &ctx, vec![]);
    let command = egui::Modifiers::COMMAND;
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(command),
            egui::Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: command,
            },
            egui::Event::Text("1.0000000000000002".into()),
            key(Key::Enter),
        ],
    );
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(shape)
            .unwrap()
            .diffuse
            .to_bits(),
        1.0000000000000002f64.to_bits()
    );
    assert!(!editor.has_draft());
    assert!(editor.history.undo());
    assert_eq!(editor.history.scene(), &before);
    assert!(!editor.history.can_undo());
    editor.sync_fields();
    ctx.memory_mut(|memory| memory.request_focus(material_edit_id()));
    frame(&mut editor, &ctx, vec![]);
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(command),
            egui::Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: command,
            },
            egui::Event::Text("0.125".into()),
        ],
    );
    assert!(editor.has_draft());
    frame(&mut editor, &ctx, vec![key(Key::Escape)]);
    assert_eq!(editor.history.scene(), &before);
    assert!(!editor.has_draft());
}

#[test]
fn material_failed_and_cancelled_open_preserve_draft_and_success_resets_reused_shape_choice() {
    let mut editor = material_editor();
    editor.material_draft.as_mut().unwrap().text = "0.125".into();
    let before = editor.history.scene().clone();
    let token = editor.scene_token;
    let directory = tempfile::tempdir().unwrap();
    assert!(
        editor
            .open_candidate(&directory.path().join("missing.mb"))
            .is_err()
    );
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.scene_token, token);
    assert_eq!(editor.material_draft.as_ref().unwrap().text, "0.125");
    editor.sync_fields();
    let source = directory.path().join("replacement.mb");
    std::fs::write(&source, material_wire::specimen()).unwrap();
    let OperationReply::PendingConfirmation {
        pending_action_id, ..
    } = editor.open_candidate(&source).unwrap()
    else {
        panic!("original nonempty document is dirty");
    };
    let original_shape = editor.material_shape;
    editor
        .resolve_file_action(
            Uuid::parse_str(&pending_action_id).unwrap(),
            crate::editor_operations::FileDecision::Cancel,
        )
        .unwrap();
    assert_eq!(editor.material_shape, original_shape);
    assert_eq!(editor.scene_token, token);
    let OperationReply::PendingConfirmation {
        pending_action_id, ..
    } = editor.open_candidate(&source).unwrap()
    else {
        panic!("confirmation required");
    };
    editor
        .resolve_file_action(
            Uuid::parse_str(&pending_action_id).unwrap(),
            crate::editor_operations::FileDecision::Discard,
        )
        .unwrap();
    assert_ne!(editor.scene_token, token);
    assert!(editor.material_shape.is_none());
    assert!(editor.material_draft.is_none());
    assert!(editor.commit_pending());
}

#[test]
fn material_draft_old_target_shared_history_and_escape_use_authoritative_state() {
    let mut editor = material_editor();
    let shape = editor.material_shape.unwrap();
    let original = editor
        .history
        .scene()
        .material_for_shape(shape)
        .unwrap()
        .diffuse;
    editor.material_draft.as_mut().unwrap().text = "1.2345678901234567".into();
    editor.material_draft.as_mut().unwrap().intent = true;
    let other = editor.history.scene().nodes()[1].id();
    editor.select(Some(other));
    assert_eq!(editor.selected, Some(other));
    let value = editor.history.scene().material_for_shape(shape).unwrap();
    assert_eq!(value.diffuse.to_bits(), 1.2345678901234567f64.to_bits());
    assert_eq!(value.shared_users, 2);
    for candidate in editor.history.scene().shapes() {
        assert_eq!(
            editor
                .history
                .scene()
                .material_for_shape(candidate.id())
                .unwrap()
                .diffuse
                .to_bits(),
            value.diffuse.to_bits()
        );
    }
    editor.history_step(false);
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(shape)
            .unwrap()
            .diffuse
            .to_bits(),
        original.to_bits()
    );
    editor.history_step(true);
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(shape)
            .unwrap()
            .diffuse
            .to_bits(),
        value.diffuse.to_bits()
    );
    let before = editor.history.scene().clone();
    editor.material_draft.as_mut().unwrap().text = "0.25".into();
    let context = egui::Context::default();
    let mut frame = context.run_ui(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| editor.cancel_on_escape(ui.ctx()),
    );
    frame.textures_delta.clear();
    assert_eq!(editor.history.scene(), &before);
    assert!(!editor.has_draft());
    assert!(
        editor
            .history
            .scene()
            .clone()
            .set_material_diffuse(shape, 0.5)
            .is_ok()
    );
    assert!(editor.history.execute(Command::Remove(other)).is_err());
}

#[test]
fn material_paint_preserves_unrounded_words_and_focus_or_drafts_block_remote_mutation() {
    let context = egui::Context::default();
    let mut editor = material_editor();
    let shape = editor.material_shape.unwrap();
    for value in [1.2345678901234567, -0.0, 4.5] {
        editor
            .history
            .execute(Command::SetMaterialDiffuse { shape, value })
            .unwrap();
        editor.sync_fields();
        let before = editor.history.scene().clone();
        for _ in 0..2 {
            let mut frame =
                context.run_ui(egui::RawInput::default(), |ui| editor.material_controls(ui));
            frame.textures_delta.clear();
        }
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(
            editor.material_draft.as_ref().unwrap().text,
            value.to_string()
        );
        assert!(!editor.has_draft());
    }
    context.memory_mut(|memory| memory.request_focus(material_edit_id()));
    let token = editor.scene_token;
    assert!(matches!(
        editor.remote_operation(
            RemoteRequest {
                expected_scene_token: Some(token),
                operation: Operation::SetMaterialDiffuse { shape, value: 0.25 }
            },
            &context
        ),
        Err(OperationError::BusyEditing)
    ));
    context.memory_mut(|memory| memory.surrender_focus(material_edit_id()));
    editor.material_draft.as_mut().unwrap().text = "0.125".into();
    assert!(matches!(
        editor.remote_operation(
            RemoteRequest {
                expected_scene_token: Some(token),
                operation: Operation::Undo
            },
            &context
        ),
        Err(OperationError::BusyEditing)
    ));
    let before = editor.history.scene().clone();
    editor.material_draft.as_mut().unwrap().scene_token = Uuid::new_v4();
    assert!(!editor.commit_pending());
    assert_eq!(editor.history.scene(), &before);
}

#[test]
fn material_invalid_or_simultaneous_channel_draft_never_partially_commits() {
    let mut editor = material_editor();
    let before = editor.history.scene().clone();
    editor.material_draft.as_mut().unwrap().text = "NaN".into();
    assert!(!editor.commit_pending());
    assert_eq!(editor.history.scene(), &before);
    editor.material_draft.as_mut().unwrap().text = "0.2".into();
    editor.channels[0] = "2".into();
    assert!(!editor.commit_pending());
    assert_eq!(editor.history.scene(), &before);
    editor.sync_fields();
    assert!(!editor.has_draft());
}

#[test]
fn material_new_document_commits_the_original_target_then_clears_its_draft() {
    let mut editor = material_editor();
    let token = editor.scene_token;
    let shape = editor.material_shape.unwrap();
    editor.material_draft.as_mut().unwrap().text = "0.125".into();
    let reply = editor.new_document().unwrap();
    let OperationReply::PendingConfirmation {
        pending_action_id, ..
    } = reply
    else {
        panic!("edited material must require confirmation");
    };
    assert_eq!(
        editor
            .history
            .scene()
            .material_for_shape(shape)
            .unwrap()
            .diffuse,
        0.125
    );
    assert!(!editor.has_draft());
    editor
        .resolve_file_action(
            Uuid::parse_str(&pending_action_id).unwrap(),
            crate::editor_operations::FileDecision::Discard,
        )
        .unwrap();
    assert_ne!(editor.scene_token, token);
    assert!(editor.material_draft.is_none());
    assert!(editor.material_shape.is_none());
    assert!(editor.history.scene().shapes().is_empty());
    assert!(editor.commit_pending());
}

#[test]
fn material_negative_source_word_is_not_changed_by_native_paint() {
    let mut bytes = material_fixture::appearance_specimen();
    let marker = [b'd', b'c', 0, 0x24];
    let offset = bytes
        .windows(marker.len())
        .position(|value| value == marker)
        .unwrap()
        + marker.len();
    let value = -1.2345678901234567f64;
    bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    let scene = azteccraft::native_scene::binary::decode(bytes.into()).unwrap();
    let mut editor = Editor {
        history: History::new(scene),
        ..Editor::default()
    };
    editor.selected = Some(editor.history.scene().nodes()[0].id());
    editor.sync_fields();
    let before = editor.history.scene().clone();
    let context = egui::Context::default();
    for _ in 0..2 {
        let mut frame =
            context.run_ui(egui::RawInput::default(), |ui| editor.material_controls(ui));
        frame.textures_delta.clear();
    }
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(
        editor.material_draft.as_ref().unwrap().text,
        value.to_string()
    );
    assert!(!editor.has_draft());
}

#[test]
fn material_multiple_direct_shapes_require_an_explicit_choice_without_guessing() {
    let extra = material_wire::group(
        b"DMSH",
        &[
            material_wire::creation(b"secondShape", Some(b"object")),
            material_wire::chunk(
                b"MESH",
                &material_wire::mesh_payload(&[0, 1, 0x6000_0002]),
                8,
            ),
        ]
        .concat(),
        4,
    );
    let bytes = material_fixture::appearance_specimen_with_fields(material_fixture::Fields {
        groups: &extra,
        ..Default::default()
    });
    let scene = azteccraft::native_scene::binary::decode(bytes.into()).unwrap();
    let mut editor = Editor {
        history: History::new(scene),
        ..Editor::default()
    };
    editor.selected = Some(editor.history.scene().nodes()[0].id());
    editor.sync_fields();
    assert!(editor.material_shape.is_none());
    assert!(editor.material_draft.is_none());
    let context = egui::Context::default();
    let before = editor.history.scene().clone();
    let mut frame = context.run_ui(egui::RawInput::default(), |ui| editor.material_controls(ui));
    frame.textures_delta.clear();
    assert_eq!(editor.history.scene(), &before);
    assert!(editor.material_shape.is_none());
    editor.material_shape = Some(editor.history.scene().shapes()[0].id());
    editor.sync_fields();
    assert!(editor.material_draft.is_some());
}

fn selected_face(editor: &Editor, face_index: usize) -> FaceTarget {
    let view = ViewScene::from_native(editor.history.scene(), 256 * 1024 * 1024).unwrap();
    let instance = view
        .instances()
        .iter()
        .find(|instance| Some(instance.transform()) == editor.selected)
        .unwrap();
    FaceTarget {
        transform: instance.transform(),
        shape: instance.shape(),
        path: instance.path().to_vec(),
        face_index,
    }
}

#[test]
fn face_lifecycle_revalidates_numeric_edits_topology_history_and_object_deletion() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let cap = selected_face(&editor, 0);
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Face),
    )
    .unwrap();
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(cap.clone()),
        },
    )
    .unwrap();
    editor.channels[0] = "2".into();
    editor.native_operation(Operation::FrameSelected);
    assert!(!editor.error);
    assert_eq!(editor.remembered_face.as_ref(), Some(&cap));
    assert_eq!(
        editor
            .history
            .scene()
            .node(id)
            .unwrap()
            .transform()
            .translation[0],
        2.0
    );
    remote(&mut editor, Operation::Undo).unwrap();
    assert_eq!(editor.remembered_face.as_ref(), Some(&cap));
    remote(&mut editor, Operation::Redo).unwrap();
    assert_eq!(editor.remembered_face.as_ref(), Some(&cap));
    remote(
        &mut editor,
        Operation::ExtrudeFace {
            target: cap.clone(),
            distance: 0.25,
        },
    )
    .unwrap();
    assert_eq!(editor.remembered_face.as_ref(), Some(&cap));
    let side = FaceTarget {
        face_index: 6,
        ..cap
    };
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(side.clone()),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    assert_eq!(editor.remembered_face, None);
    assert!(editor.history.can_redo());
    remote(&mut editor, Operation::Redo).unwrap();
    assert_eq!(editor.remembered_face, None);
    remote(&mut editor, Operation::SelectFace { target: Some(side) }).unwrap();
    remote(&mut editor, Operation::Delete(id)).unwrap();
    assert_eq!(editor.selected, None);
    assert!(editor.selected_path.is_empty());
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.selection_mode, SelectionMode::Face);
    remote(&mut editor, Operation::Undo).unwrap();
    assert!(editor.history.scene().node(id).is_ok());
    assert_eq!(editor.remembered_face, None);
}

#[test]
fn face_frame_uses_selected_corners_even_when_center_is_not_pickable() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    editor.aspect = 4.0 / 3.0;
    let view = ViewScene::from_native(editor.history.scene(), 1024 * 1024).unwrap();
    let instance = &view.instances()[0];
    let (face, target) = instance
        .faces()
        .iter()
        .find_map(|face| {
            let center = face.center(instance.world_vertices()).unwrap();
            let ndc = editor.camera.project(center, editor.aspect).unwrap()?;
            let pointer = egui::vec2((ndc.x + 1.0) * 400.0, (1.0 - ndc.y) * 300.0);
            let target = FaceTarget {
                transform: instance.transform(),
                shape: instance.shape(),
                path: instance.path().to_vec(),
                face_index: face.source_index(),
            };
            let hit = editor
                .camera
                .pick_face_center(
                    &view,
                    Some((instance.transform(), &editor.selected_path)),
                    glam::Vec2::new(pointer.x, pointer.y),
                    [800, 600],
                    1.0,
                )
                .unwrap();
            (hit.as_ref() != Some(&target)).then_some((face, target))
        })
        .expect("cube has an occluded center");
    let corners: Vec<_> = face
        .corners()
        .iter()
        .map(|&index| instance.world_vertices()[index])
        .collect();
    let mut expected = editor.camera.clone();
    expected.frame_points(&corners, editor.aspect).unwrap();
    let scene = editor.history.scene().clone();
    let token = editor.scene_token;
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Face),
    )
    .unwrap();
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(target.clone()),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::FrameSelected).unwrap();
    assert_eq!(editor.camera, expected);
    assert_eq!(editor.remembered_face.as_ref(), Some(&target));
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.scene_token, token);
    remote(&mut editor, Operation::SelectFace { target: None }).unwrap();
    let camera = editor.camera.clone();
    assert!(matches!(
        remote(&mut editor, Operation::FrameSelected),
        Err(OperationError::NoSelection)
    ));
    assert_eq!(editor.camera, camera);
    assert_eq!(editor.history.scene(), &scene);
    editor.name = "valid_human_rename".into();
    assert!(matches!(
        remote(&mut editor, Operation::FrameSelected),
        Err(OperationError::BusyEditing)
    ));
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.name, "valid_human_rename");
    editor.native_operation(Operation::FrameSelected);
    assert!(editor.error);
    assert_eq!(editor.camera, camera);
    assert_eq!(editor.remembered_face, None);
    assert_eq!(
        editor
            .history
            .scene()
            .node(target.transform)
            .unwrap()
            .name(),
        "valid_human_rename"
    );
    remote(&mut editor, Operation::Undo).unwrap();
    assert_eq!(editor.history.scene(), &scene);
}

#[test]
fn face_mode_preserves_the_remembered_target_without_editing_scene_or_redo() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let target = selected_face(&editor, 4);
    remote(
        &mut editor,
        Operation::Rename {
            id,
            name: "temporary".into(),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    let scene = editor.history.scene().clone();
    let token = editor.scene_token;
    let camera = editor.camera.clone();
    let dirty = editor.inspect(false).unwrap().dirty;
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Face),
    )
    .unwrap();
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(target.clone()),
        },
    )
    .unwrap();
    for mode in [
        SelectionMode::Object,
        SelectionMode::Hierarchy,
        SelectionMode::Face,
    ] {
        remote(&mut editor, Operation::SetSelectionMode(mode)).unwrap();
        assert_eq!(editor.remembered_face.as_ref(), Some(&target));
        assert_eq!(editor.selected, Some(id));
        assert_eq!(editor.selected_path, target.path[..target.path.len() - 1]);
    }
    remote(
        &mut editor,
        Operation::Select {
            node: Some(id),
            instance_path: None,
        },
    )
    .unwrap();
    assert_eq!(editor.remembered_face.as_ref(), Some(&target));
    remote(&mut editor, Operation::SelectFace { target: None }).unwrap();
    assert_eq!(editor.selected, Some(id));
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.history.scene(), &scene);
    assert!(editor.history.can_redo());
    assert_eq!(editor.scene_token, token);
    assert_eq!(editor.camera, camera);
    assert_eq!(editor.inspect(false).unwrap().dirty, dirty);
}

#[test]
fn explicit_face_selection_selects_its_owner_and_object_changes_clear_memory() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let target = selected_face(&editor, 5);
    editor.create(Primitive::Plane);
    let other = editor.selected.unwrap();
    let second_shape = selected_face(&editor, 0);
    remote(
        &mut editor,
        Operation::Rename {
            id: other,
            name: "temporary".into(),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Face),
    )
    .unwrap();
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(target.clone()),
        },
    )
    .unwrap();
    assert_eq!(editor.selected, Some(target.transform));
    assert_eq!(editor.selected_path, target.path[..target.path.len() - 1]);
    assert_eq!(editor.remembered_face.as_ref(), Some(&target));
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(second_shape.clone()),
        },
    )
    .unwrap();
    assert_eq!(editor.selected, Some(other));
    assert_eq!(editor.remembered_face.as_ref(), Some(&second_shape));
    assert_eq!(
        editor.selected_path,
        second_shape.path[..second_shape.path.len() - 1]
    );
    let baseline = (
        editor.history.scene().clone(),
        editor.selected,
        editor.selected_path.clone(),
        editor.selection_mode,
        editor.remembered_face.clone(),
        editor.camera.clone(),
        editor.scene_token,
        editor.name.clone(),
        editor.channels.clone(),
        editor.inspect(false).unwrap().dirty,
    );
    assert!(editor.history.can_redo());
    assert!(matches!(
        remote(
            &mut editor,
            Operation::SelectFace {
                target: Some(second_shape)
            }
        )
        .unwrap(),
        OperationReply::Unchanged
    ));
    assert_eq!(
        (
            editor.history.scene().clone(),
            editor.selected,
            editor.selected_path.clone(),
            editor.selection_mode,
            editor.remembered_face.clone(),
            editor.camera.clone(),
            editor.scene_token,
            editor.name.clone(),
            editor.channels.clone(),
            editor.inspect(false).unwrap().dirty,
        ),
        baseline
    );
    assert!(editor.history.can_redo());
    remote(
        &mut editor,
        Operation::Select {
            node: Some(target.transform),
            instance_path: None,
        },
    )
    .unwrap();
    assert_eq!(editor.remembered_face, None);
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(target),
        },
    )
    .unwrap();
    remote(
        &mut editor,
        Operation::Select {
            node: None,
            instance_path: None,
        },
    )
    .unwrap();
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.selected, None);
}

#[test]
fn invalid_face_targets_and_wrong_mode_preserve_the_selection_baseline() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let target = selected_face(&editor, 1);
    editor.name = "valid_human_draft".into();
    assert!(
        editor
            .apply_operation(Operation::SelectFace {
                target: Some(target.clone())
            })
            .is_err()
    );
    assert!(editor.has_draft());
    assert_eq!(editor.remembered_face, None);
    editor.sync_fields();
    editor.create(Primitive::Plane);
    let other = selected_face(&editor, 0);
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Face),
    )
    .unwrap();
    remote(
        &mut editor,
        Operation::SelectFace {
            target: Some(target.clone()),
        },
    )
    .unwrap();
    let scene = editor.history.scene().clone();
    let before = (
        editor.selected,
        editor.selected_path.clone(),
        editor.selection_mode,
        editor.remembered_face.clone(),
        editor.name.clone(),
        editor.channels.clone(),
        editor.scene_token,
        editor.camera.clone(),
    );
    let mut invalid = Vec::new();
    let mut bad = target.clone();
    bad.path = other.path;
    invalid.push(bad);
    let mut bad = target.clone();
    bad.transform = other.transform;
    invalid.push(bad);
    let mut bad = target.clone();
    bad.shape = other.shape;
    invalid.push(bad);
    let mut bad = target.clone();
    bad.face_index = usize::MAX;
    invalid.push(bad);
    let mut bad = target.clone();
    bad.path.clear();
    invalid.push(bad);
    let mut bad = target.clone();
    bad.path = vec![target.shape; azteccraft::native_scene::MAX_TRANSFORM_DEPTH + 2];
    invalid.push(bad);
    let mut bad = target.clone();
    bad.transform = NodeId::from_decimal("18446744073709551615").unwrap();
    invalid.push(bad);
    let mut bad = target;
    bad.shape = NativeId::from_decimal("18446744073709551615").unwrap();
    invalid.push(bad);
    for target in invalid {
        assert!(
            remote(
                &mut editor,
                Operation::SelectFace {
                    target: Some(target)
                }
            )
            .is_err()
        );
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(
            (
                editor.selected,
                editor.selected_path.clone(),
                editor.selection_mode,
                editor.remembered_face.clone(),
                editor.name.clone(),
                editor.channels.clone(),
                editor.scene_token,
                editor.camera.clone(),
            ),
            before
        );
    }
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Object),
    )
    .unwrap();
    let before = (
        editor.selected,
        editor.selected_path.clone(),
        editor.selection_mode,
        editor.remembered_face.clone(),
        editor.name.clone(),
        editor.channels.clone(),
        editor.scene_token,
        editor.camera.clone(),
    );
    assert!(remote(&mut editor, Operation::SelectFace { target: None }).is_err());
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(
        (
            editor.selected,
            editor.selected_path.clone(),
            editor.selection_mode,
            editor.remembered_face.clone(),
            editor.name.clone(),
            editor.channels.clone(),
            editor.scene_token,
            editor.camera.clone(),
        ),
        before
    );
}

mod ascii_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_ascii.rs"
    ));
}

#[test]
fn ascii_inspection_reports_canonical_curves_without_private_arrays_or_gpu_claims() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("curve.ma");
    let source = ascii_fixture::specimen();
    std::fs::write(&path, &source).unwrap();
    let mut editor = Editor::default();
    editor
        .apply_operation(Operation::OpenScene { path })
        .unwrap();
    let inspection = editor.inspect(false).unwrap();
    assert_eq!(
        inspection.document_origin,
        crate::editor_operations::DocumentOrigin::MayaAscii
    );
    assert_eq!(inspection.retained_source_bytes, source.len());
    assert_eq!(inspection.retained_source_records, 10);
    assert!(inspection.read_only_reason.unwrap().contains("read-only"));
    assert!(inspection.shapes.is_empty());
    assert_eq!(inspection.curves.len(), 1);
    assert!(!inspection.save_available && !inspection.save_as_available);
    let curve = &inspection.curves[0];
    assert_eq!(
        curve.id,
        editor.history.scene().curves()[0].id().to_decimal()
    );
    assert_eq!(
        inspection.nodes[0].shape_ids.as_slice(),
        std::slice::from_ref(&curve.id)
    );
    assert_eq!(curve.degree, 3);
    assert_eq!(curve.control_vertices, 4);
    assert_eq!(curve.knots, 6);
    assert_eq!(curve.domain, [0.0, 1.0]);
    assert!(curve.read_only && curve.draw_available);
    assert_eq!(inspection.viewport_backend, "uninitialized");
    assert!(!inspection.depth_complete && !inspection.clipping_complete);
    let value = serde_json::to_value(curve).unwrap();
    assert_eq!(value["form"], "open");
    assert!(value["control_vertices"].is_number() && value["knots"].is_number());
    assert!(!serde_json::to_string(&value).unwrap().contains("script"));

    for source in [
        String::from_utf8(source.clone())
            .unwrap()
            .replace("setAttr \".t\"", "setAttr \".v\" no; setAttr \".t\""),
        String::from_utf8(source)
            .unwrap()
            .replace("12 0 0", "100000000 0 0"),
    ] {
        let scene = azteccraft::native_scene::ascii::decode(source.into_bytes().into()).unwrap();
        let mut editor = Editor {
            history: History::new(scene),
            ..Default::default()
        };
        editor.viewport_error = None;
        let inspection = editor.inspect(false).unwrap();
        assert_eq!(inspection.curves.len(), 1);
        assert!(!inspection.curves[0].draw_available);
        assert_eq!(inspection.viewport_backend, "uninitialized");
        assert!(!inspection.depth_complete && !inspection.clipping_complete);
    }
}

#[test]
fn face_mode_on_a_curves_only_ascii_scene_refuses_face_targets_and_preserves_state() {
    let scene = azteccraft::native_scene::ascii::decode(ascii_fixture::specimen().into()).unwrap();
    let mut editor = Editor {
        history: History::new(scene),
        ..Default::default()
    };
    editor.viewport_error = None;
    remote(
        &mut editor,
        Operation::SetSelectionMode(SelectionMode::Face),
    )
    .unwrap();
    let curve = &editor.history.scene().curves()[0];
    let target = FaceTarget {
        transform: editor.history.scene().nodes()[0].id(),
        shape: curve.id(),
        path: vec![editor.history.scene().nodes()[0].transform_id(), curve.id()],
        face_index: 0,
    };
    let before = editor.history.scene().clone();
    assert!(
        remote(
            &mut editor,
            Operation::SelectFace {
                target: Some(target)
            }
        )
        .is_err()
    );
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.history.scene(), &before);
    assert!(!editor.history.can_undo());
    let inspection = editor.inspect(false).unwrap();
    assert_eq!(inspection.curves.len(), 1);
}

#[test]
fn ascii_channel_controls_are_disabled_and_show_the_shared_core_reason() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("curve.ma");
    std::fs::write(&path, ascii_fixture::specimen()).unwrap();
    let mut editor = Editor::default();
    editor
        .apply_operation(Operation::OpenScene { path })
        .unwrap();
    let id = editor.history.scene().nodes()[0].id();
    editor.select(Some(id));
    let before = editor.history.scene().clone();
    let fields = (editor.name.clone(), editor.channels.clone());
    for text_id in [name_edit_id(), channel_edit_id(0)] {
        let context = egui::Context::default();
        context.memory_mut(|memory| memory.request_focus(text_id));
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(420.0, 900.0),
                )),
                events: vec![
                    egui::Event::Text("must not become a draft".into()),
                    key(Key::Enter),
                ],
                ..Default::default()
            },
            |ui| editor.channels(ui, &mut false),
        );
        output.textures_delta.clear();
        let reason = editor
            .history
            .scene()
            .authoring_read_only_reason()
            .unwrap()
            .to_string();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text.contains(&reason))));
        assert_eq!((&editor.name, &editor.channels), (&fields.0, &fields.1));
        assert!(!context.memory(|memory| memory.has_focus(text_id)));
        assert!(!editor.has_draft());
        assert_eq!(editor.history.scene(), &before);
    }
}

#[test]
fn ascii_outliner_selection_and_frame_use_actual_curve_owner_without_authoring() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("curve.ma");
    std::fs::write(&path, ascii_fixture::specimen()).unwrap();
    let mut editor = Editor::default();
    editor
        .apply_operation(Operation::OpenScene { path })
        .unwrap();
    let before = editor.history.scene().clone();
    let token = editor.scene_token;
    let context = egui::Context::default();
    let mut output = context.run_ui(egui::RawInput::default(), |ui| editor.outliner(ui));
    output.textures_delta.clear();
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text.contains("◇  curve") => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap();
    for pressed in [true, false] {
        let mut output = context.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| editor.outliner(ui),
        );
        output.textures_delta.clear();
    }
    assert_eq!(
        editor.selected,
        Some(editor.history.scene().nodes()[0].id())
    );
    editor
        .apply_operation(Operation::Pan {
            world_units: [20.0, 10.0],
        })
        .unwrap();
    editor.apply_operation(Operation::FrameSelected).unwrap();
    assert_eq!(editor.camera.target(), glam::Vec3::new(13.5, 0.0, 0.0));
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.scene_token, token);
    assert!(!editor.is_dirty() && !editor.history.can_undo());
}

#[test]
fn channel_box_appearance_summary_precedes_controls_and_preserves_human_state() {
    for width in [240.0, 420.0] {
        let context = egui::Context::default();
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        editor.name = "pending human draft".into();
        context.memory_mut(|memory| memory.request_focus(name_edit_id()));
        let scene = editor.history.scene().clone();
        let selected = editor.selected;
        let path = editor.selected_path.clone();
        let token = editor.scene_token;
        let mut output = None;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 720.0),
                )),
                ..Default::default()
            };
            let mut delete_intent = false;
            let mut frame = context.run_ui(input, |ui| editor.channels(ui, &mut delete_intent));
            frame.textures_delta.clear();
            output = Some(frame);
        }
        let output = output.unwrap();
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some((text.galley.job.text.as_str(), text.pos.y)),
                _ => None,
            })
            .collect();
        let summary = labels
            .iter()
            .find(|(text, _)| *text == "Direct shapes (1)")
            .unwrap();
        let controls = labels
            .iter()
            .find(|(text, _)| *text == "Object name")
            .unwrap();
        assert!(summary.1 < controls.1);
        assert!(
            labels
                .iter()
                .any(|(text, _)| text.contains("Evaluation unavailable"))
        );
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.selected, selected);
        assert_eq!(editor.selected_path, path);
        assert_eq!(editor.scene_token, token);
        assert_eq!(editor.name, "pending human draft");
        assert!(context.memory(|memory| memory.has_focus(name_edit_id())));
    }
}

#[test]
fn camera_nonfinite_operations_and_native_draft_failure_preserve_their_baselines() {
    let context = egui::Context::default();
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let camera = editor.camera.clone();
    let before = editor.history.scene().clone();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for operation in [
            Operation::Orbit {
                radians: [value, 0.0],
            },
            Operation::Orbit {
                radians: [0.0, value],
            },
            Operation::Pan {
                world_units: [value, 0.0],
            },
            Operation::Pan {
                world_units: [0.0, value],
            },
            Operation::Dolly { factor: value },
        ] {
            assert!(matches!(
                editor.remote_operation(
                    RemoteRequest {
                        expected_scene_token: Some(editor.scene_token),
                        operation,
                    },
                    &context
                ),
                Err(OperationError::Camera(_))
            ));
            assert_eq!(editor.camera, camera);
            assert_eq!(editor.history.scene(), &before);
        }
    }
    editor.name = "committed_before_navigation".into();
    editor.native_operation(Operation::Dolly { factor: -1.0 });
    assert_eq!(editor.camera, camera);
    assert_eq!(
        editor.history.scene().nodes()[0].name(),
        "committed_before_navigation"
    );
    assert!(!editor.has_draft());
    assert!(editor.error);
    editor.history_step(false);
    assert_eq!(editor.history.scene(), &before);
    editor.native_operation(Operation::Dolly { factor: 0.5 });
    assert_ne!(editor.status, "Scene edited");
    assert!(editor.history.can_redo());
}

#[test]
fn native_hidden_close_checks_pending_input_without_mcp() {
    for pending in [false, true] {
        let context = egui::Context::default();
        let mut editor = Editor::default();
        let mut input = egui::RawInput::default();
        if pending {
            input.events.push(egui::Event::Text("pending".into()));
        }
        let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
        viewport.minimized = Some(true);
        viewport.events.push(egui::ViewportEvent::Close);
        eframe::App::raw_input_hook(&mut editor, &context, &mut input);
        let mut native_frame = eframe::Frame::_new_kittest();
        let output = context.run_logic(&input, |ctx| {
            eframe::App::logic(&mut editor, ctx, &mut native_frame)
        });
        let commands = &output.viewport_commands[&egui::ViewportId::ROOT];
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert_eq!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close)),
            !pending
        );
    }
}

pub(in crate::app) fn remote(
    editor: &mut Editor,
    operation: Operation,
) -> Result<OperationReply, OperationError> {
    editor.remote_operation(
        RemoteRequest {
            expected_scene_token: Some(editor.scene_token),
            operation,
        },
        &egui::Context::default(),
    )
}

#[test]
fn grid_setter_is_idempotent_and_preserves_scene_history_and_editor_baseline() {
    let mut editor = Editor::default();
    assert!(editor.grid_visible);
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    remote(
        &mut editor,
        Operation::Rename {
            id,
            name: "renamed".into(),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    assert!(editor.history.can_redo());
    let scene = editor.history.scene().clone();
    let before = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
    let fields = (editor.name.clone(), editor.channels.clone());
    let status = (editor.status.clone(), editor.error);
    for (visible, changed) in [(true, false), (false, true), (false, false), (true, true)] {
        let reply = remote(&mut editor, Operation::SetGridVisible { visible }).unwrap();
        assert_eq!(matches!(reply, OperationReply::Changed), changed);
        assert_eq!(matches!(reply, OperationReply::Unchanged), !changed);
        let mut actual = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
        assert_eq!(actual["grid_visible"], visible);
        actual["grid_visible"] = serde_json::Value::Bool(true);
        assert_eq!(actual, before);
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!((editor.name.clone(), editor.channels.clone()), fields);
        assert_eq!((editor.status.clone(), editor.error), status);
    }
}

#[test]
fn shading_setter_is_idempotent_session_only_and_neither_dirties_nor_changes_the_token() {
    use crate::editor_operations::ShadingMode;
    let mut editor = Editor::default();
    assert_eq!(editor.shading, ShadingMode::SmoothShadeAll);
    assert!(!editor.is_dirty());
    let token = editor.scene_token;
    let scene = editor.history.scene().clone();
    let before = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
    assert_eq!(before["shading"], "smooth_shade_all");
    let status = (editor.status.clone(), editor.error);
    for (mode, changed) in [
        (ShadingMode::SmoothShadeAll, false),
        (ShadingMode::Wireframe, true),
        (ShadingMode::Wireframe, false),
        (ShadingMode::SmoothShadeAll, true),
    ] {
        let reply = remote(&mut editor, Operation::SetShadingMode(mode)).unwrap();
        assert_eq!(matches!(reply, OperationReply::Changed), changed);
        assert_eq!(matches!(reply, OperationReply::Unchanged), !changed);
        assert_eq!(editor.shading, mode);
        let mut actual = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
        assert_eq!(actual["shading"], serde_json::to_value(mode).unwrap());
        actual["shading"] = before["shading"].clone();
        assert_eq!(actual, before);
        assert!(!editor.is_dirty());
        assert_eq!(editor.scene_token, token);
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!((editor.status.clone(), editor.error), status);
    }
}

#[test]
fn shading_refuses_modal_and_closing_and_survives_new_scene() {
    use crate::editor_operations::ShadingMode;
    for closing in [false, true] {
        let mut editor = Editor::default();
        if closing {
            editor.close_document().unwrap();
        } else {
            editor.create(Primitive::Cube);
            editor.new_document().unwrap();
        }
        assert!(matches!(
            editor.apply_operation(Operation::SetShadingMode(ShadingMode::Wireframe)),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(editor.shading, ShadingMode::SmoothShadeAll);
    }
    let mut editor = Editor::default();
    remote(
        &mut editor,
        Operation::SetShadingMode(ShadingMode::Wireframe),
    )
    .unwrap();
    remote(&mut editor, Operation::NewScene).unwrap();
    assert_eq!(editor.shading, ShadingMode::Wireframe);
}

#[test]
fn num4_and_num5_select_shading_and_num6_num7_only_set_status() {
    use crate::editor_operations::ShadingMode;
    let mut editor = Editor::default();
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![]);
    let token = editor.scene_token;
    frame(&mut editor, &ctx, vec![key(Key::Num4)]);
    assert_eq!(editor.shading, ShadingMode::Wireframe);
    let shifted = egui::Event::Key {
        key: Key::Num5,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::SHIFT,
    };
    let released = egui::Event::Key {
        key: Key::Num5,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut editor, &ctx, vec![shifted, released]);
    assert_eq!(editor.shading, ShadingMode::Wireframe);
    for (pressed, name) in [
        (Key::Num6, "Shaded + Textured"),
        (Key::Num7, "Use All Lights"),
    ] {
        frame(&mut editor, &ctx, vec![key(pressed)]);
        assert_eq!(editor.shading, ShadingMode::Wireframe);
        assert!(editor.status.contains(name) && editor.status.contains("not implemented"));
        assert!(!editor.error && !editor.is_dirty());
        assert_eq!(editor.scene_token, token);
    }
    frame(&mut editor, &ctx, vec![key(Key::Num5)]);
    assert_eq!(editor.shading, ShadingMode::SmoothShadeAll);
    assert!(!editor.is_dirty());
    assert_eq!(editor.scene_token, token);
}

#[test]
fn typing_digits_into_focused_fields_does_not_change_shading() {
    use crate::editor_operations::ShadingMode;
    for text_id in [name_edit_id(), channel_edit_id(0)] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        editor
            .apply_operation(Operation::SetShadingMode(ShadingMode::Wireframe))
            .unwrap();
        let ctx = egui::Context::default();
        frame(&mut editor, &ctx, vec![]);
        ctx.memory_mut(|memory| memory.request_focus(text_id));
        frame(&mut editor, &ctx, vec![]);
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(text_id));
        frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::Text("5".into()),
                key(Key::Num5),
                key(Key::Num4),
            ],
        );
        assert_eq!(editor.shading, ShadingMode::Wireframe);
        assert!(ctx.egui_wants_keyboard_input());
    }
}

#[test]
fn grid_native_draft_policy_and_remote_draft_refusal_share_the_typed_setter() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let scene = editor.history.scene().clone();
    editor.channels[0] = "invalid".into();
    assert!(matches!(
        editor.apply_operation(Operation::SetGridVisible { visible: false }),
        Err(OperationError::InvalidInput(_))
    ));
    assert!(editor.grid_visible);
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.channels[0], "invalid");
    for draft in ["2", "invalid"] {
        editor.channels[0] = draft.into();
        assert!(matches!(
            remote(&mut editor, Operation::SetGridVisible { visible: false }),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.channels[0], draft);
        assert!(editor.grid_visible);
    }
    editor.channels[0] = "2".into();
    editor.native_operation(Operation::SetGridVisible { visible: false });
    assert!(!editor.grid_visible);
    assert!(!editor.has_draft());
    assert_eq!(
        editor.history.scene().nodes()[0].transform().translation[0],
        2.0
    );
    editor.history_step(false);
    assert_eq!(editor.history.scene(), &scene);
    assert!(!editor.grid_visible);
}

#[test]
fn grid_native_modal_and_close_refuse_before_committing_drafts() {
    for closing in [false, true] {
        let mut editor = Editor::default();
        if closing {
            editor.close_document().unwrap();
        } else {
            editor.create(Primitive::Cube);
            editor.new_document().unwrap();
        }
        editor.name = "pending draft".into();
        let before = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
        let scene = editor.history.scene().clone();
        assert!(matches!(
            editor.apply_operation(Operation::SetGridVisible { visible: false }),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(
            serde_json::to_value(editor.inspect(false).unwrap()).unwrap(),
            before
        );
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.name, "pending draft");
    }
}

#[test]
fn grid_session_setting_survives_successful_and_canceled_new_and_open() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("empty.aztec");
    azteccraft::document::save_new(&path, &azteccraft::scene::Scene::default()).unwrap();
    let mut editor = Editor::default();
    remote(&mut editor, Operation::SetGridVisible { visible: false }).unwrap();
    for open in [false, true] {
        let operation = if open {
            Operation::OpenScene { path: path.clone() }
        } else {
            Operation::NewScene
        };
        assert!(matches!(
            remote(&mut editor, operation).unwrap(),
            OperationReply::DocumentApplied { .. }
        ));
        assert!(!editor.grid_visible);
        editor.create(Primitive::Cube);
        let scene = editor.history.scene().clone();
        let token = editor.scene_token;
        let operation = if open {
            Operation::OpenScene { path: path.clone() }
        } else {
            Operation::NewScene
        };
        let OperationReply::PendingConfirmation {
            pending_action_id, ..
        } = remote(&mut editor, operation).unwrap()
        else {
            panic!("expected unsaved confirmation");
        };
        remote(
            &mut editor,
            Operation::ResolveFileAction {
                pending_id: Uuid::parse_str(&pending_action_id).unwrap(),
                decision: crate::editor_operations::FileDecision::Cancel,
            },
        )
        .unwrap();
        assert!(!editor.grid_visible);
        assert_eq!(editor.scene_token, token);
        assert_eq!(editor.history.scene(), &scene);
        editor.history_step(false);
    }
}

#[test]
fn sparse_nonfinite_intents_preserve_scene_fields_and_redo() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let before = editor.history.scene().clone();
    remote(
        &mut editor,
        Operation::Rename {
            id,
            name: "temporary".into(),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    let fields = (editor.name.clone(), editor.channels.clone());
    for group in 0..3 {
        let mut components = [[None; 3]; 3];
        components[group][0] = Some(f64::NAN);
        assert!(
            remote(
                &mut editor,
                Operation::EditNode {
                    id,
                    name: Some("must_not_apply".into()),
                    translation: components[0],
                    rotation_degrees: components[1],
                    scale: components[2]
                }
            )
            .is_err()
        );
        assert_eq!(editor.history.scene(), &before);
        assert_eq!((&editor.name, &editor.channels), (&fields.0, &fields.1));
        assert!(editor.history.can_redo());
    }
}

fn sidebar_op(visible: bool) -> Operation {
    Operation::SetSidebarPanelVisible {
        panel: SidebarPanel::ChannelBox,
        visible,
    }
}

#[test]
fn sidebar_setter_is_idempotent_and_preserves_scene_history_and_editor_baseline() {
    let mut editor = Editor::default();
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    remote(
        &mut editor,
        Operation::Rename {
            id,
            name: "renamed".into(),
        },
    )
    .unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    assert!(editor.history.can_redo());
    let scene = editor.history.scene().clone();
    let before = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
    let fields = (editor.name.clone(), editor.channels.clone());
    let status = (editor.status.clone(), editor.error);
    let token = editor.scene_token;
    for (visible, changed) in [(true, false), (false, true), (false, false), (true, true)] {
        let reply = remote(&mut editor, sidebar_op(visible)).unwrap();
        assert_eq!(matches!(reply, OperationReply::Changed), changed);
        assert_eq!(matches!(reply, OperationReply::Unchanged), !changed);
        assert_eq!(
            editor.sidebar_dock,
            visible.then_some(SidebarPanel::ChannelBox)
        );
        let mut actual = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
        assert_eq!(actual["sidebar_panels"]["channel_box"], visible);
        actual["sidebar_panels"]["channel_box"] = serde_json::Value::Bool(true);
        assert_eq!(actual, before);
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!((editor.name.clone(), editor.channels.clone()), fields);
        assert_eq!((editor.status.clone(), editor.error), status);
        assert_eq!(editor.scene_token, token);
    }
}

#[test]
fn sidebar_visible_true_activates_the_panel_and_false_only_hides_the_active_one() {
    // ChannelBox is the only panel, so hiding a non-active panel is reachable only with the
    // dock already hidden, and Unchanged arises only from a repeat.
    let mut editor = Editor {
        sidebar_dock: None,
        ..Editor::default()
    };
    assert!(!editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    assert!(matches!(
        editor.apply_operation(sidebar_op(false)),
        Ok(OperationReply::Unchanged)
    ));
    assert_eq!(editor.sidebar_dock, None);
    assert!(matches!(
        editor.apply_operation(sidebar_op(true)),
        Ok(OperationReply::Changed)
    ));
    assert_eq!(editor.sidebar_dock, Some(SidebarPanel::ChannelBox));
    assert!(matches!(
        editor.apply_operation(sidebar_op(false)),
        Ok(OperationReply::Changed)
    ));
    assert_eq!(editor.sidebar_dock, None);
}

#[test]
fn sidebar_native_draft_policy_and_remote_draft_refusal_share_the_typed_setter() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let scene = editor.history.scene().clone();
    editor.channels[0] = "invalid".into();
    assert!(matches!(
        editor.apply_operation(sidebar_op(false)),
        Err(OperationError::InvalidInput(_))
    ));
    editor.native_operation(sidebar_op(false));
    assert!(editor.error);
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.channels[0], "invalid");
    for draft in ["2", "invalid"] {
        editor.channels[0] = draft.into();
        assert!(matches!(
            remote(&mut editor, sidebar_op(false)),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.channels[0], draft);
        assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    }
    editor.channels[0] = "2".into();
    editor.native_operation(sidebar_op(false));
    assert!(!editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    assert!(!editor.has_draft());
    assert_eq!(
        editor.history.scene().nodes()[0].transform().translation[0],
        2.0
    );
    editor.history_step(false);
    assert_eq!(editor.history.scene(), &scene);
    assert!(!editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
}

#[test]
fn sidebar_native_toggle_reports_no_scene_edit_status() {
    let mut editor = Editor {
        status: "sentinel".into(),
        ..Editor::default()
    };
    editor.native_operation(sidebar_op(false));
    assert!(!editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    assert_eq!(editor.status, "sentinel");
    editor.native_operation(sidebar_op(false));
    assert_eq!(editor.status, "sentinel");
}

#[test]
fn sidebar_remote_refuses_stale_token_text_focus_modal_and_close_with_baseline_intact() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let before = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
    let stale = editor.remote_operation(
        RemoteRequest {
            expected_scene_token: Some(Uuid::new_v4()),
            operation: sidebar_op(false),
        },
        &egui::Context::default(),
    );
    assert!(matches!(stale, Err(OperationError::StaleScene)));
    assert_eq!(
        serde_json::to_value(editor.inspect(false).unwrap()).unwrap(),
        before
    );
    let context = egui::Context::default();
    context.memory_mut(|memory| memory.request_focus(egui::Id::new("node_name")));
    frame(&mut editor, &context, vec![]);
    assert!(text_edit_focused(&context));
    let focused = editor.remote_operation(
        RemoteRequest {
            expected_scene_token: Some(editor.scene_token),
            operation: sidebar_op(false),
        },
        &context,
    );
    assert!(matches!(focused, Err(OperationError::BusyEditing)));
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    editor.new_document().unwrap();
    assert!(editor.files.is_modal());
    let modal = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
    assert!(matches!(
        remote(&mut editor, sidebar_op(false)),
        Err(OperationError::BusyEditing)
    ));
    assert_eq!(
        serde_json::to_value(editor.inspect(false).unwrap()).unwrap(),
        modal
    );
    let mut closing = Editor::default();
    closing.close_document().unwrap();
    assert!(matches!(
        remote(&mut closing, sidebar_op(false)),
        Err(OperationError::BusyEditing)
    ));
    assert!(closing.sidebar_panel_visible(SidebarPanel::ChannelBox));
}

#[test]
fn sidebar_native_modal_and_close_refuse_before_committing_drafts() {
    for closing in [false, true] {
        let mut editor = Editor::default();
        if closing {
            editor.close_document().unwrap();
        } else {
            editor.create(Primitive::Cube);
            editor.new_document().unwrap();
        }
        editor.name = "pending draft".into();
        let before = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
        let scene = editor.history.scene().clone();
        assert!(matches!(
            editor.apply_operation(sidebar_op(false)),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(
            serde_json::to_value(editor.inspect(false).unwrap()).unwrap(),
            before
        );
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.name, "pending draft");
    }
}

#[test]
fn sidebar_session_setting_survives_successful_and_canceled_new_and_open() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("empty.aztec");
    azteccraft::document::save_new(&path, &azteccraft::scene::Scene::default()).unwrap();
    let mut editor = Editor::default();
    remote(&mut editor, sidebar_op(false)).unwrap();
    for open in [false, true] {
        let operation = if open {
            Operation::OpenScene { path: path.clone() }
        } else {
            Operation::NewScene
        };
        assert!(matches!(
            remote(&mut editor, operation).unwrap(),
            OperationReply::DocumentApplied { .. }
        ));
        assert!(!editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
        editor.create(Primitive::Cube);
        let scene = editor.history.scene().clone();
        let token = editor.scene_token;
        let operation = if open {
            Operation::OpenScene { path: path.clone() }
        } else {
            Operation::NewScene
        };
        let OperationReply::PendingConfirmation {
            pending_action_id, ..
        } = remote(&mut editor, operation).unwrap()
        else {
            panic!("expected unsaved confirmation");
        };
        remote(
            &mut editor,
            Operation::ResolveFileAction {
                pending_id: Uuid::parse_str(&pending_action_id).unwrap(),
                decision: crate::editor_operations::FileDecision::Cancel,
            },
        )
        .unwrap();
        assert!(!editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
        assert_eq!(editor.scene_token, token);
        assert_eq!(editor.history.scene(), &scene);
        editor.history_step(false);
    }
}

#[test]
fn remote_operations_share_native_scene_selection_history_and_camera() {
    let mut editor = Editor::default();
    let OperationReply::Created { node_id } = remote(
        &mut editor,
        Operation::Create {
            primitive: Primitive::Cube,
            name: Some("remote_cube".into()),
        },
    )
    .unwrap() else {
        panic!("expected created object")
    };
    let id = NodeId::from_decimal(&node_id).unwrap();
    assert_eq!(editor.selected, Some(id));
    remote(
        &mut editor,
        Operation::Rename {
            id,
            name: "edited".into(),
        },
    )
    .unwrap();
    let transform = NativeTransform {
        translation: [4.0, 2.0, 1.0],
        ..Default::default()
    };
    let input = editor.transform_input(transform).unwrap();
    assert!(matches!(
        remote(
            &mut editor,
            Operation::SetTransform {
                id,
                transform: input
            }
        )
        .unwrap(),
        OperationReply::Changed
    ));
    assert!(matches!(
        remote(
            &mut editor,
            Operation::SetTransform {
                id,
                transform: input
            }
        )
        .unwrap(),
        OperationReply::Unchanged
    ));
    remote(&mut editor, Operation::FrameSelected).unwrap();
    assert!(editor.camera.target().distance(Vec3::new(4.0, 2.0, 1.0)) < 0.01);
    remote(
        &mut editor,
        Operation::Select {
            node: None,
            instance_path: None,
        },
    )
    .unwrap();
    assert_eq!(editor.selected, None);
    remote(
        &mut editor,
        Operation::Select {
            node: Some(id),
            instance_path: None,
        },
    )
    .unwrap();
    assert_eq!(editor.name, "edited");
    editor.history_step(false);
    assert_eq!(
        editor.history.scene().node(id).unwrap().transform(),
        NativeTransform::default()
    );
    remote(&mut editor, Operation::Redo).unwrap();
    assert_eq!(
        editor.history.scene().node(id).unwrap().transform(),
        transform
    );
    editor.channels[0] = "6".into();
    editor.create(Primitive::Plane);
    assert_eq!(
        editor
            .history
            .scene()
            .node(id)
            .unwrap()
            .transform()
            .translation[0],
        6.0
    );
    let OperationReply::Scene { inspection } = remote(&mut editor, Operation::Inspect).unwrap()
    else {
        panic!("expected inspection")
    };
    assert_eq!(inspection.nodes.len(), 2);
    assert_eq!(inspection.nodes[0].id, node_id);
    assert_eq!(inspection.nodes[0].transform.translation[0], 6.0);
    assert!(inspection.dirty && inspection.can_undo);
    assert!(!inspection.has_draft);
    remote(&mut editor, Operation::Delete(id)).unwrap();
    remote(&mut editor, Operation::Undo).unwrap();
    assert_eq!(editor.history.scene().node(id).unwrap().name(), "edited");
}

#[test]
fn remote_busy_and_stale_denials_preserve_human_state() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let before = editor.history.scene().clone();
    let camera = editor.camera.eye();
    for draft in ["2", "nan"] {
        editor.channels[0] = draft.into();
        assert!(matches!(
            remote(&mut editor, Operation::Delete(id)),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(editor.channels[0], draft);
        assert_eq!(editor.selected, Some(id));
        assert_eq!(editor.camera.eye(), camera);
        assert!(editor.history.can_undo());
        assert!(!editor.history.can_redo());
    }
    let old = Uuid::new_v4();
    assert!(matches!(
        editor.remote_operation(
            RemoteRequest {
                expected_scene_token: Some(old),
                operation: Operation::Undo,
            },
            &egui::Context::default()
        ),
        Err(OperationError::StaleScene)
    ));
    assert_eq!(editor.channels[0], "nan");
    editor.sync_fields();
    let ctx = egui::Context::default();
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("node_name")));
    assert!(matches!(
        editor.remote_operation(
            RemoteRequest {
                expected_scene_token: Some(editor.scene_token),
                operation: Operation::Delete(id),
            },
            &ctx
        ),
        Err(OperationError::BusyEditing)
    ));
    editor.request_new();
    assert!(matches!(
        remote(&mut editor, Operation::Undo),
        Err(OperationError::BusyEditing)
    ));
    let OperationReply::Scene { inspection } = remote(&mut editor, Operation::Inspect).unwrap()
    else {
        panic!("expected inspection")
    };
    assert!(inspection.modal);
    assert_eq!(editor.history.scene(), &before);
}

#[test]
fn remote_errors_and_noop_do_not_change_selection_or_history() {
    let mut editor = Editor::default();
    assert!(matches!(
        remote(&mut editor, Operation::Undo).unwrap(),
        OperationReply::Unchanged
    ));
    assert!(matches!(
        remote(&mut editor, Operation::FrameSelected),
        Err(OperationError::NoSelection)
    ));
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let before = editor.history.scene().clone();
    let missing = NodeId::from_decimal("999").unwrap();
    assert!(matches!(
        remote(
            &mut editor,
            Operation::Select {
                node: Some(missing),
                instance_path: None
            }
        ),
        Err(OperationError::Scene(_))
    ));
    assert!(matches!(
        remote(
            &mut editor,
            Operation::Rename {
                id,
                name: "".into()
            }
        ),
        Err(OperationError::Scene(_))
    ));
    let transform = TransformInput {
        scale: [f64::INFINITY; 3],
        translation: [0.0; 3],
        rotation_degrees: [0.0; 3],
    };
    assert!(matches!(
        remote(&mut editor, Operation::SetTransform { id, transform }),
        Err(OperationError::Scene(_))
    ));
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.selected, Some(id));
    editor.history_step(false);
    assert!(editor.history.scene().nodes().is_empty());
}

#[test]
fn remote_guard_distinguishes_rendered_buttons_from_name_and_channel_text_focus() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        editor.render(ui);
        ui.button("Focusable native control").request_focus();
    });
    output.textures_delta.clear();
    assert!(ctx.memory(|memory| memory.focused().is_some()));
    let OperationReply::Scene { inspection } = editor
        .remote_operation(
            RemoteRequest {
                expected_scene_token: None,
                operation: Operation::Inspect,
            },
            &ctx,
        )
        .unwrap()
    else {
        panic!("expected inspection")
    };
    assert!(!inspection.text_focused);
    editor
        .remote_operation(
            RemoteRequest {
                expected_scene_token: Some(editor.scene_token),
                operation: Operation::Rename {
                    id,
                    name: "permitted".into(),
                },
            },
            &ctx,
        )
        .unwrap();
    assert_eq!(editor.history.scene().node(id).unwrap().name(), "permitted");
    let before = editor.history.scene().clone();
    for text_id in [name_edit_id(), channel_edit_id(0)] {
        ctx.memory_mut(|memory| memory.request_focus(text_id));
        frame(&mut editor, &ctx, vec![]);
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(text_id));
        let OperationReply::Scene { inspection } = editor
            .remote_operation(
                RemoteRequest {
                    expected_scene_token: None,
                    operation: Operation::Inspect,
                },
                &ctx,
            )
            .unwrap()
        else {
            panic!("expected inspection")
        };
        assert!(inspection.text_focused);
        assert!(matches!(
            editor.remote_operation(
                RemoteRequest {
                    expected_scene_token: Some(editor.scene_token),
                    operation: Operation::Undo,
                },
                &ctx
            ),
            Err(OperationError::BusyEditing)
        ));
        assert_eq!(editor.history.scene(), &before);
    }
}

#[test]
fn typed_extrusion_uses_native_history_and_preserves_pending_edits_on_error() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let shape = editor.history.scene().shapes()[0].id();
    editor.channels[0] = "2".into();
    let target = FaceTarget {
        transform: id,
        shape,
        path: vec![
            editor.history.scene().node(id).unwrap().transform_id(),
            shape,
        ],
        face_index: 0,
    };
    editor.native_operation(Operation::ExtrudeFace {
        target: target.clone(),
        distance: 0.0,
    });
    assert!(!editor.error);
    assert_eq!(
        editor
            .history
            .scene()
            .shape(shape)
            .unwrap()
            .mesh()
            .point_count(),
        12
    );
    assert_eq!(
        editor
            .history
            .scene()
            .node(id)
            .unwrap()
            .transform()
            .translation[0],
        2.0
    );
    editor.history_step(false);
    assert_eq!(
        editor
            .history
            .scene()
            .shape(shape)
            .unwrap()
            .mesh()
            .point_count(),
        8
    );
    let before = editor.history.scene().clone();
    editor.channels[0] = "nan".into();
    editor.native_operation(Operation::ExtrudeFace {
        target: target.clone(),
        distance: 1.0,
    });
    assert!(editor.error);
    assert_eq!(editor.channels[0], "nan");
    assert_eq!(editor.history.scene(), &before);
    assert!(editor.history.can_redo());
    editor.sync_fields();
    editor.native_operation(Operation::ExtrudeFace {
        target: target.clone(),
        distance: f64::NAN,
    });
    assert!(editor.error);
    assert_eq!(editor.history.scene(), &before);
    assert!(editor.history.can_redo());
    editor.native_operation(Operation::ExtrudeFace {
        target: FaceTarget {
            face_index: 99,
            ..target
        },
        distance: 1.0,
    });
    assert!(editor.error);
    assert_eq!(editor.history.scene(), &before);
    assert!(editor.history.can_redo());
    editor.history_step(true);
    assert_eq!(
        editor
            .history
            .scene()
            .shape(shape)
            .unwrap()
            .mesh()
            .point_count(),
        12
    );
}

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    frame_with(editor, ctx, egui::vec2(1440.0, 900.0), 1.0, events)
}

/// `size` is in points; `ppp` is the native pixels per point reported to egui.
pub(in crate::app) fn frame_with(
    editor: &mut Editor,
    ctx: &egui::Context,
    size: egui::Vec2,
    ppp: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    if let Some(viewport) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
        viewport.native_pixels_per_point = Some(ppp);
    }
    let mut output = ctx.run_ui(input, |ui| editor.render(ui));
    output.textures_delta.clear();
    output
}

fn face_viewport_rect(output: &egui::FullOutput) -> egui::Rect {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == Color32::from_gray(31) => Some(rect.rect),
            _ => None,
        })
        .max_by(|a, b| a.area().total_cmp(&b.area()))
        .unwrap()
}

fn face_text_position(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text_shape) if text_shape.galley.job.text == text => {
                Some(text_shape.pos + text_shape.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "missing actual text: {text}; rendered labels: {:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            )
        })
}

fn face_pointer_click(
    editor: &mut Editor,
    ctx: &egui::Context,
    position: egui::Pos2,
    modifiers: egui::Modifiers,
    release_keys: Vec<egui::Event>,
) -> egui::FullOutput {
    frame(
        editor,
        ctx,
        vec![
            egui::Event::ModifiersChanged(modifiers),
            egui::Event::PointerMoved(position),
        ],
    );
    for pressed in [true, false] {
        let mut events = vec![
            egui::Event::ModifiersChanged(modifiers),
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
        ];
        if !pressed {
            events.extend(release_keys.iter().cloned());
        }
        frame(editor, ctx, events);
    }
    frame(editor, ctx, vec![])
}

#[test]
fn face_native_keys_coalesce_before_same_frame_click_and_preserve_authoring_state() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    editor.camera =
        Camera::new(Vec3::ZERO, 0.0, 0.0, 5.0, 45_f32.to_radians(), 0.01, 100.0).unwrap();
    let before = editor.history.scene().clone();
    let token = editor.scene_token;
    let ctx = egui::Context::default();
    let output = frame(&mut editor, &ctx, vec![]);
    let center = face_viewport_rect(&output).center();
    face_pointer_click(
        &mut editor,
        &ctx,
        center,
        egui::Modifiers::NONE,
        vec![key(Key::F8), key(Key::F11), key(Key::F11)],
    );
    assert_eq!(editor.selection_mode, SelectionMode::Face);
    assert_eq!(editor.remembered_face.as_ref().unwrap().face_index, 1);
    let target = editor.remembered_face.clone();
    frame(&mut editor, &ctx, vec![key(Key::F8)]);
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert_eq!(editor.remembered_face, target);
    frame(&mut editor, &ctx, vec![key(Key::F11)]);
    assert_eq!(editor.selection_mode, SelectionMode::Face);
    assert_eq!(editor.remembered_face, target);
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.scene_token, token);
    assert!(editor.is_dirty());
    editor.history_step(false);
    assert!(editor.history.scene().nodes().is_empty());
}

#[test]
fn face_native_modified_click_and_delete_never_fall_back_to_object_mutation() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    editor.camera =
        Camera::new(Vec3::ZERO, 0.0, 0.0, 5.0, 45_f32.to_radians(), 0.01, 100.0).unwrap();
    let ctx = egui::Context::default();
    let output = frame(&mut editor, &ctx, vec![key(Key::F11)]);
    let rect = face_viewport_rect(&output);
    face_pointer_click(
        &mut editor,
        &ctx,
        rect.center(),
        egui::Modifiers::NONE,
        vec![],
    );
    let baseline = (
        editor.history.scene().clone(),
        editor.selected,
        editor.selected_path.clone(),
        editor.remembered_face.clone(),
        editor.camera.clone(),
        editor.scene_token,
    );
    for modifiers in [egui::Modifiers::SHIFT, egui::Modifiers::CTRL] {
        face_pointer_click(
            &mut editor,
            &ctx,
            rect.left_top() + egui::vec2(20.0, 20.0),
            modifiers,
            vec![],
        );
        assert!(editor.error && editor.status.contains("unsupported"));
        assert_eq!(
            (
                editor.history.scene().clone(),
                editor.selected,
                editor.selected_path.clone(),
                editor.remembered_face.clone(),
                editor.camera.clone(),
                editor.scene_token
            ),
            baseline
        );
    }
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            key(Key::Delete),
        ],
    );
    assert!(editor.error && editor.status.contains("Face deletion is unsupported"));
    assert_eq!(editor.history.scene(), &baseline.0);
    assert_eq!(editor.remembered_face, baseline.3);
    face_pointer_click(
        &mut editor,
        &ctx,
        rect.left_top() + egui::vec2(20.0, 20.0),
        egui::Modifiers::NONE,
        vec![],
    );
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.selected, baseline.1);
    assert_eq!(editor.selected_path, baseline.2);
    editor
        .remote_operation(
            RemoteRequest {
                expected_scene_token: Some(editor.scene_token),
                operation: Operation::Delete(baseline.1.unwrap()),
            },
            &ctx,
        )
        .unwrap();
    assert!(editor.history.scene().nodes().is_empty());
}

#[test]
fn face_native_mode_waits_for_current_text_and_rejects_invalid_drafts() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let before = editor.history.scene().clone();
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(name_edit_id()));
    frame(&mut editor, &ctx, vec![]);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Text("draft".into()), key(Key::F11)],
    );
    assert!(editor.has_draft());
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert_eq!(editor.history.scene(), &before);
    frame(&mut editor, &ctx, vec![key(Key::Escape)]);
    editor.channels[0] = "nan".into();
    editor.channel_intents[0] = true;
    frame(&mut editor, &ctx, vec![key(Key::F11)]);
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert_eq!(editor.channels[0], "nan");
    assert_eq!(editor.history.scene(), &before);
    assert!(editor.error);
}

#[test]
fn face_native_closing_editor_refuses_mode_intents_without_selection_changes() {
    let mut editor = Editor::default();
    let before = editor.history.scene().clone();
    let token = editor.scene_token;
    editor.close_document().unwrap();
    assert!(editor.files.is_closing());
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![key(Key::F11)]);
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert_eq!(editor.selected, None);
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.scene_token, token);
}

#[test]
fn vertex_and_edge_modes_refuse_native_delete_and_frame_without_a_component() {
    for (mode, noun) in [
        (SelectionMode::Vertex, "Vertex"),
        (SelectionMode::Edge, "Edge"),
    ] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        editor.selection_mode = mode;
        let (before, token) = (editor.history.scene().clone(), editor.scene_token);
        editor.native_operation(Operation::Delete(editor.selected.unwrap()));
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(editor.scene_token, token);
        assert!(editor.error);
        assert_eq!(
            editor.status,
            format!("{noun} deletion is unsupported; switch to Object or Hierarchy selection")
        );
        let camera = editor.camera.clone();
        assert!(matches!(
            editor.apply_operation(Operation::FrameSelected),
            Err(OperationError::NoSelection)
        ));
        assert_eq!(editor.camera, camera);
    }
}

#[test]
fn face_native_pending_mode_prevents_keyboard_and_channel_object_delete() {
    for button in [false, true] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let before = editor.history.scene().clone();
        let token = editor.scene_token;
        let ctx = egui::Context::default();
        let output = frame(&mut editor, &ctx, vec![]);
        if button {
            let position = face_text_position(&output, "Delete selected");
            face_pointer_click(
                &mut editor,
                &ctx,
                position,
                egui::Modifiers::NONE,
                vec![key(Key::F11)],
            );
        } else {
            frame(&mut editor, &ctx, vec![key(Key::F11), key(Key::Delete)]);
        }
        assert_eq!(editor.selection_mode, SelectionMode::Face);
        assert_eq!(editor.history.scene(), &before, "button={button}");
        assert_eq!(editor.scene_token, token);
        assert!(editor.error && editor.status.contains("Face deletion is unsupported"));
    }
}

#[test]
fn face_native_edit_menu_delete_keeps_face_mode_unsupported_and_object_mode_working() {
    for face_mode in [false, true] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let before = editor.history.scene().clone();
        let ctx = egui::Context::default();
        let output = frame(
            &mut editor,
            &ctx,
            if face_mode {
                vec![key(Key::F11)]
            } else {
                vec![]
            },
        );
        let edit = face_text_position(&output, "Edit");
        let output = face_pointer_click(&mut editor, &ctx, edit, egui::Modifiers::NONE, vec![]);
        let delete = face_text_position(&output, "Delete");
        let selection = (
            editor.selection_mode,
            editor.selected,
            editor.selected_path.clone(),
            editor.remembered_face.clone(),
            editor.camera.clone(),
            editor.scene_token,
        );
        face_pointer_click(&mut editor, &ctx, delete, egui::Modifiers::NONE, vec![]);
        if face_mode {
            assert_eq!(editor.history.scene(), &before);
            assert_eq!(
                (
                    editor.selection_mode,
                    editor.selected,
                    editor.selected_path.clone(),
                    editor.remembered_face.clone(),
                    editor.camera.clone(),
                    editor.scene_token
                ),
                selection
            );
        } else {
            assert!(editor.history.scene().nodes().is_empty());
            editor.history_step(false);
            assert_eq!(editor.history.scene(), &before);
        }
    }
}

#[test]
fn face_native_event_modifiers_survive_later_modifier_changes() {
    for (event_modifiers, final_modifiers, expected) in [
        (
            egui::Modifiers::SHIFT,
            egui::Modifiers::NONE,
            SelectionMode::Object,
        ),
        (
            egui::Modifiers::NONE,
            egui::Modifiers::SHIFT,
            SelectionMode::Face,
        ),
    ] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let before = editor.history.scene().clone();
        let ctx = egui::Context::default();
        frame(&mut editor, &ctx, vec![]);
        frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::Key {
                    key: Key::F11,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: event_modifiers,
                },
                egui::Event::ModifiersChanged(final_modifiers),
            ],
        );
        assert_eq!(editor.selection_mode, expected);
        assert_eq!(editor.history.scene(), &before);
    }
}

#[test]
fn face_native_modal_cancel_does_not_replay_blocked_mode_intent() {
    for (button, open) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let ctx = egui::Context::default();
        frame(&mut editor, &ctx, vec![]);
        let directory = tempfile::tempdir().unwrap();
        if open {
            let path = directory.path().join("replacement.aztec");
            let mut candidate = azteccraft::scene::Scene::default();
            candidate.create(Primitive::Plane, "replacement").unwrap();
            azteccraft::document::save_new(&path, &candidate).unwrap();
            editor
                .apply_operation(Operation::OpenScene { path })
                .unwrap();
        } else {
            editor.new_document().unwrap();
        }
        frame(&mut editor, &ctx, vec![]);
        let output = frame(&mut editor, &ctx, vec![]);
        assert!(editor.files.is_modal());
        let baseline = (
            editor.selection_mode,
            editor.selected,
            editor.selected_path.clone(),
            editor.remembered_face.clone(),
            editor.history.scene().clone(),
            editor.camera.clone(),
            editor.scene_token,
            editor.is_dirty(),
            editor.history.can_undo(),
            editor.history.can_redo(),
        );
        let draft = (
            editor.name.clone(),
            editor.channels.clone(),
            editor.name_intent,
            editor.channel_intents,
        );
        if button {
            let cancel = face_text_position(&output, "Cancel");
            face_pointer_click(
                &mut editor,
                &ctx,
                cancel,
                egui::Modifiers::NONE,
                vec![key(Key::F11)],
            );
        } else {
            frame(&mut editor, &ctx, vec![key(Key::Escape), key(Key::F11)]);
        }
        assert!(!editor.files.is_modal());
        assert_eq!(
            (
                editor.selection_mode,
                editor.selected,
                editor.selected_path.clone(),
                editor.remembered_face.clone(),
                editor.history.scene().clone(),
                editor.camera.clone(),
                editor.scene_token,
                editor.is_dirty(),
                editor.history.can_undo(),
                editor.history.can_redo()
            ),
            baseline
        );
        assert_eq!(
            (
                editor.name.clone(),
                editor.channels.clone(),
                editor.name_intent,
                editor.channel_intents
            ),
            draft
        );
    }
}

#[test]
fn face_native_ambiguous_primary_frame_preserves_selection_scene_and_draft() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    editor.camera =
        Camera::new(Vec3::ZERO, 0.0, 0.0, 5.0, 45_f32.to_radians(), 0.01, 100.0).unwrap();
    let ctx = egui::Context::default();
    let output = frame(&mut editor, &ctx, vec![key(Key::F11)]);
    let point = face_viewport_rect(&output).center();
    face_pointer_click(&mut editor, &ctx, point, egui::Modifiers::NONE, vec![]);
    assert_eq!(editor.remembered_face.as_ref().unwrap().face_index, 1);
    editor.native_operation(Operation::Rename {
        id: editor.selected.unwrap(),
        name: "redo-payload".into(),
    });
    editor.history_step(false);
    assert!(editor.history.can_redo());
    editor.name = "pending-human-name".into();
    editor.name_intent = true;
    let baseline = (
        editor.selection_mode,
        editor.selected,
        editor.selected_path.clone(),
        editor.remembered_face.clone(),
        editor.history.scene().clone(),
        editor.camera.clone(),
        editor.scene_token,
        editor.history.can_undo(),
        editor.history.can_redo(),
        editor.is_dirty(),
    );
    let draft = (
        editor.name.clone(),
        editor.channels.clone(),
        editor.name_intent,
        editor.channel_intents,
    );
    let end = point + egui::vec2(36.0, 0.0);
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::SHIFT),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerMoved(end),
            egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(editor.error && editor.status.contains("Ambiguous coalesced primary input"));
    assert_eq!(
        (
            editor.selection_mode,
            editor.selected,
            editor.selected_path.clone(),
            editor.remembered_face.clone(),
            editor.history.scene().clone(),
            editor.camera.clone(),
            editor.scene_token,
            editor.history.can_undo(),
            editor.history.can_redo(),
            editor.is_dirty()
        ),
        baseline
    );
    assert_eq!(
        (
            editor.name.clone(),
            editor.channels.clone(),
            editor.name_intent,
            editor.channel_intents
        ),
        draft
    );
}

#[test]
fn face_native_menu_explicit_face_beats_toggle_and_modal_replacement_drops_intent() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let before = editor.history.scene().clone();
    let ctx = egui::Context::default();
    let output = frame(&mut editor, &ctx, vec![]);
    let select = face_text_position(&output, "Select");
    let output = face_pointer_click(&mut editor, &ctx, select, egui::Modifiers::NONE, vec![]);
    let components = face_text_position(&output, "Components");
    let output = face_pointer_click(&mut editor, &ctx, components, egui::Modifiers::NONE, vec![]);
    let face = face_text_position(&output, "Face   F11");
    face_pointer_click(
        &mut editor,
        &ctx,
        face,
        egui::Modifiers::NONE,
        vec![key(Key::F8)],
    );
    assert_eq!(editor.selection_mode, SelectionMode::Face);
    assert_eq!(editor.history.scene(), &before);
    editor
        .apply_operation(Operation::SetSelectionMode(SelectionMode::Object))
        .unwrap();
    editor.new_document().unwrap();
    let token = editor.scene_token;
    frame(&mut editor, &ctx, vec![key(Key::F11)]);
    assert!(editor.files.is_modal());
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    let output = frame(&mut editor, &ctx, vec![]);
    let discard = face_text_position(&output, "Discard");
    face_pointer_click(
        &mut editor,
        &ctx,
        discard,
        egui::Modifiers::NONE,
        vec![key(Key::F11)],
    );
    assert_ne!(editor.scene_token, token);
    assert!(!editor.files.is_modal());
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert!(editor.history.scene().nodes().is_empty());
}

#[test]
fn face_native_popup_intercepts_combined_menu_activation_and_viewport_click() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    editor.camera =
        Camera::new(Vec3::ZERO, 0.0, 0.0, 5.0, 45_f32.to_radians(), 0.01, 100.0).unwrap();
    let before = editor.history.scene().clone();
    let ctx = egui::Context::default();
    let output = frame(&mut editor, &ctx, vec![]);
    let center = face_viewport_rect(&output).center();
    let select = face_text_position(&output, "Select");
    let output = face_pointer_click(&mut editor, &ctx, select, egui::Modifiers::NONE, vec![]);
    let components = face_text_position(&output, "Components");
    let output = face_pointer_click(&mut editor, &ctx, components, egui::Modifiers::NONE, vec![]);
    let face = face_text_position(&output, "Face   F11");
    let mut focused = false;
    for _ in 0..32 {
        frame(&mut editor, &ctx, vec![key(Key::Tab)]);
        focused = ctx
            .memory(|memory| memory.focused())
            .and_then(|id| ctx.read_response(id))
            .is_some_and(|response| response.rect.contains(face));
        if focused {
            break;
        }
    }
    assert!(
        focused,
        "actual menu Face item must receive keyboard focus before activation"
    );
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert!(egui::Popup::is_any_open(&ctx));
    let selection = (
        editor.selection_mode,
        editor.selected,
        editor.selected_path.clone(),
        editor.remembered_face.clone(),
        editor.camera.clone(),
        editor.scene_token,
    );
    let history = (
        editor.history.can_undo(),
        editor.history.can_redo(),
        editor.is_dirty(),
    );
    let draft = (
        editor.name.clone(),
        editor.channels.clone(),
        editor.name_intent,
        editor.channel_intents,
    );
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            key(Key::Enter),
            key(Key::F8),
            egui::Event::PointerMoved(center),
            egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert_eq!(
        (
            editor.selection_mode,
            editor.selected,
            editor.selected_path.clone(),
            editor.remembered_face.clone(),
            editor.camera.clone(),
            editor.scene_token
        ),
        selection
    );
    assert_eq!(
        (
            editor.history.can_undo(),
            editor.history.can_redo(),
            editor.is_dirty()
        ),
        history
    );
    assert_eq!(
        (
            editor.name.clone(),
            editor.channels.clone(),
            editor.name_intent,
            editor.channel_intents
        ),
        draft
    );
    assert_eq!(editor.history.scene(), &before);
}

#[test]
fn real_escape_event_cancels_channel_and_name_after_egui_clears_focus() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    let before = editor.history.scene().clone();
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(("channel", 0_usize))));
    frame(&mut editor, &ctx, vec![]);
    editor.channels[0] = "3".into();
    frame(&mut editor, &ctx, vec![key(Key::Escape)]);
    assert_eq!(editor.channels[0], "0");
    assert_eq!(editor.history.scene(), &before);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("node_name")));
    frame(&mut editor, &ctx, vec![]);
    editor.name = "discarded".into();
    frame(&mut editor, &ctx, vec![key(Key::Escape)]);
    assert_eq!(editor.history.scene().node(id).unwrap().name(), "pCube1");
    editor.history_step(false);
    assert!(editor.history.scene().nodes().is_empty());
}

#[test]
fn real_grid_button_dispatches_current_state_and_disables_for_modal_and_close() {
    for blocked in [0, 1, 2] {
        let context = egui::Context::default();
        let mut editor = Editor {
            renderer: Some(crate::viewport_renderer::tests::renderer_handle()),
            ..Editor::default()
        };
        if blocked == 1 {
            editor.create(Primitive::Cube);
            editor.new_document().unwrap();
        } else if blocked == 2 {
            editor.close_document().unwrap();
        }
        let scene = editor.history.scene().clone();
        let token = editor.scene_token;
        let mut position = None;
        let mut run = |editor: &mut Editor, events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| editor.viewport(ui, token, None, None, false, true),
            );
            for shape in &output.shapes {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.job.text == "Grid"
                {
                    position = Some(text.pos + text.galley.size() * 0.5);
                }
            }
            output.textures_delta.clear();
            position
        };
        let point = run(&mut editor, vec![]).unwrap();
        for expected in [blocked != 0, true] {
            run(&mut editor, vec![egui::Event::PointerMoved(point)]);
            for pressed in [true, false] {
                run(
                    &mut editor,
                    vec![egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            assert_eq!(editor.grid_visible, expected);
            assert_eq!(editor.inspect(false).unwrap().grid_visible, expected);
            assert_eq!(editor.scene_token, token);
            assert_eq!(editor.history.scene(), &scene);
        }
    }
}

#[test]
fn real_channel_text_keeps_f64_precision_and_untouched_slots_in_one_undo() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    editor
        .history
        .execute(Command::SetTransform {
            id,
            transform: NativeTransform {
                translation: [0.0, 1.0000000000000004, 0.0],
                ..Default::default()
            },
        })
        .unwrap();
    editor.sync_fields();
    let before = editor.history.scene().clone();
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(channel_edit_id(0)));
    frame(&mut editor, &ctx, vec![]);
    let command = egui::Modifiers::COMMAND;
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(command),
            egui::Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: command,
            },
            egui::Event::Text("1.0000000000000002".into()),
            key(Key::Enter),
        ],
    );
    let translation = editor
        .history
        .scene()
        .node(id)
        .unwrap()
        .transform()
        .translation;
    assert_eq!(translation[0].to_bits(), 1.0000000000000002_f64.to_bits());
    assert_eq!(translation[1].to_bits(), 1.0000000000000004_f64.to_bits());
    assert!(editor.history.undo());
    assert_eq!(editor.history.scene(), &before);
}

#[test]
fn viewport_click_commits_draft_before_picking_its_new_geometry() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    editor.camera = Camera::default();
    let ctx = egui::Context::default();
    let mut viewport_rect = egui::Rect::NOTHING;
    let run = |editor: &mut Editor, events, rect: &mut egui::Rect| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                editor.viewport(ui, editor.scene_token, None, None, false, true);
                *rect = ui.min_rect();
            },
        );
        output.textures_delta.clear();
    };
    run(&mut editor, vec![], &mut viewport_rect);
    let new_center = glam::Vec3::new(2.0, 0.0, 0.0);
    let ndc = editor
        .camera
        .project(new_center, editor.aspect)
        .unwrap()
        .unwrap();
    let width = viewport_rect.width();
    let height = width / editor.aspect;
    let position = egui::pos2(
        viewport_rect.left() + (ndc.x + 1.0) * 0.5 * width,
        viewport_rect.bottom() - (ndc.y + 1.0) * 0.5 * height,
    );
    run(
        &mut editor,
        vec![egui::Event::PointerMoved(position)],
        &mut viewport_rect,
    );
    editor.channels[0] = "2".into();
    editor.channel_intents[0] = true;
    for pressed in [true, false] {
        run(
            &mut editor,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut viewport_rect,
        );
    }
    assert_eq!(
        editor
            .history
            .scene()
            .node(id)
            .unwrap()
            .transform()
            .translation[0],
        2.0
    );
    assert_eq!(
        editor.selected,
        Some(id),
        "fresh geometry must be hit at the new location"
    );
    assert!(!editor.has_draft());
}

#[test]
fn enter_commits_once_and_selection_commits_draft_to_previous_object() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let old = editor.selected.unwrap();
    editor.create(Primitive::Plane);
    let next = editor.selected.unwrap();
    editor.select(Some(old));
    let ctx = egui::Context::default();
    frame(&mut editor, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(("channel", 0_usize))));
    frame(&mut editor, &ctx, vec![]);
    editor.channels[0] = "2".into();
    frame(&mut editor, &ctx, vec![key(Key::Enter)]);
    assert_eq!(
        editor
            .history
            .scene()
            .node(old)
            .unwrap()
            .transform()
            .translation[0],
        2.0
    );
    editor.history_step(false);
    assert_eq!(
        editor
            .history
            .scene()
            .node(old)
            .unwrap()
            .transform()
            .translation[0],
        0.0
    );
    editor.channels[0] = "4".into();
    editor.select(Some(next));
    assert_eq!(
        editor
            .history
            .scene()
            .node(old)
            .unwrap()
            .transform()
            .translation[0],
        4.0
    );
    assert_eq!(
        editor.history.scene().node(next).unwrap().transform(),
        NativeTransform::default()
    );
    assert_eq!(editor.channels[0], "0");
}

#[test]
fn creation_commits_old_draft_and_invalid_draft_blocks_selection_and_actions() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let old = editor.selected.unwrap();
    editor.channels[1] = "5".into();
    editor.create(Primitive::Plane);
    let next = editor.selected.unwrap();
    assert_eq!(
        editor
            .history
            .scene()
            .node(old)
            .unwrap()
            .transform()
            .translation[1],
        5.0
    );
    assert_eq!(
        editor.history.scene().node(next).unwrap().transform(),
        NativeTransform::default()
    );
    editor.channels[0] = "nan".into();
    let before = editor.history.scene().clone();
    editor.select(Some(old));
    assert_eq!(editor.selected, Some(next));
    editor.create(Primitive::Cube);
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.channels[0], "nan");
    assert!(editor.error);
}

#[test]
fn create_allocates_name_after_committing_pending_rename() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let old = editor.selected.unwrap();
    editor.name = "pCube2".into();
    editor.create(Primitive::Cube);
    let created = editor.selected.unwrap();
    assert_ne!(old, created);
    assert_eq!(editor.history.scene().node(old).unwrap().name(), "pCube2");
    assert_eq!(
        editor.history.scene().node(created).unwrap().name(),
        "pCube1"
    );
}

fn open_ascii(body: &str) -> Editor {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.ma");
    std::fs::write(
        &path,
        format!("//Maya ASCII 2016 scene\nrequires maya \"2016\"; currentUnit -l centimeter -a degree -t film; {body}"),
    )
    .unwrap();
    let mut editor = Editor::default();
    editor
        .apply_operation(Operation::OpenScene { path })
        .unwrap();
    editor
}

fn channel_box_text(editor: &mut Editor) -> (String, Vec<egui::Shape>) {
    let context = egui::Context::default();
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(420.0, 900.0),
            )),
            ..Default::default()
        },
        |ui| editor.channels(ui, &mut false),
    );
    output.textures_delta.clear();
    let text = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("|");
    (text, output.shapes.into_iter().map(|s| s.shape).collect())
}

#[test]
fn channel_box_visibility_row_is_read_only_and_never_defaults_to_on() {
    const CC: &str = "setAttr \".cc\" -type \"nurbsCurve\" 3 1 0 no 3 6 0 0 0 1 1 1 4 0 -0 0 1 2 3 4 -3 2 6 0 -1;";
    for (v, expected) in [
        ("setAttr \".v\" no;", "off"),
        ("setAttr \".v\" yes;", "on"),
        ("", "on"),
    ] {
        let mut editor = open_ascii(&format!(
            "createNode transform -n \"c\"; {v} createNode nurbsCurve -n \"cShape\" -p \"c\"; {CC}"
        ));
        let id = editor.history.scene().nodes()[0].id();
        assert_eq!(
            editor.inspect(false).unwrap().nodes[0].visible,
            Some(expected == "on")
        );
        editor.select(Some(id));
        let before = editor.history.scene().clone();
        let (text, _) = channel_box_text(&mut editor);
        assert!(text.contains("Visibility"), "{text}");
        assert!(text.contains(expected), "{text}");
        assert!(!text.contains("unknown"));
        assert!(!editor.has_draft());
        assert_eq!(editor.history.scene(), &before);
    }
}

mod toolbox_wire {
    use super::*;
    use eframe::egui::accesskit::Role;

    const TOOLBOX: [&str; 7] = [
        "Select",
        "Lasso",
        "Paint Selection",
        "Move",
        "Rotate",
        "Scale",
        "Last Used",
    ];

    struct Control {
        label: String,
        rect: egui::Rect,
        disabled: bool,
        selected: Option<bool>,
    }

    fn controls(output: &egui::FullOutput) -> Vec<Control> {
        let update = output
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("accesskit enabled");
        update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Button)
            .filter_map(|(_, node)| {
                let bounds = node.bounds()?;
                Some(Control {
                    label: node.label()?.to_owned(),
                    rect: egui::Rect::from_min_max(
                        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                    ),
                    disabled: node.is_disabled(),
                    selected: node
                        .toggled()
                        .map(|t| t == eframe::egui::accesskit::Toggled::True),
                })
            })
            .collect()
    }

    fn setup() -> (Editor, egui::Context) {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut editor = Editor {
            renderer: Some(crate::viewport_renderer::tests::renderer_handle()),
            ..Editor::default()
        };
        editor.create(Primitive::Cube);
        (editor, context)
    }

    fn toolbox(output: &egui::FullOutput) -> Vec<Control> {
        let mut found: Vec<Control> = controls(output)
            .into_iter()
            .filter(|c| TOOLBOX.contains(&c.label.as_str()) && c.rect.right() < 100.0)
            .collect();
        found.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
        found
    }

    fn click(editor: &mut Editor, context: &egui::Context, at: egui::Pos2) {
        frame(editor, context, vec![egui::Event::PointerMoved(at)]);
        for pressed in [true, false] {
            frame(
                editor,
                context,
                vec![egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
    }

    fn center_of(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        let found = controls(output)
            .into_iter()
            .find(|c| c.label == label && !c.disabled)
            .unwrap_or_else(|| panic!("enabled control {label}"));
        found.rect.center()
    }

    #[test]
    fn toolbox_is_seven_labelled_icon_buttons_in_documented_order() {
        let (mut editor, context) = setup();
        let output = frame(&mut editor, &context, vec![]);
        let found = toolbox(&output);
        let labels: Vec<&str> = found.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, TOOLBOX);
        for control in &found {
            let live = matches!(control.label.as_str(), "Select" | "Move");
            assert_eq!(control.disabled, !live, "{}", control.label);
        }
        assert_eq!(found[0].selected, Some(true));
        assert_eq!(found[3].selected, Some(false));
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape
                && shape.clip_rect.right() < 100.0
            {
                let t = text.galley.job.text.as_str();
                assert!(
                    !matches!(t, "Move" | "Rotate" | "Scale" | "Select"),
                    "toolbox painted text {t}"
                );
            }
        }
    }

    #[test]
    fn select_and_move_toggle_the_active_tool_and_others_do_nothing() {
        let (mut editor, context) = setup();
        let output = frame(&mut editor, &context, vec![]);
        let found = toolbox(&output);
        let move_at = found[3].rect.center();
        let select_at = found[0].rect.center();
        click(&mut editor, &context, move_at);
        assert_eq!(editor.active_tool, ActiveTool::Move);
        click(&mut editor, &context, select_at);
        assert_eq!(editor.active_tool, ActiveTool::Select);
        let scene = editor.history.scene().clone();
        let token = editor.scene_token;
        for index in [1, 2, 4, 5, 6] {
            click(&mut editor, &context, found[index].rect.center());
            assert_eq!(
                editor.active_tool,
                ActiveTool::Select,
                "{}",
                found[index].label
            );
            assert_eq!(editor.history.scene(), &scene);
            assert_eq!(editor.scene_token, token);
            assert!(!editor.history.can_redo());
        }
    }

    #[test]
    fn hovering_enabled_and_disabled_icons_shows_their_tooltips() {
        for (index, expected) in [
            (0, "Select tool (Q)"),
            (3, "Move tool (W)"),
            (1, "Lasso: not implemented yet"),
            (
                6,
                "Last Used: not implemented yet; dynamic last-used behaviour unsupported",
            ),
        ] {
            let (mut editor, context) = setup();
            let output = frame(&mut editor, &context, vec![]);
            let at = toolbox(&output)[index].rect.center();
            let mut shown = false;
            for step in 1..=6 {
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440.0, 900.0),
                        )),
                        time: Some(f64::from(step)),
                        events: if step == 1 {
                            vec![egui::Event::PointerMoved(at)]
                        } else {
                            vec![]
                        },
                        ..Default::default()
                    },
                    |ui| editor.render(ui),
                );
                output.textures_delta.clear();
                shown |= output.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == expected)
                });
            }
            assert!(shown, "tooltip {expected}");
        }
    }

    fn selected_name(editor: &Editor) -> String {
        editor
            .history
            .scene()
            .node(editor.selected.expect("selection"))
            .unwrap()
            .name()
            .to_owned()
    }

    fn disabled(output: &egui::FullOutput, label: &str) -> bool {
        controls(output)
            .into_iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("control {label}"))
            .disabled
    }

    #[test]
    fn select_and_move_are_disabled_and_inert_while_modal_or_closing() {
        for closing in [false, true] {
            let (mut editor, context) = setup();
            if closing {
                editor.close_document().unwrap();
            } else {
                editor.new_document().unwrap();
            }
            let output = frame(&mut editor, &context, vec![]);
            let found = toolbox(&output);
            for (index, start) in [(0, ActiveTool::Move), (3, ActiveTool::Select)] {
                editor.active_tool = start;
                assert!(found[index].disabled, "{}", found[index].label);
                click(&mut editor, &context, found[index].rect.center());
                assert_eq!(editor.active_tool, start, "{}", found[index].label);
            }
        }
    }

    #[test]
    fn shelf_controls_are_disabled_without_their_preconditions() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut editor = Editor {
            renderer: Some(crate::viewport_renderer::tests::renderer_handle()),
            ..Editor::default()
        };
        let output = frame(&mut editor, &context, vec![]);
        for label in ["Undo", "Redo", "Frame selected  F"] {
            assert!(disabled(&output, label), "{label}");
        }
        for label in ["Cube", "Plane"] {
            assert!(!disabled(&output, label), "{label}");
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("curve.ma");
        std::fs::write(&path, ascii_fixture::specimen()).unwrap();
        editor
            .apply_operation(Operation::OpenScene { path })
            .unwrap();
        let output = frame(&mut editor, &context, vec![]);
        for label in ["Cube", "Plane"] {
            assert!(disabled(&output, label), "{label}");
        }
    }

    #[test]
    fn shelf_buttons_still_create_and_run_history_and_frame() {
        let (mut editor, context) = setup();
        let before = editor.camera.clone();
        let output = frame(&mut editor, &context, vec![]);
        click(
            &mut editor,
            &context,
            center_of(&output, "Frame selected  F"),
        );
        assert_ne!(editor.camera, before);
        let baseline = editor.history.scene().clone();
        let output = frame(&mut editor, &context, vec![]);
        click(&mut editor, &context, center_of(&output, "Plane"));
        assert_ne!(editor.history.scene(), &baseline);
        assert!(selected_name(&editor).contains("Plane"));
        let with_plane = editor.history.scene().clone();
        let output = frame(&mut editor, &context, vec![]);
        click(&mut editor, &context, center_of(&output, "Cube"));
        assert_ne!(editor.history.scene(), &with_plane);
        assert!(selected_name(&editor).contains("Cube"));
        let with_cube = editor.history.scene().clone();
        let output = frame(&mut editor, &context, vec![]);
        click(&mut editor, &context, center_of(&output, "Undo"));
        assert_eq!(editor.history.scene(), &with_plane);
        let output = frame(&mut editor, &context, vec![]);
        click(&mut editor, &context, center_of(&output, "Redo"));
        assert_eq!(editor.history.scene(), &with_cube);
    }
}

#[test]
fn startup_status_names_every_creatable_primitive() {
    assert_eq!(
        Editor::default().status,
        "Ready | Create a cube, plane, sphere, cylinder or cone to start"
    );
}

#[test]
fn create_menu_polygon_sphere_item_creates_a_selectable_framed_movable_sphere() {
    let mut editor = Editor::default();
    let ctx = egui::Context::default();
    let output = frame(&mut editor, &ctx, vec![]);
    let create = face_text_position(&output, "Create");
    let output = face_pointer_click(&mut editor, &ctx, create, egui::Modifiers::NONE, vec![]);
    face_text_position(&output, "Other primitives");
    let sphere = face_text_position(&output, "Polygon sphere");
    face_pointer_click(&mut editor, &ctx, sphere, egui::Modifiers::NONE, vec![]);
    let id = editor.selected.expect("menu item selects the new sphere");
    assert_eq!(editor.history.scene().node(id).unwrap().name(), "pSphere1");
    assert_eq!(editor.history.scene().shapes()[0].mesh().faces().len(), 400);
    assert!(editor.history.can_undo());
    assert!(editor.is_dirty());

    editor
        .apply_operation(Operation::SetSelectionMode(SelectionMode::Face))
        .unwrap();
    let triangle = selected_face(&editor, 0);
    editor
        .apply_operation(Operation::SelectFace {
            target: Some(triangle.clone()),
        })
        .unwrap();
    assert_eq!(editor.remembered_face.as_ref(), Some(&triangle));
    let camera = editor.camera.clone();
    editor.apply_operation(Operation::FrameSelected).unwrap();
    assert_ne!(editor.camera, camera);
    editor
        .apply_operation(Operation::SetSelectionMode(SelectionMode::Object))
        .unwrap();

    editor
        .apply_operation(Operation::SetActiveTool(ActiveTool::Move))
        .unwrap();
    let target = editor.selected_move_target().unwrap();
    editor
        .apply_operation(Operation::BeginMove {
            target,
            axis: azteccraft::native_scene::WorldAxis::X,
        })
        .unwrap();
    let move_id = editor.history.active_manipulation().unwrap().id;
    editor
        .apply_operation(Operation::UpdateMove {
            id: move_id,
            distance: crate::editor_operations::MoveDistance::Centimeters(5.0),
        })
        .unwrap();
    editor
        .apply_operation(Operation::FinishMove {
            id: move_id,
            finish: azteccraft::history::ManipulationFinish::Commit,
        })
        .unwrap();
    editor.history.scene().validate().unwrap();
    assert_eq!(
        editor
            .history
            .scene()
            .node(id)
            .unwrap()
            .transform()
            .translation[0],
        5.0
    );
    let view = ViewScene::from_native(editor.history.scene(), 256 * 1024 * 1024).unwrap();
    assert_eq!(view.instances()[0].faces().len(), 400);
}

#[test]
fn cylinder_and_cone_names_count_up_and_skip_collisions() {
    let mut editor = Editor::default();
    for (primitive, names) in [
        (
            Primitive::Cylinder,
            ["pCylinder1", "pCylinder2", "pCylinder4"],
        ),
        (Primitive::Cone, ["pCone1", "pCone2", "pCone4"]),
    ] {
        let base = names[0].trim_end_matches('1');
        for (expected, taken) in names.iter().zip([None, None, Some(format!("{base}3"))]) {
            if let Some(taken) = taken {
                editor
                    .apply_operation(Operation::Create {
                        primitive: Primitive::Cube,
                        name: Some(taken),
                    })
                    .unwrap();
            }
            editor.create(primitive);
            let id = editor.selected.unwrap();
            assert_eq!(editor.history.scene().node(id).unwrap().name(), *expected);
        }
    }
}

/// A group (translate x = 2) holding a child transform that owns one triangle mesh.
/// Mirrored by `grouped_scene_bytes()` in src/mcp_tests/official_client/selection.rs; this module builds without the mcp feature, so the builder cannot be shared.
fn group_editor() -> Editor {
    nested_group_editor(&[b"child"])
}

/// Like `group_editor()` but with the transform chain group > mid > child above the shape.
fn deep_group_editor() -> Editor {
    nested_group_editor(&[b"mid", b"child"])
}

/// A group (translate x = 2) holding the chain `names` of transforms; the last owns one triangle mesh.
fn nested_group_editor(names: &[&[u8]]) -> Editor {
    use material_wire as wire;
    let mut parent: &[u8] = b"group";
    let mut chain = wire::group(
        b"XFRM",
        &[
            wire::creation(b"group", None),
            wire::triple(b"t", [2.0, 0.0, 0.0]),
        ]
        .concat(),
        4,
    );
    for name in names {
        chain.extend(wire::group(b"XFRM", &wire::creation(name, Some(parent)), 4));
        parent = name;
    }
    chain.extend(wire::group(
        b"DMSH",
        &[
            wire::creation(b"shape", Some(parent)),
            wire::chunk(b"MESH", &wire::mesh_payload(&[0, 1, 0x6000_0002]), 8),
        ]
        .concat(),
        4,
    ));
    let scene = azteccraft::native_scene::binary::decode(wire::scene(&chain)).unwrap();
    Editor {
        history: History::new(scene),
        ..Editor::default()
    }
}

/// Clicks the viewport at the projected mean of the first instance's world vertices.
fn click_first_instance(editor: &mut Editor) {
    editor.camera = Camera::default();
    let view = ViewScene::from_native(editor.history.scene(), 1024 * 1024).unwrap();
    let vertices = view.instances()[0].world_vertices();
    let center = vertices.iter().copied().sum::<glam::Vec3>() / vertices.len() as f32;
    assert!((center.x - 2.0).abs() < 0.5, "world x {}", center.x);
    click_world_point(editor, center);
}

/// Clicks the viewport at the projection of `point` through the editor's current camera.
fn click_world_point(editor: &mut Editor, point: glam::Vec3) {
    let ctx = egui::Context::default();
    let run = |editor: &mut Editor, events: Vec<egui::Event>| {
        let mut rect = egui::Rect::NOTHING;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                editor.viewport(ui, editor.scene_token, None, None, false, true);
                rect = ui.min_rect();
            },
        );
        output.textures_delta.clear();
        rect
    };
    let rect = run(editor, vec![]);
    let ndc = editor
        .camera
        .project(point, editor.aspect)
        .unwrap()
        .unwrap();
    let width = rect.width();
    let height = width / editor.aspect;
    let position = egui::pos2(
        rect.left() + (ndc.x + 1.0) * 0.5 * width,
        rect.bottom() - (ndc.y + 1.0) * 0.5 * height,
    );
    run(editor, vec![egui::Event::PointerMoved(position)]);
    for pressed in [true, false] {
        run(
            editor,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

#[test]
fn hierarchy_click_on_a_descendant_selects_the_root_and_object_click_the_leaf() {
    let mut editor = group_editor();
    let paths = editor.history.scene().transform_paths().unwrap();
    let (root, child) = (&paths[0], &paths[1]);
    assert_eq!((root.path.len(), child.path.len()), (1, 2));
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    click_first_instance(&mut editor);
    assert_eq!(editor.selected, Some(root.node));
    assert_eq!(editor.selected_path, root.path);
    // Path containment only; painting is asserted in viewport_renderer::tests.
    let view = ViewScene::from_native(editor.history.scene(), 1024 * 1024).unwrap();
    assert!(
        view.instances()[0]
            .path()
            .starts_with(&editor.selected_path)
    );

    let mut editor = group_editor();
    click_first_instance(&mut editor);
    assert_eq!(editor.selected, Some(child.node));
    assert_eq!(editor.selected_path, child.path);
}

#[test]
fn hierarchy_click_in_a_three_level_chain_selects_the_root_not_the_parent() {
    let mut editor = deep_group_editor();
    let paths = editor.history.scene().transform_paths().unwrap();
    let (root, child) = (&paths[0], &paths[2]);
    assert_eq!(
        (root.path.len(), paths[1].path.len(), child.path.len()),
        (1, 2, 3)
    );
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    click_first_instance(&mut editor);
    assert_eq!(editor.selected, Some(root.node));
    assert_eq!(editor.selected_path, root.path);

    let mut editor = deep_group_editor();
    click_first_instance(&mut editor);
    assert_eq!(editor.selected, Some(child.node));
    assert_eq!(editor.selected_path, child.path);
}

#[test]
fn delete_of_a_group_is_a_typed_error_without_mutation_in_hierarchy_mode() {
    let mut editor = group_editor();
    let root = editor.history.scene().transform_paths().unwrap()[0].node;
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    editor.native_operation(Operation::Select {
        node: Some(root),
        instance_path: None,
    });
    let scene = editor.history.scene().clone();
    assert!(remote(&mut editor, Operation::Delete(root)).is_err());
    assert_eq!(editor.history.scene(), &scene);
}

#[test]
fn hierarchy_delete_works_on_a_flat_scene_and_select_face_is_refused() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    assert!(remote(&mut editor, Operation::SelectFace { target: None }).is_err());
    editor.native_operation(Operation::Delete(id));
    assert!(!editor.error);
    assert!(editor.history.scene().nodes().is_empty());
}

#[test]
fn selecting_in_hierarchy_mode_clears_the_remembered_face() {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let target = selected_face(&editor, 0);
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Face));
    editor.native_operation(Operation::SelectFace {
        target: Some(target.clone()),
    });
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    assert_eq!(editor.remembered_face.as_ref(), Some(&target));
    editor.native_operation(Operation::Select {
        node: None,
        instance_path: None,
    });
    assert_eq!(editor.remembered_face, None);
}

fn button_disabled(output: &egui::FullOutput, label: &str) -> bool {
    let update = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit enabled");
    let matches: Vec<bool> = update
        .nodes
        .iter()
        .filter(|(_, node)| {
            node.role() == eframe::egui::accesskit::Role::Button && node.label() == Some(label)
        })
        .map(|(_, node)| node.is_disabled())
        .collect();
    assert_eq!(matches.len(), 1, "button {label:?} found {matches:?}");
    matches[0]
}

#[test]
fn delete_controls_are_enabled_in_hierarchy_mode_and_disabled_in_component_modes() {
    for mode in [
        SelectionMode::Hierarchy,
        SelectionMode::Face,
        SelectionMode::Vertex,
        SelectionMode::Edge,
    ] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        editor.native_operation(Operation::SetSelectionMode(mode));
        assert!(editor.selected.is_some());
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = frame(&mut editor, &ctx, vec![]);
        let expect_disabled = mode.is_component();
        assert_eq!(
            button_disabled(&output, "Delete selected"),
            expect_disabled,
            "channel box, {mode:?}"
        );
        let edit = face_text_position(&output, "Edit");
        let output = face_pointer_click(&mut editor, &ctx, edit, egui::Modifiers::NONE, vec![]);
        assert_eq!(
            button_disabled(&output, "Delete"),
            expect_disabled,
            "edit menu, {mode:?}"
        );
    }
}

#[test]
fn create_menu_cylinder_and_cone_items_create_selectable_framed_movable_solids() {
    for (label, name, faces, cap) in [
        ("Polygon cylinder", "pCylinder1", 22, 20),
        ("Polygon cone", "pCone1", 21, 20),
    ] {
        let mut editor = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut editor, &ctx, vec![]);
        let create = face_text_position(&output, "Create");
        let output = face_pointer_click(&mut editor, &ctx, create, egui::Modifiers::NONE, vec![]);
        face_text_position(&output, "Other primitives");
        let item = face_text_position(&output, label);
        face_pointer_click(&mut editor, &ctx, item, egui::Modifiers::NONE, vec![]);
        let id = editor.selected.expect("menu item selects the new solid");
        assert_eq!(editor.history.scene().node(id).unwrap().name(), name);
        assert_eq!(
            editor.history.scene().shapes()[0].mesh().faces().len(),
            faces
        );
        assert!(editor.history.can_undo());
        assert!(editor.is_dirty());

        editor
            .apply_operation(Operation::SetSelectionMode(SelectionMode::Face))
            .unwrap();
        let cap_face = selected_face(&editor, cap);
        editor
            .apply_operation(Operation::SelectFace {
                target: Some(cap_face.clone()),
            })
            .unwrap();
        assert_eq!(editor.remembered_face.as_ref(), Some(&cap_face));
        let camera = editor.camera.clone();
        editor.apply_operation(Operation::FrameSelected).unwrap();
        assert_ne!(editor.camera, camera);
        editor
            .apply_operation(Operation::SetSelectionMode(SelectionMode::Object))
            .unwrap();

        editor
            .apply_operation(Operation::SetActiveTool(ActiveTool::Move))
            .unwrap();
        let target = editor.selected_move_target().unwrap();
        editor
            .apply_operation(Operation::BeginMove {
                target,
                axis: azteccraft::native_scene::WorldAxis::X,
            })
            .unwrap();
        let move_id = editor.history.active_manipulation().unwrap().id;
        editor
            .apply_operation(Operation::UpdateMove {
                id: move_id,
                distance: crate::editor_operations::MoveDistance::Centimeters(5.0),
            })
            .unwrap();
        editor
            .apply_operation(Operation::FinishMove {
                id: move_id,
                finish: azteccraft::history::ManipulationFinish::Commit,
            })
            .unwrap();
        editor.history.scene().validate().unwrap();
        assert_eq!(
            editor
                .history
                .scene()
                .node(id)
                .unwrap()
                .transform()
                .translation[0],
            5.0
        );
        let view = ViewScene::from_native(editor.history.scene(), 256 * 1024 * 1024).unwrap();
        assert_eq!(view.instances()[0].faces().len(), faces);
    }
}

#[test]
fn frame_selected_in_hierarchy_mode_frames_the_selected_root() {
    let mut editor = group_editor();
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    click_first_instance(&mut editor);
    assert!(editor.selected.is_some());
    let before = editor.camera.clone();
    remote(&mut editor, Operation::FrameSelected).unwrap();
    assert_ne!(editor.camera, before);
}

#[test]
fn hierarchy_click_on_a_flat_scene_selects_what_an_object_click_selects() {
    let mut picked = Vec::new();
    for mode in [SelectionMode::Object, SelectionMode::Hierarchy] {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let id = editor.selected.unwrap();
        editor.native_operation(Operation::SetSelectionMode(mode));
        editor.native_operation(Operation::Select {
            node: None,
            instance_path: None,
        });
        assert_eq!(editor.selected, None);
        editor.camera = Camera::default();
        click_world_point(&mut editor, glam::Vec3::ZERO);
        assert_eq!(editor.selected, Some(id), "{mode:?}");
        picked.push((editor.selected, editor.selected_path.clone()));
    }
    assert_eq!(picked[0], picked[1]);
}

#[test]
fn hierarchy_click_on_empty_space_clears_the_selection() {
    let mut editor = group_editor();
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    click_first_instance(&mut editor);
    assert!(editor.selected.is_some());
    click_world_point(&mut editor, glam::Vec3::new(-3.0, 0.0, 0.0));
    assert_eq!(editor.selected, None);
    assert!(editor.selected_path.is_empty());
}

/// Two cubes: A at the origin (selected) and B translated to x = 2.
fn two_cubes() -> (Editor, NodeId, NodeId) {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let a = editor.selected.unwrap();
    editor.create(Primitive::Cube);
    let b = editor.selected.unwrap();
    editor.channels[0] = "2".into();
    editor.channel_intents[0] = true;
    editor.select(Some(a));
    assert_eq!(editor.selected, Some(a));
    assert!(!editor.has_draft());
    editor.native_operation(Operation::SetSelectionMode(SelectionMode::Hierarchy));
    editor.camera = Camera::default();
    (editor, a, b)
}

#[test]
fn hierarchy_click_commits_a_valid_draft_to_the_old_target_first() {
    let (mut editor, a, b) = two_cubes();
    editor.channels[1] = "5".into();
    editor.channel_intents[1] = true;
    click_world_point(&mut editor, glam::Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(editor.selected, Some(b));
    let translation = |id| {
        editor
            .history
            .scene()
            .node(id)
            .unwrap()
            .transform()
            .translation
    };
    assert_eq!(translation(a)[1], 5.0);
    assert_eq!(translation(b), [2.0, 0.0, 0.0]);
    assert!(!editor.has_draft());
}

#[test]
fn hierarchy_click_with_an_invalid_draft_refuses_and_keeps_the_selection() {
    let (mut editor, a, _) = two_cubes();
    editor.channels[0] = "nan".into();
    editor.channel_intents[0] = true;
    let before = editor.history.scene().clone();
    click_world_point(&mut editor, glam::Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(editor.selected, Some(a));
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.channels[0], "nan");
    assert!(editor.error);
}
