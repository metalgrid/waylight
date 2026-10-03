# TODO: Theme support

> Derived from: .pi/THEME-PLAN.md (locked)
> Last updated: 2026-10-03

## Progress: 0/6 completed

- [ ] Add `Theme.qml` with all tokens, presets (dusk/midnight/daylight), `applyJson`, `loadFile`, fail-open validation.
- [ ] Expose `themePaths` from Rust backend (XDG + /etc order) with a unit test for the pure helper.
- [ ] Convert `Main.qml` to theme tokens only; register Theme.qml in build.rs/resources.qrc.
- [ ] QML tests: defaults regression, valid override, malformed file fallback, preset switch, clamping.
- [ ] README "Theming" section + annotated `examples/theme.json`.
- [ ] Full gate: fmt/build/test/clippy/qmllint/qmltestrunner/Cage cases + three preset screenshots.
