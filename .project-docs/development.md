# Development

Install Rust 1.99.0 with Cargo, rustfmt and Clippy. The repository pins the same
toolchain for local rustup users and CI. The user has authorized publishing the
early prototype, superseding the original first-push restriction. Full Maya
functionality coverage and exact parity remain unmeasured and unmet.

| Command | Purpose |
| --- | --- |
| `cargo run --locked` | Build and launch the default desktop target |
| `make check` | Check formatting, lint, run tests and build including desktop |
| `make fmt` | Format Rust source |
| `make test` | Run core workflow and desktop UI event tests |
| `cargo test --locked --no-default-features` | Run core suites without the desktop target or a window |
| `cargo test --locked --test scene_workflows` | Run scene tests only |
| `cargo test --locked --test history_workflows` | Run command history tests only |
| `cargo test --locked --test document_workflows` | Run native scene file tests only |
| `cargo test --locked --test viewport_workflows` | Run camera/projection/picking tests only |
| `cargo test --locked --no-default-features --test obj_workflows` | Run OBJ export tests only |

[Build and security notes](builds.md) describe the prepared three-platform CI,
dependency audit, code line coverage and pre-commit configuration. Remote CI and
native platform results need retained run evidence rather than inference from
the configured matrix. Code line coverage is separate from Maya workflow
coverage and remains unsuitable for a functionality badge or parity claim.

Optional local hooks: install `pre-commit`, then run `pre-commit install` and
`pre-commit run --all-files`. Rust checks use the pinned repository toolchain.
The lightweight private-key hook is not a comprehensive secret or dependency audit.

The crate provides a reusable scene library and a native binary behind the
default `desktop` feature. eframe 0.36.2/egui use wgpu with X11/Wayland enabled,
and rfd supplies file dialogs. Desktop launch needs a window system and compatible
graphics backend. Linux needs the selected window/dialog backends' system
packages, and each claimed platform needs clean build and native interaction
checks. Core-only commands use `--no-default-features` and require no display or
runtime services.

The prototype provides menus/shelf, Outliner, Channel Box, projected mesh drawing,
selection, name/numeric edits, undo/redo, delete and framing. Alt-left drag orbits,
middle drag pans, right drag/wheel dollies, and F frames the selected object when
a text field isn't consuming keys. Ctrl/Cmd-Z undoes and Ctrl/Cmd-Shift-Z redoes.
New/Open/Save As use the document API, with Save As requiring a new destination.
GUI Export and unfinished modeling/animation/material controls are disabled.
Docking and full Maya interaction parity aren't implemented or verified.

`Scene` owns root nodes with stable IDs,
read-only mesh access and validated rename/transform mutations. Cube and plane
primitives are unit quads in a right-handed Y-up coordinate system. Transforms
apply scale, XYZ Euler rotation in degrees, then translation. XYZ rotates about
the fixed X, Y, then Z axes, matching Maya's default row-vector `RX * RY * RZ`
convention as documented in its
[transformation API](https://help.autodesk.com/cloudhelp/2026/ENU/MAYA-API-REF/cpp_ref/class_m_transformation_matrix.html).
With glam column vectors this uses `RZ * RY * RX`. Negative and zero
scales are supported; a reflected transform also reverses geometric winding.
Hierarchy editing remains a subsequent task.

`mesh::Mesh` stores quad vertex/index arrays and remains available as `scene::Mesh`.
Scene validation requires finite local/world coordinates, index bounds and four
distinct indices per face. It preserves unused vertices, empty meshes, coincident
or zero-area geometry, nonplanarity, inconsistent winding and non-manifold states.
Loading edited arrays is supported; native topology editing is still pending.

`Scene::extrude_face` and `Command::ExtrudeFace` extrude one quad by signed
object-space displacement along its oriented Newell normal, with one segment.
The cap replaces the selected face at the same index and four side quads are
appended. Zero or rounded-to-coincident displacement still creates undoable
topology. A mesh with only one face, including the primitive plane, returns an
explicit unsupported-all-faces error; its Maya default behavior needs a pinned
fixture. Undefined normals, invalid indices/distances and coordinate overflow
leave the scene/history unchanged. World/region extrusion, divisions, taper,
offset, UVs/normals, construction-history nodes and native face controls remain
pending. Tests establish this subset's geometry, not Maya parity.

`examples/extruded-cube.aztec` was generated through cube creation and two
History extrusion commands: front face 1 by +0.5, then its right side face 7 by
+0.5. Integration tests reconstruct the 16-vertex/14-face closed quad mesh and
verify undo/redo, save/reload, reflected OBJ cap/side geometry and picking outside
the original cube. File → Open provides a native inspection path; it does not
add native face-editing controls or prove Maya equivalence.

## History

`History` wraps scene edits as explicit create/remove/rename/set-transform/extrude-face commands
and stores full scene snapshots for undo/redo. Failed edits and unchanged values
don't create history entries or clear redo. A changed edit after undo clears the
redo branch, while node ID reservations stay at their highest allocated value so
IDs from discarded branches aren't reused. Memory grows with scene size and edit
count, and no history limit or history persistence exists yet.

## Native documents

`document::load` decodes a versioned JSON envelope and validates the scene before
returning it. JSON v1 accepts structurally valid quad meshes, stable IDs, names and
transforms. Unknown fields/versions, malformed data, invalid geometry and invalid
allocator state are rejected. This is AztecCraft's native format, with no Maya,
OBJ, FBX, USD or Alembic compatibility claim.

Both loading and saving enforce a 16 MiB document limit. `document::save_new`
uses pinned `craft-atomic::write_new` for staged create-new writes. Existing files,
directories and symlinks are occupied destinations and aren't overwritten. Saving
to a new path is required, and overwrite-save, recovery and autosave are pending.
The file stores scene data and allocator reservations, without undo/redo stacks
or camera state. File integration validates an opened scene before replacement,
keeps the scene/drafts on malformed open and prompts Save As/Discard/Cancel for
unsaved changes before New/Open/close. Tests exercise real files and UI events,
but native file-dialog interaction remains unverified.

## OBJ export

`obj::export_new` writes world-space vertices and quad faces, with per-object
index offsets and winding reversed for reflected transforms. Object names use
UTF-8 percent encoding to avoid whitespace/directive injection. Export is staged
to a new destination through `craft-atomic` and preserves occupied paths. It
exports geometry only, without UVs, normals or materials. OBJ import and the GUI
Export action aren't implemented.

## Camera core

`src/viewport.rs` implements orbit/pan/dolly, perspective projection, rays,
triangle-based nearest-object picking and framing. It is exported by the library
and used by the native viewport UI.
Coordinates are right-handed/Y-up. Normalized device coordinates use X/Y in
[-1, 1], Y up and depth in [0, 1], with pixel mapping in `src/viewport_ui.rs`.
Tests exercise projection/ray agreement, transforms, clipping, framing, nearest
hits and invalid inputs. Current drawing sorts projected faces by average depth
instead of using a depth-tested mesh renderer. Occlusion/clipping fidelity and
comparison with Maya's camera controls remain pending.

An empty mesh is unpickable; framing it returns an error and preserves the camera.
Unused vertices are retained by native documents and OBJ export. Structural
validation preserves degenerate geometry; those states don't establish drawable
surface or normal fidelity.

`glam` supplies vector/quaternion math, and `serde`/`serde_json` implement the
document layer. Direct versions, the `craft-atomic` revision and the resolved
dependency graph are pinned. Debug symbols and incremental builds are disabled
in development/test profiles to limit local disk use.
