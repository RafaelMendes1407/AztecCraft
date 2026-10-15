# Known limitations

## Material data editing is a bounded candidate

The native Common Material Attributes/Diffuse control and `set_material_diffuse`
tool edit only an existing checked Phong scalar whose complete shared-shader
consumer closure is supported. Missing/default records, connected overrides,
unsupported consumers and graph edits are unavailable. Values use native f64
words and one shared History value; the slider does not clamp untouched source
data. No rendered material, texture lookup, assignment or color processor is
provided. Isolated material widget/SDK and default/headless/client checks pass;
the corrected protocol reopen journey was independently reproduced and bounded
macOS native edit/Undo/Save/reopen passed. Committed cold verification, hooks/CI
and published Save/NCRV/Face/Grid composition remain pending.

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

The native-model transition has separate provisional macOS evidence: five supplied
Maya binary scenes opened and displayed geometry, including orbit navigation.
An acute CPU tessellation defect was reproduced and corrected; material, depth
and clipping fidelity remain incomplete. Imported-scene GUI editing and the new
native lifecycle still require final independent native evidence. Earlier JSON
Save As verification does not establish native Maya writing. The current
Unix binary Save candidate uses the ordinary vendored atomic engine. Its
document-layer repeat-save checks are separate from current application checks
and pending native Save QA.

One macOS abort was observed during an automation-intercepted Open panel. The
initiating panic and trigger were not reproduced; the same binary subsequently
passed hooked and unhooked Open/Cancel. The source uses nested default-mode
pumping, but that observation does not establish the cause or a fix. Native
verification must distinguish this observation from a confirmed diagnosis.

## Core authoring limits

**Severity:** medium
**Area:** history, documents and viewport

Undo/redo stores unbounded NativeScene snapshots, so memory grows with scene size
and edit count. The native model retains separate transform/shape identities,
instance hierarchy, f64 authoring channels and original source bytes. Triangle
and quad derived views are supported; arbitrary loops/holes remain retained but
unsupported for projection. Imported editing is limited to proved source-bound
numeric changes; missing-record insertion and structural rewrites refuse safely.
The provisional Unix binary Save/Save As candidate preserves proved existing
records and reconciles uncertain installations. It does not support Maya ASCII,
Windows writing or structural rewrites, and still needs native Save QA.
Recovery, autosave and camera/history persistence remain
pending. Legacy JSON v1 and OBJ remain separate library utilities; their geometry
tests don't establish native persistence. GUI Export remains disabled. Structural
validation doesn't repair geometry or guarantee rendering fidelity.

The camera core is connected to a depth-tested opaque GPU geometry viewport with hardware clipping. Materials, transparency, normal shading, textures, document color management and reference navigation parity remain pending. Backend failures remain explicit with no CPU fallback. These limits are described in [development instructions](.project-docs/development.md).

Checked source appearance summaries now distinguish supported immutable source
associations from unavailable or unbound shapes in the Channel Box and MCP
inspection. They do not evaluate materials, resolve texture assets, prove
quadratic-filter equivalence or provide a color processor. The geometry preview
continues to report incomplete appearance; correct display and Maya comparison
remain acceptance work.

The extrusion API supports one quad with signed object-space normal displacement,
including zero-distance topology. All-face extrusion, including a one-quad plane,
is explicitly unsupported until a pinned Maya fixture establishes default behavior.
Region/world extrusion, divisions, offset/taper, UVs/normals, construction-history
nodes and native Extrude controls remain pending.

The synthetic extruded-cube example and core journey tests exercise geometry,
history, document persistence, OBJ export and camera picking. They don't replace
native face-tool interaction checks or comparison against a pinned Maya scene.

## Maya binary curve and remaining container coverage

**Severity:** high
**Area:** native documents

Public container inspection retains the observed FOR8/NCRV framing and opaque
curve records. Native decode/Open refuses NCRV, including empty groups, to avoid
silently omitting curve geometry. The next unsupported form in two retained
vendor streams is JOIN; other unseen layouts and curve semantics remain pending.
This framing coverage does not establish complete container parsing, scene Open,
NURBS rendering, native writing or Euler behavior.

## Face Center selection is a provisional subset

**Severity:** medium
**Area:** component input and viewport

Face mode supports one exact canonical face target, visible depth-tested center
markers and plain primary-click replacement/clearing. The fixed 4x4 marker,
8x8 click box, corner-mean center, center-ray visibility and 1,000,000-intersection
budget are application policies, not verified Maya defaults. Nonplanar faces can
hide their own centers. Center visibility uses a 1e-4 relative linear
view-depth tolerance, so occluders within about 1e-4 of the center's view depth
(z-fighting range, e.g. stacked planes 1e-3 apart at distance 50) do not hide it;
unobstructed finite degenerate centers can be selected.
Other component masks, multiple-face gestures, marquee, hover and marking menus
are unsupported. Native Extrude remains disabled until a real history and
manipulator workflow exists.

Ambiguous coalesced primary releases are refused with an actionable single-click
retry rather than guessed coordinates/modifiers. This limitation was reproduced
with an actual Shift click followed by an unmodified drag release in one frame.
Single-release event-time modifiers and pure Alt navigation are preserved. The
attempted combined menu activation/key/viewport-click sequence is blocked by the
pinned popup routing; separate menu+F8 and F11+F8+viewport-click tests do not prove
that combined journey. Native window, other-platform input and Maya comparison
remain separate verification requirements. See [selection policy](.project-docs/development.md#face-center-selection).

## Static ASCII curves have bounded read-only coverage

**Severity:** high
**Area:** native documents and viewport

The supported legacy 2016 static cubic ASCII class now has inert native Open,
canonical f64 curve state, evaluated GPU line geometry, conservative framing and
bounded MCP metadata. Source/SDK/UI/renderer tests cover this class; they do not
admit general Maya ASCII, arbitrary units/history/drivers, rigs, imported
instancing or Maya 2026 behavior. Unknown effects refuse the complete candidate.
Scene scripts remain retained inert data.

ASCII authoring and writing, viewport curve picking and CV/component editing are
unsupported. Use Outliner/MCP object selection, navigation and framing for
inspection. Binary NCRV is still refused. An earlier curve prototype has scoped
matrix, independent review/mutation and diagnostic macOS native Open/display
evidence; the Save/material-parent union has its own focused SDK/UI/GPU and
normal-hook checks. Later selected-parent composition, its CI and native preview
remain separate gates. Native text-commit refusal was not exercised, and no exact
visual or functional parity is claimed.
Small curves far from the origin are refused, not drawn, and Open fails, when
the f32 viewport grid at their coordinates is coarser than about 1% of the curve
size (a ~3.6cm curve is drawn at x=2e6cm and refused at x=3e6cm). Remedy
pending: a camera-relative or f64 viewport path.
[Development notes](.project-docs/development.md#read-only-static-cubic-maya-ascii)
record the supported class, limits and explicitly invoked genuine consumer.

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

## Bounded World-axis Move candidate

Move currently implements one selected object transform, World X/Y/Z arrows and
one guarded preview transaction through native UI and MCP. Plane/center/component
handles, snapping, other orientations and Rotate/Scale tools are unavailable.
Selected pivots, driven data, unsupported/unstable parents and translations with
unproved retained source words reject editing. MA and general source authoring
remain subject to their existing read-only/source guards. The 72-physical-pixel
geometry and near-parallel threshold are application policies, not Maya parity.

Native/SDK tests for this candidate pass locally: the workspace and
mcp-test-client suites ran green on 2026-10-15. Native framebuffer/desktop and
genuine source-bound Save consumer checks remain open.
The prior changed-parent full MCP-client run observed an unchanged transport
notification-flood outer-timeout failure; its single isolated reproduction
passed. The full run remains failed and the timing cause unproved.

## Move lifecycle gaps

**Severity:** low
**Area:** Move tool, MCP lifecycle

A remote-owned Move has no timeout and is not tied to the MCP client session. If
the client disconnects it stays active, preview included, until a matching
`finish_move` from a later client with the same scene token and move ID, or a human
cancel (Escape, tool/selection change, New/Open or close). Until then every other
mutating operation, Grid toggle and selection-mode change reports
`busy_manipulation`. Replacing the scene (New or Open) also resets the active tool
to Select; Maya keeps the current tool across scene changes.

If the baseline or source of an active Move cannot be validated at finish or
cancel (believed unreachable), the transaction is still cleared first, so the
history is not left busy. The scene then keeps its preview values and the
baseline is lost: the document is dirty rather than silently restored, and the
Channel Box is resynchronized to the scene.
