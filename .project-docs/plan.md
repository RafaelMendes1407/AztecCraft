# Implementation plan

Status: planning. No pushes authorized before the publication gate.

## Product and publication gate

Final objective: a Maya-identical Rust desktop application. Work locally in incremental commits. The user confirmed that the first GitHub push requires verified 80–90% coverage of **all Maya 2026 functionality, including simulation, rendering, scripting, and plugin compatibility**. Expand and pin the complete workflow inventory before reporting any percentage. Do not redefine it as a small modeling subset.

## Atomic local tasks

1. Reference and scaffold: `AGENTS.md`, `README.md`, `.gitignore`, `.editorconfig`, `.project-docs/*`, `known-issues.md`. Evidence: source links, truthful scope, no parity/publish claim; two independent review passes. Commit before source implementation.
2. Scene model/transforms: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/scene.rs`, `tests/scene_workflows.rs`, build targets and development instructions. Stable IDs, cube/plane geometry and transform editing for the desktop editor. Proof: topology, transformed vertices, ID uniqueness and invalid-input rejection. No speculative graph/plugin abstraction.
3. Undo/redo: `src/history.rs`, `tests/history_workflows.rs`. Desktop editing commands require reversible create/delete/transform actions. Proof: multi-action undo/redo, redo invalidation, failed-action nonmutation, exact scene restoration.
4. Persistence: `src/document.rs`, `tests/document_workflows.rs`. Versioned JSON for desktop save/open. Initial saves are create-new/no-clobber; occupied paths retain original bytes. Proof: round trips, malformed/version-invalid files, occupied paths, failed-open nonmutation, staged-write failures. Reuse a suitable inspected save library rather than silently copying its implementation.
5. OBJ export: `src/obj.rs`, `tests/obj_workflows.rs`. Desktop export writes world-transformed polygon geometry. Proof: multi-mesh vertex offsets, transformed vertices and malformed-name handling; no import/format compatibility claim.
6. Native shell: `src/main.rs`, `src/app.rs`, dependency lock and instructions. Dense dark provisional Maya-style menu/shelf/Outliner/Channel Box UI; real create/select/numeric editing and history, unsupported controls explicit. Proof: native build/startup, interaction checks. This spike does not establish pixel parity or docking completeness.
7. Viewport: `src/viewport.rs`, `tests/viewport_workflows.rs`, `src/app.rs` integration. Orbit/pan/dolly, projection, primitive drawing, picking and framing. Proof: projection/picking tests and native navigation/selection; projected drawing is provisional, not a depth-tested production renderer.
8. File/UI integration: `src/app.rs`, native workflow checks. Connect save/open/export actions to tested scene APIs, surface errors, preserve current scene on invalid open and protect unsaved work. Proof: UI create/edit/undo/save/reopen/export journey.
9. Modeling vertical slices: topology/component selection and extrusion, construction history, bevel/weld/loops and normals, each independently committed and reviewed. Test topology validity and realistic editing workflows before broadening tools.
10. Subsequent independently reviewed slices: UV, animation, rigging, materials, renderer, interchange, scripting and extended systems from `parity.md`. Plan each with exact workflows and reference evidence before writing code.

Tasks 2–8 have real callers: desktop authoring actions and scene file handling. Scene data remains separate from viewport caches so persistence and editing are testable without a window. Projected drawing is a fidelity spike, not a replacement for a depth-tested renderer.

## Verification and review

Keep the same non-author reviewer across the two plan passes and each two-pass diff gate. Use a cold independent verifier for the completed vertical slice. Run formatting, Clippy, headless workflow tests, and native build. Record exact tree/commit identifiers for reviewed work. Reference comparison is blocked until a Maya installation/reference manifest exists. No positive parity claim without it.

## Isolation

After bootstrap commit and plan review, create a feature worktree under `~/.claude/worktrees/azteccraft-initial` and retain it. Record ownership and reviewed hashes in the session plan. Do not remove existing state. Do not configure or push a GitHub remote before the publication gate passes.
