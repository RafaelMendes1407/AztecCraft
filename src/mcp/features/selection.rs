use crate::{
    editor_operations::{
        FaceSelectionTarget, Operation, RemoteRequest, SelectionMode as EditorMode,
    },
    mcp::{
        args::{decode, native_id, node_id, schema, scoped, token},
        registry::ToolSpec,
    },
};
use azteccraft::{native_scene::FaceTarget, scene::Primitive};
use rmcp::{
    model::{ErrorData, JsonObject},
    schemars::{self, JsonSchema},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PrimitiveInput {
    Cube,
    Plane,
    Sphere,
    Cylinder,
    Cone,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    scene_token: String,
    primitive: PrimitiveInput,
    name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SelectArgs {
    scene_token: String,
    node_id: Option<String>,
    #[schemars(length(max = 256))]
    instance_path: Option<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "SelectionMode")]
enum SelectionMode {
    Object,
    Face,
    Hierarchy,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SelectionModeArgs {
    scene_token: String,
    mode: SelectionMode,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SelectFaceArgs {
    scene_token: String,
    target: Option<FaceSelectionTarget>,
}

fn parse_create_primitive(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: CreateArgs = decode(arguments)?;
    let primitive = match args.primitive {
        PrimitiveInput::Cube => Primitive::Cube,
        PrimitiveInput::Plane => Primitive::Plane,
        PrimitiveInput::Sphere => Primitive::Sphere,
        PrimitiveInput::Cylinder => Primitive::Cylinder,
        PrimitiveInput::Cone => Primitive::Cone,
    };
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::Create {
            primitive,
            name: args.name,
        },
    ))
}

fn parse_select_node(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: SelectArgs = decode(arguments)?;
    if args.node_id.is_none() && args.instance_path.is_some() {
        return Err(ErrorData::invalid_params(
            "Clearing selection cannot include an instance_path",
            None,
        ));
    }
    let instance_path = args
        .instance_path
        .map(|path| {
            if path.len() > 256 {
                return Err(ErrorData::invalid_params(
                    "instance_path exceeds 256 IDs",
                    None,
                ));
            }
            path.into_iter()
                .map(|value| native_id(&value))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::Select {
            node: args.node_id.as_deref().map(node_id).transpose()?,
            instance_path,
        },
    ))
}

fn parse_set_selection_mode(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: SelectionModeArgs = decode(arguments)?;
    let mode = match args.mode {
        SelectionMode::Object => EditorMode::Object,
        SelectionMode::Face => EditorMode::Face,
        SelectionMode::Hierarchy => EditorMode::Hierarchy,
    };
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::SetSelectionMode(mode),
    ))
}

fn parse_select_face(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    if !arguments.contains_key("target") {
        return Err(ErrorData::invalid_params(
            "target must be a complete face target or null",
            None,
        ));
    }
    let args: SelectFaceArgs = decode(arguments)?;
    let target = args
        .target
        .map(|target| {
            if !(2..=257).contains(&target.instance_path.len()) {
                return Err(ErrorData::invalid_params(
                    "Face selection requires 2 to 257 path IDs",
                    None,
                ));
            }
            Ok(FaceTarget {
                transform: node_id(&target.node_id)?,
                shape: native_id(&target.shape_id)?,
                path: target
                    .instance_path
                    .iter()
                    .map(|value| native_id(value))
                    .collect::<Result<_, _>>()?,
                face_index: target.face_index,
            })
        })
        .transpose()?;
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::SelectFace { target },
    ))
}

fn select_face_schema() -> JsonObject {
    let mut input = schema::<SelectFaceArgs>();
    input.insert("required".into(), json!(["scene_token", "target"]));
    input
}

pub(in crate::mcp) const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "create_primitive",
        description: "Create a cube, plane, sphere, cylinder or cone in the live editor",
        schema: schema::<CreateArgs>,
        parse: parse_create_primitive,
        idempotent: false,
        read_only: false,
    },
    ToolSpec {
        name: "select_node",
        description: "Select an object using an optional full transform instance_path; ambiguous objects require a path. Null node_id clears selection without a path",
        schema: schema::<SelectArgs>,
        parse: parse_select_node,
        idempotent: true,
        read_only: false,
    },
    ToolSpec {
        name: "set_selection_mode",
        description: "Set Object, Face or Hierarchy (Root sub-mask only) selection mode without editing the scene; a valid remembered face stays inactive outside Face mode",
        schema: schema::<SelectionModeArgs>,
        parse: parse_set_selection_mode,
        idempotent: true,
        read_only: false,
    },
    ToolSpec {
        name: "select_face",
        description: "Select a complete canonical face target in Face mode, or clear it with null; explicit targets need not be visible and select their owning instance",
        schema: select_face_schema,
        parse: parse_select_face,
        idempotent: true,
        read_only: false,
    },
];
