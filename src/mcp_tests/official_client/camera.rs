use super::*;

#[tokio::test]
async fn sdk_camera_navigation_inspects_real_view_without_authoring_changes() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    editor.create(Primitive::Cube);
    editor.history_step(false);
    let before = editor.history.scene().clone();
    let token = editor.scene_token;
    let selected = editor.selected;
    let dirty = editor.is_dirty();
    let mut eye = editor.camera.eye();
    let mut target = editor.camera.target();
    let mut distance = editor.camera.distance();
    for (tool, fields) in [
        ("orbit_camera", json!({"radians":[0.2,-0.1]})),
        ("pan_camera", json!({"world_units":[1.0,-2.0]})),
        ("dolly_camera", json!({"factor":0.5})),
        ("orbit_camera", json!({"radians":[0.0,0.0]})),
        ("pan_camera", json!({"world_units":[0.0,0.0]})),
        ("dolly_camera", json!({"factor":1.0})),
    ] {
        let noop = fields["radians"] == json!([0.0, 0.0])
            || fields["world_units"] == json!([0.0, 0.0])
            || fields["factor"] == json!(1.0);
        if !noop {
            match tool {
                "orbit_camera" => {
                    let yaw = 0.8_f32;
                    let pitch = 0.3_f32;
                    eye = target
                        + distance
                            * Vec3::new(
                                yaw.sin() * pitch.cos(),
                                pitch.sin(),
                                yaw.cos() * pitch.cos(),
                            );
                }
                "pan_camera" => {
                    let back = (eye - target).normalize();
                    let right = Vec3::Y.cross(back).normalize();
                    let up = back.cross(right);
                    let translation = right - 2.0 * up;
                    target += translation;
                    eye += translation;
                }
                _ => {
                    distance *= 0.5;
                    eye = target + (eye - target) * 0.5;
                }
            }
        }
        let mut arguments = fields;
        arguments["scene_token"] = json!(token.to_string());
        let result = call(&mut editor, &context, client.peer(), tool, arguments).await;
        assert_eq!(
            result.structured_content.unwrap()["outcome"],
            if noop { "unchanged" } else { "changed" }
        );
        assert!((editor.camera.eye() - eye).length() < 0.00001);
        assert!((editor.camera.target() - target).length() < 0.00001);
        assert_eq!(editor.camera.distance(), distance);
        let inspected = call(
            &mut editor,
            &context,
            client.peer(),
            "inspect_scene",
            json!({}),
        )
        .await;
        let camera = &inspected.structured_content.unwrap()["inspection"]["camera"];
        for (field, expected) in [
            ("target", editor.camera.target()),
            ("eye", editor.camera.eye()),
        ] {
            let actual: [f32; 3] = serde_json::from_value(camera[field].clone()).unwrap();
            assert_eq!(actual, expected.to_array());
        }
        assert_eq!(camera["distance"].as_f64().unwrap() as f32, distance);
        assert_eq!(editor.history.scene(), &before);
        assert!(!editor.history.can_undo());
        assert!(editor.history.can_redo());
        assert_eq!(editor.selected, selected);
        assert_eq!(editor.scene_token, token);
        assert_eq!(editor.is_dirty(), dirty);
    }
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
async fn sdk_camera_rejects_bad_parameters_and_invalid_ranges_without_mutation() {
    let context = egui::Context::default();
    let (mut editor, client, server) = start(&context).await;
    let camera = editor.camera.clone();
    let token = editor.scene_token.to_string();
    for (tool, fields) in [
        ("orbit_camera", json!({"radians":[1e100,0]})),
        ("pan_camera", json!({"world_units":[1e100,0]})),
        ("pan_camera", json!({"world_units":[3e38,3e38]})),
        ("dolly_camera", json!({"factor":0})),
        ("dolly_camera", json!({"factor":-1})),
        ("dolly_camera", json!({"factor":1e100})),
        ("dolly_camera", json!({"factor":3e38})),
        ("dolly_camera", json!({"factor":1e-45})),
    ] {
        let mut args = fields;
        args["scene_token"] = json!(token);
        let result = call(&mut editor, &context, client.peer(), tool, args).await;
        assert_eq!(
            result.structured_content.unwrap()["error"]["code"],
            "camera_error"
        );
        assert_eq!(editor.camera, camera);
        assert!(!editor.history.can_undo());
        assert!(!editor.is_dirty());
    }
    for (tool, fields) in [
        ("orbit_camera", json!({"radians":[0]})),
        ("orbit_camera", json!({"radians":[0,0,0]})),
        ("orbit_camera", json!({"radians":["0",0]})),
        ("pan_camera", json!({"world_units":null})),
        ("pan_camera", json!({"world_units":[0,0],"extra":true})),
        ("dolly_camera", json!({"factor":"1"})),
        ("dolly_camera", json!({})),
    ] {
        let mut args = fields;
        args["scene_token"] = json!(token);
        assert!(
            call_response(&mut editor, &context, client.peer(), tool, args)
                .await
                .is_err()
        );
        assert_eq!(editor.camera, camera);
    }
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
#[ignore = "requires a real headless GPU adapter"]
async fn headless_gpu_sdk_camera_selection_and_grid_changes_enter_the_next_fresh_callback() {
    use crate::viewport_renderer::tests::{creation_context, recorded_submission};
    use rmcp::model::{CallToolRequest, ClientRequest, ServerResult};
    let context = egui::Context::default();
    let (mut temporary, client, server) = start(&context).await;
    let creation = creation_context(context.clone());
    let mut editor = Editor::new(&creation).unwrap();
    editor.mcp = temporary.mcp.take();
    let token = editor.scene_token.to_string();
    call(
        &mut editor,
        &context,
        client.peer(),
        "create_primitive",
        json!({"scene_token":token,"primitive":"cube"}),
    )
    .await;
    frame(&mut editor, &context, vec![]);
    let id = editor.selected.unwrap().to_decimal();
    let path = editor
        .selected_path
        .iter()
        .map(|id| id.to_decimal())
        .collect::<Vec<_>>();
    let scene = editor.history.scene().clone();
    let undo = editor.history.can_undo();
    let view = ViewScene::from_native(editor.history.scene(), 256 * 1024 * 1024).unwrap();
    let instance = view
        .instances()
        .iter()
        .find(|instance| instance.transform().to_decimal() == id)
        .unwrap();
    let face_target = json!({
        "node_id": id,
        "shape_id": instance.shape().to_decimal(),
        "instance_path": instance.path().iter().map(|id| id.to_decimal()).collect::<Vec<_>>(),
        "face_index": 1,
    });
    for (tool, arguments) in [
        ("dolly_camera", json!({"scene_token":token,"factor":0.75})),
        ("select_node", json!({"scene_token":token,"node_id":null})),
        (
            "select_node",
            json!({"scene_token":token,"node_id":id,"instance_path":path}),
        ),
        ("frame_selected", json!({"scene_token":token})),
        (
            "set_grid_visibility",
            json!({"scene_token":token,"visible":false}),
        ),
        (
            "set_selection_mode",
            json!({"scene_token":token,"mode":"face"}),
        ),
        (
            "select_face",
            json!({"scene_token":token,"target":face_target}),
        ),
    ] {
        let before = recorded_submission(editor.renderer.as_ref().unwrap()).vertices;
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new(tool).with_arguments(arguments.as_object().unwrap().clone()),
        ));
        let handle = client
            .peer()
            .send_cancellable_request(request, Default::default())
            .await
            .unwrap();
        wait_for_queue(&editor).await;
        frame(&mut editor, &context, vec![]);
        assert_eq!(
            recorded_submission(editor.renderer.as_ref().unwrap()).vertices,
            before,
            "remote changes cannot alter the already prepared callback"
        );
        let response = tokio::time::timeout(Duration::from_secs(2), handle.await_response())
            .await
            .unwrap()
            .unwrap();
        let ServerResult::CallToolResult(response) = response else {
            panic!("tool response required")
        };
        assert_ne!(response.is_error, Some(true));
        frame(&mut editor, &context, vec![]);
        assert_ne!(
            recorded_submission(editor.renderer.as_ref().unwrap()).vertices,
            before,
            "the next actual App frame must use current authoritative state"
        );
        let view = azteccraft::view_scene::ViewScene::from_native(
            editor.history.scene(),
            256 * 1024 * 1024,
        )
        .unwrap();
        assert_eq!(
            editor
                .camera
                .pick_view(&view, glam::Vec2::ZERO, editor.aspect)
                .unwrap()
                .unwrap()
                .transform
                .to_decimal(),
            id
        );
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.history.can_undo(), undo);
    }
    let result = call(
        &mut editor,
        &context,
        client.peer(),
        "inspect_scene",
        json!({}),
    )
    .await
    .structured_content
    .unwrap();
    let inspection = &result["inspection"];
    assert_eq!(inspection["viewport_backend"], "wgpu_geometry_preview");
    assert_eq!(inspection["viewport_error"], Value::Null);
    assert_eq!(inspection["selected_instance_path"], json!(path));
    assert_eq!(inspection["depth_complete"], true);
    assert_eq!(inspection["clipping_complete"], true);
    assert_eq!(inspection["grid_visible"], false);
    assert_eq!(inspection["selection_mode"], "face");
    assert_eq!(inspection["remembered_face"], face_target);
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}

#[tokio::test]
#[ignore = "requires a real headless GPU adapter"]
async fn headless_gpu_sdk_hidden_inspection_reads_live_terminal_health_without_ui_paint() {
    use crate::viewport_renderer::tests::{creation_context, recorded_submission};
    use rmcp::model::{CallToolRequest, ClientRequest, ServerResult};
    let context = egui::Context::default();
    let (mut temporary, client, server) = start(&context).await;
    let creation = creation_context(context.clone());
    let mut editor = Editor::new(&creation).unwrap();
    editor.mcp = temporary.mcp.take();
    let token = editor.scene_token.to_string();
    call(
        &mut editor,
        &context,
        client.peer(),
        "create_primitive",
        json!({"scene_token":token,"primitive":"cube"}),
    )
    .await;
    frame(&mut editor, &context, vec![]);
    let before = recorded_submission(editor.renderer.as_ref().unwrap()).vertices;
    let scene = editor.history.scene().clone();
    let undo = editor.history.can_undo();
    let state = creation.wgpu_render_state.as_ref().unwrap();
    let _invalid = state
        .device
        .create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
            label: Some("private-hidden-driver-marker"),
            source: eframe::wgpu::ShaderSource::Wgsl("invalid shader syntax".into()),
        });
    state
        .device
        .poll(eframe::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    for (tool, arguments) in [
        ("inspect_scene", json!({})),
        ("dolly_camera", json!({"scene_token":token,"factor":1.0})),
        ("inspect_scene", json!({})),
    ] {
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new(tool).with_arguments(arguments.as_object().unwrap().clone()),
        ));
        let handle = client
            .peer()
            .send_cancellable_request(request, Default::default())
            .await
            .unwrap();
        wait_for_queue(&editor).await;
        hidden_frame(&mut editor, &context, vec![]);
        let response = tokio::time::timeout(Duration::from_secs(2), handle.await_response())
            .await
            .unwrap()
            .unwrap();
        let ServerResult::CallToolResult(response) = response else {
            panic!("tool response required")
        };
        assert_ne!(response.is_error, Some(true));
        if tool == "inspect_scene" {
            let content = response.structured_content.unwrap();
            let inspection = &content["inspection"];
            assert_eq!(inspection["viewport_backend"], "wgpu_geometry_preview");
            assert_eq!(inspection["viewport_error"], "backend_failed");
            assert_eq!(inspection["depth_complete"], false);
            assert_eq!(inspection["clipping_complete"], false);
            assert!(!content.to_string().contains("private-hidden-driver-marker"));
        }
        assert_eq!(
            recorded_submission(editor.renderer.as_ref().unwrap()).vertices,
            before
        );
        assert_eq!(editor.history.scene(), &scene);
        assert_eq!(editor.history.can_undo(), undo);
    }
    client.cancel().await.unwrap();
    drop(editor);
    server.await.unwrap();
}
