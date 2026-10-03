# Waylight greetd greeter

A Rust/CXX-Qt greeter using the original macOS-style Qt Quick preview and original SVG artwork.
It now speaks greetd IPC, handles PAM conversations, discovers Wayland sessions, and offers
logind-authorized power controls. **It is not installed. SDDM, greetd, PAM and polkit were not changed.**
Fingerprint integration is future work in [`.pi/FUTURE.md`](.pi/FUTURE.md).

Licensed [GPL-3.0-only](LICENSE). Copyright 2026 Iskren Hadzhinedev.

## Build and isolated preview

Requires an existing Rust/C++ toolchain, Qt 6 development tools and runtime QML modules
(QtQuick, Controls, Layouts, Wayland and SVG), and pkg-config. No package installation is
performed by this project. Tested with Qt 6.11.2 and the versions in `Cargo.lock`.

```sh
cargo build --locked
QT_QPA_PLATFORM=wayland ./target/debug/waylight-greeter --preview
# Optional nested compositor, on your existing Wayland desktop:
WLR_BACKENDS=wayland QT_QPA_PLATFORM=wayland cage -- ./target/debug/waylight-greeter --preview
```

QML/SVGs are embedded; the executable works from another directory without this source tree.
Qt shared libraries/plugins still need to be installed. `Main.qml` is an injectable view;
`App.qml` creates the real Rust backend. Launch the executable, not `qml6 Main.qml`.

`--preview` ignores `GREETD_SOCK`, never connects to greetd or the system bus, never performs
power actions, never discovers real sessions or users, and never reads/writes the greeter's session
state. It rejects `--state-dir`. Its three sessions, five demo users with original bundled
PNG portraits (**Alex**, **Sam**, **Lee**, **Robin**, **Jules**), Password prompt, and
authentication/power results are simulated. Demo pictures are embedded bytes, not host files. Use **sample text only**. QML disk caching is disabled; Qt/fontconfig may still
maintain ordinary font caches, which are not greeter state.

Without `--preview`, an absolute `GREETD_SOCK` is mandatory. Missing configuration fails
closed before Qt starts; there is no automatic preview, socket discovery, or reconnect.
Unknown/duplicate arguments fail. `--help` prints usage.

## Controls

1. Choose one of up to **five user tiles**, or **More…** to enter any username manually.
   Manual entry is the default and has initial focus, even when no users are discovered.
   Choose the desktop first: clicking a pictured tile (or Space/Enter on it) immediately
   starts authentication, exactly once. More… switches to manual entry without submitting;
   type a username, then press Enter or the arrow. Tab reaches each tile; Left/Right and
   Home/End move tile focus. Selection and keyboard focus are visibly marked.
   Pictures have fixed size; display names are plain text, with the username in each tile's
   accessible name/tooltip and beneath the selected row for disambiguation.
2. The response field is enabled **only** when PAM requests a secret or visible answer, with
   a single submit arrow. The exact PAM prompt is displayed literally above **Enter answer**;
   secret does not necessarily mean password. Preview instead shows a simulated **Password**.
   Empty answers are supported. Information/error messages show **Continue** (or Enter),
   with no response field, and are acknowledged with a null response.
3. Escape closes a power/session popup first; otherwise it requests authentication cancellation.
   Cancel remains responsive while PAM is busy, but cleanup waits for the pending reply.
   You can also directly choose another tile, More…, or an authorized power control during
   authentication/cancellation. The latest valid request replaces the previous intention;
   it executes only after confirmed cleanup. Cancel/Escape clears that intention.
   Brief waiting/cleanup transitions keep the previous layout for up to 150 ms, with the
   response cleared and disabled immediately and old prompt/status text hidden. Longer waits
   show the latest request; new PAM prompts and terminal/error outcomes appear immediately.
4. Tab navigation, focus, accessible names, tooltips and scrollable long messages are available.
   All account/daemon/session text is plain text, not HTML.

The user tiles read AccountsService's `ListCachedUsers`, then `UserName`, `RealName`,
`SystemAccount`, `Locked`, and optional `IconFile`, using the existing zbus dependency. It inspects at most **64**
cached records with **one 3-second deadline** covering connection, listing and properties.
Unavailable/malformed, system, and locked records are omitted; usernames use the same validation
as manual entry. Entries are deduplicated and sorted by username (then name for duplicate ties).
Blank display names fall back to usernames. Only the first five sorted users get tiles; additional
users remain reachable through **More…**, which is always present. A missing `IconFile` does not
hide the account.

Pictures are accepted **only from immediate regular files in `/var/lib/AccountsService/icons`**.
The existing session trust walker checks ownership and permissions of every path component:
root-owned and not group/world-writable. Leaf symlinks, links escaping the cache, directories,
FIFOs, relative paths, URLs and home-directory pictures are not accepted. Opens use no-follow
and nonblocking flags; reads are capped at **256 KiB**, PNG header dimensions at **256×256**
(nonzero). Qt's existing PNG decoder validates those bounded bytes before existing Qt base64
encoding creates a data URL; QML never reopens the account's path. Only five pictures are read,
with at most 349,550 data-URL bytes per picture and a fixed QML `sourceSize` of **56×56**.
No image or encoding dependency was added. Missing, unreadable, unsupported or corrupt images
show the existing silhouette. **Cache-only limitation:** valid AccountsService pictures outside
this cache, leaf symlinks, SVG/JPEG files, and larger PNGs also show the silhouette. No copying,
account modification, download, or home read is attempted. The file checks assume a local,
root-controlled cache; the async deadline cannot preempt a stalled kernel filesystem read.

This is a one-time **cached subset**, not a complete login directory or an authorization decision;
PAM remains authoritative. Bus errors/timeouts leave **More…** available. Late results only
update the list: they do not replace manual text, change the username identity, reset authentication,
or steal focus. Intentional idle identity edits clear stale status/responses through the backend;
requested identity changes wait until authentication cancellation cleanup is confirmed.
The displayed identity changes only when the backend executes the switch, not when it is queued.

Username/session are fixed for each attempt. Fields clear on transitions/submission;
this **does not guarantee erasure of credentials from memory**. Responses and serialized
requests are never logged or persisted. NUL characters and answers whose complete JSON request
exceeds **10 KiB (10240 bytes)** are rejected locally, with the same prompt available for correction.
Empty answers remain valid. JSON escaping counts toward this limit (ASCII needing no escaping
allows 10189 bytes). This conservatively fits greetd 0.10.3's internal PAM datagram buffer; it is not a
raw-text allowance. Incoming reply frames have a separate **64 KiB** bound.

Power and identity tiles remain available during cancellable authentication and cancellation.
Loading, power execution, closing, disconnection and committed session start/handoff disable
these controls with an explanation. The worker asynchronously checks `CanSuspend`, `CanReboot`, `CanPowerOff`; only exactly `yes` enables an
action (`challenge`, `no`, missing bus and errors disable it). Restart/shutdown need confirmation:
opening/declining the dialog does not cancel authentication or clear an unchanged answer.
Accepting rechecks availability and requests cleanup before dispatch; Sleep uses the same
barrier without a confirmation dialog. Prompt updates cannot steal modal focus, and obsolete
dialogs close on committed start, power execution, disconnection or shutdown. Calls use `interactive=false`; no shell fallback or polkit rules are supplied. Capability checks
can become stale; logind remains the final authority and call failures are displayed.
**Do not test real power controls as part of isolated verification.**

## Theming

Colors, the background, the font and the element sizes of the sign-in view come from one theme
object (`Theme.qml`). The built-in defaults are token-for-token the previous literals, so the
stock look is unchanged. There is no theme picker and no flag: the greeter reads one optional
JSON file at startup and otherwise uses the built-in presets or defaults.

Search order (the first readable file wins; a readable but broken file still wins with defaults):

1. `${XDG_CONFIG_HOME:-$HOME/.config}/waylight/theme.json`
2. `/etc/waylight/theme.json`

An annotated copy of every key is in [`examples/theme.json`](examples/theme.json). The file is
plain JSON with four sections plus an optional preset:

```json
{
  "preset": "dusk",
  "colors":   { "accent": "#d9e2ff" },
  "background": { "mode": "builtin", "image": "", "color": "#182b56", "top": "#182b56", "bottom": "#182b56" },
  "font":     { "family": "sans-serif", "scale": 1.0 },
  "layout":   { "panelMaxWidth": 580 }
}
```

- `preset`: `dusk` (the stock look), `midnight` (near-black blues, dimmer fills) or `daylight`
  (light frosted gradient, dark text). The preset applies first; explicit keys override it.
- `background.mode`: `builtin` (embedded SVG), `image` (local file, drawn `PreserveAspectCrop`),
  `solid` (`color`), or `gradient` (`top` to `bottom`). The translucent overlay above the
  background is themed separately (`overlayTop`/`overlayBottom`).
- `font.scale` multiplies every font size and the clock cap (0.5–2.0); `font.family` accepts any
  family Qt can resolve.
- `layout` is a bounded size set (clock/panel positions as window-height ratios, panel, tiles,
  avatar, fields, radii, power buttons, bar margins). There is no free-form element placement.

Validation is fail-open and can never prevent sign-in: a missing or malformed file means
defaults; unknown keys are ignored; a wrong-typed value falls back to that key's default; colors
must be `#rgb`, `#rgba`, `#rrggbb` or `#aarrggbb`; ratios are clamped to 0–0.9, sizes to
1–2000 px and `font.scale` to 0.5–2.0. The complete token reference — every key, default,
accepted values and what each token draws — is [THEMING.md](THEMING.md); the source of truth
is `Theme.qml`, and the locked plan is [`.pi/THEME-PLAN.md`](.pi/THEME-PLAN.md). One text color outside the locked
table, the session picker text, is also themed as `colors.textCombo` (default `#ecedf5`) so the
light preset can stay readable.

Not themed: the embedded SVG artwork (background.svg, power icons) and the bundled demo
portraits are fixed files; the white power icons have little contrast on `daylight` (their
focus/hover affordances remain themed). `--preview` loads the same theme files, presentation
only. Themes apply at startup and presets can be switched at runtime by tests; there is no hot
reload. Preset screenshots are produced only by the isolated offscreen QML tests into
`/tmp/waylight-theme-checks/`.

## Authentication and lifecycle

`src/controller.rs` owns the sole Unix socket on one background thread with a Tokio
current-thread runtime. The Qt thread only sends bounded commands and polls event messages;
it never performs authentication, filesystem or D-Bus I/O. The worker serializes request/reply
exchanges. Unique prompt tokens reject stale/duplicate responses. Cancellation is latched
without dropping/restarting a partial read/write future. After a complete pending reply,
`CancelSession` must complete before another attempt is accepted, including after authentication
failure. In greetd 0.10.3 a complete cancellation Error also follows removal of its configuring
session. A broken transport is **not** a cleanup acknowledgement.

`src/backend.rs` owns one pending User(username, session snapshot), Manual, or Power intention;
there is no QML queue and no stored authentication response in that slot. Repeated requests
replace it without issuing repeated Cancel commands. Validation occurs before replacement
and again before execution. Only a controller idle event after cleanup releases it, without an
intermediate interactive idle view. Worker/channel loss, shutdown, disconnection, starting and
handoff discard it. A switch losing the StartSession race is discarded even if that start is
later rejected; it is never replayed.

Connect timeout: 5 seconds. Each framed exchange: 30 seconds. A timeout/EOF/invalid frame
fails closed and disables retry; it cannot prove PAM cleanup. There is no blind reconnect,
automatic credential retry, or StartSession replay. A user can spend arbitrarily long answering
a displayed prompt. A stalled PAM request may delay cancellation/close by its remaining timeout
plus up to 30 seconds for cleanup. SIGINT, SIGTERM and window close request cleanup; workers are
joined before normal exit. SIGKILL/crashes cannot promise cleanup; merely closing the socket
does not cancel greetd's configuring session.

The selected session's complete command/environment request is preflighted against the same
10 KiB outgoing limit before authentication begins. Invalid C strings or malformed environment
assignments are also rejected then: no authentication state exists to clean up, and the UI returns
to idle rather than reporting an uncertain launch.

Immediately before sending StartSession, cancellation is checked one last time. After this
boundary all competing actions are disabled. A positive reply confirms **scheduled**, not a
ready desktop; the greeter saves only the selected session ID best-effort and exits promptly,
allowing foreground Cage to exit. greetd launches the scheduled session after its greeter exits.
A missing/invalid start reply has an **indeterminate outcome**: it might already be scheduled.
The UI says so, does not replay or claim rollback, and closing it exits unsuccessfully.
Exit status `0` is reserved for acknowledged handoff; normal preview/cancelled close returns
`1`, CLI errors return `2`. An unexpected Qt exit also requests and joins worker cleanup.

Source-specific decisions were checked against greetd tag 0.10.3:
https://github.com/kennylevinsen/greetd and https://man.archlinux.org/man/greetd-ipc.7.en.

## Sessions and optional state

Only `/usr/local/share/wayland-sessions` and `/usr/share/wayland-sessions` are scanned.
Directories, files, executable paths and every traversed symlink target/ancestor must be
root-owned and not group/world-writable; each symlink itself must be root-owned (its mode bits
are irrelevant). Links are expanded one at a time, including relative targets, before handling
`..`; canonicalization cannot hide an unsafe intermediate link. Traversal is bounded to 40 links,
rejecting cycles while retaining trusted links such as `/bin` → `/usr/bin`. Unsafe entries are
omitted. Local entries take priority; an invalid or hidden local entry also masks the same system
ID. Ordering is deterministic by filename.
`Type=Application`, `Hidden`, `NoDisplay`, `Terminal`, `TryExec` and valid names are checked.
No supported sessions means login is disabled.

The desktop-entry crate is used for metadata, **not its Exec argument parser**. The supported
Exec subset is deliberately small: space/tab-separated ASCII words containing only letters,
digits and `/_-.+:=@,`. Quotes, backslashes, `%` field codes, NUL, newlines, shell operators,
expansions and other reserved characters are rejected. The first word must be an absolute
trusted executable or resolve in `/usr/local/bin`, `/usr/bin`, `/bin`; the greeter's PATH is
ignored. Installed `hyprland.desktop`, `hyprland-uwsm.desktop`, and `weston.desktop` are accepted.

Because greetd joins `cmd` and invokes `/bin/sh -c`, **every argument is POSIX single-quoted**,
and one complete quoted command string is sent. Only validated `XDG_SESSION_TYPE=wayland`,
`XDG_SESSION_DESKTOP`, and optional `XDG_CURRENT_DESKTOP` are sent in `env`. Greeter display,
D-Bus, runtime-directory and other environment variables are never copied to the session request.
The daemon/PAM retain responsibility for the actual user's environment.

`--state-dir /absolute/private/directory` enables the optional preference. The directory must
already exist, be owned by the greeter UID, and have mode `0700`. Ancestors must be root/greeter
owned and not writable by others (a root-owned sticky ancestor is allowed); symlink components
are rejected. The `session` file must be a singly-linked regular file owned by that UID, mode
`0600`. Directory-FD-relative opens, no-follow checks, exclusive temporary creation and atomic
rename protect the store. Only a validated desktop-entry ID is stored **after acknowledged
launch**. Bad/missing state falls back to the first session; save failure is nonfatal.
Use a local filesystem: the advisory preference is atomically replaced, not crash-durable
(no fsync delaying handoff). No directory is auto-created, and no state is stored by default.

## Verification

From the project root, using the already installed tools:

```sh
sh tests/check.sh
```

This runs `cargo fmt --check`, locked build/tests, Clippy with Rust warnings denied, Qt 6
`qmllint` (with staged generated QML type metadata), offscreen QML tests, and
`python tests/executable.py`. The final test requires Cage, a C compiler, wayland-client,
xkbcommon, `dbus-run-session`, and Python GObject/AT-SPI bindings plus `at-spi2-registryd`
(on Arch: `python-gobject` and `at-spi2-core`). It compiles a **test-only** virtual keyboard
helper restricted to a private headless Cage runtime socket. A private accessibility bus
observes focused/enabled controls and exact synthetic prompts before sending each input once;
fixed startup/prompt sleeps are not readiness checks. No host desktop, keyboard, accessibility
bus, or production test hook is used.

Rust tests cover arbitrary prompt sequences, empty answers, null acknowledgements, stale
responses, local rejection/correction of oversized ASCII, JSON-escaped and NUL answers,
session preflight before any request, internal datagram overhead, fragmented frames, cancellation
barriers, retry after cleanup, invalid/oversize frames, EOF/timeouts, launch uncertainty, shutdown,
actual CXX-Qt backend pending-intention transitions/dispatch and identity signals (not a model
of the backend), latest-valid replacement, one Cancel, public Cancel clearing, capability
revocation, closing/channel loss, and a StartSession race followed by rejection. Paired fake
sockets hold both create/cancel replies to verify switch/power ordering, complete cancellation
Error acceptance, and no deferred release after EOF/timeout. Power dispatch is captured as a
command; those tests cannot call host logind. Other tests cover
session validation/quoting/trust (including intermediate symlinks and cycles) and state safety.
Account tests inject records/futures into the production filtering/count/deadline helpers; they
cover malformed, locked, system, duplicate and unavailable records, failure and stalled discovery,
without using the host bus. Picture tests use original local assets and private fixtures (no sudo)
for byte/dimension bounds, corrupt/unsupported images, missing files, ownership/permission
checks, symlink escapes, FIFO/directory rejection, base64 round-trips and isolated demo portraits.
Injected-backend QML tests cover five-tile capping, keyboard/click selection, More…/manual/session
selection, image loading/fallback, late
account arrival, stable username identity, masking, focus, control visibility, literal arbitrary prompts,
empty/long answers, cancellation, disabled committed states, one-click/Space/Enter authentication,
manual fallback, pending switches, confirmation decline/accept/revalidation, modal focus and
late-discovery focus during prompts, and deterministic 150 ms presentation-timer boundaries
(rapid-switch layout retention, delayed latest-intent status, and immediate failure/terminal states).
Theme tests cover the token defaults against the locked literals, JSON overrides, color formats,
unknown keys/presets, wrong types, clamping, fail-open file loading and first-existing-path
selection, preset switching with WCAG contrast checks for `daylight`, and background modes.
A dedicated harness also compared grabs of the previous `Main.qml` (from git HEAD) and the
theme-token version at 1100×720 and 640×580; both were byte-identical.
Coordinate inputs wait for layout polish. Tests also save
only their own offscreen application content at 1100×720 and minimum 640×580 to
`/tmp/waylight-login-ux-checks/` (manual, focused tile, selected/manual Password, modal,
waiting-for-switch and switched views). Fast/slow transition captures at both sizes are in
`/tmp/waylight-transition-checks/`.

Executable tests copy the binary away from the source, test CLI/preview isolation, and run
four fake-server headless Cage cases: acknowledged handoff with restored session selection,
nonfatal state failure, lost start reply, and shutdown during a PAM prompt. The server only
inspects commands; it **never launches a desktop or authenticates anyone**. Its system-bus
address is an isolated nonexistent socket. Cage's direct child in these tests is a Python wrapper
that waits for the greeter and then exits: this demonstrates **cascaded Cage shutdown**, not the
exact production direct-child arrangement. Evidence defaults to `/tmp/waylight-checks` (override
with `WAYLIGHT_EVIDENCE`; login-UX images are `/tmp/waylight-login-ux-checks`). These checks
do not validate real PAM, logind authorization, seat ownership, or VT handoff.

## Prospective installation and recovery — separate approval required

[`examples/greetd.toml`](examples/greetd.toml) is an **uninstalled proposal**, not a command to
run now. No services, accounts, packages, PAM/polkit files or `/etc` files were modified.

After explicit deployment approval:

- Dependency licensing is settled: the project is GPL-3.0-only, matching
  [`greetd_ipc`](DEPENDENCIES.md). Build a locked release binary and install it root-owned outside writable homes.
- Provision a dedicated **unprivileged** greeter account and its private local state directory.
  Review distribution-provided greetd/Cage runtime/seat permissions; do not grant blanket
  passwordless power or add ad-hoc PAM/polkit rules.
- Back up the existing display-manager configuration and keep a known-working recovery login
  on another VT (or approved remote access). Choose a non-conflicting test VT deliberately.
- Use **foreground** `cage -s -- ...`; `-s` preserves VT switching. Do not background Cage or
  wrap the greeter with a process that stays alive after it exits.
- Only in a separately approved maintenance test, validate real PAM and VT/session handoff,
  cancellation, recovery access and the selected desktop before considering an SDDM replacement.
- If the test fails, use the recovery VT to restore the saved display-manager configuration
  and return to SDDM using the distribution's service-management procedure. Do not delete or
  overwrite existing configuration as a troubleshooting shortcut. A start with a missing
  acknowledgement may already have launched; inspect the actual seat/session before retrying.

No real login, real power action, service switch or deployment is authorized by the build plan.
