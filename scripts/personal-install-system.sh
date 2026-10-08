#!/usr/bin/env bash
# Privileged part of the explicit daily installer: only the separate personal files.
set -euo pipefail
if [[ $EUID != 0 ]]; then
    printf 'This helper must be called by the authorized daily installer.\n' >&2
    exit 1
fi
system_script=$(readlink -f -- "${BASH_SOURCE[0]}")
system_source=$(cd -- "$(dirname -- "$system_script")/.." && pwd)
install -Dm755 -- "$system_source/../artifacts/niri-hdr-glass" /usr/local/lib/niri-personal/niri
install -Dm755 -- "$system_source/scripts/personal-daily-session.sh" /usr/local/bin/niri-personal-session
install -Dm644 -- "$system_source/docs/personal/deployment/niri-personal.service" /usr/local/lib/systemd/user/niri-personal.service
install -Dm644 -- "$system_source/docs/personal/deployment/niri-personal-shutdown.target" /usr/local/lib/systemd/user/niri-personal-shutdown.target
install -Dm644 -- "$system_source/docs/personal/deployment/niri-personal.desktop" /usr/share/wayland-sessions/niri-personal.desktop
