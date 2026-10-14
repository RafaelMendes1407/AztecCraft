# Build checks and packaging

All changes stay local until verified coverage reaches the user's 80–90% gate across all Maya 2026 functionality, including simulation, rendering, scripting and plugin compatibility. These workflow files prepare future CI; they do not authorize a remote, push, release or deployment.

## Checks

`.github/workflows/check.yml` runs format, Clippy, tests and a locked release build on Ubuntu 24.04, Windows Server 2025 and macOS 15. It uses Rust 1.99.0, matching `rust-toolchain.toml`; update both pins together. Cargo commands run directly so Windows does not require Make. Jobs have timeouts, read-only repository permission and no persisted checkout credentials. Superseded pull-request runs cancel; push runs finish. No build cache is configured while the project and storage requirements remain small.

The separate Ubuntu audit job installs [cargo-audit 0.22.0](https://github.com/RustSec/rustsec/tree/cargo-audit/v0.22.0/cargo-audit) with its published lockfile and checks `Cargo.lock` against the live RustSec advisory database. Warnings fail the job. An audit needs network access and does not establish application correctness or absence of every vulnerability.

Linux also installs [cargo-llvm-cov 0.9.1](https://github.com/taiki-e/cargo-llvm-cov/tree/v0.9.1) and Rust 1.99.0's `llvm-tools-preview`, then measures headless library line coverage using the locked workspace and all test targets. The job fails below 80%; test-source lines are excluded from the measured denominator. This code metric measures exercised lines, not workflow correctness, UI interactions or Maya functionality coverage. The current library has no desktop UI target. Critical mutation behavior still needs explicit workflow assertions; passing a line percentage cannot establish that every failure path preserves state.

Dependabot prepares weekly Cargo and GitHub Actions updates after an authorized repository exists. It does not merge them. Review updates and rerun checks before accepting them.

## Local pre-commit checks

With Rust 1.99.0, Python, Go and pre-commit installed, run `pre-commit install`, then `pre-commit run --all-files`. Use rustup's Cargo shim so `rust-toolchain.toml` selects the toolchain, or ensure `cargo --version` reports 1.99.0 when using a standalone Cargo installation. Hooks check whitespace, YAML, TOML and private keys. Direct Cargo hooks always check Rust formatting, Clippy, tests and the build, including commits that change only configuration. The pinned Gitleaks hook builds through pre-commit's Go environment and scans staged changes with redacted output. It requires network access on first setup. To scan existing history separately, run `gitleaks git --redact` using Gitleaks 8.30.0.

## Current artifacts and next packaging step

The current crate has a Rust library target. `cargo build --release --locked` produces a library, not an installable desktop application. CI currently verifies builds without uploading misleading application archives. There is no installer, `.app`, code signing or release automation.

When the actual desktop binary and dependencies are integrated, add Linux packages required by its selected winit/rfd backend, then verify the build on a clean Ubuntu runner. Photocraft installs X11, Wayland, xkbcommon, OpenGL and GTK development packages; AztecCraft should install only those its selected features need. Add packaging against the real binary: an unsigned macOS `.app` with accurate metadata and bundled resources, plus Windows and Linux artifacts. Verify launch from each package before enabling uploads. Signing, notarization and release publication remain blocked until the user's publication gate passes.

## Upstream references

Inspected on 2026-10-14:

- [Photocraft CI](https://github.com/storytold/photocraft/blob/main/.github/workflows/ci.yml): direct locked Cargo checks, platform matrix, cancellation and careful cache/disk use. AztecCraft checks all three desktop platforms rather than copying its platform-selection machinery.
- [Artcraft macOS publishing](https://github.com/storytold/artcraft/blob/main/.github/workflows/artcraft-macos-publish.yml) and [Windows publishing](https://github.com/storytold/artcraft/blob/main/.github/workflows/artcraft-windows-publish.yml): platform-specific native packaging via Tauri. Its Node frontend, signing setup and release writes are not needed by the current Rust library.
- [Gitleaks pinned pre-commit hooks](https://github.com/gitleaks/gitleaks/blob/v8.30.0/.pre-commit-hooks.yaml): staged scanning through a Go-built hook, with redacted output.

No reference code, credentials, signing identities or assets were copied.
