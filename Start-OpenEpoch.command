#!/bin/bash
# Portable launcher: no game installation, compiler or package modules required.
set -u
case "$0" in
    */*) SCRIPT_PARENT="${0%/*}" ;;
    *) SCRIPT_PARENT=. ;;
esac
GAME_DIR="$(cd -P -- "$SCRIPT_PARENT" && pwd -P)" || exit 1
cd -- "$GAME_DIR" || exit 1

PYTHON_BIN=""
PATH_PYTHON="$(command -v python3 2>/dev/null || true)"
for candidate in "$PATH_PYTHON" /opt/homebrew/bin/python3 /usr/local/bin/python3 /usr/bin/python3; do
    [ -n "$candidate" ] && [ -x "$candidate" ] || continue
    # Do not trigger Apple's Command Line Tools installer via a missing shim.
    if [ "$candidate" = /usr/bin/python3 ] && [ "$(uname -s)" = Darwin ]; then
        /usr/bin/xcode-select -p >/dev/null 2>&1 || continue
    fi
    if "$candidate" -c 'import sys; raise SystemExit(sys.version_info < (3,8))' >/dev/null 2>&1; then
        PYTHON_BIN="$candidate"
        break
    fi
done

if [ -z "$PYTHON_BIN" ]; then
    printf '\nPython 3.8 oder neuer fehlt.\n'
    printf 'Installiere Python3 ueber https://www.python.org/downloads/macos/\n'
    printf 'Starte danach diese Datei erneut. Empire Earth II, Rust und Ghidra sind nicht erforderlich.\n\n'
    read -r -p 'Enter zum Schliessen ...' _ || true
    exit 1
fi

exec "$PYTHON_BIN" "$GAME_DIR/tools/serve_game.py" "$@"
