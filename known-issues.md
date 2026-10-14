# Known limitations

## Reference baseline unavailable

**Severity:** high
**Area:** parity verification

The exact Maya version/platform and reference installation have not been established. Identical behavior and visual parity cannot be verified yet. Build functionality without claiming parity; acquire a pinned reference before parity acceptance.

## Complete parity inventory not yet enumerated

**Severity:** high
**Area:** release gate

The original first-push restriction required 80–90% of all Maya 2026 functionality, including simulation, rendering, scripting, and plugin compatibility. The user's later explicit push/merge instruction authorizes early prototype publication, superseding that restriction. The full-functionality target remains unmet, and the complete inventory must still be enumerated and verified against a reference. Don't calculate a percentage from an arbitrary feature list.

[The initial inventory seed](.project-docs/inventory-seed.md) provides source-backed candidate workflows, but its 100 entries aren't a complete denominator. Component versions, cloud services, collection components and third-party plug-in scope still need to be pinned. Overall coverage remains unmeasured.

## Desktop and platform verification pending

**Severity:** high
**Area:** desktop application

The native prototype has been built and launched locally, but the complete native journey is still unverified. UI event tests cover file integration with real files, while native file-dialog interaction remains unverified. Cross-platform native interaction and packaged launch checks are also pending. The shell/viewport must pass the journey described in [the reference dossier](.project-docs/reference.md) before broad desktop support is claimed.

## Core authoring limits

**Severity:** medium
**Area:** history, documents and viewport

Undo/redo stores unbounded scene snapshots in memory, so memory grows with scene size and edit count. JSON v1 stores structurally valid quad geometry and allocator reservations, with a 16 MiB limit and create-new/no-clobber writes. Undo/redo stacks and camera state aren't persisted, and overwrite-save, recovery and autosave are pending. Native topology editing, non-quad faces and most external formats aren't supported. Degenerate, nonplanar, inconsistent-winding and non-manifold geometry is preserved as authoring data; validation doesn't repair geometry or guarantee rendering fidelity. The OBJ library API exports geometry only, with no import, UV, normal or material support, and GUI Export remains disabled.

The camera core is connected to the native viewport. Drawing sorts projected faces by average depth, and depth-tested rendering and reference navigation parity remain pending. These limits are described in [development instructions](.project-docs/development.md).

## Existing Maya plug-in compatibility unverified

**Severity:** high
**Area:** scripting and extensions

The Maya scripting/API surface and existing plug-ins aren't implemented or tested in the current slice. Establish architecture/API/ABI boundaries and run existing script and plug-in fixtures against both applications. AztecCraft-specific extensions alone can't prove Maya compatibility.
