# Developing AztecCraft

Work locally on small Rust desktop authoring slices, with implementation and independent review owned separately. Read [AGENTS.md](AGENTS.md), [the implementation plan](.project-docs/plan.md), [the reference dossier](.project-docs/reference.md) and [known limitations](known-issues.md) before changing behavior.

## Local workflow

Use an isolated branch/worktree when work is non-trivial or another session may use the checkout. Name the real user workflow, files to change and evidence needed, and review the plan before implementation. Search the existing module and shared code before adding a helper or abstraction.

Run `make check` for formatting, Clippy, tests and the library build. For a focused scene change, `cargo test --locked --test scene_workflows` runs the current workflow suite. See [development instructions](.project-docs/development.md) for the pinned toolchain and conventions.

Keep each commit independently reviewable. Use a distinct reviewer for 2 plan/diff passes and record the exact reviewed tree or commit. A bug regression must reproduce the failing behavior before the fix and pass afterward. Native UI work also requires interaction verification once an executable exists. Tests and screenshots don't establish parity without comparison against the pinned Maya reference.

## Scope and evidence

Unsupported controls must visibly say they're unsupported. Update documentation with the implemented behavior and verification limits, and distinguish work completed in a branch from behavior integrated into the current checkout. Store reference versions, fixture hashes and comparison outcomes so another reviewer can reproduce the result.

Keep reference assets, private scenes, secrets and personal paths out of commits unless explicitly approved for distribution. Preserve existing user data and logs, and obtain per-item confirmation before destructive cleanup. Inspect licenses and preserve required attribution before adopting upstream code. Unless you state otherwise, contributions are accepted under the project's dual license, MIT OR Apache-2.0 (see [README](README.md#license)).

## Publication and acceptance

The user has authorized publishing this early prototype and merging it to `main`, superseding the original first-push restriction. The original 80–90% target across **all Maya 2026 functionality**, including simulation, rendering, scripting and existing plug-in compatibility, remains an unmet acceptance requirement. The complete denominator, exact threshold, component boundaries and evidence remain unresolved. Release/signing and other external actions still require authorization within the current request.

[The initial workflow seed](.project-docs/inventory-seed.md) is a research starting point. It doesn't define completeness or authorize a percentage, and overall coverage remains unmeasured. Use fixed domain budgets, critical must-pass workflows and existing plug-in fixtures when the inventory is audited. Publishing a prototype doesn't establish the final identical-interface-and-behavior requirement.
