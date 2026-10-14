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
Hierarchy editing, history, persistence and a desktop window are subsequent tasks.

`glam` supplies tested vector/quaternion math; `serde` prepares the scene data for
the planned JSON document layer. Both direct versions and the resolved dependency
graph are pinned. Deserialized data must pass scene validation before use once
document loading is implemented. Debug symbols and incremental builds are disabled
in development/test profiles to limit local disk use.
