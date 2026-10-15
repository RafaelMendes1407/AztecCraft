//! Entry-point gate for the operation pipelines.
//!
//! Claim: every `Operation` mutates the editor only through `apply_operation_guarded` (reached
//! natively via `apply_operation` and remotely via `remote_operation_guarded`), so the guard
//! order recorded in the guard matrix cannot be bypassed.
//!
//! What is checked, over production text only (everything before a file's first
//! `#[cfg(test)] mod`, with `//` comments removed):
//! 1. `apply_operation_guarded(` is called only from `apply_operation` and
//!    `remote_operation_guarded`.
//! 2. `remote_operation_guarded(` is called only from `remote_operation` (a test wrapper) and the
//!    MCP bridge, and nothing under `src/mcp*` reaches the editor any other way.
//! 3. Every call site of the history mutators (`.execute(`, `.undo()`, `.redo()`,
//!    `.begin_move(`, `.update_move(`, `.finish_move(`, `.cancel_active_move(`, `History::new(`)
//!    and every `self.history =` assignment sits in an allowlisted function.
//!
//! Why it is sound enough: the editor state is private to `app`, so a mutation needs one of these
//! textual call forms in a function that is named here. A new caller changes the discovered set
//! and fails the test until a reviewer adds it. Limits: it is textual. Macros, aliasing a
//! mutator through a closure or function pointer, or a `std::mem::swap` of `history` evade it,
//! and function identity is the nearest preceding `fn` name (nested closures count as their
//! parent). Treat a failure as a prompt to re-read the guard ledger, not as proof of a bug.

use std::collections::BTreeSet;
use std::path::Path;

const FILES: [&str; 8] = [
    "app.rs",
    "file_ui.rs",
    "move_ui.rs",
    "viewport_ui.rs",
    "editor_operations.rs",
    "mcp.rs",
    "mcp/tools.rs",
    "mcp/transport.rs",
];

fn production(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    let end = text.find("\n#[cfg(test)]\nmod ").unwrap_or(text.len());
    text[..end]
        .lines()
        .map(|line| line.split("//").next().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn defined_fn(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    if let Some(after) = rest.strip_prefix("pub") {
        rest = after.trim_start();
        if rest.starts_with('(') {
            rest = rest[rest.find(')')? + 1..].trim_start();
        }
    }
    for qualifier in ["const ", "async ", "unsafe "] {
        rest = rest.strip_prefix(qualifier).unwrap_or(rest);
    }
    let name: String = rest
        .strip_prefix("fn ")?
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// `file::fn` for each occurrence of `needle`, excluding the `fn needle` definition itself.
fn sites(needle: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for file in FILES {
        let mut current = "<module>".to_owned();
        for line in production(file).lines() {
            if let Some(name) = defined_fn(line) {
                current = name;
            }
            if line.contains(needle) && !line.contains(&format!("fn {needle}")) {
                found.insert(format!("{file}::{current}"));
            }
        }
    }
    found
}

fn expect(label: &str, needle: &str, allowed: &[&str]) {
    let found = sites(needle);
    let allowed: BTreeSet<String> = allowed.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        found, allowed,
        "{label}: call sites of `{needle}` changed; review the guard ledger before updating"
    );
}

#[test]
fn only_the_two_pipelines_call_apply_operation_guarded() {
    expect(
        "guarded dispatch",
        "apply_operation_guarded(",
        &[
            "app.rs::apply_operation",
            "app.rs::remote_operation_guarded",
        ],
    );
}

#[test]
fn remote_pipeline_is_entered_only_by_the_mcp_bridge() {
    expect(
        "remote dispatch",
        "remote_operation_guarded(",
        &["app.rs::remote_operation", "mcp.rs::dispatch"],
    );
    for file in ["mcp.rs", "mcp/tools.rs", "mcp/transport.rs"] {
        let text = production(file);
        for forbidden in [
            "apply_operation",
            "native_operation",
            ".history",
            "History::",
        ] {
            assert!(
                !text.contains(forbidden),
                "{file} reaches the editor through `{forbidden}` instead of remote_operation_guarded"
            );
        }
    }
}

#[test]
fn native_operation_callers_are_the_known_ui_handlers() {
    expect(
        "native wrapper",
        "apply_operation(",
        &[
            "app.rs::native_operation",
            "app.rs::viewport",
            "file_ui.rs::file_modal",
            "move_ui.rs::finish_native_sample",
            "move_ui.rs::handle_move_input",
        ],
    );
    expect(
        "native UI wrapper",
        "native_operation(",
        &[
            "app.rs::create",
            "app.rs::frame_selected",
            "app.rs::history_step",
            "app.rs::menus",
            "app.rs::outliner",
            "app.rs::render",
            "app.rs::select",
            "app.rs::shortcuts",
            "app.rs::viewport",
            "file_ui.rs::open_path",
            "file_ui.rs::request_close",
            "file_ui.rs::request_new",
            "file_ui.rs::save_as_dialog",
        ],
    );
}

#[test]
fn history_mutators_live_only_in_the_dispatch_and_move_handlers() {
    // commit_pending commits a human draft; it is the one documented non-Operation mutation.
    expect(
        "history execute",
        ".execute(",
        &["app.rs::apply_operation_guarded", "app.rs::commit_pending"],
    );
    expect(
        "history undo",
        ".undo()",
        &["app.rs::apply_operation_guarded"],
    );
    expect(
        "history redo",
        ".redo()",
        &["app.rs::apply_operation_guarded"],
    );
    expect(
        "move begin",
        ".begin_move(",
        &["move_ui.rs::begin_editor_move"],
    );
    expect(
        "move update",
        ".update_move(",
        &["move_ui.rs::update_editor_move"],
    );
    expect(
        "move finish",
        ".finish_move(",
        &["move_ui.rs::finish_editor_move"],
    );
    expect(
        "move cancel",
        ".cancel_active_move(",
        &["move_ui.rs::cancel_move"],
    );
    expect(
        "history replace",
        "self.history =",
        &["file_ui.rs::replace_scene"],
    );
    expect(
        "history new",
        "History::new(",
        &["file_ui.rs::replace_scene"],
    );
}
