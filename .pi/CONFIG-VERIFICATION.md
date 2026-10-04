# CONFIG-VERIFICATION

> Evidence for `.pi/CONFIG-PLAN.md`. Gate: full `sh tests/check.sh` green after
> every deliverable. Deliverable B is not started; greetd/PAM/SDDM, `/etc`, the
> package files and `vm/` are untouched. No new runtime or build dependency:
> Qt Linguist tools (`lupdate6`/`lrelease6`) are dev-time only and the catalogs
> (`.ts` + `.qm`) are committed.

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
