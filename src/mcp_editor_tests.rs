use super::*;
use crate::mcp;

fn frame(editor: &mut Editor, context: &egui::Context, events: Vec<egui::Event>) {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.render(ui),
    );
    output.textures_delta.clear();
}

fn create_request(editor: &Editor) -> RemoteRequest {
    RemoteRequest {
        expected_scene_token: Some(editor.scene_token),
        operation: Operation::Create {
            primitive: Primitive::Cube,
            name: None,
        },
    }
}

#[test]
fn bridge_quota_queue_capacity_and_dropped_receivers_preserve_authoring() {
    let context = egui::Context::default();
    let (bridge, ingress) = mcp::test_bridge(context.clone());
    let mut editor = Editor {
        mcp: Some(bridge),
        ..Default::default()
    };
    let mut replies = Vec::new();
    for _ in 0..16 {
        replies.push(ingress.queue(create_request(&editor)).unwrap());
    }
    assert!(ingress.queue(create_request(&editor)).is_err());
    frame(&mut editor, &context, vec![]);
    assert_eq!(editor.history.scene().nodes().len(), 8);
    assert_eq!(editor.mcp.as_ref().unwrap().test_queue_len(), 8);
    assert!(replies[0].try_recv().unwrap().is_ok());
    assert!(replies[8].try_recv().is_err());
    frame(&mut editor, &context, vec![]);
    assert_eq!(editor.history.scene().nodes().len(), 16);
    drop(ingress.queue(create_request(&editor)).unwrap());
    frame(&mut editor, &context, vec![]);
    assert_eq!(editor.history.scene().nodes().len(), 16);
}

#[test]
fn ingress_eof_discards_queued_mutation_and_keeps_dirty_native_guard() {
    let context = egui::Context::default();
    let (bridge, ingress) = mcp::test_bridge(context.clone());
    let mut editor = Editor::default();
    editor.create(Primitive::Plane);
    let before = editor.history.scene().clone();
    editor.mcp = Some(bridge);
    let mut reply = ingress.queue(create_request(&editor)).unwrap();
    ingress.close();
    frame(&mut editor, &context, vec![]);
    assert_eq!(editor.history.scene(), &before);
    assert!(editor.files.is_modal());
    assert!(!editor.mcp.as_ref().unwrap().test_alive());
    assert!(reply.try_recv().is_err());
    assert!(editor.history.can_undo());
}

#[test]
fn remote_change_requests_repaint_and_large_inspection_returns_explicit_error() {
    let context = egui::Context::default();
    let (bridge, ingress) = mcp::test_bridge(context.clone());
    let mut editor = Editor {
        mcp: Some(bridge),
        ..Default::default()
    };
    let mut reply = ingress.queue(create_request(&editor)).unwrap();
    frame(&mut editor, &context, vec![]);
    assert!(matches!(
        reply.try_recv().unwrap().unwrap(),
        OperationReply::Created { .. }
    ));
    assert!(context.has_requested_repaint());
    editor.name = "x".repeat(crate::editor_operations::MAX_RESULT_BYTES);
    assert!(editor.commit_pending());
    assert!(matches!(
        editor.remote_operation(
            RemoteRequest {
                expected_scene_token: None,
                operation: Operation::Inspect,
            },
            &context
        ),
        Err(OperationError::TooLarge)
    ));
}

#[test]
fn same_frame_click_and_type_blocks_queued_mutation_without_prior_focus() {
    let context = egui::Context::default();
    let baseline_context = egui::Context::default();
    let (bridge, ingress) = mcp::test_bridge(context.clone());
    let mut editor = Editor::default();
    let mut baseline = Editor::default();
    editor.create(Primitive::Cube);
    baseline.create(Primitive::Cube);
    frame(&mut editor, &context, vec![]);
    frame(&mut editor, &context, vec![]);
    frame(&mut baseline, &baseline_context, vec![]);
    frame(&mut baseline, &baseline_context, vec![]);
    let before = editor.history.scene().clone();
    let name_rect = context.read_response(name_edit_id()).unwrap().rect;
    assert_eq!(
        name_rect,
        baseline_context.read_response(name_edit_id()).unwrap().rect
    );
    assert_eq!(context.memory(|memory| memory.focused()), None);
    assert_eq!(baseline_context.memory(|memory| memory.focused()), None);
    editor.mcp = Some(bridge);
    let mut reply = ingress.queue(create_request(&editor)).unwrap();
    let position = name_rect.center();
    let events = vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::Text("human".into()),
    ];
    frame(&mut baseline, &baseline_context, events.clone());
    frame(&mut editor, &context, events);
    assert!(matches!(
        reply.try_recv().unwrap(),
        Err(OperationError::BusyEditing)
    ));
    assert!(!baseline.name.contains("human"));
    assert_eq!(editor.name, baseline.name);
    assert_eq!(editor.channels, baseline.channels);
    assert_eq!(editor.selected, baseline.selected);
    assert_eq!(
        context.memory(|memory| memory.focused()),
        baseline_context.memory(|memory| memory.focused())
    );
    assert_eq!(editor.history.scene(), baseline.history.scene());
    assert_eq!(editor.history.can_undo(), baseline.history.can_undo());
    assert_eq!(editor.history.can_redo(), baseline.history.can_redo());
    assert_eq!(editor.history.scene(), &before);
    let mut reply = ingress.queue(create_request(&editor)).unwrap();
    let events = vec![egui::Event::Text("human".into())];
    frame(&mut baseline, &baseline_context, events.clone());
    frame(&mut editor, &context, events);
    assert!(matches!(
        reply.try_recv().unwrap(),
        Err(OperationError::BusyEditing)
    ));
    assert!(baseline.name.contains("human"));
    assert_eq!(editor.name, baseline.name);
    assert_eq!(editor.channels, baseline.channels);
    assert_eq!(editor.selected, baseline.selected);
    assert_eq!(
        context.memory(|memory| memory.focused()),
        baseline_context.memory(|memory| memory.focused())
    );
    assert_eq!(editor.history.scene(), &before);
    assert_eq!(editor.history.scene(), baseline.history.scene());
    assert!(editor.has_draft());
    assert_eq!(editor.history.undo(), baseline.history.undo());
    assert_eq!(editor.history.scene(), baseline.history.scene());
    assert!(editor.history.scene().nodes().is_empty());
    assert!(!editor.history.can_undo());
    assert!(!baseline.history.can_undo());
    assert_eq!(editor.history.redo(), baseline.history.redo());
    assert_eq!(editor.history.scene(), baseline.history.scene());
}

#[test]
fn shutdown_child_with_open_stdin() {
    if std::env::var("AZTECCRAFT_MCP_SHUTDOWN_CHILD").as_deref() != Ok("1") {
        return;
    }
    let context = egui::Context::default();
    let mut editor = Editor::default();
    editor.start_mcp(&context).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    while !editor.mcp.as_ref().unwrap().test_read_observed() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let mut output = context.run_ui(input, |ui| editor.render(ui));
    output.textures_delta.clear();
    assert!(editor.mcp.is_none());
}

#[test]
fn approved_native_close_has_bounded_process_cleanup_with_open_silent_stdin() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "app::mcp_editor_tests::shutdown_child_with_open_stdin",
            "--nocapture",
        ])
        .env("AZTECCRAFT_MCP_SHUTDOWN_CHILD", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let open_stdin = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("open stdin blocked native-close cleanup");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(open_stdin);
}

#[cfg(feature = "mcp-test-client")]
mod official_client {
    use super::*;
    use rmcp::{
        ServiceExt,
        model::{CallToolRequestParams, CallToolResponse, CallToolResult},
        service::{Peer, RoleClient},
    };
    use serde_json::{Value, json};
    use std::time::Duration;

    async fn call_response(
        editor: &mut Editor,
        context: &egui::Context,
        peer: &Peer<RoleClient>,
        name: &'static str,
        arguments: Value,
    ) -> Result<CallToolResult, rmcp::ServiceError> {
        let calling = peer.call_tool_once(
            CallToolRequestParams::new(name).with_arguments(arguments.as_object().unwrap().clone()),
        );
        tokio::pin!(calling);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            tokio::select! {
                result = &mut calling => {
                    let CallToolResponse::Complete(result) = result? else { panic!("expected complete tool result") };
                    return Ok(result);
                }
                _ = tokio::time::sleep(Duration::from_millis(1)) => frame(editor, context, vec![]),
                _ = tokio::time::sleep_until(deadline) => panic!("protocol call did not complete"),
            }
        }
    }

    async fn call(
        editor: &mut Editor,
        context: &egui::Context,
        peer: &Peer<RoleClient>,
        name: &'static str,
        arguments: Value,
    ) -> CallToolResult {
        call_response(editor, context, peer, name, arguments)
            .await
            .unwrap()
    }

    async fn wait_for_queue(editor: &Editor) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while editor.mcp.as_ref().unwrap().test_queue_len() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    async fn start(
        context: &egui::Context,
    ) -> (
        Editor,
        rmcp::service::RunningService<RoleClient, ()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (bridge, stream, serving) = mcp::test_connection(context.clone());
        let server = tokio::spawn(serving);
        let editor = Editor {
            mcp: Some(bridge),
            ..Default::default()
        };
        let (reader, writer) = tokio::io::split(stream);
        let client = ()
            .serve(rmcp::transport::async_rw::AsyncRwTransport::new(
                reader, writer,
            ))
            .await
            .unwrap();
        (editor, client, server)
    }

    #[tokio::test]
    async fn strict_sdk_requests_and_core_errors_preserve_live_editor_state() {
        let context = egui::Context::default();
        let (mut editor, client, server) = start(&context).await;
        editor.create(Primitive::Cube);
        let token = editor.scene_token.to_string();
        let id = editor.selected.unwrap().to_decimal();
        let before = editor.history.scene().clone();
        assert!(
            call_response(
                &mut editor,
                &context,
                client.peer(),
                "inspect_scene",
                json!({"unknown":true})
            )
            .await
            .is_err()
        );
        assert!(
            call_response(
                &mut editor,
                &context,
                client.peer(),
                "delete_node",
                json!({"scene_token":token,"node_id":1})
            )
            .await
            .is_err()
        );
        assert!(
            call_response(
                &mut editor,
                &context,
                client.peer(),
                "unknown_tool",
                json!({})
            )
            .await
            .is_err()
        );
        for (name, arguments, expected) in [
            (
                "rename_node",
                json!({"scene_token":token,"node_id":id,"name":""}),
                "scene_error",
            ),
            (
                "select_node",
                json!({"scene_token":token,"node_id":"999"}),
                "scene_error",
            ),
            (
                "delete_node",
                json!({"scene_token":Uuid::new_v4().to_string(),"node_id":id}),
                "stale_scene",
            ),
            (
                "set_transform",
                json!({"scene_token":token,"node_id":id,"translation":[1e40,0,0],"rotation_degrees":[0,0,0],"scale":[1,1,1]}),
                "scene_error",
            ),
        ] {
            let result = call(&mut editor, &context, client.peer(), name, arguments).await;
            assert_eq!(result.is_error, Some(true));
            assert_eq!(
                result.structured_content.unwrap()["error"]["code"],
                expected
            );
            assert_eq!(editor.history.scene(), &before);
            assert_eq!(editor.selected.unwrap().to_decimal(), id);
        }
        editor.name = "human_draft".into();
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "undo",
            json!({"scene_token":token}),
        )
        .await;
        assert_eq!(
            result.structured_content.unwrap()["error"]["code"],
            "busy_editing"
        );
        assert_eq!(editor.name, "human_draft");
        assert_eq!(editor.history.scene(), &before);
        editor.sync_fields();
        client.cancel().await.unwrap();
        drop(editor);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn legacy_sdk_cancellation_and_ingress_eof_never_apply_queued_edit() {
        use rmcp::model::{CallToolRequest, ClientRequest};
        let context = egui::Context::default();
        let (mut editor, client, server) = start(&context).await;
        editor.create(Primitive::Plane);
        let before = editor.history.scene().clone();
        let args = json!({"scene_token":editor.scene_token.to_string(),"primitive":"cube"});
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new("create_primitive")
                .with_arguments(args.as_object().unwrap().clone()),
        ));
        let handle = client
            .peer()
            .send_cancellable_request(request.clone(), Default::default())
            .await
            .unwrap();
        wait_for_queue(&editor).await;
        handle.cancel(None).await.unwrap();
        client
            .peer()
            .send_request(ClientRequest::PingRequest(Default::default()))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        frame(&mut editor, &context, vec![]);
        assert_eq!(editor.history.scene(), &before);
        let _handle = client
            .peer()
            .send_cancellable_request(request, Default::default())
            .await
            .unwrap();
        wait_for_queue(&editor).await;
        client.cancel().await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            while editor.mcp.as_ref().unwrap().test_alive() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        frame(&mut editor, &context, vec![]);
        assert_eq!(editor.history.scene(), &before);
        assert!(editor.files.is_modal());
        drop(editor);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn ui_deadline_drops_receiver_before_paused_editor_can_apply_request() {
        let context = egui::Context::default();
        let (mut editor, client, server) = start(&context).await;
        let args = json!({"scene_token":editor.scene_token.to_string(),"primitive":"cube"});
        let CallToolResponse::Complete(result) = client
            .peer()
            .call_tool_once(
                CallToolRequestParams::new("create_primitive")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap()
        else {
            panic!("expected complete reply")
        };
        assert_eq!(result.structured_content.unwrap()["error"]["code"], "busy");
        frame(&mut editor, &context, vec![]);
        assert!(editor.history.scene().nodes().is_empty());
        client.cancel().await.unwrap();
        drop(editor);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn official_client_negotiates_legacy_version_and_reads_live_editor() {
        use rmcp::model::{ClientCapabilities, ClientConfig, Implementation, ProtocolVersion};
        let context = egui::Context::default();
        let (bridge, stream, serving) = mcp::test_connection(context.clone());
        let server = tokio::spawn(serving);
        let mut editor = Editor {
            mcp: Some(bridge),
            ..Default::default()
        };
        let (reader, writer) = tokio::io::split(stream);
        let config = ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new("protocol-test", "1"),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25);
        let client = config
            .serve(rmcp::transport::async_rw::AsyncRwTransport::new(
                reader, writer,
            ))
            .await
            .unwrap();
        assert_eq!(
            client.peer_info().unwrap().protocol_version,
            ProtocolVersion::V_2025_11_25
        );
        let inspected = call(
            &mut editor,
            &context,
            client.peer(),
            "inspect_scene",
            json!({}),
        )
        .await;
        assert_eq!(
            inspected.structured_content.unwrap()["inspection"]["scene_token"],
            editor.scene_token.to_string()
        );
        client.cancel().await.unwrap();
        drop(editor);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn sdk_current_discover_lifecycle_mutates_the_same_editor() {
        use rmcp::{ClientLifecycleMode, ClientServiceExt};
        let context = egui::Context::default();
        let (bridge, stream, serving) = mcp::test_connection(context.clone());
        let server = tokio::spawn(serving);
        let mut editor = Editor {
            mcp: Some(bridge),
            ..Default::default()
        };
        let (reader, writer) = tokio::io::split(stream);
        let client = ()
            .serve_with_lifecycle(
                rmcp::transport::async_rw::AsyncRwTransport::new(reader, writer),
                ClientLifecycleMode::Discover {
                    preferred_versions: vec![rmcp::model::ProtocolVersion::V_2026_07_28],
                },
            )
            .await
            .unwrap();
        assert_eq!(
            client.peer_info().unwrap().protocol_version,
            rmcp::model::ProtocolVersion::V_2026_07_28
        );
        assert_eq!(client.peer().list_tools(None).await.unwrap().tools.len(), 9);
        let inspected = call(
            &mut editor,
            &context,
            client.peer(),
            "inspect_scene",
            json!({}),
        )
        .await;
        let token = inspected.structured_content.unwrap()["inspection"]["scene_token"].clone();
        let result = call(
            &mut editor,
            &context,
            client.peer(),
            "create_primitive",
            json!({"scene_token":token,"primitive":"plane","name":"discover_plane"}),
        )
        .await;
        assert_eq!(result.is_error, Some(false));
        assert_eq!(editor.history.scene().nodes().len(), 1);
        assert_eq!(editor.name, "discover_plane");
        let inspected = call(
            &mut editor,
            &context,
            client.peer(),
            "inspect_scene",
            json!({}),
        )
        .await;
        assert_eq!(
            inspected.structured_content.unwrap()["inspection"]["nodes"][0]["name"],
            "discover_plane"
        );
        client.cancel().await.unwrap();
        drop(editor);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn sdk_client_discovers_every_handler_and_edits_same_native_controller() {
        let context = egui::Context::default();
        let (bridge, stream, serving) = mcp::test_connection(context.clone());
        let server_task = tokio::spawn(serving);
        let mut editor = Editor {
            mcp: Some(bridge),
            ..Default::default()
        };
        let (reader, writer) = tokio::io::split(stream);
        let client = ()
            .serve(rmcp::transport::async_rw::AsyncRwTransport::new(
                reader, writer,
            ))
            .await
            .unwrap();
        let peer = client.peer();
        let listed = peer.list_tools(None).await.unwrap();
        assert_eq!(
            listed
                .tools
                .iter()
                .map(|tool| tool.name.as_ref())
                .collect::<Vec<_>>(),
            crate::editor_operations::TOOL_NAMES
        );
        let inspected = call(&mut editor, &context, peer, "inspect_scene", json!({})).await;
        let token = inspected.structured_content.unwrap()["inspection"]["scene_token"]
            .as_str()
            .unwrap()
            .to_string();
        let created = call(
            &mut editor,
            &context,
            peer,
            "create_primitive",
            json!({"scene_token":token,"primitive":"cube","name":"remote_cube"}),
        )
        .await;
        let id = created.structured_content.unwrap()["node_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(editor.name, "remote_cube");
        call(
            &mut editor,
            &context,
            peer,
            "rename_node",
            json!({"scene_token":token,"node_id":id,"name":"renamed"}),
        )
        .await;
        call(&mut editor, &context, peer, "set_transform", json!({"scene_token":token,"node_id":id,"translation":[2,0,0],"rotation_degrees":[0,0,0],"scale":[1,1,1]})).await;
        assert_eq!(editor.channels[0], "2");
        call(
            &mut editor,
            &context,
            peer,
            "frame_selected",
            json!({"scene_token":token}),
        )
        .await;
        call(
            &mut editor,
            &context,
            peer,
            "select_node",
            json!({"scene_token":token,"node_id":null}),
        )
        .await;
        assert_eq!(editor.selected, None);
        call(
            &mut editor,
            &context,
            peer,
            "select_node",
            json!({"scene_token":token,"node_id":id}),
        )
        .await;
        editor.channels[0] = "3".into();
        assert!(editor.commit_pending());
        let inspected = call(&mut editor, &context, peer, "inspect_scene", json!({})).await;
        assert_eq!(
            inspected.structured_content.unwrap()["inspection"]["nodes"][0]["transform"]["translation"]
                [0].as_f64(),
            Some(3.0)
        );
        call(
            &mut editor,
            &context,
            peer,
            "undo",
            json!({"scene_token":token}),
        )
        .await;
        assert_eq!(editor.channels[0], "2");
        editor.history_step(false);
        call(
            &mut editor,
            &context,
            peer,
            "redo",
            json!({"scene_token":token}),
        )
        .await;
        assert_eq!(editor.channels[0], "2");
        call(
            &mut editor,
            &context,
            peer,
            "delete_node",
            json!({"scene_token":token,"node_id":id}),
        )
        .await;
        assert!(editor.history.scene().nodes().is_empty());
        client.cancel().await.unwrap();
        server_task.await.unwrap();
    }
}
