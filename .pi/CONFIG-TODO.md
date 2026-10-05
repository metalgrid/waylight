# TODO: Config utility + localization

> Derived from: .pi/CONFIG-PLAN.md (locked)
> Last updated: 2026-10-05 (corrective pass)

## Progress: 9/10 completed + corrective pass complete

- [x] A1: qsTr() all user-visible strings in Main.qml; i18n/regenerate.sh; commit .ts + .qm for zh, hi, es, fr, ar, bn, pt, ru, ur, bg; embed .qm via resources.qrc.
- [x] A2: RTL support (LayoutMirroring) + QML tests (source-coverage, mirroring).
- [x] A3: waylight.json language file — Rust loader in main.rs (search order, fail-open), translator + layout direction installed before engine load; Rust unit tests.
- [x] B1: waylight-configd — zbus system service, GetAll/SetTheme/SetLanguage, atomic writes, structural validation, injected authorizer + unit tests.
- [x] B2: polkit action, D-Bus conf + activation, systemd unit files.
- [x] B3: waylight-config GUI — preset/language/token editors, raw JSON tab, embedded live preview, Apply via daemon.
- [x] B4: GUI qmltestrunner tests + daemon integration test (daemon refused without polkit auth when authorizer denies).
- [x] B5: PKGBUILD — second/third binaries, polkit/dbus/systemd files, polkit dep, pkgrel bump; package builds.
- [x] B6: README/THEMING.md docs (config file, utility usage, polkit behavior).
- [x] Gate: full check.sh + package build + .pi/CONFIG-VERIFICATION.md.

## Corrective pass (2026-10-05, uncommitted working tree on f4b7563)

- [x] B1: GUI surfaces daemon failures (status label clears "Applying…" on terminal daemon status; tst_config daemon-failure test).
- [x] B2: store.rs temp file unlinked on write_all/sync_all failure (+ /dev/full residue test).
- [x] B3: daemon JSON cap aligned with the greeter at 1,000,000 bytes (+ exact-boundary test).
- [x] B4: size-capped store reads (1,000,000 bytes, oversized → empty, + test).
- [x] B5: waylight-configd.service hardened (User=root, ProtectSystem=strict, ReadWritePaths=-/etc/waylight, RestrictAddressFamilies=AF_UNIX).
- [x] B6: previewBackend declares signal identityChosen(string).
- [x] B7: validate::config documented as the hand-written waylight.json contract.
- [x] B8: CONFIG-VERIFICATION arithmetic corrected (real totals) + missing limitations documented (structured-tab unknown keys, no D-Bus timeout).
- [x] A9: uiLanguage backend property + clock formatted via toLocaleString(Qt.locale(uiLanguage)) at both call sites; preview/test fakes updated; qmltestrunner locale test.
- [x] A10: WAYLIGHT_SKIP_SYSTEM_CONFIG=1 test kill switch on both path helpers (pure flag parameter), set by tests/executable.py; stale XHR comment fixed.
- [x] A11: RTL chevron mirroring + inverted tile Left/Right keys (+ test); fr NBSP before "?"; bg Continue «Продължи»; .ts fixed + .qm re-released; Rust locks.
- [x] A12: README/THEMING — waylight.json format, restart-required policy, no-D-Bus-timeout posture, structured-tab unknown-keys note, hardened unit documented.
- [x] Packaging: pkgver 0.2.0, pkgrel 1; local build verified from a working-tree snapshot; maintainer must `git tag v0.2.0 && git push origin v0.2.0` after review.
- [x] Gate: `sh tests/check.sh` exit 0 — 56 Rust tests, 59 QML tests (51 test functions + per-suite init/cleanup), 4 Cage cases.
