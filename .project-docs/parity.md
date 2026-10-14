# Maya parity inventory

The user confirmed the publication denominator: all Maya 2026 functionality, including simulation, rendering, scripting, and plugin compatibility. No measured coverage percentage is available. Family counts are not a substitute for workflow coverage. Expand each family into explicit user workflows, pin the complete denominator, and test against Maya before calculating publication eligibility.

“Partial” below describes implemented AztecCraft core behavior, not verified Maya
equivalence. The [provisional workflow seed](inventory-seed.md) records domains,
sources and unresolved boundaries. The reference installation and complete
denominator are still missing, so overall coverage remains unmeasured.

| Family | Required workflows | Current status |
| --- | --- | --- |
| Application shell | menus, shelves, Outliner, Channel Box, editors, docking, workspaces | Not implemented |
| Scene management | hierarchy, transforms, shapes, instancing, attributes, undo/redo, save/reopen | Partial: root cube/plane nodes, IDs, rename/transforms, snapshot undo/redo and validated JSON v1 create-new save/load. Hierarchy, instancing and generic attributes pending |
| Viewport | camera navigation, views, picking, component selection, manipulators, snapping | Partial: tested mathematical perspective/orbit/pan/dolly/ray/picking/framing core. Native interaction, drawing, manipulators and component selection pending |
| Polygon modeling | primitives, topology, extrusion, bevel, loops, weld, normals, subdivision, history | Partial: canonical cube/plane creation and transformed vertices. Component/topology editing and modeling history pending |
| Curves and surfaces | curves, NURBS, editing, construction workflows | Not implemented |
| UV editing | seams, projections, shells, unfold, packing, synchronized selection | Not implemented |
| Animation | timeline, keys, tangents, graph/dope editors, layers, playback | Not implemented |
| Rigging | joints, constraints, IK, skinning, weight editing, blend shapes, deformers | Not implemented |
| Materials and lighting | shaders, textures, networks, lights, color management | Not implemented |
| Rendering | viewport rendering, render settings/layers, final rendering | Not implemented |
| Interchange | native document, OBJ, glTF, FBX, USD, Alembic, Maya formats | Partial: AztecCraft-only JSON v1. Canonical cube/plane meshes, 16 MiB limit and no-clobber create-new save. Other formats pending |
| Scripting and extensions | commands, MEL, Python, plugins | Not implemented |
| Dynamics and extended systems | particles, cloth, hair, simulation, caches, motion graphics | Not implemented |
| Pipeline and robustness | references, recovery, large scenes, platform packaging | Not implemented |

Publication requires 80–90% of the complete Maya 2026 workflow inventory to pass, without hiding missing essential workflows in a broad percentage. Exact parity remains a separate acceptance requirement.

The headless suites cover scene edits, command history, document validation and
camera mathematics. They don't verify native controls or Maya parity. History is
unbounded and in-memory, documents don't persist history/camera state, and the
camera core is not a renderer. See [development limits](development.md).
