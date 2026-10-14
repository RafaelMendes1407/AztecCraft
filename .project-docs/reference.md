# Reference dossier

## Maya

Provisional baseline: Autodesk Maya 2026. Exact patch, platform, display scale, preferences, workspace, and plugins are unconfirmed. No installed reference build or user-owned reference scenes have been verified. Exact-parity acceptance is blocked until these are available; implementation can proceed without claiming parity.

Official references: [Maya 2026 Help](https://help.autodesk.com/view/MAYAUL/2026/ENU/) and [Maya 2026 Developer Help](https://help.autodesk.com/view/MAYADEV/2026/ENU/).

[The workflow inventory seed](inventory-seed.md) records candidate domains and primary source links. Its 100 initial entries aren't a complete denominator, and overall coverage remains unmeasured. Audit commands, node types, menus, extensions and real production workflows against the installation before assigning a percentage.

## Reference manifest to establish

Record the exact Maya 2026.x build and full Maya product entitlement, platform/CPU architecture, GPU/driver, locale/display scale, workspace/preferences, OCIO configuration and time/linear/angular units. Record every installed plug-in's version, API version, loaded state and entitlement. Keep machine-specific paths and user-owned assets local.

Component versions can change independently of the Maya year. [Bifrost](https://help.autodesk.com/cloudhelp/ENU/Bifrost-MayaPlugin/files/Bifrost_MayaPlugin_install_bifrost_for_maya_html.html) is included but separately released, and includes Graph, legacy Fluids, BOSS and `bifcmd`. [MayaUSD](https://help.autodesk.com/cloudhelp/2026/ENU/Maya-USD/files/GUID-8FB49D7F-8651-47CE-80FC-5C940E568C97.html) is included and also separately versioned. Pin Arnold/MtoA, LookdevX, MASH, XGen and Substance versions rather than substituting their latest releases.

[Maya 2026.1](https://help.autodesk.com/cloudhelp/2026/ENU/Maya-WhatsNew/files/GUID-5D9FAEE0-CB9A-41AA-9A7D-014F266C6578.htm) introduces MotionMaker, while [2026.2](https://help.autodesk.com/cloudhelp/2026/ENU/Maya-WhatsNew/files/GUID-0C8224D8-23ED-417D-9630-1F925DC1767C.htm) changes further component versions. Features specific to an update belong in the baseline only when that update is pinned.

Flow Retopology is documented as included with Maya 2026, but cloud execution and Flow Wedging need service/entitlement verification. Golaem is documented as part of the Media & Entertainment Collection, so its inclusion in the requested boundary remains unresolved. Existing third-party scripts and plug-ins also require an explicit fixture inventory, architecture/API checks and execution evidence. A plug-in written only for AztecCraft doesn't prove compatibility with an existing Maya plug-in.

## Storytold research

Inspected upstream manifests on 2026-10-14:

- https://github.com/storytold/artcraft: creative scene staging and artist-oriented workflows; product inspiration, not a Maya compatibility implementation.
- https://github.com/storytold/craft-libs/blob/main/Cargo.toml: Rust 2024 workspace, egui 0.36, shared UI/settings/atomic-save components, MIT OR Apache-2.0 workspace license.
- https://github.com/storytold/photocraft/blob/main/Cargo.toml: modular Rust workspace, eframe/egui 0.36, headless document/operations layers, pinned craft-ui revision `5c874d51eeb3de70e735c8ce87a59a7dedd47675`.
- https://docs.rs/eframe/0.36.2/eframe/: current native application API.

Additional source inspection: `crates/atomic/src/lib.rs` provides `atomic_write` and create-new `write_new` with same-directory staging, content flush, cleanup and permission handling. The document layer now adopts `craft-atomic` at revision `457a078809690994b22debdffb703ff36d3129df` as a dependency for create-new writes. `crates/ui/src/lib.rs` exports layout, preferences, render, theme and widgets. Only the UI module exports were inspected, not suitability of each widget.

Upstream atomic-save code is consumed as a pinned dependency. The scene/history/document core and camera source exist, while a native editor is still planned. Evaluate eframe/egui provisionally for a native fidelity spike, with scene data independent of UI and rendering. Manifests alone don't establish reuse suitability. Upstream licenses don't select AztecCraft's distribution license.

## Fidelity gate

egui is provisional, not proven capable of identical Maya UI. Evaluate docking, marking menus, numeric editing, focus routing, styling, and reference screenshots before adopting it as the final UI platform. A first projected viewport establishes functionality, not a production renderer or Maya parity.

Native desktop verification is pending until an executable exists. It must exercise startup, primitive creation, selection, numeric edits, undo/redo, orbit/pan/dolly, file dialogs, save/reopen/export, unsaved-work handling and error presentation on each claimed platform. Capture exact commit/build identifiers and outcomes. Differential Maya comparisons additionally require matching fixtures, settings and retained output evidence.
