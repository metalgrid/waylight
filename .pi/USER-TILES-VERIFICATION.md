# User tiles verification

Date: 2026-09-30. **Parent checks and independent review accept the completed update.**
Final parent evidence: `/tmp/waylight-tiles-final/all-checks.txt`, exit 0.
Checks pass: 21 Rust tests, 11 QML results, and four private headless Cage cases.
Review accepts the focus correction and layout-ready click tests described below.
Earlier pending-review statements record previous stages. No deployment or real-login approval is implied.
Locked scope: [USER-TILES-PLAN.md](USER-TILES-PLAN.md), unchanged.

## Implementation

- `Main.qml`: replaces the account dropdown and standalone generic avatar with a horizontal row of at most five pictured user buttons plus an always-present More… button. Manual username is initially focused. Selection does not submit; only the existing arrow/Enter submits. Tile selection, keyboard focus, accessible display name + username, plain-text bounded tooltips, arrow/Home/End/Tab navigation and Space/Enter activation are supported. Busy states disable switching. Stable selected usernames and manual/response focus survive late discovery. The central area was widened, not redesigned; selected identity height is reduced during prompts to retain minimum-size fit.
- `src/accounts.rs`: optional AccountsService `IconFile` string (confirmed against installed `/usr/share/dbus-1/interfaces/org.freedesktop.Accounts.User.xml`, property at line 896). Missing/unsupported icons preserve the account. Only the first five sorted/filtered accounts need picture reads. Immediate root-controlled AccountsService cache entries only, existing component trust walker, canonical cache containment, no-follow/nonblocking open, regular-file check and bounded read. PNG signature/IHDR and 1–256 pixel dimensions are checked before Qt's existing PNG decoder. Accepted bytes become bounded data URLs using existing QByteArray base64; no QML path reopen or added dependency. Maximum input 256 KiB, data URL 349,550 bytes, QML sourceSize 56×56.
- `src/backend.rs`: existing JSON account event now supplies separate username/name/checked picture fields. Authentication methods and controller are unchanged.
- `src/sessions.rs`: only `trusted_with` visibility widened to crate-local for fixture reuse; trust algorithm unchanged.
- `icons/demo-{alex,sam,lee,robin,jules}.png`: five original geometric 96×96 portraits generated locally with Python standard-library PNG/zlib encoding. Embedded with Rust `include_bytes!`; preview never reads host images/accounts or connects to the system bus.
- Tests adapt dropdown assumptions without removing authentication safeguards. `tests/executable.py` and `tests/keyboard.c` were not changed.

## Final checks

Command (project root):

```sh
WAYLIGHT_EVIDENCE=/tmp/waylight-user-tiles-checks sh tests/check.sh \
  > /tmp/waylight-user-tiles-checks/all-checks.txt 2>&1
```

Result: **exit 0**, recorded in `/tmp/waylight-user-tiles-checks/exit-status.txt`.

- `cargo fmt --check`, locked build, Clippy `--all-targets -- -D warnings`: passed.
- Rust: **21 passed**, including image byte/dimension bounds, PNG decode/CRC failures, base64 round-trip, original preview portraits, absent/URL/home icons preserving accounts, private-fixture ownership/permissions, cache containment, symlink/FIFO/directory/missing-file rejection. Tests do not read actual system account pictures or query AccountsService.
- Qt `qmllint`: clean.
- QML: **10 passed** (8 test cases including the two image-size rows, plus init/cleanup). Covers five-user cap/More…, no-user manual default, click/Space/Enter selection without auth, Tab/arrows/Home/End focus, pictures and corrupt/missing fallback, literal labels/tooltips, late discovery/identity preservation, busy-state guards, real arbitrary prompts, empty answers, response masking/clearing, cancellation and mocked power safeguards.
- Executable isolation: CLI/QML failures and copied-binary preview pass; no greetd/system-bus connection/session state/QML cache in preview.
- All four private headless Cage cases pass with AT-SPI readiness (unchanged harness): ack and nonfatal-state greeter exit 0; lost-start-reply and shutdown-prompt exit 1; Cage wrapper exit 0. Logs: `cage-{ack,nonfatal-state,lost-start-reply,shutdown-prompt}.txt`. No timing-input workaround or host accessibility bus.
- Existing Qt/GCC header warnings remain; corrupt-PNG negative fixtures deliberately emit libpng errors. No unexpected QML warnings in final run.

## Application-only image inspection

QML test captures, not host desktop screenshots. All show five users plus More…:

- `manual-{normal,minimum}.png`: manual default, initial username focus, single disabled arrow until identity entered.
- `focus-{normal,minimum}.png`: focused tile white border distinct from selected More… fill.
- `selected-{normal,minimum}.png`: selected picture, stable username, one submit arrow; no password field before a prompt.
- `password-{normal,minimum}.png`: selected picture and simulated Password prompt/answer arrow/Cancel.
- `password-manual-{normal,minimum}.png`: manual identity with the same prompt controls.

Evidence directory: `/tmp/waylight-user-tiles-checks/`. Visually inspected the normal 1100×720 and minimum 640×580 layouts, pictures, selection, minimum-size focus border, and selected/manual password views. Tile edges and Cancel containment are also asserted in QML. No overlap/clipping seen in these ordinary prompt views; long prompts retain the existing scroll areas.

## Self-review and boundaries

Read complete changed source/tests/docs before handoff and traced AccountsService → checked bytes → account event JSON → tile Image and tile selection → idle-only identity reset → existing explicit submit. Found and corrected a tooltip issue during self-review: default Qt ToolTip text autodetects rich text, so account tooltips now explicitly use PlainText (with regression assertion). No unchecked image path or rich-text account label is supplied by production code.

Known deliberate limits: cache-only immediate files; leaf symlinks, home pictures, other formats and >256×256/>256 KiB PNGs use the silhouette. File validation assumes a local root-controlled cache; synchronous kernel filesystem reads cannot be preempted by the async discovery timeout. Qt remains the image decoder, not a custom decoder. Long names elide to two lines and remain available via plain-text tooltip/accessibility. Only five cached users are shown; all others require More… manual entry. Real AccountsService permissions/pictures, PAM, power authorization and VT handoff remain untested by design. Cage evidence demonstrates the existing cascaded wrapper shutdown, not a production direct-child handoff.

No Git operations, package changes, account/service/system configuration changes, real login, real power, PAM or polkit operations. Earlier immutable plans/progress/evidence were not modified. Parent/independent review is the remaining gate.

## Independent-review correction: focused tile rebuild

`Main.qml` now observes incoming tiles separately from the Repeater model. `userTiles.refresh()` captures the actively focused tile's username before synchronously replacing that model, then restores the matching tile only when the row owned focus and the backend remains idle. Empty username identifies More…; a removed account falls back to More…. No deferred callback can later steal control focus, and discovery never selects an identity or submits authentication. Existing `userTiles.itemAt()` test access and keyboard handling remain unchanged.

Added `test_tile_focus_survives_discovery_reorder_and_removal` in `tests/tst_preview.qml`: empty discovery → focus More… → publish accounts → More… retains focus → Enter focuses manual input without authentication; account reorder retains username focus; removal focuses More…; session control focus is not stolen. Existing username/login/password focus-preservation checks remain intact. The new regression failed against the original implementation at the More… focus assertion (`/tmp/waylight-user-tiles-focus/regression-before.txt`).

Final command: `WAYLIGHT_EVIDENCE=/tmp/waylight-user-tiles-focus sh tests/check.sh`. Exit **0**, evidence `/tmp/waylight-user-tiles-focus/{all-checks,exit-status}.txt`: clean qmllint, 21 Rust tests, 11 QML results, executable isolation and all four private Cage cases passed. Existing Qt/GCC warnings and intentional corrupt-PNG diagnostics remain. An earlier run failed the immediate tile-click assertion (expected demo-lee); its log is retained as `transient-checks.txt`. The unchanged rerun passed, but that was not sufficient verification: the parent reproduced the failure. The readiness correction and replacement evidence below supersede that rerun-to-pass conclusion. Application captures still use the test's existing `/tmp/waylight-user-tiles-checks` paths.

Self-review re-read the changed function/tests and traced both refresh call sites. Old model identity is captured before delegate destruction; model replacement and restoration are synchronous, focus restoration is idle-only, and More… guarantees a fallback. No selection/authentication/power methods or system files changed. Only Main.qml, tests/tst_preview.qml and this evidence file were edited. **Parent TODO remains untouched/open; parent acceptance is still required.**

## Parent-reproduced correction: await layout before coordinate input

Parent failure: `/tmp/waylight-user-tiles-parent-final/all-checks.txt`, old `tests/tst_preview.qml:184`, expected `demo-lee`, actual **empty username** (More…), not demo-sam. Temporary instrumented copies under `/tmp/waylight-user-tiles-layout/diagnostic/` recorded tile centers and `isPolishScheduled(window)` / `isPolishScheduled(tiles.parent)` without modifying production QML. In 20 diagnostic baseline runs, runs 10, 12 and 17 reproduced the failure: all six tile centers remained `(550, 316.4)` with Row polish pending. The synthetic click therefore hit the overlapping last delegate (More…), which acquired focus and cleared the selection. Logs: `geometry-before-{10,12,17}.txt`. Passing baseline runs instead had distinct x centers 320, 412, 504, 596, 688, 780.

`geometry-discovery.txt` confirms the cause directly: immediately after publishing accounts, all delegates exist but overlap at x=550; `verify(waitForPolish(window))` changes both window/Row polish flags to false and positions the centers at the six distinct x coordinates above. Keyboard selection subsequently schedules ancestor layout polish (identity/status visibility), but does not recreate the overlapping Row. Thus the primary failure was missing discovery-layout readiness, not a production focus-refresh race or the identity height change itself. Image readiness and keyboard focus checks did not guarantee positioning readiness.

Correction is **test-only** in `tests/tst_preview.qml`: assert window-wide `waitForPolish` after setup activation, after publishing the six-tile model, before post-selection mouse clicks, before image-test geometry assertions, and before clicking the newly opened confirmation dialog button. Waiting on the Window covers pending Row and ancestor layouts, unlike waiting on an individual delegate. Existing click, selection, focus and authentication assertions remain intact; no sleeps, retry-on-failure wrapper, assertion removal, or production changes. `Main.qml` checksum is unchanged (`production-before.sha256`, verified after the full suite), preserving the focused-tile rebuild fix.

Replacement verification, `/tmp/waylight-user-tiles-layout/`:

- **20 consecutive corrected QML suites passed**, fail-fast shell loop; `qml-{1..20}.txt` and `repeat-status.txt`, each 11 passed / 0 failed. Separate diagnostic logs are not counted in this result.
- `WAYLIGHT_EVIDENCE=/tmp/waylight-user-tiles-layout sh tests/check.sh` **exit 0**; `all-checks.txt`, `exit-status.txt`: fmt/build/Clippy, clean qmllint, 21 Rust tests, 11 QML results, executable isolation and all four private Cage cases passed.
- Existing compiler warnings and intentional corrupt-PNG diagnostics remain. Screenshot output remains `/tmp/waylight-user-tiles-checks` per the existing test.

Only `tests/tst_preview.qml` and this document changed in this correction. No dependencies, Git, system configuration, real login or power changes. **Parent verification/acceptance remains pending; TODO untouched.**
