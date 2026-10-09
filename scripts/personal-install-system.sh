#!/usr/bin/env bash
# Privileged part of the explicit daily installer: only the separate personal files.
set -euo pipefail
if [[ $EUID != 0 ]]; then
    printf 'This helper must be called by the authorized daily installer.\n' >&2
    exit 1
fi
system_script=$(readlink -f -- "${BASH_SOURCE[0]}")
system_source=$(cd -- "$(dirname -- "$system_script")/.." && pwd)
case ${1:-} in
    --install) binary="$system_source/../artifacts/niri-hdr-glass"; config_name=config.kdl ;;
    --install-smooth) binary="$system_source/../artifacts/niri-hdr-glass-smooth"; config_name=config-smooth.kdl ;;
    *) printf 'An explicit daily build selection is required.\n' >&2; exit 1 ;;
esac
if [[ $# != 2 || ! $2 =~ ^[0-9a-f]{64}$ ]]; then
    printf 'The accepted binary SHA-256 is required.\n' >&2
    exit 1
fi
accepted_sha=$2
install -d -m755 -- /usr/local/lib/niri-personal
# Rename a new inode rather than truncating a binary that may still be executing.
binary_temp=$(mktemp /usr/local/lib/niri-personal/.niri-XXXXXXXX)
service_temp=$(mktemp /usr/local/lib/niri-personal/.service-XXXXXXXX)
trap 'rm -f -- "$binary_temp" "$service_temp"' EXIT
install -m755 -- "$binary" "$binary_temp"
# Recheck the copied bytes after any authentication delay or concurrent rebuild.
read -r copied_sha _ < <(sha256sum -- "$binary_temp")
if [[ $copied_sha != "$accepted_sha" ]]; then
    printf 'Artifact changed after acceptance check; no installed files were replaced.\n' >&2
    exit 1
fi
mv -f -- "$binary_temp" /usr/local/lib/niri-personal/niri
install -Dm755 -- "$system_source/scripts/personal-daily-session.sh" /usr/local/bin/niri-personal-session
sed "s@%h/.config/niri-personal/config.kdl@%h/.config/niri-personal/$config_name@" \
    "$system_source/docs/personal/deployment/niri-personal.service" > "$service_temp"
install -Dm644 -- "$service_temp" /usr/local/lib/systemd/user/niri-personal.service
install -Dm644 -- "$system_source/docs/personal/deployment/niri-personal-shutdown.target" /usr/local/lib/systemd/user/niri-personal-shutdown.target
install -Dm644 -- "$system_source/docs/personal/deployment/niri-personal.desktop" /usr/share/wayland-sessions/niri-personal.desktop
