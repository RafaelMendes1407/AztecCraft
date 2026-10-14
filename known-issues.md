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

The [macOS packaged-debug verification record](.project-docs/native-verification.md) now covers cube creation, numeric editing, Save As/Open, unsaved-change cancellation/discard, no-clobber protection and malformed-open state preservation at source commit `6c96345`. Native file choices were supplied through held panels using OCU `choose_file`; full interactive panel navigation remains pending. Windows/Linux native interaction and packaged launch checks, GUI OBJ export and Maya comparison also remain pending. Subsequent builds need their own verification before these results are generalized.

## Core authoring limits

**Severity:** medium
**Area:** history, documents and viewport

Undo/redo stores unbounded scene snapshots in memory, so memory grows with scene size and edit count. JSON v1 stores structurally valid quad geometry and allocator reservations, with a 16 MiB limit and create-new/no-clobber writes. Undo/redo stacks and camera state aren't persisted, and overwrite-save, recovery and autosave are pending. Native topology editing, non-quad faces and most external formats aren't supported. Degenerate, nonplanar, inconsistent-winding and non-manifold geometry is preserved as authoring data; validation doesn't repair geometry or guarantee rendering fidelity. The OBJ library API exports geometry only, with no import, UV, normal or material support, and GUI Export remains disabled.

The camera core is connected to the native viewport. Drawing sorts projected faces by average depth, and depth-tested rendering and reference navigation parity remain pending. These limits are described in [development instructions](.project-docs/development.md).

The extrusion API supports one quad with signed object-space normal displacement,
including zero-distance topology. All-face extrusion, including a one-quad plane,
is explicitly unsupported until a pinned Maya fixture establishes default behavior.
Region/world extrusion, divisions, offset/taper, UVs/normals, construction-history
nodes and native face selection/extrusion controls remain pending.

The synthetic extruded-cube example and core journey tests exercise geometry,
history, document persistence, OBJ export and camera picking. They don't replace
native face-tool interaction checks or comparison against a pinned Maya scene.

## Existing Maya plug-in compatibility unverified

**Severity:** high
**Area:** scripting and extensions

The Maya scripting/API surface and existing plug-ins aren't implemented or tested in the current slice. Establish architecture/API/ABI boundaries and run existing script and plug-in fixtures against both applications. AztecCraft-specific extensions alone can't prove Maya compatibility.

## First-click text input can be lost

**Severity:** medium
**Area:** native text fields

With pinned egui 0.36.2, a previously unfocused name field drops a Text event delivered in the same frame as the initial pointer click. Its TextEdit builder processes text only when the field already has focus (builder.rs:572), before pointer handling requests focus (builder.rs:815). The same real Editor event sequence reproduces this without MCP. Text delivered in the following frame is inserted.

The MCP regression compares native and bridge-connected Editors with identical events and proves equal drafts, focus, scene and history, plus BusyEditing rejection of queued remote changes. It proves no additional input loss from MCP; it does not fix the native first-click defect. Native input handling and Maya reference comparison remain outstanding.

## Native MCP idle responsiveness remains unverified

**Severity:** medium
**Area:** native protocol lifecycle

Native testing of the initial nine-tool server reproduced repeated reply deadlines while the window was backgrounded, including a deterministic minimized-window inspection failure. Desktop interaction resumed dispatch. Pinned eframe uses `App::logic` without `App::ui` for hidden windows; the original bridge drained only in UI rendering.

The revised hidden logic path and retained-input guard pass actual trait-callback tests, and [scoped macOS native verification](.project-docs/mcp-native-verification.md) confirmed minimized requests at source `36dffdf`. In the combined icon/MCP build `541c72c`, an idle request reached its UI reply deadline. A main-thread sample taken during investigation included Metal drawable acquisition, without establishing overlap with the timed-out request or a five-second acquisition wait; the next request recovered in the same process and scene token. The underlying cause remains unproven. These results do not establish unconditional idle/background responsiveness or other-platform native behavior.

## macOS application quit guard unverified

**Severity:** medium
**Area:** native document lifecycle

Pinned winit 0.30.13 gives the default Quit menu item the `terminate:` action and Command-Q equivalent (macOS menu.rs:66–73). Its application termination callback calls window-exit notification and internal exit (app_state.rs:65–71,166–172), while window Close separately queues CloseRequested (window_delegate.rs:144–148). Native testing of Command-Q with a synthetic dirty scene remains pending; no data loss has been demonstrated. The hidden-dispatch change guards viewport Close and does not establish interception of application termination.
