# AztecCraft working rules

- Build the desktop application in Rust; preserve the user's Maya parity objective.
- Make small, independently reviewable local commits.
- The user has authorized early prototype publication and merge to `main`, superseding the original restriction on the first push. The 80–90% target across all Maya 2026 functionality and exact parity remain unmet acceptance requirements. Don't infer release/signing or other publication authority beyond the user's current request.
- The repository is public. Push each independently reviewed and tested atomic commit promptly to its feature branch under the per-repository push lock, then arm `ci-watch`. Do not wait for a complete feature or Maya parity before pushing; the orchestrator serializes integration into `main`.
- Never claim exact visual or functional parity without comparison against the pinned Maya reference.
- Consult `.project-docs/plan.md`, `.project-docs/reference.md`, and `known-issues.md` before changing code.
- Use an independent reviewer for two plan/diff passes. Keep implementation and review ownership separate.
- Build working vertical slices; unfinished controls must be visibly unsupported.
- Once initial MCP catch-up is verified, each applicable feature slice must include its typed operation, MCP tools/schemas/inspection, real Editor protocol tests and documentation before the completed feature gate. Coordinate protected editor boundaries and adapter review with the MCP owner; do not defer MCP support to a permanent later backlog or advertise unimplemented operations.
- The initial nine-tool MCP server is a foundation, not complete catch-up with existing file, export and core operations. Track missing capabilities explicitly and plan their guarded support. New applicable slices, including native face selection/extrusion, include MCP support in the same completed feature gate while that earlier catch-up proceeds.
- Keep reference assets, scene files, and personal paths out of commits unless explicitly intended for distribution.
