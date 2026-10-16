use crate::{
    editor_operations::{ActiveTool, MoveDistance, Operation, RemoteRequest},
    mcp::{
        args::{decode, move_id, native_id, node_id, schema, scoped, token},
        registry::ToolSpec,
    },
};
use azteccraft::{
    history::MoveFinish,
    native_scene::{TransformTarget, WorldAxis},
};
use rmcp::{
    model::{ErrorData, JsonObject},
    schemars::{self, JsonSchema},
};
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ActiveToolArgs {
    scene_token: String,
    tool: ActiveTool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum AxisInput {
    X,
    Y,
    Z,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BeginMoveArgs {
    scene_token: String,
    node_id: String,
    #[schemars(length(min = 1, max = 256))]
    instance_path: Vec<String>,
    axis: AxisInput,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateMoveArgs {
    scene_token: String,
    move_id: String,
    distance: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum FinishInput {
    Commit,
    Cancel,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FinishMoveArgs {
    scene_token: String,
    move_id: String,
    finish: FinishInput,
}

fn parse_set_active_tool(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: ActiveToolArgs = decode(arguments)?;
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::SetActiveTool(args.tool),
    ))
}

fn parse_begin_move(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: BeginMoveArgs = decode(arguments)?;
    if !(1..=256).contains(&args.instance_path.len()) {
        return Err(ErrorData::invalid_params(
            "instance_path must contain 1 to 256 IDs",
            None,
        ));
    }
    let path = args
        .instance_path
        .into_iter()
        .map(|value| native_id(&value))
        .collect::<Result<Vec<_>, _>>()?;
    let axis = match args.axis {
        AxisInput::X => WorldAxis::X,
        AxisInput::Y => WorldAxis::Y,
        AxisInput::Z => WorldAxis::Z,
    };
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::BeginMove {
            target: TransformTarget {
                node: node_id(&args.node_id)?,
                path,
            },
            axis,
        },
    ))
}

fn parse_update_move(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: UpdateMoveArgs = decode(arguments)?;
    if !args.distance.is_finite() {
        return Err(ErrorData::invalid_params(
            "Move distance must be finite",
            None,
        ));
    }
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::UpdateMove {
            id: move_id(&args.move_id)?,
            distance: MoveDistance::Declared(args.distance),
        },
    ))
}

fn parse_finish_move(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: FinishMoveArgs = decode(arguments)?;
    let finish = match args.finish {
        FinishInput::Commit => MoveFinish::Commit,
        FinishInput::Cancel => MoveFinish::Cancel,
    };
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::FinishMove {
            id: move_id(&args.move_id)?,
            finish,
        },
    ))
}

pub(in crate::mcp) const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "set_active_tool",
        description: "Select the native Select or World-axis Move tool; unsupported tools are not exposed",
        schema: schema::<ActiveToolArgs>,
        parse: parse_set_active_tool,
        idempotent: true,
        read_only: false,
    },
    ToolSpec {
        name: "begin_move",
        description: "Begin one remote-owned World-axis translation of the exact selected transform instance. Numeric motion may begin while Select is active; this does not change tools",
        schema: schema::<BeginMoveArgs>,
        parse: parse_begin_move,
        idempotent: false,
        read_only: false,
    },
    ToolSpec {
        name: "update_move",
        description: "Preview a finite absolute World-axis distance from begin in declared scene linear units. Matching scene token and remote-owned Move ID are required",
        schema: schema::<UpdateMoveArgs>,
        parse: parse_update_move,
        idempotent: false,
        read_only: false,
    },
    ToolSpec {
        name: "finish_move",
        description: "Commit one undoable Move edit or cancel to the exact begin authoring state. Only the matching remote-owned Move ID may finish",
        schema: schema::<FinishMoveArgs>,
        parse: parse_finish_move,
        idempotent: false,
        read_only: false,
    },
];
