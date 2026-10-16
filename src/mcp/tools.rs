use super::{
    EditorEnvelope, UI_REPLY_TIMEOUT,
    registry::{catalog, parse_request},
    transport::{Control, Shared, check_serialized_size},
};
use crate::editor_operations::{MAX_RESULT_BYTES, OperationError, OperationReply};
use azteccraft::history::MoveError;
use rmcp::{
    RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, CancelledNotificationParam,
        ErrorData, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
        ServerConfig, Tool,
    },
    service::{NotificationContext, RequestContext},
};
use serde_json::json;
use std::{future::Future, sync::Arc};
use tokio::sync::{mpsc, oneshot};

pub(super) struct EditorServer {
    sender: mpsc::Sender<EditorEnvelope>,
    shared: Arc<Shared>,
    tools: Vec<Tool>,
}

impl EditorServer {
    pub(super) fn new(sender: mpsc::Sender<EditorEnvelope>, shared: Arc<Shared>) -> Self {
        Self {
            sender,
            shared,
            tools: catalog(),
        }
    }
}

fn tool_error(code: &str, message: impl ToString) -> CallToolResult {
    CallToolResult::structured_error(
        json!({ "error": { "code": code, "message": message.to_string() } }),
    )
}

fn tool_reply(reply: Result<OperationReply, OperationError>) -> CallToolResult {
    match reply {
        Ok(reply) => {
            if check_serialized_size(&reply, MAX_RESULT_BYTES / 4).is_err() {
                return tool_error("too_large", OperationError::TooLarge);
            }
            let value = serde_json::to_value(reply).expect("bounded editor replies must serialize");
            let result = CallToolResult::structured(value);
            if check_serialized_size(&result, MAX_RESULT_BYTES).is_err() {
                return tool_error("too_large", OperationError::TooLarge);
            }
            result
        }
        Err(error) => {
            let code = match &error {
                OperationError::BusyEditing => "busy_editing",
                OperationError::BusyManipulation => "busy_manipulation",
                OperationError::Move(error) => match error {
                    MoveError::StaleScene => "stale_scene",
                    MoveError::StaleMove => "stale_move",
                    MoveError::WrongOwner => "move_owner",
                    MoveError::Busy => "busy_manipulation",
                    _ => "move_error",
                },
                OperationError::StaleScene => "stale_scene",
                OperationError::TooLarge => "too_large",
                OperationError::NoSelection => "no_selection",
                OperationError::InvalidInput(_) => "invalid_input",
                OperationError::Scene(_) => "scene_error",
                OperationError::Camera(_) => "camera_error",
                OperationError::File(_) => "file_error",
                OperationError::NativeSave(error) => match error {
                    azteccraft::document::NativeSaveError::Unsupported => "save_unsupported",
                    azteccraft::document::NativeSaveError::InvalidDestination => {
                        "invalid_destination"
                    }
                    azteccraft::document::NativeSaveError::SourceChanged => "source_changed",
                    azteccraft::document::NativeSaveError::DestinationExists => {
                        "destination_exists"
                    }
                    azteccraft::document::NativeSaveError::Cancelled => "save_cancelled",
                    azteccraft::document::NativeSaveError::Io(_) => "save_io",
                    azteccraft::document::NativeSaveError::Encode(_) => "save_encode",
                },
                OperationError::AmbiguousSelection => "ambiguous_selection",
                OperationError::StalePendingAction => "stale_pending_action",
            };
            tool_error(code, error)
        }
    }
}

impl ServerHandler for EditorServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("AztecCraft", env!("CARGO_PKG_VERSION")))
            .with_instructions("Controls this newly launched native editor window. Inspect first; mutations require its scene token. Human text edits and dialogs block remote mutation.")
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.iter().find(|tool| tool.name == name).cloned()
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools.clone()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let request = parse_request(&request.name, request.arguments.unwrap_or_default())?;
        if !self.shared.alive() {
            return Ok(tool_error("unavailable", "Editor protocol connection is closed").into());
        }
        let (reply, receiver) = oneshot::channel();
        let deadline = std::time::Instant::now() + UI_REPLY_TIMEOUT;
        if self
            .sender
            .try_send(EditorEnvelope {
                request,
                reply,
                cancellation: context.ct.clone(),
                request_id: context.id.clone(),
                deadline,
                #[cfg(test)]
                install_probe: None,
            })
            .is_err()
        {
            return Ok(tool_error("busy", "Editor request queue is full or closed").into());
        }
        self.shared.repaint();
        let result = tokio::select! {
            _ = context.ct.cancelled() => {
                self.shared.fail_close(&context.id);
                tool_error("cancelled", "Request cancelled; inspect to reconcile any completed operation")
            },
            outcome = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), receiver) => match outcome {
                Ok(Ok(reply)) => tool_reply(reply),
                Ok(Err(_)) => {
                    self.shared.fail_close(&context.id);
                    tool_error("unavailable", "Editor did not acknowledge the request")
                },
                Err(_) => {
                    self.shared.fail_close(&context.id);
                    tool_error("busy", "Editor reply deadline exceeded; inspect before retrying")
                },
            },
        };
        Ok(result.into())
    }

    fn on_initialized(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let completion = self.shared.complete_control(Control::Initialized);
        async move {
            drop(completion);
        }
    }

    fn on_cancelled(
        &self,
        notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let completion = notification
            .request_id
            .map(|id| self.shared.complete_control(Control::Cancelled(id)));
        async move {
            drop(completion);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_operations::{FileDecision, MAX_NAME_BYTES, MoveDistance, Operation};
    use crate::mcp::tool_names;
    use rmcp::model::JsonObject;
    use uuid::Uuid;

    #[test]
    fn material_arguments_are_strict_f64_and_exact_native_identity() {
        let token = Uuid::new_v4().to_string();
        for args in [
            json!({"scene_token":token,"shape_id":"2","value":-1}),
            json!({"scene_token":token,"shape_id":2,"value":0.5}),
            json!({"scene_token":token,"shape_id":"2"}),
            json!({"scene_token":token,"shape_id":"2","value":"NaN"}),
            json!({"scene_token":token,"shape_id":"2","value":null}),
            json!({"scene_token":token,"shape_id":"2","value":0.5,"extra":true}),
        ] {
            assert!(
                parse_request("set_material_diffuse", args.as_object().unwrap().clone()).is_err()
            );
        }
        for value in [-0.0f64, 1.2345678901234567, 4.5] {
            let request = parse_request(
                "set_material_diffuse",
                json!({"scene_token":token,"shape_id":"18446744073709551615","value":value})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
            let Operation::SetMaterialDiffuse {
                shape,
                value: actual,
            } = request.operation
            else {
                panic!("wrong typed operation");
            };
            assert_eq!(shape.to_decimal(), "18446744073709551615");
            assert_eq!(actual.to_bits(), value.to_bits());
        }
    }

    #[test]
    fn move_schemas_reject_unsupported_axes_tools_ids_and_extra_fields() {
        let token = Uuid::new_v4().to_string();
        for (name, args) in [
            (
                "set_active_tool",
                json!({"scene_token":token,"tool":"rotate"}),
            ),
            (
                "set_active_tool",
                json!({"scene_token":token,"tool":"move","owner":"native"}),
            ),
            (
                "begin_move",
                json!({"scene_token":token,"node_id":"1","instance_path":[],"axis":"x"}),
            ),
            (
                "begin_move",
                json!({"scene_token":token,"node_id":"1","instance_path":["1"],"axis":"xy"}),
            ),
            (
                "begin_move",
                json!({"scene_token":token,"node_id":"1","instance_path":vec!["1";257],"axis":"z"}),
            ),
            (
                "begin_move",
                json!({"scene_token":token,"node_id":1,"instance_path":["1"],"axis":"x"}),
            ),
            (
                "update_move",
                json!({"scene_token":token,"move_id":"+1","distance":1}),
            ),
            (
                "update_move",
                json!({"scene_token":token,"move_id":"18446744073709551616","distance":1}),
            ),
            (
                "update_move",
                json!({"scene_token":token,"move_id":1,"distance":1}),
            ),
            (
                "finish_move",
                json!({"scene_token":token,"move_id":"1","finish":"release"}),
            ),
            (
                "finish_move",
                json!({"scene_token":token,"move_id":"1","finish":"commit","owner":"native"}),
            ),
        ] {
            assert!(
                parse_request(name, args.as_object().unwrap().clone()).is_err(),
                "{name}: {args}"
            );
        }
        let valid = json!({"scene_token":token,"move_id":"18446744073709551615","distance":-1.0000000000000002});
        let request = parse_request("update_move", valid.as_object().unwrap().clone()).unwrap();
        let Operation::UpdateMove {
            id,
            distance: MoveDistance::Declared(value),
        } = request.operation
        else {
            panic!()
        };
        assert_eq!(id.to_decimal(), "18446744073709551615");
        assert_eq!(value.to_bits(), (-1.0000000000000002f64).to_bits());
    }

    #[test]
    fn advertised_catalogue_schemas_and_handlers_match_exactly() {
        let tools = catalog();
        assert_eq!(
            tools
                .iter()
                .map(|tool| tool.name.as_ref())
                .collect::<Vec<_>>(),
            tool_names().collect::<Vec<_>>()
        );
        let token = Uuid::new_v4().to_string();
        for tool in tools {
            assert_eq!(tool.input_schema["additionalProperties"], false);
            assert_eq!(
                tool.annotations.as_ref().unwrap().read_only_hint,
                Some(tool.name == "inspect_scene")
            );
            let args = match tool.name.as_ref() {
                "inspect_scene" => json!({}),
                "set_active_tool" => json!({"scene_token":token,"tool":"move"}),
                "begin_move" => {
                    json!({"scene_token":token,"node_id":"1","instance_path":["1"],"axis":"x"})
                }
                "update_move" => json!({"scene_token":token,"move_id":"1","distance":1.25}),
                "finish_move" => json!({"scene_token":token,"move_id":"1","finish":"cancel"}),
                "create_primitive" => json!({"scene_token":token,"primitive":"cube"}),
                "select_node" => json!({"scene_token":token,"node_id":null}),
                "set_selection_mode" => json!({"scene_token":token,"mode":"face"}),
                "select_face" => json!({"scene_token":token,"target":null}),
                "rename_node" => json!({"scene_token":token,"node_id":"1","name":"cube"}),
                "edit_node" => json!({"scene_token":token,"node_id":"1"}),
                "set_material_diffuse" => json!({"scene_token":token,"shape_id":"2","value":1.25}),
                "extrude_face" => {
                    json!({"scene_token":token,"node_id":"1","shape_id":"2","instance_path":["1","2"],"face_index":0,"distance":1})
                }
                "set_transform" => {
                    json!({"scene_token":token,"node_id":"1","translation":[0,0,0],"rotation_degrees":[0,0,0],"scale":[1,1,1]})
                }
                "delete_node" => json!({"scene_token":token,"node_id":"1"}),
                "orbit_camera" => json!({"scene_token":token,"radians":[0,0]}),
                "pan_camera" => json!({"scene_token":token,"world_units":[0,0]}),
                "dolly_camera" => json!({"scene_token":token,"factor":1}),
                "set_grid_visibility" => json!({"scene_token":token,"visible":true}),
                "open_scene" | "save_scene_as" => json!({"scene_token":token,"path":"scene.mb"}),
                "resolve_file_action" => {
                    json!({"scene_token":token,"pending_action_id":Uuid::new_v4().to_string(),"decision":"cancel"})
                }
                _ => json!({"scene_token":token}),
            };
            assert!(parse_request(&tool.name, args.as_object().unwrap().clone()).is_ok());
        }
    }

    #[test]
    fn only_absolute_state_setters_are_annotated_idempotent() {
        for tool in catalog() {
            let expected = matches!(
                tool.name.as_ref(),
                "set_grid_visibility"
                    | "set_active_tool"
                    | "select_node"
                    | "set_selection_mode"
                    | "select_face"
            );
            assert_eq!(
                tool.annotations.unwrap().idempotent_hint == Some(true),
                expected,
                "{}",
                tool.name
            );
        }
    }

    #[test]
    fn face_selection_schemas_and_parser_require_explicit_mode_and_complete_target_or_null() {
        let token = Uuid::new_v4().to_string();
        let tools = catalog();
        let face = tools
            .iter()
            .find(|tool| tool.name == "select_face")
            .unwrap();
        assert_eq!(
            face.input_schema["required"],
            json!(["scene_token", "target"])
        );
        let target_schema = &face.input_schema["$defs"]["FaceSelectionTarget"];
        assert_eq!(target_schema["additionalProperties"], false);
        assert_eq!(target_schema["properties"]["instance_path"]["minItems"], 2);
        assert_eq!(
            target_schema["properties"]["instance_path"]["maxItems"],
            257
        );
        assert!(
            face.input_schema["properties"]["target"]
                .to_string()
                .contains("null")
        );
        for mode in ["object", "face"] {
            let args = json!({"scene_token":token,"mode":mode});
            assert!(matches!(
                parse_request("set_selection_mode", args.as_object().unwrap().clone())
                    .unwrap()
                    .operation,
                Operation::SetSelectionMode(_)
            ));
        }
        for mode in [json!(null), json!(false), json!("vertex"), json!("Face")] {
            let args = json!({"scene_token":token,"mode":mode});
            assert!(
                parse_request("set_selection_mode", args.as_object().unwrap().clone()).is_err()
            );
        }
        let clear = json!({"scene_token":token,"target":null});
        assert!(matches!(
            parse_request("select_face", clear.as_object().unwrap().clone())
                .unwrap()
                .operation,
            Operation::SelectFace { target: None }
        ));
        let valid = json!({"scene_token":token,"target":{"node_id":"1","shape_id":"2","instance_path":["1","2"],"face_index":0}});
        let Operation::SelectFace {
            target: Some(target),
        } = parse_request("select_face", valid.as_object().unwrap().clone())
            .unwrap()
            .operation
        else {
            panic!("expected face target")
        };
        assert_eq!(target.transform.to_decimal(), "1");
        assert_eq!(target.shape.to_decimal(), "2");
        assert_eq!(
            target
                .path
                .iter()
                .map(|id| id.to_decimal())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert_eq!(target.face_index, 0);
        for (field, values) in [
            (
                "node_id",
                vec![json!(1), json!("+1"), json!("18446744073709551616")],
            ),
            ("shape_id", vec![json!(null), json!(""), json!("-1")]),
            (
                "instance_path",
                vec![
                    json!([]),
                    json!(["1"]),
                    json!(vec!["1"; 258]),
                    json!(["1", 2]),
                    json!(["1", "+2"]),
                ],
            ),
            (
                "face_index",
                vec![json!(-1), json!(0.5), json!("0"), json!(null)],
            ),
        ] {
            for value in values {
                let mut args = valid.clone();
                args["target"][field] = value;
                assert!(
                    parse_request("select_face", args.as_object().unwrap().clone()).is_err(),
                    "{field}"
                );
            }
        }
        for field in ["node_id", "shape_id", "instance_path", "face_index"] {
            let mut args = valid.clone();
            args["target"].as_object_mut().unwrap().remove(field);
            assert!(parse_request("select_face", args.as_object().unwrap().clone()).is_err());
        }
        for args in [
            json!({"scene_token":token}),
            json!({"scene_token":token,"target":null,"extra":true}),
            json!({"scene_token":token,"target":{"node_id":"1","shape_id":"2","instance_path":["1","2"],"face_index":0,"extra":true}}),
        ] {
            assert!(parse_request("select_face", args.as_object().unwrap().clone()).is_err());
        }
        let mut boundary = valid;
        boundary["target"]["instance_path"] = json!(vec!["1"; 257]);
        assert!(parse_request("select_face", boundary.as_object().unwrap().clone()).is_ok());
    }

    #[test]
    fn strict_parameters_reject_unknown_fields_invalid_ids_and_name_byte_overflow() {
        let token = Uuid::new_v4().to_string();
        for invalid in [json!({"extra":true}), json!({"scene_token":token})] {
            assert!(parse_request("inspect_scene", invalid.as_object().unwrap().clone()).is_err());
        }
        for id in [
            json!(1),
            json!("+1"),
            json!(""),
            json!("18446744073709551616"),
        ] {
            let args = json!({"scene_token":token,"node_id":id});
            assert!(parse_request("delete_node", args.as_object().unwrap().clone()).is_err());
        }
        let fitting = "é".repeat(MAX_NAME_BYTES / 2);
        let mut args = json!({"scene_token":token,"node_id":"1","name":fitting});
        assert!(parse_request("rename_node", args.as_object().unwrap().clone()).is_ok());
        args["name"] = format!("{}a", args["name"].as_str().unwrap()).into();
        assert!(parse_request("rename_node", args.as_object().unwrap().clone()).is_err());
    }

    #[test]
    fn grid_schema_and_decoder_require_an_explicit_boolean_and_current_token() {
        let token = Uuid::new_v4().to_string();
        let tool = catalog()
            .into_iter()
            .find(|tool| tool.name == "set_grid_visibility")
            .unwrap();
        assert_eq!(
            tool.input_schema["properties"]["visible"]["type"],
            "boolean"
        );
        assert_eq!(tool.input_schema["additionalProperties"], false);
        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
        assert!(required.contains(&json!("scene_token")));
        assert!(required.contains(&json!("visible")));
        assert_eq!(tool.annotations.unwrap().idempotent_hint, Some(true));
        for arguments in [
            json!({"scene_token": token}),
            json!({"visible": false}),
            json!({"scene_token": token, "visible": null}),
            json!({"scene_token": token, "visible": "false"}),
            json!({"scene_token": token, "visible": 0}),
            json!({"scene_token": token, "visible": []}),
            json!({"scene_token": token, "visible": {},}),
            json!({"scene_token": token, "visible": false, "extra": true}),
            json!({"scene_token": "invalid", "visible": false}),
        ] {
            assert!(
                parse_request(
                    "set_grid_visibility",
                    arguments.as_object().unwrap().clone()
                )
                .is_err()
            );
        }
        for visible in [false, true] {
            let request = parse_request(
                "set_grid_visibility",
                json!({"scene_token": token, "visible": visible})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
            assert_eq!(
                request.expected_scene_token,
                Some(Uuid::parse_str(&token).unwrap())
            );
            assert!(
                matches!(request.operation, Operation::SetGridVisible { visible: actual } if actual == visible)
            );
        }
    }

    #[test]
    fn native_file_and_instance_arguments_enforce_bounds_and_exact_decisions() {
        let token = Uuid::new_v4().to_string();
        let pending = Uuid::new_v4().to_string();
        for path in [String::new(), "x\0.mb".into(), "é".repeat(2049)] {
            let args = json!({"scene_token":token,"path":path});
            for tool in ["open_scene", "save_scene_as"] {
                assert!(parse_request(tool, args.as_object().unwrap().clone()).is_err());
            }
        }
        let args = json!({"scene_token":token,"path":"é".repeat(2048)});
        assert!(parse_request("open_scene", args.as_object().unwrap().clone()).is_ok());
        for path in [
            json!(["+1"]),
            json!([1]),
            json!(["18446744073709551616"]),
            json!(vec!["1"; 257]),
        ] {
            let args = json!({"scene_token":token,"node_id":"1","instance_path":path});
            assert!(parse_request("select_node", args.as_object().unwrap().clone()).is_err());
        }
        let args = json!({"scene_token":token,"node_id":null,"instance_path":[]});
        assert!(parse_request("select_node", args.as_object().unwrap().clone()).is_err());
        for args in [
            json!({"scene_token":token,"pending_action_id":pending,"decision":"save_as"}),
            json!({"scene_token":token,"pending_action_id":"bad","decision":"cancel"}),
            json!({"scene_token":token,"pending_action_id":pending,"decision":"cancel","path":"ignored.mb"}),
            json!({"scene_token":token,"path":"scene.mb","extra":true}),
        ] {
            let name = if args.get("pending_action_id").is_some() {
                "resolve_file_action"
            } else {
                "open_scene"
            };
            assert!(parse_request(name, args.as_object().unwrap().clone()).is_err());
        }
        let save = json!({"scene_token":token,"pending_action_id":pending,"decision":"save"});
        assert!(matches!(
            parse_request("resolve_file_action", save.as_object().unwrap().clone())
                .unwrap()
                .operation,
            Operation::ResolveFileAction {
                decision: FileDecision::Save,
                ..
            }
        ));
        let args = json!({"scene_token":token,"node_id":"1","translation":[16777217.125,0,0],"rotation_degrees":[90,0,0],"scale":[1,1,1]});
        let request = parse_request("set_transform", args.as_object().unwrap().clone()).unwrap();
        let Operation::SetTransform { transform, .. } = request.operation else {
            panic!("expected transform")
        };
        assert_eq!(transform.translation[0], 16777217.125);
        assert_eq!(transform.rotation_degrees[0], 90.0);
    }

    #[test]
    fn sparse_node_schema_and_decoder_preserve_component_omissions() {
        let token = Uuid::new_v4().to_string();
        let tool = catalog()
            .into_iter()
            .find(|tool| tool.name == "edit_node")
            .unwrap();
        for field in ["translation", "rotation_degrees", "scale"] {
            let field = &tool.input_schema["properties"][field];
            assert_eq!(field["type"], "array");
            assert_eq!(field["minItems"], 3);
            assert_eq!(field["maxItems"], 3);
        }
        let args = json!({"scene_token":token,"node_id":"1","name":null,"translation":[16777217.125,null,null]});
        let request = parse_request("edit_node", args.as_object().unwrap().clone()).unwrap();
        let Operation::EditNode {
            name,
            translation,
            rotation_degrees,
            scale,
            ..
        } = request.operation
        else {
            panic!("expected sparse edit")
        };
        assert!(name.is_none());
        assert_eq!(translation, [Some(16777217.125), None, None]);
        assert_eq!(rotation_degrees, [None; 3]);
        assert_eq!(scale, [None; 3]);
        for field in ["translation", "rotation_degrees", "scale"] {
            for invalid in [
                json!(null),
                json!([]),
                json!([1, 2]),
                json!([1, 2, 3, 4]),
                json!([false, null, null]),
                json!(["1", null, null]),
            ] {
                let mut args = json!({"scene_token":token,"node_id":"1"});
                args[field] = invalid;
                assert!(parse_request("edit_node", args.as_object().unwrap().clone()).is_err());
            }
        }
        for fields in [
            json!({"unknown":true}),
            json!({"node_id":1}),
            json!({"node_id":"+1"}),
            json!({"name":"x".repeat(MAX_NAME_BYTES + 1)}),
        ] {
            let mut args = json!({"scene_token":token,"node_id":"1"});
            args.as_object_mut()
                .unwrap()
                .extend(fields.as_object().unwrap().clone());
            assert!(parse_request("edit_node", args.as_object().unwrap().clone()).is_err());
        }
        assert!(
            serde_json::from_str::<JsonObject>(&format!(
                r#"{{"scene_token":"{token}","node_id":"1","translation":[1e400,null,null]}}"#
            ))
            .is_err()
        );
    }

    #[test]
    fn extrusion_schema_and_runtime_require_explicit_bounded_targets() {
        let token = Uuid::new_v4().to_string();
        let tool = catalog()
            .into_iter()
            .find(|tool| tool.name == "extrude_face")
            .unwrap();
        assert_eq!(
            tool.input_schema["properties"]["instance_path"]["minItems"],
            2
        );
        assert_eq!(
            tool.input_schema["properties"]["instance_path"]["maxItems"],
            257
        );
        let valid = json!({"scene_token":token,"node_id":"1","shape_id":"2","instance_path":["1","2"],"face_index":1,"distance":-0.75});
        let request = parse_request("extrude_face", valid.as_object().unwrap().clone()).unwrap();
        let Operation::ExtrudeFace { target, distance } = request.operation else {
            panic!("expected extrusion")
        };
        assert_eq!(target.transform.to_decimal(), "1");
        assert_eq!(target.shape.to_decimal(), "2");
        assert_eq!(target.face_index, 1);
        assert_eq!(distance, -0.75);
        for (field, values) in [
            (
                "instance_path",
                vec![
                    json!(null),
                    json!([]),
                    json!(["1"]),
                    json!(vec!["1"; 258]),
                    json!([1, "2"]),
                    json!(["+1", "2"]),
                    json!(["1", "18446744073709551616"]),
                ],
            ),
            (
                "node_id",
                vec![
                    json!(1),
                    json!(""),
                    json!("+1"),
                    json!("18446744073709551616"),
                ],
            ),
            (
                "shape_id",
                vec![
                    json!(2),
                    json!(""),
                    json!("+2"),
                    json!("18446744073709551616"),
                ],
            ),
            (
                "face_index",
                vec![json!(-1), json!(1.5), json!("1"), json!(null)],
            ),
            ("distance", vec![json!(null), json!("1"), json!(false)]),
        ] {
            for value in values {
                let mut args = valid.clone();
                args[field] = value;
                assert!(
                    parse_request("extrude_face", args.as_object().unwrap().clone()).is_err(),
                    "{field}"
                );
            }
        }
        for required in [
            "node_id",
            "shape_id",
            "instance_path",
            "face_index",
            "distance",
        ] {
            let mut args = valid.clone();
            args.as_object_mut().unwrap().remove(required);
            assert!(parse_request("extrude_face", args.as_object().unwrap().clone()).is_err());
        }
        let mut args = valid.clone();
        args["extra"] = json!(true);
        assert!(parse_request("extrude_face", args.as_object().unwrap().clone()).is_err());
        let mut args = valid;
        args["instance_path"] = json!(vec!["1"; 257]);
        assert!(parse_request("extrude_face", args.as_object().unwrap().clone()).is_ok());
    }

    #[test]
    fn result_bound_covers_structured_and_textual_copies_and_surfaces_size_error() {
        let result = tool_reply(Ok(OperationReply::Created {
            node_id: "x".repeat(MAX_RESULT_BYTES),
        }));
        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            result.structured_content.as_ref().unwrap()["error"]["code"],
            "too_large"
        );
        assert!(check_serialized_size(&result, MAX_RESULT_BYTES).is_ok());
    }
}
