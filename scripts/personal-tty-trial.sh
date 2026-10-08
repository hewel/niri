#!/usr/bin/env bash
# Explicit SDR/HDR trial on a real Linux virtual console, without session setup.
set -euo pipefail
usage() {
    printf 'Usage: %s [sdr|hdr|video]\nDefault: sdr. Run after logging in on Ctrl+Alt+F3.\n' "$0"
}
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
    usage
    exit 0
fi
if [[ $# -gt 1 ]]; then
    usage >&2
    exit 1
fi
case ${1:-sdr} in
    sdr) config_name=glass.kdl ;;
    hdr) config_name=hdr-dp-1.kdl ;;
    video) config_name=hdr-dp-1.kdl ;;
    *) usage >&2; exit 1 ;;
esac
trial_console=$(tty || true)
if [[ ! "$trial_console" =~ ^/dev/tty[0-9]+$ ]]; then
    printf 'Run this script on a real TTY: Ctrl+Alt+F3, then log in.\n' >&2
    exit 1
fi
# Resolve the short local symlink as well as direct calls to this script.
trial_script=$(readlink -f -- "${BASH_SOURCE[0]}")
trial_source=$(cd -- "$(dirname -- "$trial_script")/.." && pwd)
trial_binary="$trial_source/../artifacts/niri-hdr-glass"
if [[ ! -x "$trial_binary" ]]; then
    trial_binary="$trial_source/target/release/niri"
fi
if [[ ${1:-sdr} == video ]]; then
    exec python3 "$trial_source/scripts/personal-hdr-video.py" --binary "$trial_binary"
fi
if [[ ! -x "$trial_binary" ]]; then
    printf 'No trial binary found. Build with scripts/personal-build.sh first.\n' >&2
    exit 1
fi
printf 'Trial: %s\nBinary: %s\nExit: Super+Shift+E; return to the original desktop: Ctrl+Alt+F2.\n' \
    "${1:-sdr}" "$trial_binary"
exec env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
    -u NIRI_SOCKET -u NOTIFY_SOCKET \
    dbus-run-session "$trial_binary" \
    -c "$trial_source/examples/personal/$config_name" \
    -- python3 "$trial_source/examples/personal/demo.py"
