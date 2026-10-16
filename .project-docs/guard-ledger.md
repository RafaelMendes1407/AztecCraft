# Operation guard ledger

Contributor guide to how an `Operation` reaches the editor. Two pipelines exist and stay explicit.
The behavior is recorded cell by cell in `tests/snapshots/guard_matrix.txt` (test
`guard_matrix_matches_committed_snapshot`); `entry_point_tests` fails when a new call site bypasses
them. Anchors below are `file:line` at the commit that added this file; refresh them when you move
code. Regenerate the matrix with `UPDATE_SNAPSHOTS=1` only for an intended behavior change.

## Pipelines

Remote, `remote_operation_guarded` (`src/app/dispatch.rs:77`), checks in this order:

1. `Inspect` returns before any guard (`src/app/dispatch.rs:84`).
2. Scene token mismatch is `StaleScene` (`src/app/dispatch.rs:89`).
3. An active Move, Rotate or Scale is `BusyManipulation` unless it is Remote and the operation is
   `UpdateMove`, `UpdateManipulation` or `FinishMove` (`src/app/dispatch.rs:92`). This is default-deny.
4. Closing, modal (all but `ResolveFileAction`), focused text edit, any draft or pending native
   input is `BusyEditing` (`src/app/dispatch.rs:103`).
5. Then `apply_operation_guarded` with provenance Remote (`src/app/dispatch.rs:112`).

Native, `apply_operation` (`src/app/dispatch.rs:143`) calls `apply_operation_guarded` (defined at `src/app/dispatch.rs:149`, called at `src/app/dispatch.rs:147`) with
provenance Native and no token or draft checks. In order:

1. `guard_move_dispatch` first, with side effects (`src/app/dispatch.rs:155`, `src/app/features/move_tool.rs:139`). For
   `SetActiveTool`, `Select`, `SetSelectionMode`, `SelectFace`, `NewScene`, `OpenScene` and
   `RequestClose` it cancels any Move (`cancel_move`, `src/app/features/move_tool.rs:109`) and consumes the
   pointer sequence only on a real tool change that cancelled nothing
   (`src/app/features/move_tool.rs:134`). Other operations during a Move are `BusyManipulation` except
   `UpdateMove`/`FinishMove`.
2. Closing is `BusyEditing` (`src/app/dispatch.rs:156`), so a native `NewScene` while closing cancels the
   Move and then fails.
3. `SetGridVisible`, `SetShadingMode`, `SetSidebarPanelVisible` and `SelectAttributeEditorTab` while modal are `BusyEditing` (`src/app/dispatch.rs:179`).
4. File operations are decided in one match (`src/app/dispatch.rs:168`), before any draft commit. For
   `OpenScene` and `SaveSceneAs`, path validity (`src/app/dispatch.rs:173`) runs first. `OpenScene` then
   reaches `open_candidate` with no modal check (`src/app/dispatch.rs:179`); `SaveSceneAs` is refused while
   modal (`src/app/dispatch.rs:181`) and `SaveScene` likewise (`src/app/dispatch.rs:188`). `NewScene`,
   `RequestClose` and `ResolveFileAction` decide internally (`src/app/dispatch.rs:194`-`src/app/dispatch.rs:199`).
5. `SetActiveTool` and the Move, Rotate and Scale operations are `BusyEditing` while modal (`src/app/dispatch.rs:202`).
6. Validation before the draft commit: `selection_path` and Select-clear-with-path
   (`src/app/dispatch.rs:216`), Face mode and face-target resolution for `SelectFace` (`src/app/dispatch.rs:231`).
7. `commit_pending` (`src/app/dispatch.rs:615`) commits a valid human draft first, except for `UpdateMove`,
   `UpdateManipulation` and `FinishMove` (`src/app/dispatch.rs:253`). On failure the operation returns `InvalidInput(status)`
   and the status text and error flag change.
8. The operation handler runs and `sync_fields` refreshes Channel Box drafts.

## Behavior that stays inside handlers

These are not expressible as a shared guard and must not be hoisted without a new matrix review.

- `may_install` is threaded through `save_document` (`src/file_ui.rs:124`) and
  `resolve_file_action_guarded` (`src/file_ui.rs:369`, `src/file_ui.rs:445`). A refusal is
  `NativeSave: save cancelled before installation`. Only a dirty Maya binary document reaches it
  (matrix states `no_install`, `no_install_modal`).
- Read-only Maya ASCII is decided by the scene (`authoring_read_only_reason`,
  `src/native_scene.rs:989`) and surfaces as `ReadOnlyFormat`; Move, Rotate and Scale map it in
  `begin_error` (`src/app/features/manipulation_tool.rs:119`).
- `EditNode` with no fields returns `Unchanged` after only checking the node (`src/app/dispatch.rs:348`).
- Delete in Face mode is refused only in `native_operation` (`src/app/dispatch.rs:5`), above the
  pipeline. The matrix calls the pipeline directly, so its `delete(face_mode)` cells show the
  operation succeeding in both columns; `native_operation_refuses_face_mode_delete_before_the_pipeline`
  covers the UI refusal. The remote pipeline has no such refusal today, which is a bug
  (cristim/AztecCraft#83, listed in `known-issues.md`); the snapshot records current behavior and
  must change with the fix. The Move, Rotate and Scale Face-mode refusal is in the shared target check
  (`check_manipulation_target`, `src/app/features/manipulation_tool.rs:139`) and applies to both pipelines (`begin_move(face_mode)` cells).
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
- `SetShadingMode` follows Grid: session-only, idempotent (`Unchanged` when the mode already matches),
  never dirty, never a scene-token change; guards are those of `SetGridVisible`. Handler is in
  `src/app/dispatch.rs`; matrix cell is `set_shading_mode(wireframe)`.
- `SetSidebarPanelVisible` follows Grid: remote refuses a stale token, a Move (`BusyManipulation`), a
  differing draft, text focus, modal and closing (`BusyEditing`); native refuses closing and modal
  before any draft commit (`src/app/dispatch.rs:156`, `src/app/dispatch.rs:159`), is
  `BusyManipulation` during a manipulation (`src/app/features/move_tool.rs:139`), and otherwise commits a valid
  draft first and refuses an invalid one (`src/app/dispatch.rs:253`). The handler is
  `src/app/dispatch.rs:301`; matrix cells are `set_sidebar_panel_visible(channel_box,false)` and
  `set_sidebar_panel_visible(attribute_editor,true)`.
- `SelectAttributeEditorTab` has the same guards as `SetSidebarPanelVisible` (modal check
  `src/app/dispatch.rs:159`, the `commit_pending` gate `src/app/dispatch.rs:253`, Move default-deny).
  It is view state only and never enters history. After the draft commit it is refused as
  `InvalidInput` unless the Attribute Editor is the active dock panel (no hidden state that
  inspection cannot show), when the tab does not resolve, or when the shape is beyond the tab
  cap. The handler is `src/app/features/attribute_editor.rs` (`select_attribute_editor_tab`).
  Setting `AttributeEditor` visible hides the Channel Box (one dock panel), so
  `set_sidebar_panel_visible(attribute_editor,true)` is `Changed` from the default state. The
  matrix has 3 new samples per state: `set_sidebar_panel_visible(attribute_editor,true)`,
  `select_attribute_editor_tab(shape)` (Attribute Editor active) and
  `select_attribute_editor_tab(shape,panel_hidden)`. MCP reaches them as
  `set_sidebar_panel_visibility` (`panel` `attribute_editor`) and `select_attribute_editor_tab`,
  covered cell by cell in `src/mcp_tests/official_client/attribute_editor.rs`.
- The File menu and the Status Line file buttons share `file_actions` (`src/app/menus.rs`), the one
  native caller of `Operation::SaveScene`; New and Open go through `request_new` and `open_dialog`.
  Enable state, disabled reason and handler come from the same helper, so the guards are those of
  the menu path, and the entry-point allowlist names `file_actions` instead of `menus`.
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
