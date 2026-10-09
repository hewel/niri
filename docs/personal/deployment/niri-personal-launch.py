#!/usr/bin/python3
"""Stable session entry: pin one immutable bundle for the entire login."""
import hashlib
import json
import os
from pathlib import Path
import sys

STORE = Path("/usr/local/lib/niri-personal")

def legacy_binary(store):
    """Permit an old wrapper's in-flight handoff only to its pinned flat build."""
    pin = store / "legacy-session.json"
    binary = store / "niri"
    wrapper = store / "legacy-session.sh"
    if not pin.is_file() or not binary.is_file() or not wrapper.is_file():
        raise SystemExit("Missing personal build pin; launch through the login entry.")
    record = json.loads(pin.read_text())
    for path, field in ((binary, "binary_sha256"), (wrapper, "wrapper_sha256")):
        with path.open("rb") as source:
            actual = hashlib.file_digest(source, "sha256").hexdigest()
        if actual != record[field]:
            raise SystemExit("Retained legacy session changed; refusing an unpinned launch.")
    return binary


def main(store=STORE):
    selection = store / "selection.json"
    execute = sys.argv[1:] == ["--exec"]
    if execute and not os.environ.get("NIRI_PERSONAL_BUNDLE"):
        # A legacy wrapper can have passed its checks before bootstrap rewrites
        # ExecStart. It has no bundle environment, so finish with the exact flat
        # executable it checked, even after subsequent selection changes.
        binary = legacy_binary(store)
        os.execv(binary, [str(binary), "--session", "-c", str(Path.home() / ".config/niri/config.kdl")])
    if not selection.is_file():
        legacy_binary(store)
        wrapper = store / "legacy-session.sh"
        os.execv(wrapper, [str(wrapper), *sys.argv[1:]])

    if execute:
        bundle = Path(os.environ["NIRI_PERSONAL_BUNDLE"])
    else:
        identifier = json.loads(selection.read_text())["current"]
        bundle = store / "releases" / identifier
    if bundle.parent != store / "releases" or len(bundle.name) != 64 or any(c not in "0123456789abcdef" for c in bundle.name):
        raise SystemExit("Invalid personal session build selection.")
    os.environ["NIRI_PERSONAL_BUNDLE"] = str(bundle)
    if sys.argv[1:] == ["--exec"]:
        binary = bundle / "niri"
        os.execv(binary, [str(binary), "--session", "-c", str(Path.home() / ".config/niri/config.kdl")])
    os.execv(bundle / "session.sh", [str(bundle / "session.sh"), *sys.argv[1:]])


if __name__ == "__main__":
    main()
