#!/bin/sh
# Run from the project root. All execution is isolated; no real login or power.
set -eu
cargo fmt --check
cargo build --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
# Cargo embeds QML, but qmllint needs the source files next to its qmldir.
lint=$(mktemp -d /tmp/waylight-lint.XXXXXX)
trap 'rm -rf "$lint"' EXIT HUP INT TERM
mkdir -p "$lint/Waylight"
for module in target/debug/build/waylight-greeter-*/out/qt-build-utils/qml_modules/Waylight; do
    if [ -f "$module/plugin.qmltypes" ]; then
        cp "$module/qmldir" "$module/plugin.qmltypes" "$lint/Waylight/"
        break
    fi
done
cp Main.qml App.qml "$lint/Waylight/"
/usr/lib/qt6/bin/qmllint -I "$lint" Main.qml App.qml tests/tst_preview.qml
# QML tests capture only their own injected-backend window, never the desktop.
mkdir -p /tmp/waylight-login-ux-checks /tmp/waylight-transition-checks
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software /usr/lib/qt6/bin/qmltestrunner -input tests
python tests/executable.py
