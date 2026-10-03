# Login interaction polish — implementation verification

Date: 2026-09-30. Implements immutable `.pi/LOGIN-UX-PLAN.md`.
Status: **Parent validation and independent review accept the scoped build.**
Parent full checks pass: `/tmp/waylight-login-ux-parent/all-checks.txt`, exit 0.
The parent reviewed backend logic, production backend/socket tests, UI controls, and the minimum-size switching image.
Independent review reports no material findings. Deployment and real login remain outside this acceptance.

## Implementation

- `src/backend.rs`: one `Option<Intent>` containing User(username, session index snapshot), Manual, or Power; no answers. The controller's session list is immutable for the process. Validate before replacing and before executing. Cancellable requests initiate one Cancel; requests during cleanup replace the intention. Controller idle releases it directly without publishing intermediate idle for a new login/power action. Public Cancel clears the slot; internal cancellation preserves it.
- Starting/handoff/disconnection, public shutdown, send failure and event-channel loss clear the intention. Starting clears it before later start-rejection cleanup can produce idle. `closing` disables interaction; `identityChosen` is emitted only upon execution. Manual entry/begin remain explicit and idle-only.
- `Main.qml`: click/Space/Return/Enter starts a tile's authentication with one backend call. More… focuses manual input only after cleanup. Identity tiles and authorized power controls work during cancellable/cancelling states. There is no view-side queue. Waiting status identifies the latest request while the displayed identity remains the old one.
- Restart/shutdown dialogs open before cancellation. Decline/Escape preserves unchanged prompt/answer; acceptance revalidates and requests the backend barrier. Sleep requests directly. Prompt updates cannot steal modal focus; committed/power/disconnected/closing states dismiss obsolete dialogs. Focus is restored appropriately, including late tile discovery during prompts. Disabled-state explanations supplement, rather than hide, controller outcome/error messages.
- No changes to `src/controller.rs`, `src/power.rs`, transport framing, logind policy, dependencies, preview isolation or discovery/trust code. The only native test seam is the existing CXX-Qt `make_unique` factory binding for constructing the actual QObject in Rust tests; it is not a QML action or runtime test mode.

## Checks and evidence

Final command (exit 0):

```sh
WAYLIGHT_EVIDENCE=/tmp/waylight-login-ux-checks sh tests/check.sh
```

Full output: `/tmp/waylight-login-ux-checks/all-checks.txt`.

- `cargo fmt --check`, locked build, **22 Rust tests**, Clippy with Rust warnings denied: pass.
- Qt 6.11.2 `qmllint`: pass.
- Offscreen QML: **17 passing invocations**, including data rows and init/cleanup; no failures/skips.
- Private headless Cage executable checks: all four pass; acknowledged handoff and nonfatal state cases greeter=0, lost-start-reply and shutdown-prompt greeter=1, all Cage=0. Logs: `cage-{ack,nonfatal-state,lost-start-reply,shutdown-prompt}.txt` in the evidence directory.
- CLI/QML-load errors and embedded preview isolation checks pass: no greetd/system-bus connection, session-state write or QML disk cache.
- Existing Qt/GCC C++ header `QChar` SFINAE warning remains; no new Rust/Clippy/QML warning was accepted.

### Production backend regression coverage

`production_backend_intentions_and_socket_cleanup_barriers` constructs the actual CXX-Qt backend and calls its actual invokables and `poll`. It checks all cancellable states, latest-valid replacement, one Cancel, session snapshot, deferred identity signal, no intermediate idle signal, public Cancel, shutdown/closing, starting/handoff/disconnection, event loss, command send loss, capability revalidation and blocked states.

The same test joins the unmodified production controller `attempt` with paired fake Unix sockets. It withholds create and cancel replies, observes no premature follow-up request, then proves a new CreateSession only after backend observation of a complete cleanup Success or Error. It captures the actual Power command after the barrier instead of invoking logind. EOF/timeout do not release pending actions. A tile request made after the daemon receives StartSession but before the backend polls `starting` is discarded, including when start is rejected and cleanup later succeeds.

QML tests use an injectable fake backend only for view behavior: all four activation paths exactly once, More…/manual submission, latest pending identity, Escape clearing, sleep barrier, confirmation decline/accept/capability revalidation, retained synthetic answer, asynchronous modal focus, obsolete-dialog closure, closing/committed-state guards, preserved unknown-start warning, and late-discovery focus. Previous prompt/masking/empty-answer/image/fallback/session tests remain. Layout polish is awaited before coordinate tests; the private AT-SPI executable harness was retained unchanged.

## Application-only visual inspection

Inspected only test-window images, never the desktop, at normal 1100×720 and minimum 640×580:

- `manual-{normal,minimum}.png`
- `password-{normal,minimum}.png`
- `password-manual-{normal,minimum}.png`
- `modal-{normal,minimum}.png`
- `switch-wait-{normal,minimum}.png`
- `switched-{normal,minimum}.png`

All under `/tmp/waylight-login-ux-checks/`. Five tiles plus More…, prompt/response, cancellation, and power controls fit. Modal controls are legible at both sizes. Waiting screenshots retain Alex while naming the requested Sam; switched screenshots select Sam and focus its response field. An initial fake-only stale waiting message in the switched screenshot was corrected in the test backend; final images show the new prompt/status. Selected/focused-tile images are also generated.

## Limits and handoff

- **No real PAM authentication, account login, desktop launch, seat/VT handoff, logind authorization or host power operation was performed or verified.** Fake-server requests are inspected, never executed. Tests use only synthetic names/answers and isolated buses/sockets.
- No Git, installation, package, service, PAM, account, polkit, or host configuration changes. Previous plans remain immutable.
- Cage's direct child in executable checks remains the private Python wrapper; these establish cascaded exit, not the exact production direct-child arrangement.
- Real-mode PAM may request arbitrary prompts or no secret prompt. Tile activation starts the real authentication conversation; it does not invent a password challenge.
- Cancellation still waits for the existing actor's framed reply/cleanup deadlines. No retry/reconnect/transport redesign was added. Credential text clearing still does not promise secure memory erasure.
- Worker self-review, parent validation, and independent review are complete. `.pi/LOGIN-UX-TODO.md` records completion.
