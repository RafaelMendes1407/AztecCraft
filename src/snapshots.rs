//! Byte-compared text snapshots under `tests/snapshots`.
//!
//! `UPDATE_SNAPSHOTS=1` rewrites a snapshot; every other run fails on any difference, including a
//! missing file. Snapshots are public, so callers normalize machine-specific values first.

// Only the SDK snapshot tests use these.
#![cfg_attr(not(feature = "mcp-test-client"), allow(dead_code))]

use std::path::PathBuf;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(name)
}

pub(crate) fn assert_snapshot(name: &str, actual: &str) {
    let path = path(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some_and(|value| value == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "snapshot {name} is unreadable ({error}); regenerate with UPDATE_SNAPSHOTS=1 and review the diff"
        )
    });
    if expected != actual.as_bytes() {
        let expected = String::from_utf8_lossy(&expected);
        let line = expected
            .lines()
            .zip(actual.lines())
            .position(|(left, right)| left != right)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
        panic!(
            "snapshot {name} differs, first at line {}:\n  expected: {}\n  actual:   {}\nregenerate with UPDATE_SNAPSHOTS=1 only if the change is intended",
            line + 1,
            expected.lines().nth(line).unwrap_or("<end>"),
            actual.lines().nth(line).unwrap_or("<end>")
        );
    }
}

fn is_uuid(bytes: &[u8]) -> bool {
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

/// Replaces UUIDs (scene tokens, pending-action ids) and the given absolute roots, then refuses
/// text that still looks machine-specific so a snapshot cannot leak a home or temp path.
pub(crate) fn normalize_text(text: &str, roots: &[&std::path::Path]) -> String {
    let mut text = text.to_owned();
    for root in roots {
        let root = root.to_str().unwrap();
        text = text.replace(root, "<dir>");
        if let Ok(canonical) = std::fs::canonicalize(root) {
            text = text.replace(canonical.to_str().unwrap(), "<dir>");
        }
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if index + 36 <= bytes.len()
            && text.is_char_boundary(index + 36)
            && is_uuid(&bytes[index..index + 36])
        {
            out.push_str("<uuid>");
            index += 36;
        } else {
            let ch = text[index..].chars().next().unwrap();
            out.push(ch);
            index += ch.len_utf8();
        }
    }
    for forbidden in ["/Users/", "/home/", "/private/", "/tmp", "/var/", "C:\\"] {
        assert!(
            !out.contains(forbidden),
            "snapshot text contains a machine-specific path ({forbidden})"
        );
    }
    out
}

#[cfg(feature = "mcp")]
pub(crate) fn normalize_json(
    value: &serde_json::Value,
    roots: &[&std::path::Path],
) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::String(text) => Value::String(normalize_text(text, roots)),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| normalize_json(v, roots)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, v)| (key.clone(), normalize_json(v, roots)))
                .collect(),
        ),
        other => other.clone(),
    }
}
