# Native verification record

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
