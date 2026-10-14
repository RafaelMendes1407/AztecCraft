# AztecCraft working rules

- Build the desktop application in Rust; preserve the user's Maya parity objective.
- Make small, independently reviewable local commits.
- The user has authorized early prototype publication and merge to `main`, superseding the original restriction on the first push. The 80–90% target across all Maya 2026 functionality and exact parity remain unmet acceptance requirements. Don't infer release/signing or other publication authority beyond the user's current request.
- The repository is public. Push each independently reviewed and tested atomic commit promptly to its feature branch under the per-repository push lock, then arm `ci-watch`. Do not wait for a complete feature or Maya parity before pushing; the orchestrator serializes integration into `main`.
- Never claim exact visual or functional parity without comparison against the pinned Maya reference.
- Consult `.project-docs/plan.md`, `.project-docs/reference.md`, and `known-issues.md` before changing code.
- Use an independent reviewer for two plan/diff passes. Keep implementation and review ownership separate.
- Build working vertical slices; unfinished controls must be visibly unsupported.
- Keep reference assets, scene files, and personal paths out of commits unless explicitly intended for distribution.
