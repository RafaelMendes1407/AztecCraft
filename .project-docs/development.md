# Development

Install Rust 1.99.0 with Cargo, rustfmt and Clippy. The repository pins the same
toolchain for local rustup users and eventual CI. No runtime environment variables
or services are required. CI configuration is prepared locally; publication is
still prohibited until the parity gate passes.

| Command | Purpose |
| --- | --- |
| `make check` | Check formatting, lint, run tests and build |
| `make fmt` | Format Rust source |
| `make test` | Run headless workflow tests |
| `cargo test --locked --test scene_workflows` | Run scene tests only |
| `cargo test --locked --test history_workflows` | Run command history tests only |
| `cargo test --locked --test document_workflows` | Run native scene file tests only |
| `cargo test --locked --test viewport_workflows` | Run camera/projection/picking tests only |

[Build and security notes](builds.md) describe the prepared three-platform CI,
dependency audit, code line coverage and pre-commit configuration. The workflows
haven't run on GitHub because the publication gate still prohibits a push. Code
line coverage is separate from Maya workflow coverage and remains unsuitable for
a functionality badge or parity claim.

Optional local hooks: install `pre-commit`, then run `pre-commit install` and
`pre-commit run --all-files`. Rust checks use the pinned repository toolchain.
The lightweight private-key hook is not a comprehensive secret or dependency audit.

The current crate is a headless library. `Scene` owns root nodes with stable IDs,
read-only mesh access and validated rename/transform mutations. Cube and plane
primitives are unit quads in a right-handed Y-up coordinate system. Transforms
apply scale, XYZ Euler rotation in degrees, then translation. XYZ rotates about
the fixed X, Y, then Z axes, matching Maya's default row-vector `RX * RY * RZ`
convention as documented in its
[transformation API](https://help.autodesk.com/cloudhelp/2026/ENU/MAYA-API-REF/cpp_ref/class_m_transformation_matrix.html).
With glam column vectors this uses `RZ * RY * RX`. Negative and zero
scales are supported; a reflected transform also reverses geometric winding.
Hierarchy editing and a desktop window remain subsequent tasks.

## History

`History` wraps scene edits as explicit create/remove/rename/set-transform commands
and stores full scene snapshots for undo/redo. Failed edits and unchanged values
don't create history entries or clear redo. A changed edit after undo clears the
redo branch, while node ID reservations stay at their highest allocated value so
IDs from discarded branches aren't reused. Memory grows with scene size and edit
count, and no history limit or history persistence exists yet.

## Native documents

`document::load` decodes a versioned JSON envelope and validates the scene before
returning it. JSON v1 accepts canonical cube/plane meshes, stable IDs, names and
transforms. Unknown fields/versions, malformed data, invalid geometry and invalid
allocator state are rejected. This is AztecCraft's native format, with no Maya,
OBJ, FBX, USD or Alembic compatibility claim.

Both loading and saving enforce a 16 MiB document limit. `document::save_new`
uses pinned `craft-atomic::write_new` for staged create-new writes. Existing files,
directories and symlinks are occupied destinations and aren't overwritten. Saving
to a new path is required, and overwrite-save, recovery and autosave are pending.
The file stores scene data and allocator reservations, without undo/redo stacks
or camera state. The UI still needs to keep the current document unchanged on a
failed open and handle unsaved edits.

## Camera core

`src/viewport.rs` implements orbit/pan/dolly, perspective projection, rays,
triangle-based nearest-object picking and framing. The current viewport tests
compile this source directly, and it isn't exported through `src/lib.rs` yet.
Coordinates are right-handed/Y-up. Normalized device coordinates use X/Y in
[-1, 1], Y up and depth in [0, 1], and pixel mapping belongs to the future UI.
Tests exercise projection/ray agreement, transforms, clipping, framing, nearest
hits and invalid inputs. Native interaction, drawing and depth-tested rendering
remain pending, as does comparison with Maya's camera controls.

`glam` supplies vector/quaternion math, and `serde`/`serde_json` implement the
document layer. Direct versions, the `craft-atomic` revision and the resolved
dependency graph are pinned. Debug symbols and incremental builds are disabled
in development/test profiles to limit local disk use.
