use crate::{
    editor_operations::{Operation, RemoteRequest},
    mcp::{
        args::{decode, native_id, schema, scoped, token},
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
struct MaterialDiffuseArgs {
    scene_token: String,
    shape_id: String,
    value: f64,
}

fn parse_set_material_diffuse(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: MaterialDiffuseArgs = decode(arguments)?;
    if !args.value.is_finite() || args.value < 0.0 {
        return Err(ErrorData::invalid_params(
            "Diffuse must be finite and nonnegative",
            None,
        ));
    }
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::SetMaterialDiffuse {
            shape: native_id(&args.shape_id)?,
            value: args.value,
        },
    ))
}

pub(in crate::mcp) const TOOLS: &[ToolSpec] = &[ToolSpec {
    name: "set_material_diffuse",
    description: "Edit the checked saved Phong diffuse scalar for an exact shape through shared undo history. Requires complete shared-shader users; affects all those shapes. Values are finite and nonnegative, may exceed one, and do not enable rendered materials",
    schema: schema::<MaterialDiffuseArgs>,
    parse: parse_set_material_diffuse,
    idempotent: false,
    read_only: false,
}];
