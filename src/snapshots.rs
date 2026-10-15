//! Byte-compared text snapshots under `tests/snapshots`.
//!
//! `UPDATE_SNAPSHOTS=1` rewrites a snapshot; every other run fails on any difference, including a
//! missing file. Snapshots are public, so callers normalize machine-specific values first.

// Only the SDK snapshot tests use this.
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
