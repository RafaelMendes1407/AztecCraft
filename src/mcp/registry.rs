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

#[cfg(test)]
mod tests {
    use super::*;

    const DOC_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.project-docs/mcp-tools.md");

    fn render() -> String {
        let mut doc = String::from(
            "# MCP tool catalogue\n\nGenerated from the tool registry in `src/mcp/registry.rs`; do not edit by hand. \
Regenerate with `UPDATE_DOCS=1 cargo test --features mcp registry::tests::tool_doc`. \
Argument schemas are served by `tools/list`; see [mcp.md](mcp.md) for behaviour and limits.\n\n",
        );
        doc.push_str(&format!("{} tools.\n\n", specs().count()));
        doc.push_str("| Tool | Access | Idempotent | Description |\n|---|---|---|---|\n");
        for spec in specs() {
            doc.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                spec.name,
                if spec.read_only {
                    "read-only"
                } else {
                    "mutating"
                },
                if spec.idempotent { "yes" } else { "no" },
                spec.description.replace('|', "\\|"),
            ));
        }
        doc
    }

    #[test]
    fn tool_doc_matches_registry() {
        let rendered = render();
        if std::env::var_os("UPDATE_DOCS").is_some() {
            std::fs::write(DOC_PATH, &rendered).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(DOC_PATH).expect("run with UPDATE_DOCS=1 to create it"),
            rendered,
            "stale .project-docs/mcp-tools.md; regenerate with UPDATE_DOCS=1"
        );
    }

    #[test]
    fn tool_names_are_unique() {
        let mut names = tool_names().collect::<Vec<_>>();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total);
    }
}
