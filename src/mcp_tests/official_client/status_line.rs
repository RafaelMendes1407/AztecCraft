use super::*;
use crate::editor_operations::SidebarPanel;

async fn channel_box(
    editor: &mut Editor,
    context: &egui::Context,
    client: &rmcp::service::RunningService<RoleClient, ()>,
) -> (bool, String) {
    let inspected = call(editor, context, client.peer(), "inspect_scene", json!({}))
        .await
        .structured_content
        .unwrap();
    (
        inspected["inspection"]["sidebar_panels"]["channel_box"]
            .as_bool()
            .unwrap(),
        inspected["inspection"]["scene_token"]
            .as_str()
            .unwrap()
            .to_owned(),
    )
}

#[tokio::test]
async fn sdk_sidebar_panel_visibility_is_idempotent_and_guarded() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    editor.create(Primitive::Cube);
    frame(&mut editor, &context, vec![]);
    let token = editor.scene_token.to_string();
    let scene = editor.history.scene().clone();
    assert_eq!(
        channel_box(&mut editor, &context, &client).await,
        (true, token.clone())
    );
    for (visible, outcome) in [
        (false, "changed"),
        (false, "unchanged"),
        (true, "changed"),
        (true, "unchanged"),
    ] {
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "set_sidebar_panel_visibility",
            json!({"scene_token": token, "panel": "channel_box", "visible": visible}),
        )
        .await;
        assert_eq!(result.structured_content.unwrap()["outcome"], outcome);
        assert_eq!(
            channel_box(&mut editor, &context, &client).await,
            (visible, token.clone())
        );
        assert_eq!(editor.scene_token.to_string(), token);
        assert_eq!(editor.history.scene(), &scene);
    }
    let hide =
        |token: &str| json!({"scene_token": token, "panel": "channel_box", "visible": false});

    let stale = call(
        &mut editor,
        &context,
        client.peer(),
        "set_sidebar_panel_visibility",
        hide(&Uuid::new_v4().to_string()),
    )
    .await;
    assert_eq!(
        stale.structured_content.unwrap()["error"]["code"],
        "stale_scene"
    );
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));

    for panel in ["attribute_editor", "ChannelBox", ""] {
        let rejected = call_response(
            &mut editor,
            &context,
            client.peer(),
            "set_sidebar_panel_visibility",
            json!({"scene_token": token, "panel": panel, "visible": false}),
        )
        .await;
        let Err(rmcp::ServiceError::McpError(error)) = rejected else {
            panic!("expected a JSON-RPC error for {panel:?}");
        };
        assert_eq!(
            error.code,
            rmcp::model::ErrorCode::INVALID_PARAMS,
            "{panel}"
        );
        assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
        assert_eq!(editor.scene_token.to_string(), token);
        assert_eq!(editor.history.scene(), &scene);
    }

    editor.name = "human_draft".into();
    let draft = call(
        &mut editor,
        &context,
        client.peer(),
        "set_sidebar_panel_visibility",
        hide(&token),
    )
    .await;
    assert_eq!(
        draft.structured_content.unwrap()["error"]["code"],
        "busy_editing"
    );
    assert_eq!(editor.name, "human_draft");
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    editor.sync_fields();

    editor.request_close();
    let modal = call(
        &mut editor,
        &context,
        client.peer(),
        "set_sidebar_panel_visibility",
        hide(&token),
    )
    .await;
    assert_eq!(
        modal.structured_content.unwrap()["error"]["code"],
        "busy_editing"
    );
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    assert_eq!(editor.history.scene(), &scene);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn sdk_sidebar_panel_visibility_is_busy_during_an_active_move() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let token = editor.scene_token.to_string();
    let created = call(
        &mut editor,
        &context,
        client.peer(),
        "create_primitive",
        json!({"scene_token": token, "primitive": "cube"}),
    )
    .await
    .structured_content
    .unwrap();
    let path: Vec<_> = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect();
    call(
        &mut editor,
        &context,
        client.peer(),
        "begin_move",
        json!({"scene_token": token, "node_id": created["node_id"], "instance_path": path, "axis": "x"}),
    )
    .await;
    let preview = editor.history.scene().clone();
    let busy = call(
        &mut editor,
        &context,
        client.peer(),
        "set_sidebar_panel_visibility",
        json!({"scene_token": token, "panel": "channel_box", "visible": false}),
    )
    .await;
    assert_eq!(
        busy.structured_content.unwrap()["error"]["code"],
        "busy_manipulation"
    );
    assert!(editor.sidebar_panel_visible(SidebarPanel::ChannelBox));
    assert_eq!(editor.history.scene(), &preview);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}
