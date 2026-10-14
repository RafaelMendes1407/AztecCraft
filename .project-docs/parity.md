# Maya parity inventory

The user confirmed the publication denominator: all Maya 2026 functionality, including simulation, rendering, scripting, and plugin compatibility. No measured coverage percentage is available. Family counts are not a substitute for workflow coverage. Expand each family into explicit user workflows, pin the complete denominator, and test against Maya before calculating publication eligibility.

“Partial” below describes implemented AztecCraft core behavior, not verified Maya
equivalence. The [provisional workflow seed](inventory-seed.md) records domains,
sources and unresolved boundaries. The reference installation and complete
denominator are still missing, so overall coverage remains unmeasured.

| Family | Required workflows | Current status |
| --- | --- | --- |
| Application shell | menus, shelves, Outliner, Channel Box, editors, docking, workspaces | Partial: native menus/shelf, Outliner, Channel Box and status/unsupported controls. Full editors, docking and workspaces pending |
| Scene management | hierarchy, transforms, shapes, instancing, attributes, undo/redo, save/reopen | Partial: root cube/plane nodes, IDs, rename/transforms, snapshot undo/redo and validated JSON v1 create-new save/load. Hierarchy, instancing and generic attributes pending |
| Viewport | camera navigation, views, picking, component selection, manipulators, snapping | Partial: perspective camera math connected to native projected drawing/navigation/selection. Depth-tested rendering, manipulators, snapping and component selection pending |
| Polygon modeling | primitives, topology, extrusion, bevel, loops, weld, normals, subdivision, history | Partial: cube/plane creation, structurally validated quad meshes and undoable single-quad object-space extrusion API. Native face controls, all-face/region extrusion and Maya construction history pending |
| Curves and surfaces | curves, NURBS, editing, construction workflows | Not implemented |
| UV editing | seams, projections, shells, unfold, packing, synchronized selection | Not implemented |
| Animation | timeline, keys, tangents, graph/dope editors, layers, playback | Not implemented |
| Rigging | joints, constraints, IK, skinning, weight editing, blend shapes, deformers | Not implemented |
| Materials and lighting | shaders, textures, networks, lights, color management | Not implemented |
| Rendering | viewport rendering, render settings/layers, final rendering | Not implemented |
| Interchange | native document, OBJ, glTF, FBX, USD, Alembic, Maya formats | Partial: structurally validated quad meshes in JSON v1, 16 MiB limit, create-new writes, New/Open/Save As UI and geometry-only OBJ API. GUI Export, OBJ import and other formats pending |
| Scripting and extensions | commands, MEL, Python, plugins | Not implemented |
| Dynamics and extended systems | particles, cloth, hair, simulation, caches, motion graphics | Not implemented |
| Pipeline and robustness | references, recovery, large scenes, platform packaging | Partial: unsigned platform packaging and fixture checks; cross-platform native interaction, references, recovery and large-scene workflows pending |

The original first-push restriction required 80–90% of the complete Maya 2026 workflow inventory to pass. The user's later explicit instruction to push/merge to `main` authorizes early prototype publication, superseding that restriction. The full-functionality target remains unmet, and missing essential workflows must stay visible rather than be hidden in a broad percentage. Exact parity remains a separate acceptance requirement.

Core suites cover scene edits, command history, document validation, OBJ export
and camera mathematics, while UI tests cover editing/file-action events. They
don't replace native file-dialog checks or Maya comparison. History is unbounded
and in-memory, documents don't persist history/camera state, and projected
drawing isn't a depth-tested mesh renderer. See [development limits](development.md).
