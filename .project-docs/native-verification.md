# Native verification record

## M1 material candidate, bounded macOS verification

The current source adds a real Common Material Attributes/Diffuse draft and
`set_material_diffuse` operation backed by the checked native shared-shader value.
The isolated default248/headless192/client314 tests and strict lint passed,
including real TextEdit events and the SDK edit/Undo/Save journey. A later targeted
SDK test also passed actual protocol Open of each saved output, changed document
tokens, inspected exact effective words and another Active Save from the reopened
origin; the retained independent verifier reproduced the corrected journey.
The independent source-word oracle and three intended core mutation failures with
exact restored positives are separately recorded. The production MCP build was
copied and hashed under its build lock. Root operated that macOS package: select
a supported shape, edit Diffuse with Enter, Save As to a fresh working file,
toolbar Undo to a dirty baseline and Redo to saved/clean, then File Open the saved
working file and inspect the retained value. An independent private byte check
found changes only inside the authorized eight-byte diffuse envelope; all five
original fixture hashes were unchanged. This proves that bounded native data
journey, not rendered Phong fidelity or other platforms. Committed cold, ordinary
hooks, CI and published combined-parent checks remain pending. Originals must
never be Save destinations. Material rendering, assignment, textures and color
processing stay unavailable. The historical record below is unchanged.

Recorded 2026-10-14 for source commit `6c96345f7c9754b26dabff0d2ae0eec198441c9b`.
The verification operator exercised a locally packaged debug macOS application.
These results apply to that build and platform, without establishing Maya parity
or Windows/Linux native interaction support.

## Method and evidence boundaries

The journey used Open Computer Use to operate the running application and inspect
screenshots/controls. Text editing used foreground coordinate clicks. Native
file panels were held while OCU `choose_file` supplied a file choice, verifying
the application/panel/file integration. Interactive Finder browsing and choosing
files entirely through panel controls weren't exercised.

Saved fixture JSON was inspected on disk, and occupied-file protection was checked
by comparing file hashes before and after the failed save. Fixture files, paths
and captures remain private and aren't distributed with this repository.

## Observed workflow

| Action | Observed result |
| --- | --- |
| Create cube | Cube appeared in the Outliner and projected viewport |
| Set Translate X to `2`, then Enter | Channel edit committed and the cube moved in the viewport |
| Save As to a new file through the native panel | On-disk JSON had `version: 1` and the cube's translation `[2, 0, 0]` |
| Create plane, request New, then Cancel the unsaved-change modal | Both objects and the current selection were preserved |
| Undo plane creation | Plane was removed, leaving the edited cube |
| Open the saved fixture through the native panel and explicitly Discard unsaved changes | Saved cube with Translate X `2` loaded, selection cleared, and Undo/Redo were cleared |
| Save As to an occupied destination | Error appeared in the application and the existing file hash stayed unchanged |
| Open malformed JSON through the native panel | Invalid-JSON error appeared, while the selected cube, Translate X `2` and current document filename stayed unchanged |

The checks cover successful persistence, explicit cancellation and
discard decisions, no-clobber failure and failed-open state preservation. They
complement the automated core and UI-event tests with native application evidence.

## Remaining verification

- Windows and Linux native interaction and package launch checks remain pending.
- Full interactive file-panel navigation remains pending.
- GUI OBJ Export is disabled, so the complete native export journey remains pending.
- New icon work and subsequent modeling/MCP changes need verification against their own builds.
- Maya reference comparison, the complete functionality denominator and the original 80–90% full-functionality target remain unfulfilled. Overall Maya coverage is unmeasured.

## Native Maya binary Save integration scope

The preceding JSON results remain historical evidence for `6c96345`. They do not
verify the newer Maya binary writer, current native file lifecycle or current
MCP catalog.

The document adapter at `f7354a2` was checked with the ordinary vendored atomic
engine. Its focused Unix tests passed, and five retained private Maya binary
inputs each passed six working-copy save cycles: fresh Save As, edited Active
Save, repeated Save, second fresh Save As, a further edited Active Save and
another repeated Save. The checks compared the latest destination baseline,
original parse Arc, reopened internal storage words and unchanged prior leaves.
Original asset sizes and hashes matched before and after. Independent focused
and private-corpus execution was performed before the appearance-parent rebase;
the rebased runtime logs and original hashes were independently checked. This
is document-layer evidence, without a native window or file picker.

The application candidate composes that adapter with native Save/Save As,
Save/Discard/Cancel and the same typed MCP operations. It retains the current
appearance summaries and editing tools. Its current-branch Editor/official SDK
checks, independent committed verification and native GUI verification remain
pending. Device-free tests and controlled uncertain-write tests do not establish
a GUI journey or promise disk-fault behavior on every platform.

The all-five native acceptance journey is pending: open a working copy with
visible geometry, edit an existing supported channel, Save As to a fresh leaf,
edit and Active Save, repeat Save, verify Undo/Redo dirty state before reopening,
then use a second Save As and Active Save destination. Original inputs must
never be active write destinations. Queued New/Open/Close Save/Cancel/Discard,
occupied destinations, source conflicts, failed Open and native picker Cancel
also remain pending for this application candidate.

The synchronous native picker still holds the UI loop. A queued remote request
can time out during the panel; after Cancel, expired work must not mutate the
document. Full interactive panel navigation remains a separate acceptance item.
An observed macOS abort during an intercepted Open panel has no reproduced
initiating trigger; subsequent hooked and unhooked Open/Cancel passed with the
same binary. That observation supplies no confirmed cause or fix.

Windows writing, Maya ASCII, structural source rewriting, complete appearance
and Maya parity remain unsupported or unverified. No private paths, assets,
captures or raw numeric source values are distributed with this record.
