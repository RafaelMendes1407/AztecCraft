use super::features::{
    extrude, file, history, inspect, material, move_tool, node_edit, selection, view,
};
use crate::editor_operations::{MAX_NAME_BYTES, RemoteRequest};
use rmcp::model::{ErrorData, JsonObject, Tool, ToolAnnotations};
use serde_json::Value;

/// One advertised tool: its catalogue entry and the decoder for its arguments.
pub(super) struct ToolSpec {
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    pub(super) schema: fn() -> JsonObject,
    pub(super) parse: fn(JsonObject) -> Result<RemoteRequest, ErrorData>,
    /// Repeating the call with the same arguments returns `unchanged`.
    pub(super) idempotent: bool,
    pub(super) read_only: bool,
}

/// Catalogue order is the order of this list, then each feature's own order.
const FEATURES: &[&[ToolSpec]] = &[
    inspect::TOOLS,
    move_tool::TOOLS,
    selection::TOOLS,
    node_edit::TOOLS,
    extrude::TOOLS,
    material::TOOLS,
    history::TOOLS,
    view::TOOLS,
    file::TOOLS,
];

pub(super) fn specs() -> impl Iterator<Item = &'static ToolSpec> {
    FEATURES.iter().flat_map(|tools| tools.iter())
}

#[cfg(test)]
pub(crate) fn tool_names() -> impl Iterator<Item = &'static str> {
    specs().map(|spec| spec.name)
}

pub(super) fn catalog() -> Vec<Tool> {
    specs()
        .map(|spec| {
            Tool::new(spec.name, spec.description, (spec.schema)()).with_annotations(
                ToolAnnotations::new()
                    .read_only(spec.read_only)
                    .idempotent(spec.idempotent)
                    .open_world(false),
            )
        })
        .collect()
}

pub(super) fn parse_request(name: &str, arguments: JsonObject) -> Result<RemoteRequest, ErrorData> {
    if arguments
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.len() > MAX_NAME_BYTES)
    {
        return Err(ErrorData::invalid_params(
            "name exceeds 4096 UTF-8 bytes",
            None,
        ));
    }
    match specs().find(|spec| spec.name == name) {
        Some(spec) => (spec.parse)(arguments),
        None => Err(ErrorData::new(
            rmcp::model::ErrorCode::METHOD_NOT_FOUND,
            "Unknown AztecCraft tool",
            None,
        )),
    }
}
