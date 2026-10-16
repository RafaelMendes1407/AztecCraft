use crate::{
    editor_operations::{Operation, RemoteRequest, TransformInput},
    mcp::{
        args::{decode, node_id, schema, scoped, token},
        registry::ToolSpec,
    },
};
use rmcp::{
    model::{ErrorData, JsonObject},
    schemars::{self, JsonSchema},
};
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RenameArgs {
    scene_token: String,
    node_id: String,
    name: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransformArgs {
    scene_token: String,
    node_id: String,
    translation: [f64; 3],
    rotation_degrees: [f64; 3],
    scale: [f64; 3],
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EditNodeArgs {
    scene_token: String,
    node_id: String,
    name: Option<String>,
    #[serde(default)]
    translation: [Option<f64>; 3],
    #[serde(default)]
    rotation_degrees: [Option<f64>; 3],
    #[serde(default)]
    scale: [Option<f64>; 3],
}

fn parse_rename_node(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: RenameArgs = decode(arguments)?;
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::Rename {
            id: node_id(&args.node_id)?,
            name: args.name,
        },
    ))
}

fn parse_set_transform(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: TransformArgs = decode(arguments)?;
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::SetTransform {
            id: node_id(&args.node_id)?,
            transform: TransformInput {
                translation: args.translation,
                rotation_degrees: args.rotation_degrees,
                scale: args.scale,
            },
        },
    ))
}

fn parse_edit_node(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: EditNodeArgs = decode(arguments)?;
    if args
        .translation
        .iter()
        .chain(&args.rotation_degrees)
        .chain(&args.scale)
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ErrorData::invalid_params(
            "Channel values must be finite",
            None,
        ));
    }
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::EditNode {
            id: node_id(&args.node_id)?,
            name: args.name,
            translation: args.translation,
            rotation_degrees: args.rotation_degrees,
            scale: args.scale,
        },
    ))
}

pub(in crate::mcp) const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "rename_node",
        description: "Rename an existing object",
        schema: schema::<RenameArgs>,
        parse: parse_rename_node,
        idempotent: false,
        read_only: false,
    },
    ToolSpec {
        name: "set_transform",
        description: "Set f64 translation in declared scene units, rotation in XYZ degrees, and scale",
        schema: schema::<TransformArgs>,
        parse: parse_set_transform,
        idempotent: false,
        read_only: false,
    },
    ToolSpec {
        name: "edit_node",
        description: "Edit optional name and individual f64 channels in one undo entry. Omitted arrays and null components are untouched; translation uses declared scene units and supplied rotations use degrees. Does not change selection",
        schema: schema::<EditNodeArgs>,
        parse: parse_edit_node,
        idempotent: false,
        read_only: false,
    },
];
