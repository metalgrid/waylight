# User picker implementation verification

Date: 2026-09-30. Parent checks and independent reviews accept this build-only update.
Final parent run: `/tmp/waylight-picker-parent-final/all-checks.txt`, exit 0.
It passes 18 Rust tests, 9 QML checks, and all four headless Cage cases.
The parent inspected the normal manual-entry image and minimum-size password image.
Independent review also accepts the corrected test harness. Earlier open-review notes below describe prior stages.
Real AccountsService access, PAM login, and deployment remain unverified.
Locked `.pi/USER-PICKER-PLAN.md` and previous accepted greetd plans were not changed.

## Implemented behavior

- `src/accounts.rs` uses existing zbus 5.19.0 only. Read-only `ListCachedUsers` plus `UserName`,
  `RealName`, `SystemAccount`, and `Locked`; no avatar paths or account file reads. Interface names
  and types were checked in installed `/usr/share/dbus-1/interfaces/org.freedesktop.Accounts*.xml`.
- One overall three-second timeout covers connection/list/property reads; at most 64 returned
  records are inspected. Unavailable/malformed/system/locked entries are omitted. Invalid usernames
  use the existing nonempty/256-byte/no-control-character rule, now shared with both Begin guards.
  Display names are bounded to 256 bytes without controls; blanks fall back to username. Sort by
  username then display name, deduplicate by username. Timeout/failure yields no cached entries;
  manual Other user remains available. The list is a one-shot cached subset, never PAM authority.
- A separate `Event::Accounts` updates one JSON property atomically, not authentication View state.
  Discovery runs independently, including while authentication is awaiting responses. It neither
  changes the typed identity nor clears responses, status, or focus.
- Preview returns before system discovery and supplies only demo-alex/Alex (demo) and
  demo-sam/Sam (demo), with a simulated Password prompt.
- Other user/manual entry is the initial selection and focus. Cached identities are tracked by
  username, not model index. `Backend::reset_identity` accepts only idle/nonclosing calls, clearing
  stale prompt/status/token without changing the authentication state. Intentional UI identity edits
  clear responses. No QML writes to backend state properties were added.
- Only idle identity submission or secret/visible answer submission displays an arrow. Information
  and error prompts display Continue, not a response field. All real PAM prompts remain literal
  plain text, with the neutral Enter answer placeholder. Preview uses Password. Empty responses,
  unlimited QML response entry and the existing encoded-size/NUL correction paths remain intact.
- Busy states disable identity changes. Escape closes the user popup before cancelling. Deferred
  identity focus checks busy state again, so it cannot steal focus from a prompt that arrived first.
  Cancellation restores identity focus only on confirmed idle cleanup. Session and power behavior
  remains unchanged.

## Final runnable checks

Command (exit 0):

```sh
WAYLIGHT_EVIDENCE=/tmp/waylight-user-picker-checks sh tests/check.sh \
  > /tmp/waylight-user-picker-checks/all-checks.txt 2>&1
```

- `cargo fmt --check`, `cargo build --locked`, `cargo test --locked`: **18 Rust tests passed**.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- Qt 6 qmllint: clean.
- Offscreen QML: **9 reported passes**, including init/cleanup, six test functions with two image
  data rows (seven substantive case invocations). Covers picker keyboard selection, manual fallback,
  idle text focus, late/reordered discovery without changing username/text/focus/auth responses,
  idle cleanup, busy guards, cancellation focus barrier, real arbitrary plain prompts, input/control
  visibility for all relevant states, empty/long responses, session choice, and power confirmations.
- Accounts tests inject records and futures into the production filtering/count/deadline helpers;
  unavailable records and failed/stalled list/connect/property futures are bounded without host bus
  access. A controller preview test checks synthetic users and Password events.
- Existing executable sentinel tests passed unchanged: copied binary with cwd `/`, embedded QML/SVG,
  CLI/load failures, preview never connects to either greetd or the private system-bus listener,
  and no session preference/QML cache writes.
- All four existing fake-greetd nested Cage cases passed unchanged: acknowledged launch (greeter 0),
  nonfatal state save (0), lost start reply (1), shutdown at prompt (1), Cage 0 in each. Their initial
  `sample` keyboard input still lands in the manual username field. Fake system bus address only;
  no real account discovery, authentication, session command execution, or power operation.
- Existing Qt/GCC C++ header `QChar`/SFINAE warning persists; no new Rust/Clippy or QML warnings.

## Application-only images

The worker opened and visually inspected all eight final PNGs below, generated offscreen by
`grabImage(window.contentItem)` with only synthetic injected backend data. No desktop/window-manager
screenshot was taken. At both sizes, idle has no fake response field, the picker shows Other user and
both demo entries, Password has one focused masked input/arrow, and session/power controls remain.
The small avatar scales correctly; the minimum manual Password view also asserts Cancel is inside
the scroll viewport rather than clipped.

In `/tmp/waylight-user-picker-checks/`:

- `other-user-normal.png`, `picker-normal.png`, `password-normal.png`, `password-manual-normal.png`
  — 1100×720.
- `other-user-minimum.png`, `picker-minimum.png`, `password-minimum.png`, `password-manual-minimum.png`
  — 640×580.
- `all-checks.txt` and `cage-{ack,nonfatal-state,lost-start-reply,shutdown-prompt}.txt` — final logs.

## Changed files / handoff

- `Main.qml` — user picker/manual identity, applicable-only inputs/submit controls, focus and small layout.
- `src/accounts.rs` — discovery, shared username validation, synthetic preview entries, injected tests.
- `src/backend.rs` — accounts event publication and idle-only `reset_identity`; shared Begin validation.
- `src/controller.rs` — independent Accounts event/task, synthetic preview entries/Password, shared validation.
- `src/main.rs` — accounts module registration.
- `src/controller_tests.rs` — synthetic preview event regression.
- `tests/tst_preview.qml` — picker/focus/visibility/prompt regressions and app-only image captures.
- `tests/check.sh` — creates the fixed QML image evidence directory.
- `README.md` — controls, cached subset/limits, synthetic isolation, test/image documentation.
- `.pi/USER-PICKER-TODO.md` — implementation tasks complete; parent review left unchecked.
- `.pi/USER-PICKER-VERIFICATION.md` — this evidence and handoff record.

Worker reread all changed code and README completely before handoff. Authentication transport,
cleanup/start boundaries, trusted sessions, optional state store and logind implementation were not
redesigned. No dependency/lockfile additions, Git initialization/commits, system configuration,
packages, accounts, services, PAM/polkit changes, real login, real power, or installation were performed.

## Limitations

AccountsService API success is checked through injected helpers and installed interface declarations,
not a live system-bus account query. The new list is deliberately cached and bounded; Other user is
always needed for uncached accounts and discovery failures. Mock/offscreen tests do not validate real
PAM, AccountsService access under a future greeter account, logind authorization, VT handoff, or actual
seat ownership. Nested Cage tests retain the accepted Python-wrapper/cascaded-exit limitation, not an
exact direct-child production deployment. Separate authorization and recovery planning remain required
for real deployment. Parent and independent review are not claimed complete.

## Parent failure investigation / harness correction (2026-09-30)

This follow-up supersedes the earlier **unchanged executable harness** claim above. The caller
reported that the picker architect review had accepted the implementation, but the parent's
`/tmp/waylight-user-picker-parent/all-checks.txt` failed during the first Cage `ack` case.
`.pi/USER-PICKER-TODO.md` is deliberately unchanged; parent acceptance remains pending.

### What is established, and what is not

The old harness had no client-readiness acknowledgement. Its virtual-keyboard `sent` response
only confirms a Wayland roundtrip to **Cage**, not that Qt has a mapped/active window, has consumed
input, or has polled the backend's new prompt. Sleeping 0.8 seconds after process creation and
0.15 seconds after a server write cannot establish those conditions. Server markers likewise
mean **reply sent**, not **prompt displayed and focused**. This is a test synchronization defect;
no production picker/authentication change was needed to correct it.

The original parent log contains only `[AssertionError()]`, with no request, stage, or assertion
traceback. Its temporary socket/state directory was removed. Therefore **the exact original
failed request/assertion cannot be recovered**, and it would be incorrect to claim proof that a
particular username, modifier, focus event, or prompt caused that specific run. Eleven unchanged
baseline executions passed during this investigation; these passes alone were not treated as a
fix. No picker implementation defect was observed in the reviewed flow or controlled checks.

Controlled failures establish the missing handshake independently of that lost evidence:

1. `legacy-startup-race.py` delays execution of the unmodified greeter by 1.2 seconds and pauses
   the synthetic key producer after `sam` for 0.8 seconds. The old initial 0.8-second guess sends
   the prefix before Qt exists and the suffix after it is ready. The fake server receives exactly
   `{'type': 'create_session', 'username': 'ple'}`, rejects it, and closes. The UI remains in its
   intentional disconnected/no-retry state; the old wrapper then times out after 15 + 3 seconds.
   This reproduces the parent's **assertion plus wrapper timeout** signature with a known cause.
2. `legacy-slow-prompt.py` externally SIGSTOPs only the test greeter while each fake reply is
   written and resumes it 0.6 seconds later. The old 0.15-second Enter guess produces no prompt
   response, and the fake server times out. The replacement waits for the actual focused prompt
   and passes the same pauses. No production instrumentation or retry is used.

These are deliberately injected schedules, not a claim that the parent ran under those exact
schedules. The original one-off cause remains unproven because its diagnostics discarded it.

### Correction

Only `tests/executable.py`, `README.md`, and this verification record changed in this follow-up.
No QML, Rust, keyboard-helper, Cargo dependency, locked plan, or TODO changes were made.

- `inside()` uses the existing standard Qt AT-SPI interface, on a **new private DBus bus**, to
  wait for an active window and visible/enabled/focused username or response/Continue control.
  It checks the literal expected synthetic prompt and observes `sample` in the username before
  sending one Enter. It never reads a secret/visible response value, calls an accessibility
  action, changes focus, or writes UI state. Qt's intentionally redacted password-field accessible
  name is handled by its password role plus the exact visible prompt label.
- Readiness polling is bounded and reports expected control/prompt, window activity, and focused
  controls on failure. Inputs/authentication/StartSession are never replayed. All exact protocol,
  command/environment, state-save, exit-code, empty/null response and cleanup assertions remain;
  cancellation is additionally checked against the complete request.
- The ordinary four cases now run in **headless Cage**, not on a host-backed nested output. This
  removes host input/modifier/focus interference entirely while preserving the actual Wayland
  keyboard path, real copied executable, embedded QML, and cascaded Cage exit. Private virtual
  keyboard restrictions are unchanged. A fresh private accessibility registry exits with each case.
- Fake-server failures identify case + protocol stage, expected/actual synthetic request and
  request types seen. The wrapper notices the failure marker promptly rather than masking it
  with a later generic `app.wait` timeout. Successful logs show each observed readiness boundary.
- Added test prerequisites are already installed: Python GObject / AT-SPI (`python-gobject`,
  `at-spi2-core`) and `dbus-run-session`. No packages were installed. Production dependencies and
  preview-isolation behavior are unchanged. README documents the new test prerequisites/backend.

### Final checks and evidence

Final complete check, exit **0**:

```sh
WAYLIGHT_EVIDENCE=/tmp/waylight-picker-fix-final sh tests/check.sh \
  > /tmp/waylight-picker-fix-final/all-checks.txt 2>&1
```

- **18 Rust tests**, **9 reported QML passes**, fmt/build/Clippy/qmllint passed.
- CLI/QML-load and preview no-greetd/no-system-bus/no-state/no-QML-cache sentinels passed unchanged.
- All four fake cases passed: greeter exits `0, 0, 1, 1`; Cage exits 0; expected request sequences
  retain one create/start and four prompt responses, or create/cancel for shutdown at prompt.
- **10 additional runs of all four final headless cases: 40/40 passed**, logs at
  `/tmp/waylight-picker-fix-final/repeat-{1..10}/`.
- **5 additional runs of all four host-backed nested Wayland cases: 20/20 passed** using a temporary
  copy with only the project path and Cage backend/parent display adjusted. They use the final
  readiness handshake and private input/accessibility sockets, not host-key injection. Logs at
  `/tmp/waylight-picker-investigation/nested-{1..5}/`; script `handshake-nested.py` in that directory's
  parent. Its printed `headless` label is inherited from the copied script; each Cage log verifies
  `WLR_BACKENDS: wayland` for these additional checks.
- **Delayed startup (1.2 seconds): 4/4 passed**, and **delayed prompts (0.6 seconds): 4/4 passed**,
  with final harness code. Logs/scripts under `/tmp/waylight-picker-investigation/handshake-slow-start*`
  and `handshake-slow-prompt*`. `make-probes.py` there regenerates these controlled variants and the
  two intentionally failing legacy variants from the archived baseline and current harness.
- An intentional synthetic `ple` input against the required `sample` assertion still fails (exit 1)
  immediately with `ack/create_session: ... expected synthetic create_session(sample), got ...
  username: ple`; see `/tmp/waylight-picker-investigation/handshake-wrong-user/run.txt` and its
  `cage-ack.txt`. This verifies strict rejection and useful diagnostics, not just happy-path passes.
- Legacy counterexample logs/scripts are under `/tmp/waylight-picker-investigation/legacy-startup-race*`
  and `legacy-slow-prompt*`; the first includes the exact synthetic wrong username and secondary
  wrapper timeout. Historical parent failure logs were left untouched.

No service, PAM, package, account, production session/power, or installation changes were made.
No real credentials were entered or logged. No test-owned Cage/greeter/registry processes remain.
The known Qt/GCC header warning and Cage's harmless pre-existing X0-bind/DRM-lease messages remain.
Parent should rerun the full command and review the test-only AT-SPI dependency/backend change;
this worker does not mark the parent TODO accepted.
