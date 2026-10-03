# THEME-VERIFICATION: Theme support for Waylight

> Status: **COMPLETE** — all gates pass. 2026-10-03. Corrective pass (architect review
> findings) applied and re-verified the same day, see "Corrective pass" below.
> Plan: `.pi/THEME-PLAN.md` (locked) · Tracker: `.pi/THEME-TODO.md` (6/6)

## Corrective pass (architect verdict: accepted with warnings) — 2026-10-03

### Findings → fixes

1. **(Warning) `background.image` was accepted verbatim** — any string (remote URLs,
   relative paths) landed in `backgroundImage`, but Main.qml can only render local files.
   Fix in `Theme.qml` `_assignToken` (string kind): the `backgroundImage` property accepts
   only strings starting with `/` and stores them normalized as `file://` + path (mirroring
   `loadFile`'s normalization); everything else — remote URLs, relative paths, non-strings,
   and the `""` default itself — resolves to the default `""`. Assigning `""` (instead of
   keeping the current value) is required for the default case because `resetTo` restores
   defaults *through* `applyMap`; a pure "keep" guard could never reset the token once set.
   `fontFamily` shares the string kind and stays unrestricted. Main.qml needs no change; it
   renders the validated value.
2. **(Warning) A readable 0-byte file lost the documented "first existing wins; broken
   content = defaults" rule**: the old acceptance condition (`status === 0 &&
   responseText !== ""`) used a non-empty body as the existence heuristic. Fix: acceptance
   is now `readyState 4 && (status === 200 || (status === 0 &&
   getAllResponseHeaders() !== ""))`; a readable 0-byte file wins and its empty content
   fails open to defaults via `applyJson("")` with the existing warning. **Deviation from
   the review text** ("accept readyState 4 + status 0 regardless of body length"): probed
   Qt 6.11 XHR behavior shows a *missing* file is indistinguishable by status/body alone
   (status 0, empty body, no headers) while an existing 0-byte file answers with
   `last-modified`/`content-length: 0` headers. Accepting every status-0 response would let
   the first *missing* path win the search order and shadow later candidates (e.g. a missing
   user config shadowing `/etc/waylight/theme.json`) — it broke the existing
   `test_theme_autoload_first_existing_path_wins` integration test. The headers check is the
   minimal condition that accepts 0-byte files "regardless of body length" while preserving
   "first existing wins".
3. **(Suggestion) No bound on the synchronous XHR.** Fix: `responseText` longer than
   1,000,000 chars is treated as unreadable (`console.warn`, return false, not applied).
   New fixture `tests/fixtures/theme-oversized.json` (valid JSON head + >1 MiB of
   insignificant padding) proves the cap — not the parser — rejects it, and that an
   oversized first candidate does not win the search order.
4. **(Suggestion) Unguarded call site.** Fix: `Main.qml` now calls
   `theme.loadFirstExisting(typeof backend.themePaths === "string" ? backend.themePaths :
   "[]")`.
5. **(Suggestion) Harness coupling risk.** Fix: comment in `tests/executable.py` at the
   Cage child env documenting that `QML_XHR_ALLOW_FILE_READ` is intentionally NOT set so
   headless runs stay deterministic and cannot couple to a developer's or `/etc/waylight/`
   theme file.
6. Tests added/extended in `tests/tst_theme.qml` (matching existing style):
   `test_background_image_is_local_absolute_only` (http/https/file-URL strings, relative
   paths and non-strings keep `""`; absolute path → `file://`; `fontFamily` unaffected),
   `test_zero_byte_file_wins_with_defaults` (new 0-byte fixture
   `tests/fixtures/theme-empty.json`; wins solo and in the search order),
   `test_oversized_file_is_unreadable`, `test_preset_then_explicit_keys_order` (preset
   `daylight` + explicit `colors.accent` → explicit accent wins while other daylight tokens
   survive), and non-array `loadFirstExisting` inputs (`"null"`, number, string, `{}`)
   return false. The existing background test was updated for `file://` normalization.
7. Docs: `THEMING.md` "Validation and limits" now states that `background.image` must be an
   absolute local path (stored as `file://`) and that theme files over 1 MiB are ignored.

### Commands run and results (corrective pass)

| Command | Result |
| --- | --- |
| `sh tests/check.sh` (fmt, build --locked, test --locked, clippy -D warnings, qmllint, qmltestrunner, executable.py) | **exit 0** — 23 Rust tests passed, clippy clean, qmllint clean, 38 QML tests passed (34 + 4 new), all 4 headless Cage cases PASS (ack=0, nonfatal-state=0, lost-start-reply=1, shutdown-prompt=1; Cage=0 each). Log: `/tmp/waylight-corrective-check.log` |

## What changed

### New files

- `Theme.qml` — `QtObject` (not a singleton), instantiated in `Main.qml` as `id: theme`
  (`objectName: "theme"`). 56 tokens (25 colors + 3 background colors, background mode/image,
  font family/scale, 23 layout sizes/ratios) plus `textCombo`. API: `applyJson(text)`,
  `loadFile(path)`, `loadFirstExisting(pathsJson)`, `resetTo(presetName)`, `scaled(size)`.
  Defaults are the single `defaultsMap`; presets (`dusk` = no overrides, `midnight`, `daylight`)
  are override maps applied onto the defaults and validated by the same rules as user JSON.
  Fail-open throughout: parse error → defaults + `console.warn`; unknown keys ignored; wrong
  type → that key's default; colors `#rgb|#rgba|#rrggbb|#aarrggbb` (normalized on input, since
  QColor has no `#rgba` form); ratios clamped 0–0.9, sizes 1–2000, `font.scale` 0.5–2.0;
  `background.mode` limited to `builtin|image|solid|gradient`. `loadFile` uses synchronous XHR
  (`file://` opt-in required, see below) and returns whether the file was readable;
  `loadFirstExisting` implements the plan's "first existing file wins" over `backend.themePaths`.
- `examples/theme.json` — fully annotated example; every `_`-prefixed key doubles as
  documentation of the unknown-keys-ignored rule. Script-checked: all real keys equal the locked
  defaults.
- `tests/tst_theme.qml` — 11 qmltestrunner tests: defaults regression against the locked
  literals (plus a defaultsMap↔property divergence sweep), valid JSON override, color-format
  validation, malformed/wrong-type fail-open, unknown preset/keys, clamping bounds, preset
  switching with WCAG contrast assertions (daylight textPrimary/textSecondary/textBar/textCombo
  vs its own background and popup ≥ 4.5:1), background modes/fallback, `loadFile` /
  `loadFirstExisting` fail-open against repo fixtures, and `scaled()`.
- `tests/fixtures/theme-valid.json`, `tests/fixtures/theme-malformed.json` — deterministic
  loader fixtures.

### Modified files

- `Main.qml` — every token-table color/size/position replaced with `theme.*` references;
  `font.pixelSize` values and the clock cap go through `theme.scaled()`; background renders as
  builtin/image/solid/gradient per `backgroundMode` with the themed overlay always above it;
  `Component.onCompleted` calls `theme.loadFirstExisting(backend.themePaths)` before presenting
  (guarded, so test fakes without `themePaths` are unaffected). No behavior, focus-order,
  accessibility, or state-machine changes — all pre-existing QML tests pass unmodified except
  for additions below.
- `build.rs` — `Theme.qml` added to the `Waylight` `QmlModule`. **`resources.qrc` intentionally
  unchanged**: existing QML files are embedded by the QmlModule (they load via
  `qrc:/qt/qml/Waylight/…`), and a second registration would duplicate the resource path.
- `src/backend.rs` — pure `theme_paths_json(xdg_config_home, home)` helper (env values as
  parameters, no env mutation) returning a serde_json array string:
  `${XDG_CONFIG_HOME:-$HOME/.config}/waylight/theme.json` (empty-string variables treated as
  unset; no user base → system path only), then `/etc/waylight/theme.json`. Exposed as
  `#[qproperty(QString, theme_paths, cxx_name = "themePaths")]`, computed once in `Default`.
  Unit test `theme_paths_search_order_is_user_then_system` covers XDG precedence, empty/unset
  fallbacks, system-only, and JSON escaping.
- `src/main.rs` — sets `QML_XHR_ALLOW_FILE_READ=1` next to the existing
  `QML_DISABLE_DISK_CACHE=1` (Qt 6.11 disables local-file XHR by default; needed to read the
  optional theme file; presentation-only, before threads spawn).
- `tests/tst_preview.qml` — fake backend gained `property string themePaths: "[]"` (inert for
  existing tests); new `test_theme_autoload_first_existing_path_wins` (full Main integration:
  missing path skipped, fixture applied, presets switchable at runtime) and
  `test_theme_preset_screenshots` (grabs dusk/midnight/daylight into
  `/tmp/waylight-theme-checks/`, fake backend, offscreen only).
- `tests/check.sh` — qmllint now covers `Theme.qml` and `tests/tst_theme.qml`;
  `qmltestrunner` runs with `QML_XHR_ALLOW_FILE_READ=1`; creates
  `/tmp/waylight-theme-checks`.
- `tests/executable.py` — child env now also isolates `XDG_CONFIG_HOME` (theme reading is a new
  filesystem input; harness must not observe a developer's real theme file).
- `README.md` — new "Theming" section (format, search order, presets, validation rules,
  `textCombo` deviation note, non-themed artwork, screenshot location) and a verification-section
  paragraph on the theme tests and the byte-identical check.

## Pixel-identity proof (beyond the gates)

A throwaway harness (`/tmp/waylight-pixel-check/`) rendered `git show HEAD:Main.qml` and the new
`Main.qml` (with `Theme.qml`) in the same offscreen software renderer with the clock frozen
identically in both and grabbed at 1100×720 **and** 640×580 (covering the default and
short-panel layout branches, with `username.activeFocus` asserted in both): the PNG grabs are
**byte-identical** (`cmp`). A first attempt exposed a harness artifact (two live windows fighting
over active focus), not a regression — fixed by grabbing one window at a time.

## Preset screenshots (preview-only, offscreen qmltestrunner)

`/tmp/waylight-theme-checks/preset-{dusk,midnight,daylight}.png` — visually reviewed:
dusk matches the stock look (see byte-identity above), midnight is a near-black blue variant
with dimmer fills, daylight is a light frosted gradient with dark, high-contrast text and a
readable session picker. Known limitation: the embedded white power-icon SVGs have low contrast
on daylight (artwork is fixed; hover/focus affordances remain themed) — documented in README.

## Commands run and results (final state)

| Command | Result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo build --locked` | success (pre-existing C++ header warnings only) |
| `cargo test --locked` | 23 passed, 0 failed (incl. `backend::tests::theme_paths_search_order_is_user_then_system`) |
| `cargo clippy --locked --all-targets -- -D warnings` | clean |
| `qmllint -I <staged module> Main.qml App.qml Theme.qml tests/tst_preview.qml tests/tst_theme.qml` | clean (exit 0) |
| `qmltestrunner -input tests` (offscreen, software, `QML_XHR_ALLOW_FILE_READ=1`) | 34 passed, 0 failed |
| `python tests/executable.py` | PASS CLI/QML-load isolation + all 4 headless Cage cases (ack=0, nonfatal-state=0, lost-start-reply=1, shutdown-prompt=1, Cage=0 each) |
| `sh tests/check.sh` (all of the above, `set -eu`) | **exit code 0** — full log: `/tmp/waylight-check-final.log` |

## Plan deviations (for parent review)

1. **`colors.textCombo`** added beyond the locked token table (default `#ecedf5` = the exact
   previous literal; session-picker text, 3 sites). The table omitted this literal, and the
   locked daylight requirement (≥4.5:1 body text) is unreachable while it stays hardwired.
   Default look is proven byte-identical. All other tokens follow the table exactly.
2. **`resources.qrc` not modified** — see `build.rs` above; "registered like existing files"
   means the QmlModule (the same way `Main.qml`/`App.qml` are registered).
3. **`src/main.rs` environment opt-in** `QML_XHR_ALLOW_FILE_READ=1` and **`tests/check.sh` /
   `tests/executable.py` harness adjustments** were required by Qt 6.11's default-off local-file
   XHR and by isolation of the new config-file input. No other environment mutation exists.
4. Synchronous XHR inside `Theme.loadFile` (single small local file at startup, keeping
  `loadFile` deterministic and testable; the async alternative would need polling in tests).

## Not done / out of scope (per plan)

- No hot reload, no theme picker UI, no network, no new dependencies, no `vm/` changes, greetd/
  PAM/SDDM untouched, no system changes, no installer. Real-mode theme behavior is identical to
  preview (same QML path); only offscreen/injected-backend and headless-Cage flows were run.
