use super::*;

async fn cube_with_vertex_target(
    editor: &mut Editor,
    context: &egui::Context,
    peer: &Peer<RoleClient>,
) -> (String, String, Value) {
    let token = editor.scene_token.to_string();
    let created = call(
        editor,
        context,
        peer,
        "create_primitive",
        json!({"scene_token":token,"primitive":"cube"}),
    )
    .await
    .structured_content
    .unwrap();
    let id = created["node_id"].as_str().unwrap().to_owned();
    let inspected = call(editor, context, peer, "inspect_scene", json!({}))
        .await
        .structured_content
        .unwrap();
    let node = inspected["inspection"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == id)
        .unwrap();
    let mut path = node["instance_paths"][0].as_array().unwrap().clone();
    path.push(node["shape_ids"][0].clone());
    let target =
        json!({"node_id":id,"shape_id":node["shape_ids"][0],"instance_path":path,"vertex_index":7});
    (token, id, target)
}

fn baseline(
    editor: &Editor,
) -> (
    Option<azteccraft::scene::NodeId>,
    Vec<azteccraft::native_scene::NativeId>,
    Option<azteccraft::native_scene::VertexTarget>,
    SelectionMode,
) {
    (
        editor.selected,
        editor.selected_path.clone(),
        editor.remembered_vertex.clone(),
        editor.selection_mode,
    )
}

#[tokio::test]
async fn sdk_vertex_mode_select_inspect_unchanged_clear_and_frame_share_editor_state() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let (token, id, target) = cube_with_vertex_target(&mut editor, &context, client.peer()).await;
    let peer = client.peer();
    let mode = call(
        &mut editor,
        &context,
        peer,
        "set_selection_mode",
        json!({"scene_token":token,"mode":"vertex"}),
    )
    .await;
    assert_eq!(mode.structured_content.unwrap()["outcome"], "changed");
    assert_eq!(editor.selection_mode, SelectionMode::Vertex);
    let args = json!({"scene_token":token,"target":target});
    let selected = call(&mut editor, &context, peer, "select_vertex", args.clone()).await;
    assert_eq!(selected.structured_content.unwrap()["outcome"], "changed");
    assert_eq!(editor.remembered_vertex.as_ref().unwrap().vertex_index, 7);
    assert_eq!(editor.remembered_face, None);
    let repeat = call(&mut editor, &context, peer, "select_vertex", args).await;
    assert_eq!(repeat.structured_content.unwrap()["outcome"], "unchanged");
    let scene = editor.history.scene().clone();
    let dirty = editor.is_dirty();
    let history_len = editor.history.can_undo();
    for mode in ["vertex", "object", "face", "vertex"] {
        call(
            &mut editor,
            &context,
            peer,
            "set_selection_mode",
            json!({"scene_token":token,"mode":mode}),
        )
        .await;
        let inspected = call(&mut editor, &context, peer, "inspect_scene", json!({}))
            .await
            .structured_content
            .unwrap();
        assert_eq!(inspected["inspection"]["selection_mode"], mode);
        assert_eq!(inspected["inspection"]["remembered_vertex"], target);
        assert!(inspected["inspection"]["remembered_face"].is_null());
        assert_eq!(inspected["inspection"]["selected_node"], id);
    }
    let camera = editor.camera.clone();
    let framed = call(
        &mut editor,
        &context,
        peer,
        "frame_selected",
        json!({"scene_token":token}),
    )
    .await;
    assert_ne!(framed.is_error, Some(true));
    assert_ne!(editor.camera, camera);
    let cleared = call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":token,"target":null}),
    )
    .await;
    assert_eq!(cleared.structured_content.unwrap()["outcome"], "changed");
    assert_eq!(editor.remembered_vertex, None);
    assert_eq!(editor.selected.unwrap().to_decimal(), id);
    let no_target = call(
        &mut editor,
        &context,
        peer,
        "frame_selected",
        json!({"scene_token":token}),
    )
    .await;
    assert_eq!(no_target.is_error, Some(true));
    assert_eq!(
        no_target.structured_content.unwrap()["error"]["code"],
        "no_selection"
    );
    let state = call(&mut editor, &context, peer, "inspect_scene", json!({}))
        .await
        .structured_content
        .unwrap();
    assert!(state["inspection"]["remembered_vertex"].is_null());
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.is_dirty(), dirty);
    assert_eq!(editor.history.can_undo(), history_len);
    assert_eq!(editor.scene_token.to_string(), token);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn sdk_vertex_mode_select_node_stays_exact_and_clears_the_remembered_vertex() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let (token, id, target) = cube_with_vertex_target(&mut editor, &context, client.peer()).await;
    let peer = client.peer();
    let plane = call(
        &mut editor,
        &context,
        peer,
        "create_primitive",
        json!({"scene_token":token,"primitive":"plane"}),
    )
    .await
    .structured_content
    .unwrap()["node_id"]
        .as_str()
        .unwrap()
        .to_owned();
    for (tool, args) in [
        (
            "set_selection_mode",
            json!({"scene_token":token,"mode":"vertex"}),
        ),
        ("select_node", json!({"scene_token":token,"node_id":id})),
        (
            "select_vertex",
            json!({"scene_token":token,"target":target}),
        ),
    ] {
        assert_ne!(
            call(&mut editor, &context, peer, tool, args).await.is_error,
            Some(true)
        );
    }
    assert!(editor.remembered_vertex.is_some());
    let other = call(
        &mut editor,
        &context,
        peer,
        "select_node",
        json!({"scene_token":token,"node_id":plane}),
    )
    .await;
    assert_ne!(other.is_error, Some(true));
    assert_eq!(editor.selected.unwrap().to_decimal(), plane);
    assert_eq!(editor.selection_mode, SelectionMode::Vertex);
    assert_eq!(editor.remembered_vertex, None);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn sdk_vertex_invalid_targets_and_wrong_modes_keep_the_baseline() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let (token, id, target) = cube_with_vertex_target(&mut editor, &context, client.peer()).await;
    let peer = client.peer();
    // Object, Face and Hierarchy mode: even null is refused and nothing changes.
    for mode in ["object", "face", "hierarchy"] {
        call(
            &mut editor,
            &context,
            peer,
            "set_selection_mode",
            json!({"scene_token":token,"mode":mode}),
        )
        .await;
        let before = baseline(&editor);
        for value in [target.clone(), Value::Null] {
            let refused = call(
                &mut editor,
                &context,
                peer,
                "select_vertex",
                json!({"scene_token":token,"target":value}),
            )
            .await;
            assert_eq!(refused.is_error, Some(true), "{mode}");
            assert_eq!(
                refused.structured_content.unwrap()["error"]["code"],
                "invalid_input"
            );
            assert_eq!(baseline(&editor), before);
        }
    }
    call(
        &mut editor,
        &context,
        peer,
        "set_selection_mode",
        json!({"scene_token":token,"mode":"vertex"}),
    )
    .await;
    call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":token,"target":target}),
    )
    .await;
    let other = call(
        &mut editor,
        &context,
        peer,
        "create_primitive",
        json!({"scene_token":token,"primitive":"plane"}),
    )
    .await
    .structured_content
    .unwrap()["node_id"]
        .as_str()
        .unwrap()
        .to_owned();
    call(
        &mut editor,
        &context,
        peer,
        "select_node",
        json!({"scene_token":token,"node_id":id}),
    )
    .await;
    call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":token,"target":target}),
    )
    .await;
    let before = baseline(&editor);
    assert!(before.2.is_some());
    let before_scene = editor.history.scene().clone();
    let before_dirty = editor.is_dirty();
    for field in ["node_id", "shape_id", "instance_path", "vertex_index"] {
        let mut invalid = target.clone();
        invalid[field] = match field {
            "node_id" => json!(other),
            "shape_id" => json!("18446744073709551615"),
            "instance_path" => json!(["18446744073709551615", target["shape_id"]]),
            _ => json!(usize::MAX),
        };
        let result = call(
            &mut editor,
            &context,
            peer,
            "select_vertex",
            json!({"scene_token":token,"target":invalid}),
        )
        .await;
        assert_eq!(result.is_error, Some(true), "{field}");
        assert_eq!(baseline(&editor), before, "{field}");
    }
    let mut one_past = target.clone();
    one_past["vertex_index"] = json!(8);
    let rejected = call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":token,"target":one_past}),
    )
    .await;
    assert_eq!(rejected.is_error, Some(true));
    assert_eq!(baseline(&editor), before);
    for malformed in [
        json!({"scene_token":token}),
        json!({"scene_token":token,"target":{"node_id":id,"shape_id":target["shape_id"],"instance_path":target["instance_path"],"vertex_index":-1}}),
        json!({"scene_token":token,"target":{"node_id":id,"shape_id":target["shape_id"],"instance_path":["1"],"vertex_index":0}}),
        json!({"scene_token":token,"target":{"node_id":id,"shape_id":target["shape_id"],"instance_path":target["instance_path"],"face_index":0}}),
    ] {
        let result = call_response(&mut editor, &context, peer, "select_vertex", malformed).await;
        assert!(matches!(result, Err(rmcp::ServiceError::McpError(_))));
        assert_eq!(baseline(&editor), before);
    }
    assert_eq!(editor.history.scene(), &before_scene);
    assert_eq!(editor.is_dirty(), before_dirty);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn sdk_vertex_targets_may_name_occluded_and_off_screen_points() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let (token, _, target) = cube_with_vertex_target(&mut editor, &context, client.peer()).await;
    let peer = client.peer();
    call(
        &mut editor,
        &context,
        peer,
        "set_selection_mode",
        json!({"scene_token":token,"mode":"vertex"}),
    )
    .await;
    // From the default camera one corner (vertex 0) is hidden behind the cube;
    // every index is accepted whether or not it is visible.
    for index in 0..8 {
        let mut explicit = target.clone();
        explicit["vertex_index"] = json!(index);
        let result = call(
            &mut editor,
            &context,
            peer,
            "select_vertex",
            json!({"scene_token":token,"target":explicit}),
        )
        .await;
        assert_ne!(result.is_error, Some(true), "{index}");
        assert_eq!(
            editor.remembered_vertex.as_ref().unwrap().vertex_index,
            index
        );
    }
    let panned = call(
        &mut editor,
        &context,
        peer,
        "pan_camera",
        json!({"scene_token":token,"world_units":[1000.0,1000.0]}),
    )
    .await;
    assert_ne!(panned.is_error, Some(true));
    assert_eq!(panned.structured_content.unwrap()["outcome"], "changed");
    let result = call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":token,"target":target}),
    )
    .await;
    assert_ne!(result.is_error, Some(true));
    assert_eq!(editor.remembered_vertex.as_ref().unwrap().vertex_index, 7);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn sdk_vertex_selection_refuses_stale_token_draft_modal_and_active_move() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let (token, _, target) = cube_with_vertex_target(&mut editor, &context, client.peer()).await;
    let peer = client.peer();
    call(
        &mut editor,
        &context,
        peer,
        "set_selection_mode",
        json!({"scene_token":token,"mode":"vertex"}),
    )
    .await;
    let before = baseline(&editor);
    let scene = editor.history.scene().clone();
    let args = json!({"scene_token":token,"target":target});
    let code = |result: CallToolResult| result.structured_content.unwrap()["error"]["code"].clone();

    let stale = call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":Uuid::nil().to_string(),"target":target}),
    )
    .await;
    assert_eq!(code(stale), "stale_scene");
    assert_eq!(baseline(&editor), before);

    editor.name = "human_draft".into();
    let draft = call(&mut editor, &context, peer, "select_vertex", args.clone()).await;
    assert_eq!(code(draft), "busy_editing");
    assert_eq!(editor.name, "human_draft");
    assert_eq!(baseline(&editor), before);
    editor.sync_fields();

    editor.request_close();
    let modal = call(&mut editor, &context, peer, "select_vertex", args.clone()).await;
    assert_eq!(code(modal), "busy_editing");
    assert_eq!(baseline(&editor), before);
    assert_eq!(editor.history.scene(), &scene);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();

    let (mut editor, client, server) = start(&context).await;
    let (token, id, target) = cube_with_vertex_target(&mut editor, &context, client.peer()).await;
    let peer = client.peer();
    // Move refuses component modes, so it starts in Object mode; the busy guard
    // must still win over the wrong-mode validation.
    let path: Vec<_> = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect();
    call(
        &mut editor,
        &context,
        peer,
        "begin_move",
        json!({"scene_token":token,"node_id":id,"instance_path":path,"axis":"x"}),
    )
    .await;
    let before = baseline(&editor);
    let preview = editor.history.scene().clone();
    let busy = call(
        &mut editor,
        &context,
        peer,
        "select_vertex",
        json!({"scene_token":token,"target":target}),
    )
    .await;
    assert_eq!(code(busy), "busy_manipulation");
    assert_eq!(baseline(&editor), before);
    assert_eq!(editor.history.scene(), &preview);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}
