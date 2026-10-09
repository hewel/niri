#!/usr/bin/env bash
# Apply the reviewed daily entry only after a recorded human HDR video acceptance.
set -euo pipefail
install_script=$(readlink -f -- "${BASH_SOURCE[0]}")
install_source=$(cd -- "$(dirname -- "$install_script")/.." && pwd)
personal_root="$install_source/.."
daily_stage="$personal_root/daily"
if [[ $# != 1 || ( ${1:-} != --install && ${1:-} != --install-smooth && ${1:-} != --install-unified ) ]]; then
    printf 'After acceptance: %s --install | --install-smooth | --install-unified\n' "$0"
    exit 1
fi
install_mode=$1
trial_binary="$personal_root/artifacts/niri-hdr-glass"
acceptance="$personal_root/validation/hdr-video/acceptance.json"
candidate_config="$daily_stage/config.kdl"
installed_config="$HOME/.config/niri-personal/config.kdl"
if [[ $install_mode == --install-smooth || $install_mode == --install-unified ]]; then
    trial_binary="$personal_root/artifacts/niri-hdr-glass-smooth"
    acceptance="$personal_root/validation/corner-aa/daily-acceptance.json"
    candidate_config="$daily_stage/config-smooth.kdl"
    installed_config="$HOME/.config/niri-personal/config-smooth.kdl"
    if [[ $install_mode == --install-unified ]]; then
        candidate_config="$daily_stage/unified/candidate/config.kdl"
        if [[ ! -f "$daily_stage/unified/manifest.json" || -e "$HOME/.config/niri/.pending-personal" ]]; then
            printf 'Prepare a new unified migration; an existing pending directory must be preserved/reviewed first.\n' >&2
            exit 1
        fi
    fi
    # The overlay includes the existing personal config, preserving local output/rule edits.
    if [[ ! -f "$HOME/.config/niri-personal/config.kdl" ]]; then
        printf 'Install the original accepted HDR/Glass daily entry before this upgrade.\n' >&2
        exit 1
    fi
    personal_state=$(systemctl --user show -p ActiveState --value niri-personal.service)
    if [[ $personal_state != inactive && $personal_state != failed ]]; then
        printf 'Log out of the personal session before upgrading it; current state: %s\n' "$personal_state" >&2
        exit 1
    fi
fi
accepted_sha=$(python3 - "$acceptance" "$trial_binary" <<'PY'
import hashlib
import json
import sys
from pathlib import Path
path = Path(sys.argv[1])
if not path.is_file():
    raise SystemExit('Acceptance has not been recorded; daily setup remains staged.')
record = json.loads(path.read_text())
if record.get('result') != 'passed' or record.get('source') != 'user-observed':
    raise SystemExit('A user-observed pass is required before daily setup.')
sha = hashlib.sha256(Path(sys.argv[2]).read_bytes()).hexdigest()
if record.get('binary_sha256') != sha:
    raise SystemExit('The accepted binary does not match this artifact; verify the selected build first.')
print(sha)
PY
)
"$trial_binary" validate -c "$candidate_config"
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
for path in "$HOME/.config/niri-personal/config.kdl" "$HOME/.config/niri-personal/config-smooth.kdl" \
    /usr/local/lib/niri-personal/niri /usr/local/bin/niri-personal-session \
    /usr/local/lib/niri-personal/unify-config.py \
    /usr/local/lib/systemd/user/niri-personal.service \
    /usr/local/lib/systemd/user/niri-personal-shutdown.target \
    /usr/share/wayland-sessions/niri-personal.desktop; do
    if [[ -e "$path" || -L "$path" ]]; then
        mkdir -p -- "$backup/previous$(dirname -- "$path")"
        cp -a -- "$path" "$backup/previous$path"
    fi
done
# Install the new overlay before switching the service to its path. The original
# config watched by an older compositor is never changed during a smooth upgrade.
if [[ $install_mode == --install-unified ]]; then
    # The active stock compositor still reads its original config. Apply only
    # after the session wrapper's no-running-desktop check, on the next login.
    cp -a -- "$daily_stage/unified" "$HOME/.config/niri/.pending-personal"
else
    mkdir -p -- "$HOME/.config/niri-personal"
    install -m 644 -- "$candidate_config" "$installed_config"
fi
"${install_privileges[@]}" /usr/bin/bash "$install_source/scripts/personal-install-system.sh" "$install_mode" "$accepted_sha"
systemctl --user daemon-reload
printf 'Installed Niri HDR + Glass. Backup: %s\nChoose it in GDM at your next login; the original Niri entry remains available.\n' "$backup"
