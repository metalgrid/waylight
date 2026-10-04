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
# Stale build directories can linger from older build-script hashes; prefer
# the newest plugin.qmltypes so qmllint sees all current module types.
module=""
for candidate in target/debug/build/waylight-greeter-*/out/qt-build-utils/qml_modules/Waylight; do
    if [ -f "$candidate/plugin.qmltypes" ]; then
        if [ -z "$module" ] || [ "$candidate/plugin.qmltypes" -nt "$module/plugin.qmltypes" ]; then
            module="$candidate"
        fi
    fi
done
if [ -n "$module" ]; then
    cp "$module/qmldir" "$module/plugin.qmltypes" "$lint/Waylight/"
fi
cp Main.qml App.qml Theme.qml ConfigApp.qml ConfigMain.qml "$lint/Waylight/"
/usr/lib/qt6/bin/qmllint -I "$lint" Main.qml App.qml Theme.qml ConfigApp.qml ConfigMain.qml tests/tst_preview.qml tests/tst_theme.qml tests/tst_i18n.qml tests/tst_config.qml
# QML tests capture only their own injected-backend window, never the desktop.
mkdir -p /tmp/waylight-login-ux-checks /tmp/waylight-transition-checks /tmp/waylight-theme-checks
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QML_XHR_ALLOW_FILE_READ=1 /usr/lib/qt6/bin/qmltestrunner -input tests
python tests/executable.py
