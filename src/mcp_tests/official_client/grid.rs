use super::*;

#[tokio::test]
async fn sdk_grid_visibility_is_explicit_idempotent_and_preserves_authoring_state() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    editor.create(Primitive::Cube);
    editor.name = "temporary".into();
    assert!(editor.commit_pending());
    editor.history_step(false);
    frame(&mut editor, &context, vec![]);
    let token = editor.scene_token.to_string();
    let scene = editor.history.scene().clone();
    let baseline = serde_json::to_value(editor.inspect(false).unwrap()).unwrap();
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
            "set_grid_visibility",
            json!({"scene_token": token, "visible": visible}),
        )
        .await;
        assert_eq!(result.structured_content.unwrap()["outcome"], outcome);
        let mut inspected = call(
            &mut editor,
            &context,
            client.peer(),
            "inspect_scene",
            json!({}),
        )
        .await
        .structured_content
        .unwrap()["inspection"]
            .clone();
        assert_eq!(inspected["grid_visible"], visible);
        assert_eq!(inspected["viewport_error"], "uninitialized");
        assert_eq!(inspected["depth_complete"], false);
        inspected["grid_visible"] = json!(true);
        assert_eq!(inspected, baseline);
        assert_eq!(editor.history.scene(), &scene);
        assert!(editor.history.can_redo());
    }
    for arguments in [
        json!({"scene_token": token}),
        json!({"visible": false}),
        json!({"scene_token": token, "visible": null}),
        json!({"scene_token": token, "visible": "false"}),
        json!({"scene_token": token, "visible": 0}),
        json!({"scene_token": token, "visible": false, "extra": true}),
    ] {
        assert!(
            call_response(
                &mut editor,
                &context,
                client.peer(),
                "set_grid_visibility",
                arguments
            )
            .await
            .is_err()
        );
        assert_eq!(
            serde_json::to_value(editor.inspect(false).unwrap()).unwrap(),
            baseline
        );
        assert_eq!(editor.history.scene(), &scene);
    }
    let stale = call(
        &mut editor,
        &context,
        client.peer(),
        "set_grid_visibility",
        json!({"scene_token": Uuid::new_v4().to_string(), "visible": false}),
    )
    .await;
    assert_eq!(
        stale.structured_content.unwrap()["error"]["code"],
        "stale_scene"
    );
    assert_eq!(
        serde_json::to_value(editor.inspect(false).unwrap()).unwrap(),
        baseline
    );
    editor.native_operation(Operation::SetGridVisible { visible: false });
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
    assert_eq!(inspected["inspection"]["grid_visible"], false);
    assert_eq!(editor.history.scene(), &scene);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}
