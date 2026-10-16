use super::*;

#[tokio::test]
async fn move_sdk_absolute_preview_cancel_commit_busy_and_tokens_share_one_history() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let token = editor.scene_token.to_string();
    let created = call(
        &mut editor,
        &context,
        client.peer(),
        "create_primitive",
        json!({"scene_token":token,"primitive":"cube"}),
    )
    .await
    .structured_content
    .unwrap();
    let node = created["node_id"].as_str().unwrap().to_owned();
    let path: Vec<_> = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect();
    let before = editor.history.scene().clone();
    let begin = call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"x"}),
    )
    .await
    .structured_content
    .unwrap();
    let move_id = begin["move_id"].as_str().unwrap().to_owned();
    assert_eq!(begin["outcome"], "move_started");
    assert_eq!(
        editor.active_tool,
        crate::editor_operations::ActiveTool::Select
    );
    for distance in [1.0, -2.0, 3.0] {
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "update_move",
            json!({"scene_token":token,"move_id":move_id,"distance":distance}),
        )
        .await;
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            editor.history.scene().nodes()[0].transform().translation[0],
            distance
        );
        assert!(!editor.has_draft());
    }
    let preview = editor.history.scene().clone();
    let camera = editor.camera.clone();
    for (name, arguments) in [
        ("undo", json!({"scene_token":token})),
        ("redo", json!({"scene_token":token})),
        ("new_scene", json!({"scene_token":token})),
        ("request_close", json!({"scene_token":token})),
        ("save_scene", json!({"scene_token":token})),
        (
            "save_scene_as",
            json!({"scene_token":token,"path":"unused-move.mb"}),
        ),
        (
            "open_scene",
            json!({"scene_token":token,"path":"unused-move.mb"}),
        ),
        (
            "set_active_tool",
            json!({"scene_token":token,"tool":"move"}),
        ),
        ("select_node", json!({"scene_token":token,"node_id":null})),
        (
            "orbit_camera",
            json!({"scene_token":token,"radians":[0.1,0.1]}),
        ),
        (
            "set_material_diffuse",
            json!({"scene_token":token,"shape_id":"1","value":0.25}),
        ),
        (
            "create_session_material",
            json!({"scene_token":token,"color":[1,0,0]}),
        ),
        (
            "assign_session_material",
            json!({"scene_token":token,"shape_id":"1","color":[1,0,0]}),
        ),
        (
            "set_session_material_color",
            json!({"scene_token":token,"material_id":"1","color":[1,0,0]}),
        ),
        (
            "set_selection_mode",
            json!({"scene_token":token,"mode":"face"}),
        ),
        ("select_face", json!({"scene_token":token,"target":null})),
        (
            "set_grid_visibility",
            json!({"scene_token":token,"visible":false}),
        ),
        (
            "set_shading_mode",
            json!({"scene_token":token,"mode":"wireframe"}),
        ),
    ] {
        let grid = editor.grid_visible;
        let shading = editor.shading;
        let mode = editor.selection_mode;
        let result = call(&mut editor, &context, client.peer(), name, arguments).await;
        assert_eq!((editor.grid_visible, editor.selection_mode), (grid, mode));
        assert_eq!(editor.shading, shading);
        assert_eq!(
            result.structured_content.unwrap()["error"]["code"],
            "busy_manipulation",
            "{name}"
        );
        assert_eq!(editor.history.scene(), &preview);
        assert_eq!(editor.camera, camera);
    }
    let stale_id = call(
        &mut editor,
        &context,
        client.peer(),
        "finish_move",
        json!({"scene_token":token,"move_id":"18446744073709551615","finish":"commit"}),
    )
    .await;
    assert_eq!(
        stale_id.structured_content.unwrap()["error"]["code"],
        "stale_move"
    );
    let stale_token = call(
        &mut editor,
        &context,
        client.peer(),
        "update_move",
        json!({"scene_token":Uuid::new_v4().to_string(),"move_id":move_id,"distance":7}),
    )
    .await;
    assert_eq!(
        stale_token.structured_content.unwrap()["error"]["code"],
        "stale_scene"
    );
    assert_eq!(editor.history.scene(), &preview);
    let inspected = call(
        &mut editor,
        &context,
        client.peer(),
        "inspect_scene",
        json!({}),
    )
    .await
    .structured_content
    .unwrap();
    assert_eq!(
        inspected["inspection"]["move_tool"]["active_tool"],
        "select"
    );
    assert_eq!(
        inspected["inspection"]["move_tool"]["active"]["provenance"],
        "remote"
    );
    assert_eq!(
        inspected["inspection"]["move_tool"]["active"]["distance"],
        3.0
    );
    assert_eq!(
        inspected["inspection"]["nodes"][0]["authoring_transform"]["translation"][0],
        3.0
    );
    call(
        &mut editor,
        &context,
        client.peer(),
        "finish_move",
        json!({"scene_token":token,"move_id":move_id,"finish":"cancel"}),
    )
    .await;
    assert_eq!(editor.history.scene(), &before);
    let activated = call(
        &mut editor,
        &context,
        client.peer(),
        "set_active_tool",
        json!({"scene_token":token,"tool":"move"}),
    )
    .await;
    assert!(!activated.is_error.unwrap_or(false));
    let begin = call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"y"}),
    )
    .await
    .structured_content
    .unwrap();
    let id = begin["move_id"].as_str().unwrap();
    assert_ne!(id, move_id);
    call(
        &mut editor,
        &context,
        client.peer(),
        "update_move",
        json!({"scene_token":token,"move_id":id,"distance":2.5}),
    )
    .await;
    call(
        &mut editor,
        &context,
        client.peer(),
        "finish_move",
        json!({"scene_token":token,"move_id":id,"finish":"commit"}),
    )
    .await;
    let after = editor.history.scene().clone();
    call(
        &mut editor,
        &context,
        client.peer(),
        "undo",
        json!({"scene_token":token}),
    )
    .await;
    assert_eq!(editor.history.scene(), &before);
    call(
        &mut editor,
        &context,
        client.peer(),
        "redo",
        json!({"scene_token":token}),
    )
    .await;
    assert_eq!(editor.history.scene(), &after);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn move_sdk_declared_inches_cancel_and_saved_translation_preserve_other_source_words() {
    use binary_fixture::{chunk, creation, group, mesh_payload, scene_with_units, triple};
    let original = scene_with_units(
        &[
            group(
                b"XFRM",
                &[creation(b"object", None), triple(b"t", [25.4, -0.0, 0.0])].concat(),
                4,
            ),
            group(
                b"DMSH",
                &[
                    creation(b"shape", Some(b"object")),
                    chunk(b"MESH", &mesh_payload(&[0, 1, 0x6000_0002]), 8),
                ]
                .concat(),
                4,
            ),
        ]
        .concat(),
        b"in",
        b"deg",
    );
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("move-original.mb");
    std::fs::write(&source, &original).unwrap();
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let old_token = editor.scene_token.to_string();
    let opened = call(
        &mut editor,
        &context,
        client.peer(),
        "open_scene",
        json!({"scene_token":old_token,"path":source}),
    )
    .await;
    assert_eq!(
        opened.structured_content.unwrap()["outcome"],
        "document_applied"
    );
    let token = editor.scene_token.to_string();
    let target = editor.history.scene().transform_paths().unwrap().remove(0);
    let node = target.node.to_decimal();
    let path: Vec<_> = target.path.iter().map(|id| id.to_decimal()).collect();
    call(
        &mut editor,
        &context,
        client.peer(),
        "select_node",
        json!({"scene_token":token,"node_id":node,"instance_path":path}),
    )
    .await;
    let baseline = editor.history.scene().clone();
    for finish in ["cancel", "commit"] {
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "begin_move",
            json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"x"}),
        )
        .await
        .structured_content
        .unwrap();
        let id = result["move_id"].as_str().unwrap();
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "update_move",
            json!({"scene_token":token,"move_id":id,"distance":2}),
        )
        .await;
        assert!(!result.is_error.unwrap_or(false));
        let transform = editor
            .history
            .scene()
            .node(target.node)
            .unwrap()
            .transform();
        assert_eq!(transform.translation[0], 12.0);
        assert_eq!(transform.translation[1].to_bits(), (-0.0f64).to_bits());
        call(
            &mut editor,
            &context,
            client.peer(),
            "finish_move",
            json!({"scene_token":token,"move_id":id,"finish":finish}),
        )
        .await;
        if finish == "cancel" {
            assert_eq!(editor.history.scene(), &baseline);
        }
    }
    let committed = editor.history.scene().clone();
    call(
        &mut editor,
        &context,
        client.peer(),
        "undo",
        json!({"scene_token":token}),
    )
    .await;
    assert_eq!(editor.history.scene(), &baseline);
    call(
        &mut editor,
        &context,
        client.peer(),
        "redo",
        json!({"scene_token":token}),
    )
    .await;
    assert_eq!(editor.history.scene(), &committed);
    #[cfg(unix)]
    {
        let destination = directory.path().join("move-saved.mb");
        let saved = call(
            &mut editor,
            &context,
            client.peer(),
            "save_scene_as",
            json!({"scene_token":token,"path":destination}),
        )
        .await;
        assert_eq!(saved.structured_content.unwrap()["outcome"], "saved");
        let written = std::fs::read(&destination).unwrap();
        let offset = original
            .windows(8)
            .position(|word| word == 25.4f64.to_be_bytes())
            .unwrap();
        let mut expected = original.to_vec();
        expected[offset..offset + 8].copy_from_slice(&30.48f64.to_be_bytes());
        assert_eq!(written, expected);
        let reopened = call(
            &mut editor,
            &context,
            client.peer(),
            "open_scene",
            json!({"scene_token":token,"path":destination}),
        )
        .await;
        assert_eq!(
            reopened.structured_content.unwrap()["outcome"],
            "document_applied"
        );
        assert_ne!(editor.scene_token.to_string(), token);
        assert_eq!(
            editor.active_tool,
            crate::editor_operations::ActiveTool::Select
        );
        assert!(editor.history.active_manipulation().is_none());
        assert_eq!(
            editor.history.scene().nodes()[0].transform().translation[0],
            12.0
        );
    }
    assert_eq!(
        std::fs::read(&source).unwrap().as_slice(),
        original.as_ref()
    );
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn move_sdk_never_commits_real_native_text_drafts_or_takes_their_focus() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    editor.create(Primitive::Cube);
    frame(&mut editor, &context, vec![]);
    context.memory_mut(|memory| memory.request_focus(name_edit_id()));
    frame(&mut editor, &context, vec![]);
    frame(
        &mut editor,
        &context,
        vec![egui::Event::Text("human".into())],
    );
    assert!(editor.has_draft());
    let token = editor.scene_token.to_string();
    let node = editor.selected.unwrap().to_decimal();
    let path: Vec<_> = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect();
    let before = editor.history.scene().clone();
    let name = editor.name.clone();
    for (tool, args) in [
        (
            "set_active_tool",
            json!({"scene_token":token,"tool":"move"}),
        ),
        (
            "begin_move",
            json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"x"}),
        ),
        (
            "update_move",
            json!({"scene_token":token,"move_id":"1","distance":1}),
        ),
        (
            "finish_move",
            json!({"scene_token":token,"move_id":"1","finish":"commit"}),
        ),
    ] {
        let result = call(&mut editor, &context, client.peer(), tool, args).await;
        assert_eq!(
            result.structured_content.unwrap()["error"]["code"],
            "busy_editing"
        );
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(editor.name, name);
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(name_edit_id())
        );
        assert!(editor.history.active_manipulation().is_none());
    }
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn move_sdk_refuses_begin_in_face_mode_and_reports_it_in_inspection() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let token = editor.scene_token.to_string();
    let created = call(
        &mut editor,
        &context,
        client.peer(),
        "create_primitive",
        json!({"scene_token":token,"primitive":"cube"}),
    )
    .await
    .structured_content
    .unwrap();
    let node = created["node_id"].as_str().unwrap().to_owned();
    let path: Vec<_> = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect();
    let mode = call(
        &mut editor,
        &context,
        client.peer(),
        "set_selection_mode",
        json!({"scene_token":token,"mode":"face"}),
    )
    .await;
    assert!(!mode.is_error.unwrap_or(false));
    let before = editor.history.scene().clone();
    let refused = call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"x"}),
    )
    .await;
    assert_eq!(
        refused.structured_content.unwrap()["error"]["code"],
        "invalid_input"
    );
    assert!(editor.history.active_manipulation().is_none());
    assert_eq!(editor.history.scene(), &before);
    let inspected = call(
        &mut editor,
        &context,
        client.peer(),
        "inspect_scene",
        json!({}),
    )
    .await
    .structured_content
    .unwrap();
    let tool = &inspected["inspection"]["move_tool"];
    assert_eq!(tool["numeric_available"], false);
    assert!(
        tool["numeric_unavailable_reason"]
            .as_str()
            .unwrap()
            .contains("Face selection")
    );
    call(
        &mut editor,
        &context,
        client.peer(),
        "set_selection_mode",
        json!({"scene_token":token,"mode":"object"}),
    )
    .await;
    let begun = call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"x"}),
    )
    .await;
    assert_eq!(begun.structured_content.unwrap()["outcome"], "move_started");
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

mod ascii_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_ascii.rs"
    ));
}

#[tokio::test]
async fn move_sdk_refuses_begin_on_a_read_only_ascii_curve_scene_with_a_typed_error() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let scene = azteccraft::native_scene::ascii::decode(ascii_fixture::specimen().into()).unwrap();
    editor.history = azteccraft::history::History::new(scene);
    let node = editor.history.scene().nodes()[0].id();
    editor.select(Some(node));
    let token = editor.scene_token.to_string();
    let path: Vec<_> = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect();
    let before = editor.history.scene().clone();
    let refused = call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token":token,"node_id":node.to_decimal(),"instance_path":path,"axis":"x"}),
    )
    .await;
    let error = &refused.structured_content.unwrap()["error"];
    assert_eq!(error["code"], "scene_error");
    assert!(error["message"].as_str().unwrap().contains("read-only"));
    assert!(editor.history.active_manipulation().is_none());
    assert_eq!(editor.history.scene(), &before);
    let inspected = call(
        &mut editor,
        &context,
        client.peer(),
        "inspect_scene",
        json!({}),
    )
    .await
    .structured_content
    .unwrap();
    assert_eq!(
        inspected["inspection"]["move_tool"]["numeric_available"],
        false
    );
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn move_sdk_refuses_a_binary_scene_lacking_the_translate_leaf_with_a_typed_error() {
    use binary_fixture::{chunk, creation, group, mesh_payload, scene_with_units};
    let original = scene_with_units(
        &[
            group(b"XFRM", &creation(b"object", None), 4),
            group(
                b"DMSH",
                &[
                    creation(b"shape", Some(b"object")),
                    chunk(b"MESH", &mesh_payload(&[0, 1, 0x6000_0002]), 8),
                ]
                .concat(),
                4,
            ),
        ]
        .concat(),
        b"cm",
        b"deg",
    );
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("absent-leaf.mb");
    std::fs::write(&source, &original).unwrap();
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let old_token = editor.scene_token.to_string();
    call(
        &mut editor,
        &context,
        client.peer(),
        "open_scene",
        json!({"scene_token":old_token,"path":source}),
    )
    .await;
    let token = editor.scene_token.to_string();
    let target = editor.history.scene().transform_paths().unwrap().remove(0);
    let node = target.node.to_decimal();
    let path: Vec<_> = target.path.iter().map(|id| id.to_decimal()).collect();
    call(
        &mut editor,
        &context,
        client.peer(),
        "select_node",
        json!({"scene_token":token,"node_id":node,"instance_path":path}),
    )
    .await;
    let before = editor.history.scene().clone();
    let refused = call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token":token,"node_id":node,"instance_path":path,"axis":"x"}),
    )
    .await;
    let error = &refused.structured_content.unwrap()["error"];
    assert_eq!(error["code"], "absent_channel_leaf");
    assert!(error["message"].as_str().unwrap().contains("translateX"));
    assert!(editor.history.active_manipulation().is_none());
    assert_eq!(editor.history.scene(), &before);
    let inspected = call(
        &mut editor,
        &context,
        client.peer(),
        "inspect_scene",
        json!({}),
    )
    .await
    .structured_content
    .unwrap();
    let tool = &inspected["inspection"]["move_tool"];
    assert_eq!(tool["numeric_available"], false);
    assert!(
        tool["numeric_unavailable_reason"]
            .as_str()
            .unwrap()
            .contains("translateX")
    );
    assert_eq!(
        std::fs::read(&source).unwrap().as_slice(),
        original.as_ref()
    );
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}
