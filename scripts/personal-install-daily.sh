#!/usr/bin/env bash
# Apply the reviewed daily entry only after a recorded human HDR video acceptance.
set -euo pipefail
install_script=$(readlink -f -- "${BASH_SOURCE[0]}")
install_source=$(cd -- "$(dirname -- "$install_script")/.." && pwd)
personal_root="$install_source/.."
daily_stage="$personal_root/daily"
if [[ ${1:-} != --install || $# != 1 ]]; then
    printf 'After the HDR video trial passes: %s --install\n' "$0"
    exit 1
fi
trial_binary="$personal_root/artifacts/niri-hdr-glass"
python3 - "$personal_root/validation/hdr-video/acceptance.json" "$trial_binary" <<'PY'
import hashlib
import json
import sys
from pathlib import Path
path = Path(sys.argv[1])
if not path.is_file():
    raise SystemExit('HDR video acceptance has not been recorded; daily setup remains staged.')
record = json.loads(path.read_text())
if record.get('result') != 'passed' or record.get('source') != 'user-observed':
    raise SystemExit('A user-observed HDR video pass is required before daily setup.')
if record.get('binary_sha256') != hashlib.sha256(Path(sys.argv[2]).read_bytes()).hexdigest():
    raise SystemExit('The accepted binary does not match this artifact; verify the selected build first.')
PY
"$trial_binary" validate -c "$daily_stage/config.kdl"
if sudo -n true 2>/dev/null; then
    install_privileges=(sudo -n)
elif command -v pkexec >/dev/null; then
    install_privileges=(pkexec)
else
    printf 'Administrator authentication is required; run this installer in a terminal with sudo available.\n' >&2
    exit 1
fi
backup=$(mktemp -d "$daily_stage/backup-XXXXXXXX")
cp -a -- "$HOME/.config/niri" "$backup/original-niri-config"
for path in "$HOME/.config/niri-personal/config.kdl" \
    /usr/local/lib/niri-personal/niri /usr/local/bin/niri-personal-session \
    /usr/local/lib/systemd/user/niri-personal.service \
    /usr/local/lib/systemd/user/niri-personal-shutdown.target \
    /usr/share/wayland-sessions/niri-personal.desktop; do
    if [[ -e "$path" || -L "$path" ]]; then
        mkdir -p -- "$backup/previous$(dirname -- "$path")"
        cp -a -- "$path" "$backup/previous$path"
    fi
done
"${install_privileges[@]}" /usr/bin/bash "$install_source/scripts/personal-install-system.sh"
mkdir -p -- "$HOME/.config/niri-personal"
install -m 644 -- "$daily_stage/config.kdl" "$HOME/.config/niri-personal/config.kdl"
systemctl --user daemon-reload
printf 'Installed Niri HDR + Glass. Backup: %s\nChoose it in GDM at your next login; the original Niri entry remains available.\n' "$backup"
