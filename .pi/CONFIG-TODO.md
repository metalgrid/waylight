# TODO: Config utility + localization

> Derived from: .pi/CONFIG-PLAN.md (locked)
> Last updated: 2026-10-04

## Progress: 3/10 completed

- [x] A1: qsTr() all user-visible strings in Main.qml; i18n/regenerate.sh; commit .ts + .qm for zh, hi, es, fr, ar, bn, pt, ru, ur, bg; embed .qm via resources.qrc.
- [x] A2: RTL support (LayoutMirroring) + QML tests (source-coverage, mirroring).
- [x] A3: waylight.json language file — Rust loader in main.rs (search order, fail-open), translator + layout direction installed before engine load; Rust unit tests.
- [ ] B1: waylight-configd — zbus system service, GetAll/SetTheme/SetLanguage, atomic writes, structural validation, injected authorizer + unit tests.
- [ ] B2: polkit action, D-Bus conf + activation, systemd unit files.
- [ ] B3: waylight-config GUI — preset/language/token editors, raw JSON tab, embedded live preview, Apply via daemon.
- [ ] B4: GUI qmltestrunner tests + daemon integration test (daemon refused without polkit auth when authorizer denies).
- [ ] B5: PKGBUILD — second/third binaries, polkit/dbus/systemd files, polkit dep, pkgrel bump; package builds.
- [ ] B6: README/THEMING.md docs (config file, utility usage, polkit behavior).
- [ ] Gate: full check.sh + package build + .pi/CONFIG-VERIFICATION.md.
