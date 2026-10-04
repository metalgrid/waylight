#!/bin/sh
# Regenerate the Qt Linguist catalogs for the greeter's Main.qml strings and
# recompile the committed .qm files. Dev-time only: qt6-tools is NOT a build
# dependency, because the generated .ts and .qm files are committed.
set -eu
cd "$(dirname "$0")/.."

# The tools may be suffixed -6 (Arch qt6-tools) or plain (PATH or /usr/lib/qt6/bin).
find_tool() {
    for candidate in "$1-6" "$1" "/usr/lib/qt6/bin/$1-6" "/usr/lib/qt6/bin/$1"; do
        if command -v "$candidate" >/dev/null 2>&1; then
            command -v "$candidate"
            return 0
        fi
    done
    echo "regenerate.sh: $1 not found (install qt6-tools)" >&2
    return 1
}
LUPDATE=$(find_tool lupdate)
LRELEASE=$(find_tool lrelease)

LANGS="zh hi es fr ar bn pt ru ur bg"
CATALOGS="i18n/waylight.ts"
for lang in $LANGS; do
    CATALOGS="$CATALOGS i18n/waylight_$lang.ts"
done

# -no-obsolete drops strings that no longer exist in Main.qml; existing
# translations for unchanged sources are preserved.
"$LUPDATE" Main.qml -no-obsolete -ts $CATALOGS

# Fill still-unfinished messages from the reviewed table without ever
# overwriting an existing translation (a human translator wins).
python3 i18n/translations.py

STATUS=0
for lang in $LANGS; do
    "$LRELEASE" "i18n/waylight_$lang.ts" -qm "i18n/waylight_$lang.qm" || STATUS=1
done
exit "$STATUS"
