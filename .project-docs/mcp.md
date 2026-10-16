# Native editor MCP

The optional MCP server controls the new native Editor window launched by its client. Tools use that window's canonical native scene, history, selection, camera and viewport grid setting. It cannot attach to an already running instance. This candidate exposes the tools in the generated [tool catalogue](mcp-tools.md); it does not establish Maya scripting, plug-in or full-functionality compatibility.

The Save integration in this working branch is provisional. Its ordinary vendored
atomic engine and document adapter have separate consumer checks. Current
application/protocol verification and native Save QA remain required; document
layer results alone do not establish a working GUI Save journey.

## Build and launch

Install the prerequisites in [development instructions](development.md), then build explicitly:

```sh
cargo build --release --locked --features mcp
```

Configure the client to launch the resulting binary with `--mcp-stdio`. A native graphical session is required. The default desktop build and default CI package omit MCP; rebuilding without `--features mcp` replaces the binary at the same target path. Use the exact feature-enabled artifact.

An example subprocess configuration, subject to the client's own configuration format:

```json
{
  "mcpServers": {
    "azteccraft": {
      "command": "<absolute-path-to-mcp-enabled-azteccraft>",
      "args": ["--mcp-stdio"]
    }
  }
}
```

On Windows the executable is `azteccraft.exe`. No socket or HTTP service is exposed. The local subprocess launcher is the authority boundary, including access to local files through `open_scene`. Stdout is reserved for MCP messages; diagnostics use stderr.

Pinned rmcp 3.5.1 handles protocol decoding, discovery, initialization and request metadata. Official SDK tests exercise MCP 2026-07-28 Discover and legacy 2025-11-25 initialization. Cancellation tests exercise the legacy request-cancellation path. These tests do not establish compatibility with every MCP client.

## Tools and state

Material inspection adds an optional bounded `material` projection to each shape:
shader type, authoritative diffuse f64, complete shared-user count and separate
editing/rendering availability. Null means supported editing is unavailable, not
a guessed default. `set_material_diffuse` follows the same token, focus, differing
draft, modal/close and native-input guards as other mutations. Values may exceed
one; no source record is inserted and no other shader input is editable. The
isolated official SDK material edit/inspect/Undo/Save/reopen journey and full
client matrix pass. The corrected test performs actual protocol Open after Save,
requiring new document tokens and inspected material values; independent focused
verification reproduced it. Bounded macOS native data QA also passed.
SDK edit/inspection/History coverage is portable; Save/reopen assertions are
Unix-only because Windows native binary writing remains unsupported. Committed
cold, hooks/CI and published combined-parent verification remain pending;
material data changes do not enable rendered Phong output.

Call `inspect_scene` first. Every other tool requires `scene_token`, a UUID string from that inspection. IDs are unsigned 64-bit decimal strings, including values beyond JavaScript's exact integer range. JSON numbers are rejected for IDs. Unknown argument fields, invalid types and unknown tools are protocol errors.

| Tool | Arguments besides `scene_token` | Result |
| --- | --- | --- |
| `inspect_scene` | No arguments; no token required | Committed mesh/curve metadata including Move preview, typed document origin, retained-source counts/read-only reason, units, instance paths, selected identity, history, pending action, dirty/draft/focus/modal/closing flags and camera |
| `set_active_tool` | `tool`: `select` or `move` | Activate a supported native tool through shared guards |
| `begin_move` | `node_id`, full `instance_path` (1 to 256 decimal IDs), `axis`: `x`, `y` or `z` | Begin a remote-owned translation of the exact selected transform; returns `move_id` |
| `update_move` | `move_id`: decimal string; `distance`: finite absolute f64 | Preview World distance from begin in declared linear units |
| `finish_move` | `move_id`; `finish`: `commit` or `cancel` | One Undo at commit, or exact begin-state restoration |
| `create_primitive` | `primitive`: `cube` or `plane`; optional `name` | Created `node_id`; selects the new node |
| `select_node` | Optional `node_id` and `instance_path`, at most 256 decimal NativeId strings | Full root-to-transform prefix; ambiguous objects require a path. Null or omitted node clears selection and cannot include a path |
| `set_selection_mode` | Required `mode`: `object` or `face` | Changes the transient selection mask; a valid remembered face remains inactive in Object mode |
| `select_face` | Required `target`: complete target object or null | In Face mode, selects that exact owning instance and canonical face; null clears only the face |
| `rename_node` | `node_id`, `name` | Changed or unchanged authoring state |
| `set_transform` | `node_id`, `translation`, `rotation_degrees`, `scale`, each an array of three finite f64 numbers | Translation in declared scene units, rotation in degrees, dimensionless scale |
| `edit_node` | `node_id`; optional `name`, `translation`, `rotation_degrees`, `scale` | One undo entry for the supplied name and individual channels; selection stays unchanged |
| `extrude_face` | `node_id`, `shape_id`, `instance_path`, `face_index`, `distance` | Extrudes one current canonical face through the shared native history |
| `set_material_diffuse` | `shape_id`: decimal NativeId; `value`: finite nonnegative f64 | Edits the checked existing Phong diffuse scalar for all shapes sharing that shader, through one History command |
| `delete_node` | `node_id` | Changed authoring state |
| `undo` | None | Changed or unchanged history state |
| `redo` | None | Changed or unchanged history state |
| `frame_selected` | None | Frames the selected object in Object mode or selected face corners in Face mode; no active face returns `no_selection` |
| `set_grid_visibility` | `visible`: required boolean | Changed or unchanged grid visibility in this window's viewport |
| `orbit_camera` | `radians`: two finite numbers, yaw and pitch deltas | Changed or unchanged camera; pitch clamps near the poles |
| `pan_camera` | `world_units`: two finite numbers, camera right and up displacement in scene world units | Changed or unchanged camera target and eye |
| `dolly_camera` | `factor`: positive finite distance multiplier | Changed or unchanged camera distance; below one moves closer |
| `new_scene` | None | New empty document immediately, or a pending confirmation when dirty |
| `open_scene` | `path`: 1 to 4096 UTF-8 bytes, no NUL | Supported `.mb`, read-only static cubic `.ma`, or legacy `.aztec` candidate; applies immediately or stages an immutable candidate for confirmation |
| `save_scene` | None | Active Unix Maya binary Save, or reconciliation of an uncertain installation without another write |
| `save_scene_as` | `path`: 1 to 4096 UTF-8 bytes, no NUL | Fresh `.mb` destination only, never overwrite; an existing receipt is reconciled instead |
| `resolve_file_action` | `pending_action_id`: UUID; `decision`: `cancel`, `discard` or `save` | Resolve only that exact pending action; source-changed queued Open requires Cancel and reopening |
| `request_close` | None | Native close guard; dirty documents require confirmation, and approved remote closure waits for its own successful response flush |

Transforms use the [existing coordinate conventions](development.md). The `rotation_degrees` input and normalized transform inspection always use degrees; `authoring_transform.rotation` retains the document's declared angular units, reported in `units`. Translation retains declared linear units. Camera navigation uses centimeter view space. Inspection exposes shape counts and smoothing capability, without mesh arrays, source bytes or private file paths. This is bounded metadata inspection, not a geometry dump.

`select_face.target` contains required `node_id`, `shape_id`, `instance_path` and
`face_index`. IDs are decimal strings; the full path has 2 to 257 IDs, ending in the
shape beneath the requested transform, and the face index is the current source
face ordinal before triangulation. A valid explicit target may name another
object or a currently occluded/clipped face. It does not invoke viewport picking
or silently change the selection mask. Invalid targets preserve selection;
Object mode rejects even a null face request. An omitted target is invalid.

Inspection reports `selection_mode`, the complete `remembered_face` target or
null, and fixed `face_selection_policy: "visible_centers_only"`. A remembered
face is inactive in Object mode and is revalidated after edits and Undo/Redo.
Changing object/path or replacing the document clears it; successful New/Open
retains the current mask. This transient state does not edit the native scene,
file binding, history or dirty status. Native actions retain the existing valid
human-draft commitment policy; remote requests refuse differing drafts.

Native Face selection uses 4x4 physical-pixel center markers and an 8x8 click box,
with center-ray visibility and a bounded intersection budget. Plain primary
click replaces or clears the face; modified selection and ambiguous coalesced
primary releases are unsupported. Alt camera navigation remains available.
F11 and Select > Components > Face enter Face mode; F8 toggles Object/Face.
Exact Maya CBS, numeric, visual and event-order parity remain unverified. Native
Extrude remains disabled pending a real history/manipulator workflow; the
existing explicit `extrude_face` tool keeps its checked authoring semantics.

Supported static cubic Maya ASCII `.ma` scenes open through the same native
document lifecycle. Original script bodies remain inert retained data. Unknown
commands, node classes, geometry dependencies or unsupported curve classes refuse
the complete candidate before adoption. This subset is read-only: authoring and
Save are unavailable, while Outliner/`select_node`, navigation and
`frame_selected` remain available. Viewport curve picking and CV/component editing
are unsupported. Binary NCRV remains unsupported.

Inspection keeps mesh `shapes` intact and adds a bounded `curves` array. Each
curve has decimal `id`, `name`, degree, typed `form` (`open`), numeric
`control_vertices` and `knots` counts, f64 parameter `domain`, `read_only` and
`draw_available`. Each direct transform owner's `shape_ids` includes its curves.
No CV/knot arrays or script text are returned. `document_origin` is `untitled`,
`maya_binary`, `maya_ascii` or `legacy_aztec`; `retained_source_bytes` and
`retained_source_records` report counts only, with records including retained
trailing trivia. `read_only_reason` carries the core refusal message or null.

Curve `draw_available` means at least one currently visible instance successfully
prepared as draw geometry in one bounded whole-view preparation. Hidden curves or
whole-view preparation refusal report false while canonical inspection remains
available. This field does not mean the backend is initialized or healthy, that
GPU submission/framebuffer rendering succeeded, or that Maya fidelity is proven.
The separate viewport health fields retain their existing meaning.

Scoped local tests exercised the genuine legacy 2016 reference through the
official SDK's actual Editor Open/inspection/selection/framing path, including
strict catalog discovery and read-only edit refusals. Synthetic tests cover
dirty failed-Open baselines, input/token/focus/modal/close guards and the actual
Editor GPU callback. The Save/material-parent union also exercises ASCII Save
and Save As refusals and Save acknowledgements before queued ASCII source
revalidation. A separate earlier diagnostic native preview observed genuine
Open, Outliner selection, visible curve geometry and disabled controls; it did
not exercise a native text commit. Earlier matrix and isolated mutation evidence
do not verify every later source composition. The twenty-four-tool count describes
the catalog at the Face-selection integration parent; curves add no tools, and a later
integration parent must use its own count. [Development notes](development.md#read-only-static-cubic-maya-ascii)
record explicit fixture identity and consumer commands.

`edit_node` uses the Channel Box's sparse native History command. Each supplied
channel array has exactly three finite numbers or nulls. An omitted array or a
null component leaves that channel untouched; a null whole array is invalid.
Omitted `name` or `name: null` leaves the name untouched; an empty string attempts
a name change and follows normal validation. Only supplied rotations convert
from degrees to the document's declared angular unit, preserving untouched raw
radian words. `set_transform` remains a deliberate replacement of all nine
channels. Name and channel changes validate atomically, including unsupported
imported renames. An empty edit validates the node and returns unchanged after
the normal guards, without creating history or synchronizing fields. Identical
values preserve redo. These operations retain the document token, file binding,
camera and full instance selection.

`extrude_face` requires an explicit root-to-shape `instance_path` of 2 to 257
decimal NativeId strings, ending in `shape_id` beneath the transform identified
by `node_id`. Unlike `select_node`, this path includes the shape. `face_index`
is the current canonical face ordinal before triangulation. The document token
does not identify a topology revision: an old ordinal that remains in range may
refer to a different face, so inspect current state before editing. No selected
face or default path is inferred.

Distance is finite f64 in declared scene linear units, applied along the mesh's
local face normal; transforms are not baked. Negative distances are supported,
and zero distance still changes topology. Shared shapes change for all their
instances. Successful extrusion creates one undo entry and preserves selection,
camera, document token and unrelated objects. Imported structural edits and
unsupported topology or mesh metadata refuse without mutation. Extruding the
single-face plane refuses because all-faces extrusion is unsupported. The native face
extrusion control remains visibly unsupported; this tool exposes the existing
checked History command.

Successful tool results provide `structuredContent` and equivalent text content. Authoring outcomes are `scene`, `created`, `changed` or `unchanged`; file lifecycle outcomes are `document_applied`, `pending_confirmation`, `cancelled`, `close_requested`, `saved`, `save_uncertain` and `save_reconciled`. A scene result nests fields under `inspection`; a created result includes `node_id`. Authoring mutations share native undo history. Selection and camera operations do not create authoring undo entries or mark the document dirty. Navigation arguments use camera units, not pointer pixels. Invalid resulting camera ranges return `camera_error` without changing the camera; finite JSON numbers outside the camera's f32 range are also rejected by camera validation.

Tools that set an absolute state return unchanged when repeated and carry `idempotentHint: true`; the generated [tool catalogue](mcp-tools.md) marks which ones, and a drift test keeps it equal to the registry.

`set_grid_visibility` requires an explicit JSON boolean and the current scene
token. Missing, null, string, numeric and unknown fields refuse; there is no
remote toggle or inferred value. Repeating the same value returns unchanged.
Inspection reports authoritative `grid_visible` even without a GPU, independently
of `viewport_error`, `depth_complete` and `clipping_complete`. The setting starts
true, survives successful or canceled New/Open, and belongs to this viewport's
session. It does not edit NativeScene, create an undo entry, clear redo, mark the
document dirty or change its token. Native clicks may first commit a valid human
draft; remote setters reject drafts through the ordinary shared guards.

The fixed XZ grid retains its existing extent, spacing and colors. Grid settings,
global or additional panels, persistence and a default shortcut are unsupported.
This scope does not establish Maya grid appearance or behavior parity.

Example create arguments after inspection:

```json
{
  "scene_token": "<scene-token-from-inspection>",
  "primitive": "cube",
  "name": "remote_cube"
}
```

Inspection also exposes a fixed-size `source_appearance` summary for the scene and each shape. `checked_viewer_source` means the supported unique-instance component, shading-group, Phong/file/placement and UV source associations were checked. It does not mean materials are evaluated or ready to render. Unsupported or ambiguous associations report `unavailable` with fixed issue flags; source-free scenes report `unbound`. The remaining evaluation, quadratic-filtering and color-processor issues stay explicit, and `appearance_incomplete` remains true. No texture path, UV array, source name or source payload is added to inspection.

The native Channel Box uses the same summaries. A selected transform reports all its directly attached shapes as `Direct shapes (N)`; a transform without direct shapes and an empty selection report `Scene totals (N)`. These labels do not imply subtree or selected-instance appearance evaluation.

Each new Editor and each applied New/Open replaces the scene token. A `document_applied` reply carries the new token. A dirty file action returns `pending_confirmation` with the old token, `pending_action_id` and kind; inspection exposes that same id and kind. Cancel clears only the matching pending action and retains the token. Discard applies the exact staged candidate once unless a prior Save decision marked its source changed; that blocked action requires Cancel and reopening. Stale or replayed action ids refuse without effects. Normal edits, undo and redo retain the token. The token identifies a document generation, not an edit revision or credential. Reinspect after a stale token, deleted ID, cancellation, timeout or lost acknowledgement; do not automatically retry a mutation.

## Native Save candidate

Save and Save As share the native Editor operations. Native actions first commit
a valid human draft; remote actions refuse focus, differing drafts, pending input,
modal state and stale tokens. Save supports source-bound Maya binary scenes on
Unix with proved existing-record edits. New, legacy and Windows writing remain
unsupported. Read-through-symlink documents can Save As to a fresh destination;
active Save through that link remains refused.

`saved`, `save_uncertain` and `save_reconciled` replies include `scene_token`,
`dirty` and `pending_action_status` (`none`, `retained` or `source_changed`).
Inspection reports `document_origin` (`untitled`, `maya_binary`, `legacy_aztec`),
`has_file`, `save_available`, `save_as_available` and
`save_reconciliation_required`, without exposing paths. Availability describes
an eligible attempt, not a promise that disk state is unchanged.

An uncertain outcome means installation occurred but durability or verification
was not acknowledged. The document retains its exact written snapshot and opaque
receipt, stays dirty, and does not change its active path or saved baseline.
The next Save or Save As only reconciles that receipt; it does not encode or
install again, even if a new destination was requested. Successful reconciliation
acknowledges the written snapshot while retaining any newer live edits as dirty.
A later Save is a separate write. Selection, camera, history and document token
are not replaced by saving.

A successful Save decision revalidates a queued Open source before consuming it.
If saving changed that source, the reply still acknowledges the Save with
`pending_action_status: source_changed`; the same pending action remains blocked
against Discard until Cancel and reopening. A newer dirty snapshot or expired
request retains the pending action. Cancel preserves uncertainty. Explicit
Discard warns that a destination may already contain written bytes and abandons
the receipt only after a successful document replacement; Close retains it until
actual Editor drop, including a failed close-response flush.

Each request has one absolute deadline shared by queue waiting and installation
guards. Cancellation, connection loss or expiry is checked before dispatch and
each actual install attempt. Actual disk outcomes are acknowledged in Editor
state even if the reply is lost. A fresh guard after Save prevents consuming a
pending New/Open/Close after cancellation, without undoing the real write.

## Human edits, errors and limits

The production viewport uses the same canonical geometry and camera as native
selection and the navigation tools. Inspection reports `viewport_backend` as
`wgpu_geometry_preview` for the initialized GPU pipeline, or `uninitialized` in
device-free test Editors. `depth_complete` and `clipping_complete` describe a
healthy initialized backend; both become false on a terminal backend failure.
`viewport_error` is null or one fixed category: `uninitialized`, `backend_failed`,
`camera_input`, `non_finite_clip` or `resource_limit`. Inspection reads the live
backend latch even while hidden, without exposing driver messages. A successful
submission clears a recoverable category; it cannot clear a terminal failure.

Rendering remains an opaque flat geometry preview. Materials, transparency,
normal shading, textures and the document color-management transform are not
implemented; `appearance_incomplete` and the persistent viewport HUD retain that
limitation. Remote camera/selection/grid changes appear in the next fresh native
frame, after the current immutable callback snapshot. Hiding the grid leaves
surfaces, object edges, picking and camera unchanged. Empty hidden-grid frames
retain the viewport background without uploading or drawing an empty vertex
buffer. There is no CPU fallback.

Remote mutations return `busy_editing` while a native text field is focused, a differing draft exists, or a modal dialog is open. This includes valid uncommitted drafts. They do not commit unrelated human edits. `resolve_file_action` is the sole modal exception; text focus, drafts, pending native input and scene-token checks still apply. Inspection remains available and reports these states without returning draft text. Native inputs are processed before queued remote requests at the end of each UI frame, and remote changes request repaint.

Approved remote closure reports `closing: true` while waiting for its matching successful JSON-RPC response to flush. Ordinary mutations refuse during closure; readonly inspection stays available. An unrelated response, error, cancellation, deadline or failed write cannot authorize exit. Failed delivery withdraws approval, preserves document state and suppresses an automatic disconnect retry; an explicit native Close remains available. The resolver response owns this delivery barrier when Discard or a successful Save approves a pending Close. A flushed response establishes transport delivery, not proof that a client processed it.

Hidden/minimized windows dispatch through eframe's UI-thread logic callback without painting or forcing foreground. Visible windows still dispatch only after that frame's native UI input. Interactive native input retained while hidden sets the inspection flag `native_input_pending` and blocks remote mutation until visible UI processes it. Harmless focus changes and unheld pointer motion do not block idle hidden requests. Native Close while input is pending is canceled; restore/process the input and retry Close.

Tool failures set `isError` and return `error.code` plus `error.message`. Codes include `busy_manipulation`, `stale_move`, `move_owner`, `move_error`, `busy_editing`, `stale_scene`, `stale_pending_action`, `ambiguous_selection`, `too_large`, `no_selection`, `invalid_input`, `scene_error`, `file_error`, `camera_error`, `busy`, `unavailable` and `cancelled`. File errors redact private paths. Save errors use fixed codes save_unsupported, invalid_destination, source_changed, destination_exists, save_cancelled, save_io and save_encode. `busy` covers queue saturation and the five-second UI reply deadline. The SDK may suppress a cancelled response. Cancellation before application skips a queued request; cancellation after a synchronous operation starts cannot undo it. Inspect to reconcile the result.

Prototype bounds are 64 KiB per ingress JSON line, 4096 UTF-8 bytes per input name, at most 16 retained RPC slots and 16 pending supported notification callbacks, 16 queued editor requests, and eight requests dispatched per frame. RPC admission remains held until reply write or connection close; callback permits remain held until completion/drop or close, including reused request IDs. A cancelled request whose reply the SDK suppresses retains its RPC slot until connection close. Enough abandoned calls can exhaust admission and close the session. A new connection launches a new Editor window; it cannot attach to the old window.

Unsupported and repeated notifications are filtered before SDK dispatch. Oversized ingress, invalid JSON, fatal SDK decoding errors, duplicate outstanding IDs or exhausted protocol admission close the connection. SDK compatibility handling can skip nonstandard messages; the adapter consumes those ignored lines without stalling later buffered requests.

Tool results are limited to 512 KiB and framed replies to at most 1 MiB, including the terminating newline. Inspection preflights source size before cloning nodes, then bounds serialized content. Large scenes can return `too_large` rather than partial inspection. These bounds do not promise inspection of arbitrary large scenes.

The [known first-click text defect](../known-issues.md#first-click-text-input-can-be-lost) remains in pinned egui. Tests prove the bridge adds no input loss compared with the same native baseline and preserves text actually inserted in the following frame; they do not fix that defect.

## Shutdown and verification

Ingress EOF or protocol failure closes the bridge immediately, skips queued mutations and invokes the existing native close guard before SDK handler draining, except after a failed remote-close delivery attempt as described above. A clean editor closes. Unsaved committed scenes, including valid drafts committed by native close handling, prompt Save/Discard/Cancel confirmation, with Save enabled only for active-save or receipt-reconciliation eligibility. An invalid draft keeps the GUI open with a validation error; the user must fix or cancel that edit before Close can reach confirmation. This deliberately departs from MCP's recommendation to exit promptly for a dirty document. A client can forcibly terminate its subprocess and lose unsaved work; universal shutdown compatibility is not claimed. Cancelling native close does not revive an EOF-closed protocol connection.

Approved native close cancels protocol work and uses a one-second bounded runtime shutdown rather than waiting indefinitely on silent stdin. The subprocess regression holds stdin open and checks bounded cleanup. Tests also cover actual Editor input/order/history, official SDK tool journeys, both lifecycles, cancellation, deadlines, floods, framing and stalled output. They use real Editor/egui state without an OS window. Native protocol/window verification and cross-platform graphical interaction remain separate evidence requirements.

EOF automatic close is deferred while native input remains unprocessed; its close request is retained until visible consumption. Native verification of the initial server reproduced background/minimized timeouts because its queue drained only in the UI callback. The revised actual `App::logic` path passes minimized/occluded-state tests, pending-input baseline comparisons and hidden close/cancellation tests. [Scoped macOS native verification](mcp-native-verification.md) confirmed minimized requests after the correction, while a later idle deadline and recovery leave broader responsiveness unverified.

Run the official client suite explicitly so it is not omitted by feature gating:

```sh
cargo test --locked --features mcp-test-client
cargo clippy --locked --all-targets --features mcp-test-client -- -D warnings
cargo clippy --locked --all-targets --features mcp -- -D warnings
cargo build --locked --features mcp
```

No-default core builds remain independent of desktop and MCP dependencies. See [build checks](builds.md) for CI and the exact dependency license references.

## Capability synchronization

Every existing and new user-facing capability and its relevant state must be accessible through MCP, including while catch-up is in progress. Each applicable feature slice includes its typed operation, MCP schema/handler and inspection changes, real Editor protocol tests and documentation before the completed feature gate. The feature owner coordinates protected editor boundaries and adapter review with the MCP owner. Do not advertise a tool before the operation exists or leave MCP support as a permanent later backlog.

The current catalog omits vertices/edges/UVs, multiple-face gestures, GUI Export, material assignment/rendering, animation, simulation and scripting. Face mode and one full canonical face target are exposed through the same operations as native Center selection. Its single material-data edit is the bounded existing Phong diffuse scalar described above. `extrude_face` exposes the existing bounded single-quad operation through an explicit current target; native face-extrusion UI remains unsupported. Core-only APIs and unsupported controls do not imply an advertised MCP tool. The current binary decoder refuses shared-instance MB input, and the native UI cannot create instances; SDK ambiguity behavior has no reachable producer in this slice. Canonical path/ambiguity checks and reachable explicit-path protocol tests do not establish full instanced-file support.

Nine verified tools establish the initial server foundation, not catch-up with every existing operation. Existing capabilities without tools need reviewed follow-up slices; their absence does not defer MCP support for new applicable features, including the native face slice.

| Existing capability | Current MCP exposure |
| --- | --- |
| Object create/select/rename/transform/delete, history and framing | Nine tools listed above, including inspection |
| Native New/Open and pending confirmation | `new_scene`, `open_scene` and `resolve_file_action` share native candidate and lifecycle handlers |
| Read-only static cubic Maya ASCII curves | `open_scene` loads the admitted inert class; `inspect_scene` returns canonical class/counts/domain/read-only/prepared-draw metadata, and `select_node`/`frame_selected` use the same owner and camera. Curve picking, CV editing, native writing and broader ASCII evaluation remain unsupported |
| Native Close | `request_close` and matching pending resolution use the native close guard and response-delivery barrier |
| Native Save/Save As | Provisional `save_scene`/`save_scene_as` and matching pending Save; source-bound Unix binary only, current integration/native Save QA remains pending |
| Core OBJ export | No tool; file authority and create-new/no-clobber behavior need a dedicated slice |
| Native Face/Center selection | `set_selection_mode`, `select_face` and selection inspection share authoritative typed state; other component masks and multi-face gestures are unsupported |
| Core mesh extrusion | `extrude_face` exposes the checked single-quad History command with an explicit current face target; native face-extrusion UI remains unsupported |
| Native Select/Move tool and World-axis object translation | Four typed Move tools share the native transaction and guards; bounded tool/active preview/capture state is inspectable. Candidate native/SDK verification remains pending |
| Interactive camera orbit/pan/dolly | Three navigation tools use the same typed operations as native captured gestures; live camera state is inspectable |
| Viewport Grid visibility | `set_grid_visibility` shares the checkable native Grid button's typed setter; `grid_visible` exposes the authoritative session state. Fixed grid only; options, global/additional panels and persistence are unsupported |
| GPU geometry viewport | `inspect_scene` exposes live backend/error/depth/clipping/appearance state; camera and object selection share the native frame consumer. Materials and complete appearance remain unsupported |

Initial catch-up remains incomplete while these implemented capabilities lack their reviewed MCP support. Full Maya parity remains a separate unmet objective.

## Protocol references

- [MCP stdio transport](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio)
- [MCP tool results](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
- [Pinned official Rust SDK](https://docs.rs/rmcp/3.5.1/rmcp/)

## World-axis Move candidate

Select and Move are real native tools, activated with their vector buttons or
unmodified Q/W outside text fields and dialogs. Move supports one selected full
transform instance in object mode, with World X/Y/Z shafts only. Rotation, scale,
plane/center handles, components, snapping, alternate orientations and modified
drags remain unsupported. Unsupported transforms, driven data, selected pivots,
unstable parents and source translations without retained existing words refuse
the gesture. This is a bounded application policy, not verified Maya parity.

The four Move tools share native Editor operations and one canonical History
transaction. `begin_move` returns a decimal `move_id` bound to the current scene
token and remote ownership. Numeric begin may retain the Select tool. Updates
supply absolute World distance from begin in the document's declared linear unit,
not an increment. A commit produces at most one Undo entry; zero commits and
cancellation preserve the existing Redo branch. Native geometry uses centimeters
directly, with one checked conversion for declared SDK distances.

During a gesture, inspection reads the live authoritative preview. Unrelated
remote mutations, Save, Undo/Redo and camera operations return
`busy_manipulation`. Only matching remote-owned updates/finish are accepted;
remote calls cannot finish a native drag. Viewport Grid visibility, selection mode and
Face selection are likewise refused with `busy_manipulation` while any Move is
active. Human Escape, tool/selection/selection-mode/lifecycle
changes cancel a native drag before their existing native guard path. Move is
Object-mode only: while Face selection is active `begin_move` returns
`invalid_input`, native handles are not drawn or hit, and `inspection.move_tool`
reports numeric Move unavailable with the reason. Native captured mapping,
camera, DPI, modifier, focus or release loss cancels and consumes the pointer
sequence. A finite geometric refusal, or a refused final preview update, on a valid explicit
release commits the last validated preview, with a visible refusal and last-valid
status; it never
applies a substitute motion.

`inspection.move_tool` reports the active tool, World/object-translation class,
canonical numeric availability/reason, and any active ID/target/axis/provenance
and distance in `inspection.units.linear`. Its optional `viewport` is explicitly
labelled `last_native_viewport_capture`. It separates visible axis geometry and
unavailable reasons from the last captured pointer's optional hit. Missing or
stale capture is unavailable. Visible geometry is not a global pickability or
Ready claim, especially in tiny viewports and the excluded center region.
Baseline Scalars and source bytes are never exposed.

This source candidate includes native-event and official-SDK regression tests.
Their execution, native framebuffer/desktop proof and current-parent publication
remain separate completion gates until recorded below; source wiring alone is
not verification.
