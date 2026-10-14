# Known limitations

## Reference baseline unavailable

**Severity:** high
**Area:** parity verification

The exact Maya version/platform and reference installation have not been established. Identical behavior and visual parity cannot be verified yet. Build functionality without claiming parity; acquire a pinned reference before parity acceptance.

## Complete parity inventory not yet enumerated

**Severity:** high
**Area:** release gate

The user requires 80–90% of all Maya 2026 functionality before the first push, including simulation, rendering, scripting, and plugin compatibility. The complete workflow inventory must still be enumerated and verified against a reference. Do not push and do not calculate a percentage from an arbitrary feature list.

[The initial inventory seed](.project-docs/inventory-seed.md) provides source-backed candidate workflows, but its 100 entries aren't a complete denominator. Component versions, cloud services, collection components and third-party plug-in scope still need to be pinned. Overall coverage remains unmeasured.

## Desktop and platform verification pending

**Severity:** high
**Area:** desktop application

The current checkout exposes a headless scene/history/document library and a tested camera core, with no executable target. Native desktop startup, interaction and packaging haven't been verified on macOS, Windows or Linux. The planned shell/viewport must pass the native journey described in [the reference dossier](.project-docs/reference.md) before desktop support is claimed.

## Core authoring limits

**Severity:** medium
**Area:** history, documents and viewport

Undo/redo stores unbounded scene snapshots in memory, so memory grows with scene size and edit count. JSON v1 stores canonical cube/plane scene data and allocator reservations, with a 16 MiB limit and create-new/no-clobber writes. Undo/redo stacks and camera state aren't persisted, and overwrite-save, recovery and autosave are pending. Arbitrary topology and external formats aren't supported.

The camera core is tested as mathematical code, compiled directly by its workflow tests. UI integration, native navigation, drawing and depth-tested rendering remain pending. These limits are described in [development instructions](.project-docs/development.md).

## Existing Maya plug-in compatibility unverified

**Severity:** high
**Area:** scripting and extensions

The Maya scripting/API surface and existing plug-ins aren't implemented or tested in the current slice. Establish architecture/API/ABI boundaries and run existing script and plug-in fixtures against both applications. AztecCraft-specific extensions alone can't prove Maya compatibility.
