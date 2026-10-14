# Build specification

Build AztecCraft, an independently implemented desktop Autodesk Maya clone in Rust. Final acceptance demands visual, interaction, workflow, functional, and separately measured compatibility parity. An experienced Maya artist should find the same controls, use the same inputs, follow the same workflows, and obtain equivalent results in supported operations.

## Source of truth

Pin a Maya 2026 patch version, platform, display scaling, workspace, preferences, and plugin set. Prefer observed reference behavior, then official documentation, then reproducible screenshots, recordings, scene files, and interaction traces. Label assumptions and do not mix releases. Storytold's Craft ecosystem informs modular Rust engineering and reuse, while Maya defines product behavior. Inspect source and licenses before reusing components.

## Interface and interaction

Reproduce menus/menu sets, status line, shelves, toolbox, workspace controls, single/four viewports, panel menus, Outliner, Channel Box/Layer Editor, Attribute Editor, Tool Settings, timeline/range/playback, command line, Script Editor, and help line. Match density, dimensions, typography, color, control states, tooltips, marking/context menus, docking and workspace restoration. Preserve shortcuts, modifiers, navigation, selection, snapping, pivots, field scrubbing, focus, cancellation, and undo grouping. Do not modernize or rearrange reference workflows. Every visible control must work or be explicitly unsupported.

## Engine

Separate authoring state, evaluated scene data, and GPU resources. Implement stable identities, transform/shape hierarchy, instancing, typed attributes, connections, dependency evaluation, history, component selections, units, time, and assets. Investigate transform order, pivots, rotation orders, joint orientation, nonuniform/negative scale, attribute limits and evaluation semantics. Use undoable commands/transactions for mutations, one transaction per continuous gesture. Validate external data and detect invalid topology, graph cycles, and stale references. Build only abstractions used by real current callers.

## Viewport and modeling

Implement cameras, framing, grid/axes, shading modes, lights, object/component picking, marquees, occlusion/backface rules, manipulators and snapping. Test overlap, dense geometry, mirrored transforms and high DPI. Implement primitives, duplication/hierarchy, combining/separating, extrusion, bevels, loops/cuts, welds, deletion/hole filling, normals, subdivision, construction history, curves and NURBS. Preserve editable polygon topology independently of render triangles. Cover boundaries, degeneracy, disconnected/nonmanifold input, and attribute propagation. Implement synchronized UV editing, seams, projections, shells, unfold and packing; measure algorithm differences rather than claiming equivalence.

## Animation and rigging

Implement keyability, keys, frame/time semantics, scrubbing/playback, tangents, Graph Editor, Dope Sheet, infinity modes, animation layers, joints, constraints, IK, skin binding/weights, blend shapes and deformers. Compare matrices, deformed vertices, curves and poses against reference scenes. Preserve fractional time and rotation interpolation.

## Shading, rendering and extended systems

Implement materials, textures, lights, shading networks/editors, color management, render settings/layers and rendering. Verify viewport and final-render parity separately. Arnold compatibility is a distinct implementation/integration project and cannot be inferred from a basic renderer. Include dynamics, particles, cloth, hair/grooming, motion graphics, caches, referencing and large-scene workflows in the full inventory even when scheduled later.

## Persistence and compatibility

Create a versioned native document format with complete implemented-data round trips, atomic saves, recovery, external paths, missing-asset diagnostics and migrations. Define native, OBJ, glTF, FBX, USD, Alembic, Maya ASCII/Binary, MEL/Python and plugin compatibility independently. Verify libraries, specifications and licenses. Report unsupported import constructs and avoid silent data loss. Use a shared command registry for UI, tests and scripts when implemented callers need it. A Rust plugin API does not imply Maya plugin ABI compatibility.

## Performance and robustness

Measure release startup, load/save, navigation, picking, transforms, topology edits, evaluation, playback and memory with benchmark scenes and recorded hardware. Profile before optimizing. Expensive tasks need progress/cancellation and stale-result protection. Handle GPU/device errors, malformed assets, unsupported formats and corrupted documents explicitly. Evaluate UI suitability through menus, shelves, docking, Channel Box, timeline, GPU viewport, floating dialogs, marking menus and numeric focus. egui is provisional until fidelity evidence supports it.

## Delivery and proof

Work in runnable vertical slices: reference/scaffold, scene shell, modeling, UV/interchange, animation, rigging, shading/rendering/scripting, then remaining full inventory. Make small local commits. Do not push or publish until 80–90% of all Maya 2026 functionality is verified, including simulation, rendering, scripting, and plugin compatibility. Milestones do not shrink final scope. Maintain a reference manifest, parity/evidence matrix, architecture decisions, benchmark records, known discrepancies, compatibility tables and build/test instructions.

Differential tests use identical scenes, display/window settings, preferences, cameras, input sequences and evaluation time. Compare pixels/control geometry, scene hierarchy/attributes, geometry/topology, animation, undo/redo, save/reopen, interchange and errors. Use explicit per-comparison tolerances. Early workflow: launch, create cube, select through viewport/Outliner, transform via shortcuts/manipulators/numbers, extrude, edit history, undo/redo, key/scrub, save/reopen and export. Compilation or a screenshot alone is not acceptance. Mark every feature implemented+verified, implemented+unverified, partial, unsupported or reference-blocked. Never fabricate coverage, benchmarks or parity.
