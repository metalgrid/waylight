# CONFIG-PLAN: waylight-config utility + greeter localization (locked)

> Approved by user 2026-10-03. Two deliverables: (A) localization of the greeter in the
> 10 most widely-used languages + language selection; (B) `waylight-config` GUI + privileged
> `waylight-configd` D-Bus daemon authorized by polkit. seatd is NOT involved in configuration
> (it is the seat layer for cage/greetd already); polkit governs privilege.

## Deliverable A — localization

- Languages (user-approved 2026-10-03): `en` (source) plus translations for `zh` (Simplified),
  `hi`, `es`, `fr`, `ar` (RTL), `bn`, `pt` (Brazil), `ru`, `ur` (RTL), `bg`. Qt Linguist
  toolchain (qt6-tools, lupdate6/lrelease6) is installed on the build machine.
- Mechanism: Qt Linguist. Wrap every user-visible string in `Main.qml` with `qsTr()`
  (Accessible names/tooltips included). `lupdate` → `i18n/waylight_<lang>.ts`, translations
  committed, `lrelease` → `.qm` embedded via `resources.qrc` (prefix
  `/qt/qml/Waylight/i18n/`). Add `i18n/regenerate.sh` (lupdate+lrelease, dev-time only;
  qt6-tools NOT a build dependency since .qm are committed).
- RTL: `LayoutMirroring.enabled: Qt.application.layoutDirection === Qt.RightToLeft` at the
  Main.qml root; verify mirrored anchors for top-left label / top-right session picker and
  unmirrored clock/panel centering.
- Language selection: new file `waylight.json` (NOT theme.json), same search order as
  themePaths (`${XDG_CONFIG_HOME:-$HOME/.config}/waylight/waylight.json` then
  `/etc/waylight/waylight.json`), format `{"language": "ar"}`. Fail-open: missing/unknown →
  English. Greeter resolves language at startup (main.rs, serde_json) and installs a
  QTranslator + sets layout direction BEFORE engine load; language changes require restart
  (no hot reload — same policy as themes).
- UI language of `waylight-config` follows the same resolved language.

## Deliverable B — config utility

Three pieces, one package:

1. **`waylight-configd`** (headless system D-Bus service, Rust + zbus):
   - Bus name `dev.waylight.Config`, object `/dev/waylight/Config`, interface
     `dev.waylight.Config1`.
   - Methods: `GetAll() -> (s theme, s config, s theme_path, s config_path)`,
     `SetTheme(s json)`, `SetLanguage(s code)`.
   - Writes `/etc/waylight/theme.json` and `/etc/waylight/waylight.json` atomically
     (temp + rename, 0644 root:root, parent dir 0755). Never touches greetd, PAM, users,
     services, or the network.
   - Validation: JSON parses, is an object, ≤ 1 MiB, only known top-level sections
     (colors/background/font/layout for theme; language for config). Token-level validation
     stays fail-open in the greeter (documented; no duplicated validator).
   - polkit: action `dev.waylight.config.set`, `<allow_active>auth_admin_keep</allow_active>`,
     `<allow_inactive>no</allow_inactive>`, `<allow_any>no</allow_any>`; checked per call via
     `check_authorization`. Authorizer injected behind a trait so unit tests run without
     polkit. Runs as root via a dbus-activated systemd service
     (`/usr/share/dbus-1/system-services/dev.waylight.Config.service` +
     `/usr/share/dbus-1/system.d/dev.waylight.Config.conf` +
     `/usr/lib/systemd/system/waylight-configd.service`).
2. **`waylight-config`** (Qt Quick GUI, second binary in the same crate):
   - Panes: preset picker, language picker (native names: Deutsch, Español…), structured
     editors for common tokens (accent, background mode+image path, font family+scale,
     clock/panel ratios, tile sizes) with clamped ranges, and a raw JSON editor tab.
   - Live preview: embeds the greeter `Main.qml` in preview mode with the edited theme
     applied in-memory (no writes until Apply).
   - Apply → daemon `SetTheme`/`SetLanguage` (polkit prompt). Revert reloads from daemon.
     Shows the greeter's fail-open warnings when the edited JSON is questionable.
3. **Polkit action + D-Bus + systemd files** as above; packaged by PKGBUILD (`pkgrel` bump):
   new binaries, polkit action, dbus conf + activation, systemd unit; `polkit` becomes a
   runtime dependency.

## Non-goals

No remote management, no user-session theme editing (greeter reads its own user path only),
no theme marketplace, no hot reload, no changes to greetd config or PAM, no new auth surface
beyond the polkit action, no network access.

## Verification gate (parent)

Full `sh tests/check.sh` green; daemon unit tests (validation, atomic write, injected
authorizer); qmltestrunner: i18n retranslation tests + RTL mirroring test + config GUI model
tests; package builds and installs both binaries + all system files; polkit denial verified
(without auth the daemon must refuse); `pkexec`-style manual test deferred to user.
