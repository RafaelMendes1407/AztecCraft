# Operation guard ledger

Contributor guide to how an `Operation` reaches the editor. Two pipelines exist and stay explicit.
The behavior is recorded cell by cell in `tests/snapshots/guard_matrix.txt` (test
`guard_matrix_matches_committed_snapshot`); `entry_point_tests` fails when a new call site bypasses
them. Anchors below are `file:line` at the commit that added this file; refresh them when you move
code. Regenerate the matrix with `UPDATE_SNAPSHOTS=1` only for an intended behavior change.

## Pipelines

Remote, `remote_operation_guarded` (`src/app.rs:298`), checks in this order:

1. `Inspect` returns before any guard (`src/app.rs:304`).
2. Scene token mismatch is `StaleScene` (`src/app.rs:310`).
3. An active Move is `BusyManipulation` unless it is a Remote Move and the operation is
   `UpdateMove` or `FinishMove` (`src/app.rs:314`). This is default-deny.
4. Closing, modal (all but `ResolveFileAction`), focused text edit, any draft or pending native
   input is `BusyEditing` (`src/app.rs:322`).
5. Then `apply_operation_guarded` with provenance Remote (`src/app.rs:331`).

Native, `apply_operation` (`src/app.rs:622`) calls `apply_operation_guarded` (defined at `src/app.rs:626`, called at `src/app.rs:623`) with
provenance Native and no token or draft checks. In order:

1. `guard_move_dispatch` first, with side effects (`src/app.rs:632`, `src/move_ui.rs:82`). For
   `SetActiveTool`, `Select`, `SetSelectionMode`, `SelectFace`, `NewScene`, `OpenScene` and
   `RequestClose` it cancels any Move (`cancel_move`, `src/move_ui.rs:54`) and consumes the
   pointer sequence only on a real tool change that cancelled nothing
   (`src/move_ui.rs:99`). Other operations during a Move are `BusyManipulation` except
   `UpdateMove`/`FinishMove`.
2. Closing is `BusyEditing` (`src/app.rs:633`), so a native `NewScene` while closing cancels the
   Move and then fails.
3. `SetGridVisible` while modal is `BusyEditing` (`src/app.rs:636`).
4. File operations are decided in one match (`src/app.rs:639`), before any draft commit. For
   `OpenScene` and `SaveSceneAs`, path validity (`src/app.rs:641`) runs first. `OpenScene` then
   reaches `open_candidate` with no modal check (`src/app.rs:650`); `SaveSceneAs` is refused while
   modal (`src/app.rs:652`) and `SaveScene` likewise (`src/app.rs:659`). `NewScene`,
   `RequestClose` and `ResolveFileAction` decide internally (`src/app.rs:665`-`src/app.rs:670`).
5. `SetActiveTool` and the three Move operations are `BusyEditing` while modal (`src/app.rs:673`).
6. Validation before the draft commit: `selection_path` and Select-clear-with-path
   (`src/app.rs:684`), Face mode and face-target resolution for `SelectFace` (`src/app.rs:699`).
7. `commit_pending` (`src/app.rs:1091`) commits a valid human draft first, except for `UpdateMove`
   and `FinishMove` (`src/app.rs:719`). On failure the operation returns `InvalidInput(status)`
   and the status text and error flag change.
8. The operation handler runs and `sync_fields` refreshes Channel Box drafts.

## Behavior that stays inside handlers

These are not expressible as a shared guard and must not be hoisted without a new matrix review.

- `may_install` is threaded through `save_document` (`src/file_ui.rs:127`) and
  `resolve_file_action_guarded` (`src/file_ui.rs:370`, `src/file_ui.rs:442`). A refusal is
  `NativeSave: save cancelled before installation`. Only a dirty Maya binary document reaches it
  (matrix states `no_install`, `no_install_modal`).
- Read-only Maya ASCII is decided by the scene (`authoring_read_only_reason`,
  `src/native_scene.rs:934`) and surfaces as `ReadOnlyFormat`; Move maps it in
  `src/move_ui.rs:156`.
- `EditNode` with no fields returns `Unchanged` after only checking the node (`src/app.rs:773`).
- Delete in Face mode is refused only in `native_operation` (`src/app.rs:243`), above the
  pipeline. The matrix calls the pipeline directly, so its `delete(face_mode)` cells show the
  operation succeeding in both columns; `native_operation_refuses_face_mode_delete_before_the_pipeline`
  covers the UI refusal. The remote pipeline has no such refusal today, which is a bug
  (cristim/AztecCraft#83, listed in `known-issues.md`); the snapshot records current behavior and
  must change with the fix. The Move Face-mode refusal is in `begin_editor_move`
  (`src/move_ui.rs:138`) and applies to both pipelines (`begin_move(face_mode)` cells).
- The `read_only_ascii` fixture is a curve-only scene with no mesh. `set_material_diffuse`,
  `select_face(face_mode)` and `extrude_face` therefore fail there with a missing-shape error, not
  `ReadOnlyFormat`; read-only refusal is shown by `create`, `delete`, `rename` and the other
  edits that pass target validation.

## Observed behavior (recorded, not endorsed)

The matrix records these asymmetries; treat a change as a behavior change that needs review.

- Inspect: remote `inspect` is `Ok` in every state. Native `inspect` is `BusyManipulation` during
  a Move and `BusyEditing` while closing, and `Ok` under a modal.
- Native mutations (`create`, `delete`, `rename`, `undo`) succeed under a modal pending file
  action; the remote pipeline refuses all of them as `BusyEditing`.
- Native `set_active_tool` while closing consumes the pointer sequence (the Move guard runs
  first) and then fails with `BusyEditing`.
- Native `delete` with a changed material draft commits the draft first, then fails; the status
  becomes "Material data edited" and the scene has changed even though the result is an `Err`.
- Human-draft commits (`commit_pending`) are the only history mutation besides operations; the
  entry-point test names them.

## Safety net

| Check | Where |
| --- | --- |
| Tool catalog bytes | `tests/snapshots/mcp_tool_catalog.json` |
| Inspection bytes for four editor states | `tests/snapshots/inspection_*.json` |
| Operation x pipeline x state matrix, all platforms | `tests/snapshots/guard_matrix.txt` |
| Matrix cells that write files (Save, Save As, Resolve(save)), Unix only | `tests/snapshots/guard_matrix_file_writing_unix.txt` |
| Entry points | `src/entry_point_tests.rs` |

Snapshots are public: they hold no scene
tokens, UUIDs, absolute paths or home directories (`src/snapshots.rs` rejects those).
