# Build-only verification and parent handoff

Date: 2026-09-30. Implementation completed sequentially in `/home/iso/Projects/untitled`.
The locked `.pi/GREETD-PLAN.md` and original preview plan/assets were preserved.
Initial review identified three findings that required correction. The corrections are complete.
**Parent checks and independent corrective review accept this build-only delivery.**
The parent reran `sh tests/check.sh` with exit status 0.
Final parent evidence: `/tmp/waylight-parent-final/all-checks.txt`.
The checks include 15 Rust tests, QML tests, and four isolated nested Cage cases.
The parent confirmed that greetd remains inactive and SDDM remains active.
Real PAM login and VT handoff still require separate approval and testing.

## Final reproducible check

```sh
cd /home/iso/Projects/untitled
mkdir -p /tmp/waylight-corrective-checks
WAYLIGHT_EVIDENCE=/tmp/waylight-corrective-checks sh tests/check.sh \
  > /tmp/waylight-corrective-checks/all-checks.txt 2>&1
```

Final exit status: **0**.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo build --locked` | PASS |
| `cargo test --locked` | **15 passed**, zero failed/ignored |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS; no Rust warnings |
| `/usr/lib/qt6/bin/qmllint -I <staged-generated-module> Main.qml App.qml tests/tst_preview.qml` | PASS, no QML warnings |
| Offscreen `/usr/lib/qt6/bin/qmltestrunner -input tests` | **3 test functions passed**; Qt reports 5 passes including init/cleanup |
| `python tests/executable.py` | PASS: CLI/QML-load failures, isolated preview, 4 nested Cage fake-server cases (cascaded shutdown) |

C++ builds emit an installed GCC 16 / Qt 6.11.2 `QChar` `-Wsfinae-incomplete` header warning.
This is not suppressed. It does not cause a build/Clippy failure. Nested Cage can also log an
occupied Xwayland X0 socket before selecting another display; all four compositor exits passed.

Nested results (fake socket; synthetic username `sample`; empty answers only). The direct child
of Cage is the Python test wrapper, which waits for the greeter and then exits. These results
prove **cascaded Cage shutdown**, not the exact production greeter-as-direct-child arrangement:

- Acknowledged start: saved `weston.desktop` was restored, exact request command was
  `["'/usr/bin/weston'"]`, only expected session env was present; greeter **0**, Cage **0**.
- Acknowledged start with missing state directory: save was nonfatal; greeter **0**, Cage **0**.
- Lost start reply: exactly one StartSession, no replay or state write; after shutdown request,
  greeter **1**, Cage **0**.
- Shutdown at a secret prompt: CreateSession followed by CancelSession, no start/state write;
  greeter **1**, Cage **0**.

The executable was copied to a temporary directory and run with `/` as cwd. Embedded QML/SVGs
loaded without source files. Preview was given listening fake greetd/system-bus sockets and
connected to neither. Invalid/missing/duplicate arguments returned **2**. A deliberately
missing Qt Quick Controls style exercised QML load failure and joined shutdown (exit **1**).
The virtual keyboard helper checks both private runtime ownership-by-test convention and a
relative nested Wayland display name; it cannot accidentally use the inherited absolute host
Wayland display. No test process remained after the final suite.

Canonical corrective evidence: `/tmp/waylight-corrective-checks/all-checks.txt`;
`exit-status.txt` records **0**. `rust-tests.txt` is the preliminary passing targeted Rust run.
Additional final Cage logs:
`/tmp/waylight-corrective-checks/cage-{ack,nonfatal-state,lost-start-reply,shutdown-prompt}.txt`.
`cargo fmt` was applied before the checks; no test processes remained after the suite.
The previous implementation's own-application-only offscreen capture remains at
`/tmp/waylight-checks/application-only.png`; it was not recaptured during this correction.
No personal-desktop screenshot was taken.

## Reviewed findings corrected

1. **Intermediate symlink trust bypass:** `src/sessions.rs::trusted_with` walks components and
   expands `read_link` targets one at a time, checking every encountered node/link. Relative
   targets start at the link parent; `..` is processed only after checking the traversed node.
   More than 40 link traversals fails closed, including cycles. Production `trusted` still
   requires UID 0 and non-writable directories/files. Fixture-only policy injection models
   root-like ownership (including a nontrusted link owner) without root or host mutations.
   The new test accepts trusted relative/absolute chains and `/bin/true`, rejects a trusted
   link via a writable intermediate directory/link to a trusted target, rejects an untrusted
   link owner, rejects writable-node/`..` bypasses, and rejects self/two-link cycles.
2. **greetd internal datagram overflow:** verified against
   `/tmp/greeter-source-review/kennylevinsen-greetd-9afd7b1/greetd/src/session/`:
   `conv.rs::question` and `worker.rs::recv` use 10240-byte receive buffers;
   `interface.rs::post_response/send_args` serialize `ParentToSessionChild` directly as JSON.
   The enum has default external tagging, with no length prefix inside the Unix datagram:
   - public `{"type":"post_auth_message_response","response":""}` = 51 bytes;
     internal `{"PamResponse":{"resp":""}}` = 27 bytes (**24 bytes smaller**);
   - public `{"type":"start_session","cmd":[],"env":[]}` = 42 bytes;
     internal `{"Args":{"env":[],"cmd":[]}}` = 28 bytes (**14 bytes smaller**).
   Identical string/array serialization preserves those differences, including escaping/null.
   `encode_request` caps complete outgoing JSON at 10240 bytes, conservatively fitting both
   internal forms. The 4-byte public IPC header is separate; incoming replies retain 64 KiB.
   Tests verify overhead and the exact ASCII boundary, reject oversize ASCII/escaped answers
   without socket writes, preserve prompt state/token, and accept a corrected answer.
   Start command/environment preflight occurs **before CreateSession**, so an invalid/oversize
   selection returns idle without authentication state to clean up or any uncertain start.
   Tests cover oversized command/environment and malformed C strings/assignments, with no
   request bytes or `starting` event emitted. Existing cancellation/start uncertainty tests pass.
3. **NUL truncation at PAM:** `pam/ffi.rs::to_cstr` copies Rust bytes into a C string, so
   embedded NUL would truncate the effective answer. `encode_request` rejects NUL locally
   with a static error; it does not log/interpolate input. The fake-socket regression sends
   `sample\0suffix` (embedded NUL) for both visible/secret prompts, checks unchanged token/state and no
   posted bytes, then successfully submits correction. Empty answers remain covered and valid.
   JSON expansion fixtures now use non-NUL escaped control characters.

Corrective source/docs changed only: `src/sessions.rs`, `src/controller.rs`,
`src/controller_tests.rs`, `README.md`, and `.pi/GREETD-VERIFICATION.md`.
No dependency, QML, service, system configuration, package, or real-login/power change was made.
The locked plan and final parent-review TODO were left unchanged.

## Changed/new source and review focus

- `Cargo.toml`, `Cargo.lock`, `build.rs`, `resources.qrc`, `App.qml`: CXX-Qt 0.10.0 executable,
  embedded resources, locked dependencies; `.qmlls.ini` and `target/` are generated build artifacts.
- `src/main.rs`: strict CLI, isolated preview configuration, Qt load-failure handling,
  SIGINT/SIGTERM shutdown and final worker join/exit result.
- `src/backend.rs`: `Bridge`, `BackendRust`, QML invokables and nonblocking event polling;
  immediate UI guards, stale-prompt/cancellation gating.
- `src/controller.rs`: `wire`, `exchange`, `cleanup`, `attempt`, `run`; bounded native-endian
  frames, pinned partial exchange, explicit cancellation barrier, final start boundary,
  indeterminate start outcomes, no reconnect/replay, preview path before external I/O.
- `src/sessions.rs`: `trusted`, `command`, `quote`, `from_entry`, `discover`; safe Exec subset,
  root-controlled paths, fixed executable resolution, one shell-quoted command, env allowlist.
- `src/state.rs`: FD-relative private directory/file validation, `load`, `save`; only session ID,
  exclusive temporary creation/atomic replacement after acknowledged start, nonfatal failures.
- `src/power.rs`: asynchronous logind `capabilities` and noninteractive `perform`; no fallback.
- `Main.qml`: injectable original view with username/session inputs, actual PAM prompts,
  plain text/scrolling/focus, cancellation/close handling, gated/confirmed power controls.
- `src/controller_tests.rs`, `tests/tst_preview.qml`, `tests/executable.py`, `tests/keyboard.c`,
  `tests/check.sh`: protocol, UI, executable and nested-compositor verification.
- `README.md`, `DEPENDENCIES.md`, `examples/greetd.toml`: architecture, limits, licenses,
  uninstalled prospective deployment/recovery guidance.
- `.pi/GREETD-TODO.md`, this file: implementation progress and review handoff.

## Deliberate limits / remaining approvals

- No real PAM authentication, actual desktop launch, seat ownership, VT handoff or host power
  methods were tested. Fake servers inspect but never execute session commands. Real-mode
  executable tests use an isolated nonexistent system-bus address; power UI tests use a fake backend.
- No Git initialization/commits/worktrees; no package installation (only Cargo dependencies),
  sudo, account changes, `/etc`, PAM/polkit edits, or greetd/SDDM service actions occurred.
- Real transport failures cannot prove PAM cleanup. Retry is disabled and uncertainty is
  displayed/logged. SIGKILL/crashes cannot be made cleanup-safe. A normal close waits for the
  bounded outstanding exchange/cleanup; no immediate PAM interruption is promised.
- State is advisory atomic replacement on a private **local** filesystem, not crash-durable
  fsync storage. Exec is intentionally a restricted subset, not a general desktop-entry parser.
- Preview performs no greeter state I/O or greetd/system-bus connection. Qt/fontconfig can still
  maintain ordinary font caches; do not describe preview as literally zero filesystem writes.
- `greetd_ipc` is **GPL-3.0-only**. Distribution needs a licensing/compliance decision; no project
  license was selected. `DEPENDENCIES.md` records direct licenses and the transitive review requirement.
- Fingerprint remains only in `.pi/FUTURE.md`.

The corrective worker reread all changed logic and tests, traced session discovery/selection,
backend prompt recovery, IPC serialization and the preflight/start boundary, and checked the
README/evidence against actual results. All three reviewed blockers are addressed in the
implementation and regression checks. The parent and independent reviewer subsequently accepted these corrections.
This acceptance does not authorize deployment.
