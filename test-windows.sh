#!/usr/bin/env bash
# Runs the Windows version's tests under Wine, from Linux:
#   1. both test suites (app and installer), compiled for Windows;
#   2. the real installer: install, update, refuse while the app is open,
#      uninstall, and the leftovers check.
#
# Needs the same tools as build-windows.sh, plus Wine. Run build-windows.sh
# first so dist/ holds the installer to test.
#
# Wine gets its own prefix in target/wine-test, with its Desktop/Documents
# folders unlinked from yours and menu integration off, so nothing appears
# on your real desktop or in your app menu.
set -euo pipefail
cd "$(dirname "$0")"

TARGET=x86_64-pc-windows-gnu
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
SETUP="dist/Countdown-Setup-$VERSION.exe"
export WINEPREFIX="$PWD/target/wine-test"
export WINEDEBUG=-all
export WINEDLLOVERRIDES="mscoree,mshtml=;winemenubuilder.exe=d"
FAILED=0
check() { if eval "$2"; then echo "  ok    $1"; else echo "  FAIL  $1"; FAILED=1; fi; }

if [ ! -d "$WINEPREFIX/drive_c" ]; then
    echo "Creating the test Wine prefix…"
    wineboot -i >/dev/null 2>&1
    wineserver -w
    for f in "$WINEPREFIX"/drive_c/users/"$USER"/*; do
        if [ -L "$f" ]; then rm "$f" && mkdir "$f"; fi
    done
fi
U="$WINEPREFIX/drive_c/users/$USER"
PROGRAMS="$U/AppData/Local/Programs/Countdown"
START_MENU="$U/AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Countdown.lnk"
UNINSTALL_KEY='HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Countdown'
APP_ID_KEY='HKCU\Software\Classes\AppUserModelId\Iarimanana.Countdown'

echo "== Building the test programs for Windows"
cargo zigbuild --target "$TARGET" --workspace --tests --message-format=json-render-diagnostics 2>/dev/null \
    | python3 -I -c '
import json, sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m["profile"]["test"]:
        print(m["target"]["name"], m["executable"])' > target/windows-tests.txt

echo "== Test suites under Wine"
while read -r name exe; do
    # Wine's DirectWrite crashes when many threads lay out text at once, so
    # the UI tests run one at a time. (Real Windows doesn't have this problem,
    # and the app itself draws text on a single thread.)
    if wine "$exe" --test-threads=1 >"target/windows-tests-$name.log" 2>&1; then
        echo "  ok    $name: $(grep 'test result' "target/windows-tests-$name.log")"
    else
        echo "  FAIL  $name (see target/windows-tests-$name.log)"
        FAILED=1
    fi
done < target/windows-tests.txt

echo "== Real installer: $SETUP"
check "installs silently" "wine '$SETUP' --silent"
check "app installed" "[ -f '$PROGRAMS/Countdown.exe' ] && [ -f '$PROGRAMS/uninstall.exe' ] && [ -f '$PROGRAMS/countdown.ico' ]"
check "Start menu and desktop shortcuts" "[ -f '$START_MENU' ] && [ -f '$U/Desktop/Countdown.lnk' ]"
check "listed in Apps & features" "wine reg query '$UNINSTALL_KEY' >/dev/null 2>&1"
check "notifications registered as Countdown" "wine reg query '$APP_ID_KEY' /v DisplayName 2>/dev/null | grep -q Countdown"
check "updates over itself" "wine '$SETUP' --silent"

wine "$PROGRAMS/Countdown.exe" >/dev/null 2>&1 &
timeout 10 tail -f /dev/null || true
check "app opens and keeps running" "pgrep -f 'Countdown.exe' >/dev/null"
check "update refused while the app is open" "! wine '$SETUP' --silent 2>/dev/null"
wineserver -k || true

check "uninstalls silently" "wine '$PROGRAMS/uninstall.exe' --uninstall --silent"
timeout 6 tail -f /dev/null || true
wineserver -w
check "install folder removed" "[ ! -d '$PROGRAMS' ]"
check "shortcuts removed" "[ ! -f '$START_MENU' ] && [ ! -f '$U/Desktop/Countdown.lnk' ]"
check "Apps & features entry removed" "! wine reg query '$UNINSTALL_KEY' >/dev/null 2>&1"
check "notification registration removed" "! wine reg query '$APP_ID_KEY' >/dev/null 2>&1"

if [ "$FAILED" = 0 ]; then echo "All Windows checks passed."; else echo "Some Windows checks FAILED."; exit 1; fi
