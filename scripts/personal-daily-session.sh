#!/usr/bin/env bash
# Separate production session. Never start alongside an existing graphical session.
set -euo pipefail
if [[ ${1:-} != --login && ${1:-} != --check ]]; then
    if [[ -n ${SHELL:-} ]] && rg --fixed-strings --line-regexp --quiet "$SHELL" /etc/shells; then
        exec "$SHELL" -l -c 'exec "$NIRI_PERSONAL_BUNDLE/session.sh" --login'
    fi
fi
session_states=$(systemctl --user show -p ActiveState --value \
    niri.service niri-personal.service graphical-session.target)
if [[ $session_states =~ (^|$'\n')(active|activating|reloading)($|$'\n') ]]; then
    printf 'A graphical session is already running. Select Niri HDR + Glass after logging out.\n' >&2
    exit 1
fi
if [[ ${1:-} == --check ]]; then
    printf 'No conflicting graphical session.\n'
    exit 0
fi
: "${NIRI_PERSONAL_BUNDLE:?Start this session through niri-personal-session}"
python3 "$NIRI_PERSONAL_BUNDLE/unify-config.py" apply --binary "$NIRI_PERSONAL_BUNDLE/niri"
if [[ ! -f "$HOME/.config/niri/config.kdl" ]]; then
    printf 'Missing configuration: %s\n' "$HOME/.config/niri/config.kdl" >&2
    exit 1
fi
"$NIRI_PERSONAL_BUNDLE/niri" validate -c "$HOME/.config/niri/config.kdl"
session_import=()
for name in DISPLAY WAYLAND_DISPLAY XDG_SEAT XDG_SESSION_ID XDG_SESSION_TYPE \
    XDG_SESSION_CLASS XDG_VTNR XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP XDG_DATA_DIRS PATH NIRI_PERSONAL_BUNDLE; do
    if [[ -v $name ]]; then
        session_import+=("$name")
    fi
done
systemctl --user import-environment "${session_import[@]}"
session_status=0
systemctl --user --wait start niri-personal.service || session_status=$?
systemctl --user start --job-mode=replace-irreversibly niri-personal-shutdown.target
systemctl --user unset-environment WAYLAND_DISPLAY DISPLAY XDG_SESSION_TYPE XDG_CURRENT_DESKTOP NIRI_SOCKET NIRI_PERSONAL_BUNDLE
exit "$session_status"
