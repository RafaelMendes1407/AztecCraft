# AztecCraft

AztecCraft is a Rust desktop 3D authoring project with the objective of reproducing Autodesk Maya 2026's interface and behavior, including modeling, animation, rigging, simulation, rendering, scripting and existing plug-in compatibility.

The current checkout contains a headless scene library with undo/redo and validated JSON scene files, plus a tested camera/projection/picking core awaiting desktop integration. There isn't a desktop application to launch yet, and overall Maya coverage is unmeasured.

[Current capabilities](#current-capabilities) · [Build and test](#build-and-test) · [Roadmap](#roadmap-and-publication) · [Project documentation](.project-docs/INDEX.md)

## Current capabilities

| Area | This checkout | Evidence or next step |
| --- | --- | --- |
| Scene data | Cube/plane topology, stable IDs, rename, remove and transform edits | [Six scene workflow tests](tests/scene_workflows.rs) |
| Transform math | Scale, fixed-axis XYZ Euler rotation in degrees, then translation, including zero/negative scales | [Coordinate conventions](.project-docs/development.md) and scene tests |
| Invalid edits | Reject invalid names, non-finite transforms and transformed-coordinate overflow without changing the scene | Scene tests |
| History | Undo/redo create, remove, rename and transform commands, with failed/no-op edits preserving history | [History workflow tests](tests/history_workflows.rs) |
| Native scene files | JSON v1 for canonical cube/plane meshes, validated load, 16 MiB limit and create-new saves that preserve occupied destinations | [Document workflow tests](tests/document_workflows.rs) |
| Camera math | Orbit/pan/dolly, perspective projection, rays, nearest-geometry picking and framing | [Viewport workflow tests](tests/viewport_workflows.rs), with native interaction still pending |
| Desktop shell and viewport | Planned | Native startup, navigation, picking and panel checks required |
| OBJ export and Maya formats | Pending | The JSON document format doesn't implement OBJ or Maya compatibility |
| Modeling, UVs, animation and rigging | Full workflows pending | [Implementation plan](.project-docs/plan.md) |
| Materials, simulation and rendering | Pending | Reference scenes and output comparisons required |
| Maya formats, scripting and plug-ins | Pending | Existing scene/script/plug-in fixtures required |

The tests cover this library's behavior. The exact Maya reference installation hasn't been pinned, so they don't establish visual or functional parity.

## Build and test

Install Rust through rustup and GNU Make. [rust-toolchain.toml](rust-toolchain.toml) pins Rust 1.99.0, rustfmt and Clippy. Run these commands from the repository root:

```sh
make check
```

| Command | What it does |
| --- | --- |
| `make build` | Build the library with the locked dependency graph |
| `make test` | Run headless tests |
| `make lint` | Check formatting and run Clippy with warnings treated as errors |
| `cargo test --locked --test scene_workflows` | Run the scene workflow suite |

There are no runtime services or environment variables for this slice. `cargo run` isn't available because this checkout has no executable target. [Development instructions](.project-docs/development.md) describe the scene conventions and optional local hooks.

## Architecture and platforms

[src/scene.rs](src/scene.rs) owns validated scene state, [src/history.rs](src/history.rs) applies commands using scene snapshots, and [src/document.rs](src/document.rs) loads and saves the native JSON format through pinned `craft-atomic` create-new writes. [src/viewport.rs](src/viewport.rs) contains the camera math and is compiled directly by its tests while UI integration is pending. The scene model remains independent of the future window so editing and file handling can be tested headlessly.

History is in-memory and unbounded, and isn't stored in scene files. Loading currently accepts only the canonical cube/plane topology, and saving requires a new destination. The camera core doesn't provide a depth-tested renderer or establish Maya navigation parity. See [the detailed limits](.project-docs/development.md).

There are no verified desktop platform builds or packages yet. macOS, Windows and Linux desktop support must each be established through native build and interaction checks. The provisional UI approach is eframe/egui, with docking, focus, marking menus and Maya-style editing still to be evaluated against the reference.

## Roadmap and publication

Development proceeds through small, independently reviewed local commits:

1. Add OBJ export to the implemented scene/history/document core.
2. Connect a native shell and viewport to the tested operations, and verify the create/edit/undo/save/reopen/export journey on the desktop.
3. Build reviewed modeling, UV, animation and rigging slices, followed by materials, simulation, rendering, interchange, scripting and extensions.
4. Compare the complete audited workflow inventory with the pinned Maya build, including existing plug-ins and all major domains.

**The first GitHub push requires verified 80–90% of all Maya 2026 functionality, including simulation, rendering, scripting and plug-in compatibility.** The exact threshold and complete denominator still need to be defined. Creating a GitHub repository, opening PRs and publishing releases remain gated as well.

The [100-workflow research seed](.project-docs/inventory-seed.md) is provisional and doesn't supply the denominator. Its measurement proposal uses fixed domain budgets, zero credit for untested/partial workflows and mandatory compatibility evidence, so adding trivial rows can't inflate coverage. Exact final parity is a separate acceptance requirement.

Once a desktop executable exists, native verification must cover startup, creation/selection, numeric edits, undo/redo, navigation, file operations, error handling and visibly unsupported controls. That checklist is pending, and reference screenshots/outputs are needed before any fidelity claim. See [known limitations](known-issues.md).

## References and development

The [reference dossier](.project-docs/reference.md) links Autodesk's documentation and the Craft projects researched for product and implementation ideas. AztecCraft is an independent implementation, and those sources don't establish Maya compatibility or grant a license for this project. No project distribution license has been selected in this checkout.

See [the build specification](.project-docs/build-prompt.md), [the detailed plan](.project-docs/plan.md) and [contribution guidance](CONTRIBUTING.md) before making changes. Reference assets, private scene files and personal paths stay out of commits unless explicitly approved for distribution.
