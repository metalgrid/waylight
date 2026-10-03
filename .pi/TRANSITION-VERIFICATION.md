# Brief authentication transition presentation — verification

Date: 2026-09-30. Targeted fix within accepted LOGIN-UX scope.
Status: **Parent checks and independent review accept this fix.**
Parent full suite passes: `/tmp/waylight-transition-parent/all-checks.txt`, exit 0.
Independent review reports no material findings. No system or authentication configuration changed.

## Cause and fix

`Main.qml` previously used the actual backend state for both interaction guards and layout/status visibility. A tile switch immediately sets `cancelling`; confirmed cleanup dispatches the next Begin as `waiting`; the next `App.qml` 16 ms poll delivers its prompt. Even preview's fast controller therefore exposed an intermediate cleanup message and removed the response row for one or two frames. This was a presentation issue, not a missing cancellation acknowledgement.

The view now gives contiguous `waiting`/`cancelling` presentation one 150 ms single-shot deadline. Before that boundary it retains the preceding layout, including prompt/status space, but hides old prompt/status text (also from accessibility), clears the response immediately, and disables response/submit controls using the **actual** backend state. Initial Begin uses the same policy without inventing a password prompt. Identity changes still follow the backend's identity signal immediately; selection and all action/modal/focus guards are not delayed.

A new PAM prompt, idle outcome/error, loading, power, disconnected, starting, handoff or closing state stops the timer and presents immediately. If cleanup continues, expiration shows the current backend message; subsequent intention replacements update it live without restarting the deadline. Text-only notifications snapshot after the backend's text-before-state batch, avoiding loss of the previous layout when the next view clears its prompt. The snapshot contains only state/prompt/status, never response text.

No Rust/controller/backend/App polling changes. Pending-intention ownership, actor serialization, cancellation barrier, terminal behavior and preview isolation are unchanged. The same presentation policy runs in preview and real mode.

## Checks

Final command, exit 0:

```sh
WAYLIGHT_EVIDENCE=/tmp/waylight-transition-checks sh tests/check.sh
```

Full output: `/tmp/waylight-transition-checks/all-checks.txt`.

- `cargo fmt --check`, locked build, **22 Rust tests**, Clippy with Rust warnings denied: pass.
- Qt 6 `qmllint`: pass.
- Offscreen QML: **20 passing invocations**, including data rows and init/cleanup; zero failures/skips.
- Existing empty/long answers, literal PAM prompts, focus, modal decline/accept/revalidation, switching, Escape, discovery and control-guard checks pass.
- New QML rows run the same transition assertions with `preview=true` and `false`: initial tile wait does not introduce an intermediate layout; rapid cancellation and subsequent waiting retain response and Cancel coordinates; secrets clear immediately; disabled controls and direct submit cannot answer; next prompt/focus is immediate; timer stops; repeated intentions do not extend its deadline; prolonged cleanup shows the latest pending username; terminal/error/closing states cannot remain hidden.
- Deterministic boundary driver verifies the production Timer's 150 ms interval/running state, stops its wall-clock scheduling while holding the view, then emits its native `triggered` signal to advance the boundary. No arbitrary sleeps decide before/after assertions. Coordinate tests await layout polish.
- CLI/QML-load error and preview isolation checks pass. Four existing private headless Cage fake-server cases pass: acknowledged handoff, nonfatal state persistence failure, lost start reply, shutdown during prompt. Logs: `cage-{ack,nonfatal-state,lost-start-reply,shutdown-prompt}.txt`.
- Existing Qt/GCC `QChar` header warning remains; corrupt PNG fixtures emit their expected decoder diagnostics. No new QML/Rust warning.

## Own-application captures inspected

Inspected `/tmp/waylight-transition-checks/switch-{fast,slow}-{normal,minimum}.png`, at 1100×720 and 640×580. These are only the injected-backend application's content, not the host desktop. The fast-boundary captures retain the empty disabled response row and its space, with no cleanup/old PAM text; slow-boundary captures show the requested Sam cleanup status with the still-current Alex identity. The existing login-UX capture set also regenerates, including the eventual switched prompt. These are deterministic held-boundary captures, not a frame-by-frame recording of a real login.

## Files and limits

Changed: `Main.qml`, `tests/tst_preview.qml`, `tests/check.sh`, `README.md`, `.pi/TRANSITION-VERIFICATION.md`.

No Git operations, dependencies, service/configuration changes, real authentication/login, power operation or deployment. Existing isolated fake-server tests only inspect session requests and do not execute them. Their Cage direct child remains the test wrapper, so they establish cascaded shutdown rather than the production direct-child arrangement. Credential clearing still does not promise secure memory erasure. Parent review remains pending.
