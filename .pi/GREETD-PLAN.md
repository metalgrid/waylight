# Development Plan: Usable greetd Client

> Created: 2026-09-30
> Status: LOCKED
> User approves the build and isolated tests. Changes require a new plan.

## Goal and boundaries

Build a Rust/CXX-Qt executable around the approved QML appearance.
Support real greetd authentication, session selection, and permitted logind power controls.
Keep an explicitly isolated preview mode.
Do not change services, PAM, polkit, system files, or installed packages.
The user installed greetd 0.10.3. It is inactive; SDDM is active.
Real PAM login and VT handoff require a later approved system test.
Fingerprint work remains in .pi/FUTURE.md.

## Ownership and integration

One worker owns implementation in this directory, sequentially. There is no Git repository.
Do not initialize Git or create commits. Preserve existing preview plan and assets.
The parent reviews the complete implementation and runs checks before acceptance.
An architect performs final review. Correct material findings before delivery.

## Step 1: Executable and resources

Add Cargo.toml, Cargo.lock, build.rs, src/main.rs, and an App.qml entry point.
Use CXX-Qt 0.10.0 crates together. Embed QML and SVG resources.
Use greetd_ipc 0.10.3 protocol types with bounded transport framing.
Use freedesktop-desktop-entry 0.8.3 without gettext for metadata.
Use zbus 5.19.0 with Tokio and without the default async runtime for logind.
Use one background thread with a Tokio current-thread runtime. Keep UI operations on Qt's thread.
Record dependency licenses. Do not select a project license for the user.
Default mode requires GREETD_SOCK and fails closed. Reject unknown arguments.
--preview must not connect to greetd or the system bus, run power actions, or write state.
Load resources without depending on the working directory. Handle QML load failure and worker shutdown.

## Step 2: Authentication controller

Use explicit states and one actor owning the socket. Permit only one outstanding request/reply exchange.
Snapshot username and session for each attempt. Reject stale events and duplicate prompt responses.
Support arbitrary visible, secret, info, and error authentication messages.
Allow empty string answers. Acknowledge info/error messages with a null response.
Do not infer authentication methods from prompt text. Render external strings as plain text.
Do not log responses or serialized authentication requests. Clear QML fields on state changes and submission.
Do not claim that clearing QML text guarantees memory erasure.
Bound incoming frame size before allocation. Handle partial frames, invalid JSON, timeouts, EOF, and write errors.
Use explicit cancellation after failed authentication and abandonment.
Latch cancellation during an exchange without dropping its partial read/write future.
At the next request boundary, send CancelSession and consume its reply before accepting another attempt.
A complete cancellation Error can follow a cleared daemon configuration in greetd 0.10.3.
A transport failure means uncertain cleanup. Do not silently reconnect or retry credentials.
Keep UI cancellation responsive even when PAM stalls; do not promise immediate PAM interruption.
Check cancellation immediately before sending StartSession. After that boundary, disable cancellation and all competing actions.
A positive start reply means scheduled, not desktop-ready. Save state best-effort and exit promptly.
A missing start reply means indeterminate outcome. Never replay StartSession or claim rollback.
Only an acknowledged start permits a successful handoff exit. Shutdown must stop and join workers.

## Step 3: Sessions and state

Discover sessions only in /usr/local/share/wayland-sessions and /usr/share/wayland-sessions.
Accept root-controlled paths and files. Reject unsafe writable paths and symlinks to untrusted targets.
Handle duplicate IDs predictably. Honor Type, Hidden, NoDisplay, Terminal, and TryExec.
Use the desktop-entry crate for metadata only: its Exec parser does not correctly support quoted arguments.
For this version, accept only plain whitespace-separated Exec arguments.
Reject quotes, backslashes, field codes, NUL, and unsupported reserved characters. Document this limited subset.
All three current local session entries fit this subset, including uwsm-managed Hyprland.
Resolve executables through trusted system paths. Disable login when no valid sessions exist.
Important: greetd joins cmd elements and executes them through /bin/sh -c.
Quote every validated argument with POSIX shell quoting. Send one quoted command string in cmd.
Pass only validated session metadata in env. Never copy the greeter's display, D-Bus, or runtime environment.
Persist only the selected desktop-entry ID after a positive launch acknowledgement.
Use an explicit private greeter state directory and atomic replacement with ownership/symlink checks.
State failure is nonfatal. Do not persist credentials or executable commands.

## Step 4: Interface and power

Keep Main.qml as an injectable view; App.qml creates the Rust backend.
Replace the fixed username with input. Show actual PAM prompts after beginning authentication.
Preserve icon tooltips, session selector, keyboard focus, accessibility, and the original appearance.
Distinguish errors, waiting, cancellation, disconnected state, and session handoff.
Do not silently truncate authentication responses at the preview's 256-character limit.
Escape closes a dialog before cancelling authentication. Support readable long prompts and messages.
Query logind capabilities asynchronously. Enable only capability result yes.
Use noninteractive logind calls. Do not add polkit rules or shell-command fallbacks.
Require confirmation for restart and shutdown. Disable power controls during authentication and handoff.
Preview power actions remain simulated. Tests must never call host power methods.

## Step 5: Verification

Run Cargo formatting, locked build/tests, Clippy, Qt 6 qmllint, and offscreen QML tests.
Use fake Unix-socket greetd tests for prompt sequences, empty answers, errors, retry, fragmented frames, cancellation, and launch outcomes.
Test exact session command quoting, unsupported entries, trust checks, and nonfatal state errors.
Test username and session selection, visible/secret prompts, focus, cancellation, and power confirmations in QML.
Run executable and nested Cage tests against a fake server. Confirm greeter and Cage exit after acknowledged launch.
Do not claim mocks verify real PAM, seat ownership, or VT handoff.

## Step 6: Documentation

Update README.md with build, isolated preview, architecture, tests, and known limits.
Provide an example greetd configuration and prospective installation/recovery instructions.
Use foreground Cage and a dedicated unprivileged greeter account. Preserve VT switching.
Do not install the example. Any system configuration or real-login test requires separate approval.

## Reviewed source evidence

Architect reviews close the design gate with the constraints above.
Official author mirror: https://github.com/kennylevinsen/greetd tag 0.10.3.
Local source: /tmp/greeter-source-review/kennylevinsen-greetd-9afd7b1.
context.rs cancellation removes configuring before contacting its worker; EOF alone does not cancel it.
StartSession schedules before reply; actual launch follows greeter termination.
session/worker.rs joins command elements and invokes /bin/sh -c.
Protocol documentation: https://man.archlinux.org/man/greetd-ipc.7.en.
