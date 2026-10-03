# TODO: Theme support

> Derived from: .pi/THEME-PLAN.md (locked)
> Last updated: 2026-10-03

## Progress: 6/6 completed

- [x] Add `Theme.qml` with all tokens, presets (dusk/midnight/daylight), `applyJson`, `loadFile`, fail-open validation.
- [x] Expose `themePaths` from Rust backend (XDG + /etc order) with a unit test for the pure helper.
- [x] Convert `Main.qml` to theme tokens only; register Theme.qml in build.rs/resources.qrc.
- [x] QML tests: defaults regression, valid override, malformed file fallback, preset switch, clamping.
- [x] README "Theming" section + annotated `examples/theme.json`.
- [x] Full gate: fmt/build/test/clippy/qmllint/qmltestrunner/Cage cases + three preset screenshots.

## Notes

- `resources.qrc` intentionally unchanged: like `Main.qml`/`App.qml`, `Theme.qml` is embedded via
  the `QmlModule` in `build.rs`; adding it to `resources.qrc` would duplicate the resource at the
  same `/qt/qml/Waylight` path.
- `colors.textCombo` (default `#ecedf5`, the session-picker text) is one token beyond the locked
  table: that literal was missing from the table, and daylight's locked ≥4.5:1 goal is unreachable
  while it stays hardwired. Default keeps pixel identity (proven byte-identical vs git HEAD).
- Qt 6 disables local-file XHR by default; the greeter opts in with `QML_XHR_ALLOW_FILE_READ=1`
  in `src/main.rs` (next to the existing `QML_DISABLE_DISK_CACHE`), and `tests/check.sh` sets it
  for `qmltestrunner`.
- `tests/executable.py` now isolates `XDG_CONFIG_HOME` for the greeter child processes so a
  developer's real theme file cannot affect harness results.
- Corrective pass (2026-10-03, architect accepted-with-warnings, all applied; gate re-run
  exit 0): `background.image` is local-absolute-only and normalized to `file://`; a readable
  0-byte theme file wins the search order (existence detected via response headers, since a
  missing file is indistinguishable by status/body alone); theme files over 1 MiB are
  treated as unreadable; `Main.qml` guards the `themePaths` type at the call site; the Cage
  harness deliberately leaves `QML_XHR_ALLOW_FILE_READ` unset (documented in
  `tests/executable.py`). Fixtures gained `theme-empty.json` and `theme-oversized.json`;
  `tests/tst_theme.qml` grew to 15 tests (38 total with previews).
