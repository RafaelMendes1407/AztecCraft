use crate::{
    editor_operations::{Operation, RemoteRequest, SidebarPanel},
    mcp::{
        args::{decode, schema, scoped, token},
        registry::ToolSpec,
    },
};
use rmcp::{
    model::{ErrorData, JsonObject},
    schemars::{self, JsonSchema},
};
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PanelInput {
    ChannelBox,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SidebarPanelArgs {
    scene_token: String,
    panel: PanelInput,
    visible: bool,
}

fn parse_set_sidebar_panel_visibility(arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    let args: SidebarPanelArgs = decode(arguments)?;
    let panel = match args.panel {
        PanelInput::ChannelBox => SidebarPanel::ChannelBox,
    };
    Ok(scoped(
        token(&args.scene_token)?,
        Operation::SetSidebarPanelVisible {
            panel,
            visible: args.visible,
        },
    ))
}

pub(in crate::mcp) const TOOLS: &[ToolSpec] = &[ToolSpec {
    name: "set_sidebar_panel_visibility",
    description: "Set one sidebar panel's visibility explicitly. Session-only and idempotent; does not edit the scene. Only channel_box is implemented",
    schema: schema::<SidebarPanelArgs>,
    parse: parse_set_sidebar_panel_visibility,
    idempotent: true,
    read_only: false,
}];
