use super::*;

#[tokio::test]
async fn sdk_face_mode_target_inspection_and_clear_share_authoritative_editor_state() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let token = editor.scene_token.to_string();
    let mut ids = Vec::new();
    for primitive in ["cube", "plane"] {
        let created = call(
            &mut editor,
            &context,
            client.peer(),
            "create_primitive",
            json!({"scene_token":token,"primitive":primitive}),
        )
        .await;
        ids.push(
            created.structured_content.unwrap()["node_id"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    call(
        &mut editor,
        &context,
        client.peer(),
        "select_node",
        json!({"scene_token":token,"node_id":ids[0]}),
    )
    .await;
    call(
        &mut editor,
        &context,
        client.peer(),
        "rename_node",
        json!({"scene_token":token,"node_id":ids[0],"name":"redo_payload"}),
    )
    .await;
    call(
        &mut editor,
        &context,
        client.peer(),
        "undo",
        json!({"scene_token":token}),
    )
    .await;
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
    assert_eq!(inspected["inspection"]["selection_mode"], "object");
    assert!(inspected["inspection"]["remembered_face"].is_null());
    assert_eq!(
        inspected["inspection"]["face_selection_policy"],
        "visible_centers_only"
    );
    let plane = inspected["inspection"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == ids[1])
        .unwrap();
    let mut path = plane["instance_paths"][0].as_array().unwrap().clone();
    path.push(plane["shape_ids"][0].clone());
    let target = json!({"node_id":ids[1],"shape_id":plane["shape_ids"][0],"instance_path":path,"face_index":0});
    let scene = editor.history.scene().clone();
    let camera = editor.camera.clone();
    let dirty = editor.is_dirty();
    assert!(editor.history.can_redo());
    for (tool, args) in [
        (
            "set_selection_mode",
            json!({"scene_token":token,"mode":"face"}),
        ),
        ("select_face", json!({"scene_token":token,"target":target})),
    ] {
        let result = call(&mut editor, &context, client.peer(), tool, args).await;
        assert_ne!(result.is_error, Some(true));
    }
    assert_eq!(editor.selected.unwrap().to_decimal(), ids[1]);
    assert_eq!(editor.remembered_face.as_ref().unwrap().face_index, 0);
    let selection = (
        editor.selected,
        editor.selected_path.clone(),
        editor.remembered_face.clone(),
    );
    for mode in ["face", "object", "face"] {
        call(
            &mut editor,
            &context,
            client.peer(),
            "set_selection_mode",
            json!({"scene_token":token,"mode":mode}),
        )
        .await;
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
        assert_eq!(inspected["inspection"]["selection_mode"], mode);
        assert_eq!(inspected["inspection"]["remembered_face"], target);
        assert_eq!(inspected["inspection"]["selected_node"], ids[1]);
        assert_eq!(
            inspected["inspection"]["selected_instance_path"],
            plane["instance_paths"][0]
        );
    }
    for field in ["node_id", "shape_id", "instance_path", "face_index"] {
        let mut invalid = target.clone();
        invalid[field] = match field {
            "node_id" => json!(ids[0]),
            "shape_id" => json!("18446744073709551615"),
            "instance_path" => json!(["18446744073709551615", target["shape_id"]]),
            _ => json!(usize::MAX),
        };
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "select_face",
            json!({"scene_token":token,"target":invalid}),
        )
        .await;
        assert_eq!(result.is_error, Some(true), "{field}");
        assert_eq!(
            (
                editor.selected,
                editor.selected_path.clone(),
                editor.remembered_face.clone()
            ),
            selection
        );
    }
    for (tool, args) in [
        (
            "set_selection_mode",
            json!({"scene_token":Uuid::nil().to_string(),"mode":"object"}),
        ),
        (
            "select_face",
            json!({"scene_token":Uuid::nil().to_string(),"target":null}),
        ),
    ] {
        let result = call(&mut editor, &context, client.peer(), tool, args).await;
        assert_eq!(
            result.structured_content.unwrap()["error"]["code"],
            "stale_scene"
        );
    }
    let cleared = call(
        &mut editor,
        &context,
        client.peer(),
        "select_face",
        json!({"scene_token":token,"target":null}),
    )
    .await;
    assert_ne!(cleared.is_error, Some(true));
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.selected, selection.0);
    assert_eq!(editor.selected_path, selection.1);
    call(
        &mut editor,
        &context,
        client.peer(),
        "set_selection_mode",
        json!({"scene_token":token,"mode":"object"}),
    )
    .await;
    let wrong_mode = call(
        &mut editor,
        &context,
        client.peer(),
        "select_face",
        json!({"scene_token":token,"target":target}),
    )
    .await;
    assert_eq!(wrong_mode.is_error, Some(true));
    let null_target = call(
        &mut editor,
        &context,
        client.peer(),
        "select_face",
        json!({"scene_token":token,"target":null}),
    )
    .await;
    assert_eq!(null_target.is_error, Some(true));
    assert_eq!(
        null_target.structured_content.unwrap()["error"]["code"],
        "invalid_input"
    );
    let state = call(
        &mut editor,
        &context,
        client.peer(),
        "inspect_scene",
        json!({}),
    )
    .await
    .structured_content
    .unwrap();
    assert_eq!(state["inspection"]["selection_mode"], "object");
    assert_eq!(state["inspection"]["selected_node"], ids[1]);
    assert_eq!(
        state["inspection"]["selected_instance_path"],
        plane["instance_paths"][0]
    );
    assert!(state["inspection"]["remembered_face"].is_null());
    assert_eq!(editor.remembered_face, None);
    assert_eq!(editor.selection_mode, SelectionMode::Object);
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.camera, camera);
    assert_eq!(editor.scene_token.to_string(), token);
    assert_eq!(editor.is_dirty(), dirty);
    assert!(editor.history.can_redo());
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}
