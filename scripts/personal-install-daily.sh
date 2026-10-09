#!/usr/bin/env bash
# One deployment module owns install, retry, rollback and status.
set -euo pipefail
install_script=$(readlink -f -- "${BASH_SOURCE[0]}")
exec python3 "$(dirname -- "$install_script")/personal-deploy.py" "$@"
