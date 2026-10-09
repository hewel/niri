#!/usr/bin/env bash
# Privileged adapter has fixed destination paths, selected by the deployment module.
set -euo pipefail
if [[ $EUID != 0 || $# != 2 ]]; then
    printf 'Use personal-install-daily.sh install or rollback.\n' >&2
    exit 1
fi
system_script=$(readlink -f -- "${BASH_SOURCE[0]}")
exec /usr/bin/python3 "$(dirname -- "$system_script")/personal-deploy.py" _publish "$1" "$2"
