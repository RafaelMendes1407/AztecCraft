# AztecCraft

A Rust desktop 3D authoring application targeting Autodesk Maya workflow and interface parity.

The project is at the reference and implementation stage. It is not a Maya replacement yet. Exact parity requires a pinned Maya build and differential verification, not screenshots alone.

Development happens in small local commits. **Do not push or create a GitHub repository until 80–90% of all Maya 2026 functionality is verified, including simulation, rendering, scripting, and plugin compatibility.** The intended owner is `cristim`. Expand and pin the full workflow inventory before reporting any coverage percentage. A modeling prototype does not pass this gate.

See [the build specification](.project-docs/build-prompt.md), [the implementation plan](.project-docs/plan.md), and [known limitations](known-issues.md).
