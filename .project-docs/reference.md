# Reference dossier

## Maya

Provisional baseline: Autodesk Maya 2026. Exact patch, platform, display scale, preferences, workspace, and plugins are unconfirmed. No installed reference build or user-owned reference scenes have been verified. Exact-parity acceptance is blocked until these are available; implementation can proceed without claiming parity.

Official reference: https://help.autodesk.com/view/MAYAUL/2026/ENU/

## Storytold research

Inspected upstream manifests on 2026-10-14:

- https://github.com/storytold/artcraft: creative scene staging and artist-oriented workflows; product inspiration, not a Maya compatibility implementation.
- https://github.com/storytold/craft-libs/blob/main/Cargo.toml: Rust 2024 workspace, egui 0.36, shared UI/settings/atomic-save components, MIT OR Apache-2.0 workspace license.
- https://github.com/storytold/photocraft/blob/main/Cargo.toml: modular Rust workspace, eframe/egui 0.36, headless document/operations layers, pinned craft-ui revision `5c874d51eeb3de70e735c8ce87a59a7dedd47675`.
- https://docs.rs/eframe/0.36.2/eframe/: current native application API.

Additional source inspection: `crates/atomic/src/lib.rs` provides `atomic_write` and create-new `write_new` with same-directory staging, content flush, cleanup and permission handling. `crates/ui/src/lib.rs` exports layout, preferences, render, theme and widgets. Evaluate a pinned craft-atomic dependency for persistence rather than copying its save engine; inspect the chosen revision and its license before adoption. Only the UI module exports were inspected, not suitability of each widget.

No upstream implementation code has been copied. Existing document/editor functionality is absent in this empty project. Use eframe/egui provisionally for a native fidelity spike, with a pure Rust scene layer independent of UI and rendering. Manifests alone do not establish reuse suitability.

## Fidelity gate

egui is provisional, not proven capable of identical Maya UI. Evaluate docking, marking menus, numeric editing, focus routing, styling, and reference screenshots before adopting it as the final UI platform. A first projected viewport establishes functionality, not a production renderer or Maya parity.
