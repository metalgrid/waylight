# CONFIG-VERIFICATION

> Evidence for `.pi/CONFIG-PLAN.md`. Gate: full `sh tests/check.sh` green after
> every deliverable. greetd/PAM/SDDM, `/etc`, the package files and `vm/` are
> untouched. No new runtime or build dependency: Qt Linguist tools
> (`lupdate6`/`lrelease6`) are dev-time only and the catalogs (`.ts` + `.qm`)
> are committed.

## Corrective pass (2026-10-05, on top of f4b7563)

All fixes from the accepted architect reviews of Deliverables A and B, plus
the packaging version bump. Nothing committed — the working tree is left for
parent review. Full gate green **after** the pass (see Gate below).

### B-fixes (config utility)

1. **Daemon failure now surfaces in the GUI.** `ConfigMain.qml` used to keep
   the local "Applying…"/"Reverting…" placeholder forever when the daemon
   failed: the status label's binding preferred `localStatus`, and only the
   color reacted to `status === "error"`. A new `Connections.onStatusChanged`
   handler clears `localStatus` on any terminal daemon status (ready or
   error), so `backend.message` (e.g. `InvalidTheme`) becomes visible. The
   label got `objectName: "statusLabel"` for tests. New
   `test_daemon_failure_surfaces_message_and_clears_applying` drives a mock
   backend whose `applyTheme` mirrors the real flow (status → busy, then a
   deferred Failed reply): it asserts the intermediate placeholder, the
   clear-on-failure, the daemon error text in firebrick, and recovery on a
   later successful apply.
2. **`store.rs::commit` no longer leaks the temp file on a mid-write
   failure.** `write_all`/`sync_all` errors now unlink the temp file exactly
   like the rename-failure path. `commit_removes_the_temp_file_when_the_write_fails`
   exercises the real failure path via `/dev/full` (ENOSPC on write) and
   asserts no residue and no target.
3. **Size cap aligned with the greeter: 1,048,576 → 1,000,000 bytes.**
   `Theme.qml` ignores files over 1,000,000 *characters*; a ~1.01 MB file
   used to pass the daemon yet was wholly ignored by the greeter. The daemon
   now caps at the same 1,000,000 bytes (characters ≤ bytes in UTF-8, so a
   daemon-accepted file can never be wholly ignored), the error message names
   the exact limit, and `theme_enforces_the_size_cap` asserts the exact
   boundary (exactly 1,000,000 bytes passes, one byte more is rejected).
4. **Size-capped reads.** `store::read` (used by GetAll) now reads at most
   1,000,000 bytes and hands out nothing for oversized files (symmetry with
   `i18n::read_limited`; the one-byte-past-cap read detects oversize so no
   truncated document is ever returned). `read_caps_the_size_and_fails_open`
   covers missing/capped/oversized.
5. **`system/waylight-configd.service` hardened:** explicit `User=root`,
   `ProtectSystem=strict`, `ReadWritePaths=-/etc/waylight` (prefixed `-`: the
   directory may not exist until the daemon creates it 0755),
   `RestrictAddressFamilies=AF_UNIX` (system-bus socket only).
6. **`ConfigMain.qml`'s previewBackend declares `signal identityChosen(string
   name)`**, so Main.qml's `Connections` binds to a real signal instead of an
   implicitly synthesized one.
7. **`validate::config` documented** (kept, not dropped): the daemon always
   writes the canonical `{"language": "<code>"}` form, so `config()` is not
   on the daemon's write path; its doc comment now states it is the structural
   contract for hand-written waylight.json.
8. **This file corrected:** the real qmltestrunner arithmetic is recorded
   below (the old text claimed 12 config tests / 56 = 44 + 12; tst_config had
   10 tests before this pass, 11 after — qmltestrunner also counts each
   suite's initTestCase/cleanupTestCase in its totals). The three documented
   limitations the plan requires are in the B known-limitations list (two
   were missing: structured-tab unknown-key behavior and the no-D-Bus-timeout
   posture).

### A-fixes (localization)

9. **Clock locale follows the UI language.** `i18n::install_language` now
   caches the resolved language (`i18n::installed_language`), the Backend
   QObject exposes it as the `uiLanguage` qproperty, and both clock labels in
   Main.qml format via
   `now.toLocaleString(Qt.locale(backend.uiLanguage), …)` — the
   locale-first overload (the `(date, format, locale)` form of
   `Qt.formatDateTime` is not callable from Qt 6.11 QML and silently ignores
   the format string in the `(date, locale, …)` shape; verified by probe).
   `ConfigMain.qml`'s previewBackend and both test fakes gained the same
   property (default "en") so bindings never break. New
   `test_clock_formatting_follows_ui_language` pins the clock timer, fixes a
   Monday, and asserts the date label switches to `bg` („януари") and the
   time label to Arabic-Indic digits under `ar`, restoring for `en`. The
   clock labels carry `clockDate`/`clockTime` objectNames and the wall-clock
   timer `clockTimer`.
10. **Cage harness isolation:** `WAYLIGHT_SKIP_SYSTEM_CONFIG=1` (consulted by
    the new pure `backend::skip_system_config()` and a `skip_system` flag
    parameter on both `theme_paths_json` and `config_language_json`) drops the
    `/etc/waylight` candidates from both search lists; `tests/executable.py`
    sets the variable in the harness environment next to the XDG isolation
    (theme.json XHR reads are process-level — main.rs enables them in-process
    — so the env var covers exactly the /etc coupling the translated-Cage runs
    suffered from). The stale comment about `QML_XHR_ALLOW_FILE_READ` being
    "deliberately NOT set" is corrected (main.rs sets it in-process now).
    `skip_system_flag_drops_the_etc_candidates` covers both helpers including
    the no-user-base case.
11. **RTL cosmetics:** the session-picker chevron `x` is a mirrored binding
    (`root.rtl ? 14 : session.width - width - 14`); tile
    `Keys.onLeftPressed`/`onRightPressed` invert under RTL (Left moves to the
    next tile in display order). `rtl` is a plain root property bound to
    `Qt.application.layoutDirection === Qt.RightToLeft` (not readonly, so
    tests can drive the RTL branches the same way they drive
    `LayoutMirroring.enabled`). New
    `test_rtl_inverts_tile_arrows_and_mirrors_chevron` asserts the chevron
    edge and the inverted wrap behavior with two users + More…. French power
    dialog titles now use U+00A0 before the question mark
    ("Redémarrer cet ordinateur ?") and Bulgarian Continue is «Продължи»
    (was «Напред») — fixed in the committed .ts, in the translations.py table
    (so regeneration preserves them) and re-released into the .qm; locked by
    the new `french_titles_keep_the_nbsp_before_the_question_mark` and
    `bulgarian_continue_uses_the_standard_verb` Rust tests.
12. **Docs:** README documents the waylight.json shape with supported codes,
    the exact 1,000,000-byte cap, the hardened unit, and a new "Utility
    behavior worth knowing" subsection (restart-required policy, no D-Bus
    method timeout + Revert recovery, structured-tab unknown-key behavior,
    clock locale). THEMING.md's size note states characters.

### Packaging

13. **`packaging/PKGBUILD`: `pkgver` 0.1.0 → 0.2.0, `pkgrel` 1** (resets with
    the new version). The published source line stays
    `$pkgname::git+$url.git#tag=v$pkgver`. For the local verification build
    the source was pinned to a scratch git snapshot commit of exactly this
    working tree (`git+file:///tmp/…#commit=d4cd221461ead8eacadc1343bce5095bb79fba0a`)
    — the tree must stay uncommitted, so `#commit=<HEAD>` would have built
    stale code. `makepkg -f` green; `waylight-greeter-0.2.0-1-x86_64.pkg.tar.zst`
    contains the three binaries, the four system files (verified in the
    tarball, including the hardened unit), tmpfiles and docs; the packaged
    greeter binary embeds the new `WAYLIGHT_SKIP_SYSTEM_CONFIG` switch.
    **Maintainer action after review:** `git tag v0.2.0 && git push origin
    v0.2.0` — the published PKGBUILD resolves `v$pkgver` to that tag, and the
    package is not reproducible from the published source line until it
    exists.

### Gate (after the corrective pass)

- `sh tests/check.sh` — **exit 0**: `cargo fmt --check`, locked build, clippy
  `-D warnings`, qmllint (five module files + four test files),
  qmltestrunner, python harness (CLI/QML-load isolation + four headless Cage
  cases, all translated-harness-neutral now that /etc cannot leak in).
- **Real test totals:** 56 Rust tests (`cargo test --locked`, lib target;
  was 51, +5: skip-system flag, temp-on-write-failure, capped read, fr NBSP,
  bg Continue). 59 QML tests passed, 0 failed, 0 skipped — 51 test functions
  (ConfigEditor 11, GreeterView 20, I18n 6, ThemeTokens 14) plus each suite's
  initTestCase/cleanupTestCase, which qmltestrunner counts in its totals
  (48 functions + 8 = 56 before this pass; tst_config had 10 functions, not
  12 as previously claimed).
- No system changes; `vm/` untouched; greetd/PAM/SDDM untouched; no new
  dependencies (the `/dev/full` failure-path test and the env kill switch add
  none).

---

## Deliverable A — localization (complete 2026-10-04)

### A1 — i18n infrastructure

- Every user-visible string in `Main.qml` is wrapped in `qsTr()`: window title,
  corner label ("PREVIEW · No system changes" / "SIGN IN · Wayland"), session
  picker `Accessible.name` + `ToolTip.text`, the More… tile display name and its
  Accessible name/description, the "…" glyph, "Username"/"Password"/"Enter
  answer" placeholders, "Sign in"/"Submit answer"/"Authentication response"
  Accessible names, "Continue", "Cancel", all five `blockedReason` sentences,
  both power-dialog titles, and the dialog buttons.
- Dialog buttons: `standardButtons: Dialog.Yes | Dialog.No` was replaced by an
  explicit `DialogButtonBox` footer (`objectName: "confirmYes"/"confirmNo"`,
  `qsTr("Yes")/qsTr("No")`, AcceptRole/RejectRole) because standard-button texts
  do not translate. Behavior is unchanged (accept → power request after
  revalidation, reject/Escape → close).
- **objectName stability:** power buttons and the More… tile now carry an
  untranslated `key` in their models (`Sleep`/`Restart`/`Shut Down`, `more`);
  `objectName` binds to the key and only the displayed label/tooltip/Accessible
  strings translate. `username`, `password`, `session`, `login`,
  `answerLogin`, `continueControl`, `cancel`, `confirmPower` objectNames are
  unchanged. The Cage harness matches "Username" and "Continue" (a11y names) —
  unchanged in English and verified by the four passing Cage cases.
  `→` glyphs stay untranslated.
- Languages: `en` (source, no catalog) + `zh, hi, es, fr, ar, bn, pt (Brazil),
  ru, ur, bg`. `i18n/regenerate.sh` finds `lupdate6`/`lrelease6` (or
  unsuffixed/`/usr/lib/qt6/bin` variants), runs `lupdate Main.qml -no-obsolete
  -ts` over all 11 catalogs, applies the reviewed table via
  `i18n/translations.py` (never overwrites existing translations; missing
  entries are reported but not fatal — lrelease drops them and the greeter
  falls back to the English source), then `lrelease` per language. Idempotent:
  a second run leaves the tree unchanged ("31 finished, 0 unfinished" per
  catalog). All `.ts` and `.qm` files are committed; context is `Main`;
  31 messages each.
- Embedding: `resources.qrc` lists all 10 `.qm` under the existing
  `/qt/qml/Waylight` prefix, so the runtime paths are
  `:/qt/qml/Waylight/i18n/waylight_<lang>.qm`. qt6-tools remains a dev-time
  dependency only.

### A2 — RTL + tests

- `Main.qml` root: `LayoutMirroring.enabled:
  Qt.application.layoutDirection === Qt.RightToLeft` and
  `LayoutMirroring.childrenInherit: true`.
- `tests/tst_i18n.qml` (qmltestrunner, same fake-backend style as
  `tst_preview.qml`):
  - `test_english_source_strings_unchanged` — every locked English string,
    blockedReason matrix (incl. `closing`), dialog titles via `action`,
    Yes/No texts, power keys/labels and the " (preview only)" suffix.
  - `test_committed_catalogs_and_source_coverage` — every `waylight_<lang>.qm`
    committed and non-empty, no `waylight_en.qm`, and `i18n/waylight.ts` lists
    all 31 sources under the `Main` context (catches unwrapped strings).
  - `test_layout_mirroring_follows_direction` — asserts `childrenInherit`,
    that the production binding mirrors exactly when the direction is RTL, and
    drives mirroring: corner label swaps to the right, session picker to the
    left, clock column and power row stay centered, then restores.
  - `test_power_object_names_survive_labels` — objectName === untranslated key
    for all three power buttons; More… tile key `more` with empty username.
- **Direction/translation levers verified and rejected** (all probed on this
  machine): `QT_LAYOUT_DIRECTION=RTL` — no effect; RTL locale (`LANG=ar_EG`,
  `LC_ALL=ur_PK`) — no effect under qmltestrunner; `Qt.application.layoutDirection`
  — read-only from QML ("Cannot assign to read-only property"); importing the
  built `Waylight` module into qmltestrunner — impossible (the module has no
  plugin `.so`; its types are embedded in the greeter binary's resources);
  installing a `QTranslator` from pure QML — no public API. Therefore the QML
  tests assert the LTR side of the direction binding plus full mirroring
  geometry by driving `LayoutMirroring.enabled` directly (exactly the effect
  the binding produces under RTL), and the real translated lookups are proven
  end-to-end in Rust (below). The greeter's own RTL startup is exercised
  through `waylight_set_layout_direction` in production code.

### A3 — language file

- `waylight.json` format `{"language": "<code>"}`; search order identical to
  `themePaths`: `${XDG_CONFIG_HOME:-$HOME/.config}/waylight/waylight.json`
  then `/etc/waylight/waylight.json`. Implemented as the pure
  `backend::config_language_json(xdg, home)` mirroring `theme_paths_json`
  (same empty/`None` handling and JSON escaping), exposed as the Backend
  qproperty `configPaths` (for symmetry with `themePaths`; Deliverable B
  consumes it).
- Startup resolution in `src/main.rs`, all before the QML engine loads:
  `validated_language` (JSON object, `"language"` string, supported set
  `en zh hi es fr ar bn pt ru ur bg`, else `None`), `language_candidates`
  (decode, malformed → empty), `read_limited` (64 KiB cap, non-UTF-8 → skip),
  `resolve_language` (first readable + valid file wins), `install_language`
  (installs `:/qt/qml/Waylight/i18n/waylight_<code>.qm` via the
  `waylight_i18n.h` helpers; missing catalog → English + stderr warning;
  sets `QGuiApplication::layoutDirection` explicitly to RightToLeft for
  `ar`/`ur` and LeftToRight otherwise, so the host locale cannot flip the
  greeter). Everything is fail-open; `env` handling follows the existing
  main.rs pattern (no mutation beyond the existing two Qt variables).
- C++ side: `waylight_i18n.h` (crate root, picked up through cxx-qt-build's
  crate include root) — install (application-owned translator, registry for
  removal), deterministic layout direction, and a test-only direct catalog
  lookup. `src/i18n.rs` is the `cxx_qt::bridge`.

### Tests and evidence

- Rust unit tests (all passing, `cargo test --locked`, 31 total):
  - `i18n::tests` — every language translates core strings away from the
    source; exact es/ar table matches incl. More…-tile and dialog strings;
    power keys translate (`Suspender`/`السكون`, …) while keys stay put;
    missing/`en`/`de` catalogs return empty. These load the **embedded**
    `:/qt/qml/Waylight/i18n/*.qm` resources — proving the embedding and real
    `QTranslator` retranslation lookups for es and ar.
  - `backend::tests::config_paths_search_order_is_user_then_system` mirrors
    the theme test; `main.rs` tests cover JSON validation (unsupported, wrong
    type, non-object, malformed, extra keys), candidate decoding, ordered
    walk with tempdir files (invalid skipped, first valid wins, unsupported →
    English) and the 64 KiB read cap.
  - `main.rs` test also verifies the config helper shares the theme path
    order and escaping.
- QML: `qmltestrunner -input tests` now runs 44 tests (38 prior + 6 i18n),
  0 failures, 0 skipped.
- End-to-end smoke (offscreen preview, isolated `HOME`/`XDG_CONFIG_HOME`):
  `{"language": "es"}` starts with empty stderr (translator installed);
  `{"language": "de"}` and a malformed file start in English with empty
  stderr (fail-open).
- Full gate `sh tests/check.sh`: fmt --check, locked build, 31 Rust tests,
  clippy `-D warnings`, qmllint (incl. `tst_i18n.qml`), 44 QML tests, CLI
  isolation and all four headless Cage cases — **green**.
- qmllint notes: `Qt.application.layoutDirection` is flagged
  `missing-property` (qmllint's `QQmlApplication` typeinfo predates the
  property; it exists at runtime and qmllint exits 0) — same status as the
  pre-existing unused-import info in `tst_preview.qml`.

### Known limitations (documented, by design)

- Language changes require a restart (no hot reload), same policy as themes.
- Backend status sentences ("Sign in as X requested…", "Connecting…",
  "Cancellation requested…") are produced by the Rust controller/backend and
  are not part of A1's Main.qml scope; they remain English.
- `pt` uses Brazilian Portuguese wording (locked plan) with the file code
  `pt`; the `.ts` language attribute is `pt_BR`.
- Session names and PAM prompts are host data and intentionally untranslated.

### Follow-ups deferred to Deliverable B

- `waylight-config` language picker reuses `configPaths` and the same
  catalogs; `waylight-configd::SetLanguage` writes the same file format.

## Deliverable B — config utility (complete 2026-10-05)

### B1 — crate restructure + `waylight-configd`

- The crate is now `src/lib.rs` + three binaries: `src/main.rs`
  (`waylight-greeter`, behavior and CLI flags unchanged), `src/bin/
  waylight-configd.rs` and `src/bin/waylight-config.rs`. The cxx-qt QML
  module (greeter UI, Theme, translations, both QObject bridges) is built
  exactly once in the library; cxx-qt-build links its whole-archive
  initializer through the lib target, so every binary registers the module
  and embeds the resources. Proven at runtime: the greeter preview,
  the config GUI and the cargo test binaries all load `:/qt/qml/Waylight/…`
  resources, and the GUI with `waylight.json = {"language": "es"}` installs
  the translator (no fallback warning on stderr).
- The language resolution helpers (`LANGUAGES`, `validated_language`,
  `language_candidates`, `read_limited`, `resolve_language`,
  `install_language`) moved from `main.rs` into `src/i18n.rs` so the GUI and
  the daemon share them; the greeter binary calls `i18n::install_language`.
- `src/configd/` (headless, no Qt objects):
  - `validate.rs` — structural validation only: JSON object, ≤ 1 MiB,
    known top-level sections (`colors/background/font/layout` for theme,
    `language` for config; sections must be objects, `language` must be a
    string, code must be one of the 11 supported codes). Token-level
    validation stays fail-open in `Theme.qml` (documented in code; no
    duplicated validator). Unknown keys inside known sections round-trip.
  - `store.rs` — atomic writes to the injected `Paths` (system default
    `/etc/waylight/{theme,waylight}.json`): temp file in the target
    directory (pid + attempt counter, `create_new`), 0644, `sync_all`,
    `rename(2)`, best-effort directory fsync; `/etc/waylight` is created
    0755 only when missing and an existing directory is never
    re-permissioned. Reads fail open to empty.
  - `polkit.rs` — `Authorizer` trait (async via hand-rolled boxed futures,
    no new dependency) + production `Polkit` authorizer: one plain zbus
    proxy call to `org.freedesktop.PolicyKit1.Authority.CheckAuthorization`
    per mutating call, subject = the caller's **system-bus-name** (unique
    name from the message header, so polkit evaluates the real caller),
    action `dev.waylight.config.set`, flags = ALLOW_USER_INTERACTION,
    fail-closed on polkit errors.
  - `mod.rs` — `ConfigService<A: Authorizer>` with the `#[interface(name =
    "dev.waylight.Config1")]` impl: `GetAll() -> (s theme, s config,
    s theme_path, s config_path)` (read-only, no polkit), `SetTheme(s)`,
    `SetLanguage(s)` (authorize → validate → atomic write). D-Bus errors
    via `#[derive(DBusError)]` with prefix `dev.waylight.Config1.Error`
    (`Denied`, `InvalidTheme`, `InvalidLanguage`, `Io`). `run()` connects
    to the system bus, publishes the object, requests `dev.waylight.Config`
    and serves until SIGTERM/SIGINT.
- **Unit tests** (51 Rust tests total, was 31): validation matrix (known
  sections, unknown section/preset rejection, section object-ness, 1 MiB
  cap, config section, supported codes), atomic writes in tempdirs (0644
  file, 0755 created directory, existing directory untouched, replace
  existing content, failure leaves the target intact, no temp residue,
  fail-open reads), injected authorizers (allowed write lands on disk;
  **denied authorization refuses without writing anything — no directory is
  even created**; missing caller identity denied even with an allowing
  authorizer; invalid theme rejected before any write), and the locked
  GetAll/path shape.

### B2 — system files (`system/`)

- `dev.waylight.config.policy` — polkit action `dev.waylight.config.set`,
  `allow_active=auth_admin_keep`, `allow_inactive=no`, `allow_any=no`,
  English description/message.
- `dev.waylight.Config.conf` — D-Bus system policy: `root` may own
  `dev.waylight.Config`; the default context may send to it (authorization
  is polkit's job, per call).
- `dev.waylight.Config.service` — D-Bus activation: `Exec=/usr/bin/
  waylight-configd`, `User=root`, `SystemdService=waylight-configd.service`.
- `waylight-configd.service` — systemd `Type=dbus` unit with
  `BusName=dev.waylight.Config`, `Restart=on-failure` and cheap hardening
  (`NoNewPrivileges`, `PrivateTmp`, `ProtectHome`, kernel/control-group
  protections); D-Bus-activated, no static enablement required.

### B3 — `waylight-config` GUI

- `src/config_backend.rs` — `ConfigBackend` QObject (theme/config/
  themePath/configPath/status/message qproperties, `reload`/`applyTheme`/
  `applyLanguage`/`poll` invokables, `applied` signal) bridging QML to a
  worker thread that owns the zbus system-bus connection and a typed
  `Config1Proxy` client for the daemon (same pattern as the greeter's
  Backend: bounded channel pair, 16 ms QML poll Timer).
- `ConfigApp.qml` (entry: real ConfigBackend + poll Timer) and
  `ConfigMain.qml` (the editor window):
  - preset picker (dusk/midnight/daylight) that **expands** a preset into
    concrete tokens through the greeter's own `Theme.resetTo`, so the
    daemon never sees a `preset` key;
  - language picker with the 11 native names → `{"language": code}`;
  - structured editors with the plan's clamped ranges: accent (#hex +
    swatch), background mode + absolute image path, font family + scale
    (0.5–2.0), clockTopRatio/panelTopRatio sliders (0–0.9), panelMaxWidth/
    tileWidth/tileHeight/avatarSize SpinBoxes (1–2000); all other tokens
    load from the daemon and round-trip untouched;
  - raw JSON tab with the same structural checks the daemon applies
    (invalid JSON, unknown sections, non-object sections flagged; Apply
    refuses locally; switching tabs carries accepted raw edits back into
    the structured draft);
  - live preview: a second top-level window instantiating the greeter's
    own `Main.qml` (it is an ApplicationWindow, so a separate window is
    the correct embedding) with a mock backend (full property/function
    surface, `themePaths: "[]"`) and the draft applied in memory via the
    preview's internal `Theme` (`contentData` objectName `theme`) through
    `applyJson`; debounced 150 ms; previewing never writes;
  - Apply → `SetTheme` (+ `SetLanguage` when the picked language differs
    from the loaded one); Revert → `reload()` from the daemon; daemon
    replies (including validation errors) surface in the status line;
    after a successful Apply the state is re-synced from the daemon.
- The GUI resolves its UI language exactly like the greeter
  (`i18n::install_language` before engine load): the embedded preview and
  RTL mirroring follow `waylight.json`; the GUI's own chrome is
  `qsTr`-wrapped English source (no catalogs for the new contexts yet —
  documented limitation, translations can be added to the committed
  catalogs without code changes).

### B4 — tests

- `tests/tst_config.qml` (qmltestrunner, 10 tests at the time of Deliverable
  B — 11 after the corrective pass, which added the daemon-failure test;
  offscreen, mocked backend with the exact ConfigBackend surface): daemon load into editors
  and language sync, serialization emits only the four known sections,
  preset expansion without a `preset` key (and unknown preset → dusk),
  clamping parity with the greeter, structural-problem messages, Apply
  sends the serialized theme and the changed language (and re-syncs),
  raw-tab flow incl. local refusal of garbage, Revert reloads from the
  daemon, preview applies the draft in memory without touching the
  daemon, language table completeness.
- `tests/check.sh` now lints `ConfigApp.qml`, `ConfigMain.qml` and
  `tst_config.qml` and picks the **newest** module build directory for
  qmllint staging (stale build dirs from changed build scripts would
  otherwise shadow new types).
- Daemon "integration without polkit": covered by the injected-authorizer
  unit tests above (deny → refuse, nothing written). A live polkit prompt
  test is a manual post-install step (deferred to the user, as planned).

### B5 — packaging

- `packaging/PKGBUILD`: `pkgrel=3`; `depends` gains `polkit` (and `dbus`
  for the shipped dbus-1 files); `package()` installs all three binaries to
  `/usr/bin` and the four system files to their exact paths
  (`/usr/share/polkit-1/actions/dev.waylight.config.policy`,
  `/usr/share/dbus-1/system.d/dev.waylight.Config.conf`,
  `/usr/share/dbus-1/system-services/dev.waylight.Config.service`,
  `/usr/lib/systemd/system/waylight-configd.service`). Nothing is enabled
  or started at install time. `makepkg -f` verified: builds green (the
  C/CXXFLAGS `unset` workaround stays) and the package contains both new
  binaries and all four system files (see Gate below).

### B6 — docs

- README: new "Configuration utility" section (three binaries, GUI panes,
  polkit prompt behavior, exact file destinations and modes, restart-the-
  greeter note, service wiring) + THEMING.md pointer from the Theming
  section.
- THEMING.md: new "Editing with waylight-config" section documenting the
  preset-key asymmetry (greeter accepts `preset`, daemon stores expanded
  tokens only).
- This file extended; `.pi/CONFIG-TODO.md` B1–B6 + Gate checked.

### Tests and evidence (Deliverable B)

- Full gate `sh tests/check.sh` — **green** (exit 0): fmt --check, locked
  build (three binaries), 51 Rust tests, clippy `-D warnings`, qmllint on
  five module files + four test files, 56 QML tests, CLI isolation, four
  headless Cage cases. (Totals corrected in the corrective-pass section:
  the QML total was right, but the config share was 10 tests, not 12, and
  qmltestrunner's totals include each suite's init/cleanup — 48 test
  functions + 8.)
- Offscreen GUI smoke: `QT_QPA_PLATFORM=offscreen` run for 10 s with the
  packaged (uninstalled) daemon absent — zero stderr, UI stays up, status
  line carries the daemon error; with `XDG_CONFIG_HOME` pointing at a
  temp `waylight.json = {"language": "es"}` the translator installs from
  the embedded resources (no fallback warning), proving both binaries
  embed the QML module.
- `waylight-configd` on the host system bus without the packaged D-Bus
  policy: refused to own `dev.waylight.Config` (fail-closed, exit 1,
  `org.freedesktop.DBus.Error.AccessDenied`) and changed nothing — the
  bus policy file is required, exactly as designed.
- Package build: `makepkg -f` in `packaging/` (source pinned to the local
  commit, see Gate) produced `waylight-greeter-0.1.0-3-x86_64.pkg.tar.zst`;
  `pacman -Qlp` lists `usr/bin/waylight-{greeter,config,configd}` and all
  four system files. No live polkit/systemd/D-Bus activation test was run
  (no system changes); restart-the-greeter behavior is documented.

### Known limitations (documented, by design)

- The GUI's own strings are English (`qsTr`-wrapped); the committed
  catalogs cover the `Main` context only. The preview window is fully
  localized.
- The structured editors serialize exactly the known tokens into the four
  known sections: unknown keys *inside* a known section are dropped when a
  draft passes through the structured tab. The raw JSON tab round-trips
  them untouched, and the greeter ignores unknown keys either way.
- The D-Bus client has no method timeout: a hung `waylight-configd` leaves
  the GUI busy (Apply disabled). Revert is the recovery; a genuinely stuck
  daemon needs a systemctl restart outside the GUI.
- `GetAll` is unauthenticated by design (reads only what any local user
  can already read); every mutation is polkit-gated.
- The daemon binds `/etc/waylight` only; user-level config precedence
  (`$XDG_CONFIG_HOME`) is untouched and intentionally out of scope for the
  system utility (locked plan: "no user-session theme editing").
