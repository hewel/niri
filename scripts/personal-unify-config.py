#!/usr/bin/env python3
"""Prepare/apply a checked, one-time migration to the host's existing niri config."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


CHANGED = ("cfg/outputs.kdl", "cfg/layout.kdl", "cfg/rules.kdl")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate(binary, config):
    subprocess.run([str(binary), "validate", "-c", str(config)], check=True)


def prepare(config, stage, binary):
    stage = stage.resolve()
    if stage.is_relative_to(config) and stage != config / ".pending-personal":
        raise SystemExit("Use the default .pending-personal stage or a directory outside the config tree.")
    if stage.exists():
        raise SystemExit(f"Stage already exists; preserve/review it first: {stage}")
    stage.mkdir(parents=True)
    shutil.copytree(config, stage / "original", ignore=shutil.ignore_patterns(".pending-personal"))
    shutil.copytree(stage / "original", stage / "candidate")
    candidate = stage / "candidate"
    outputs = candidate / CHANGED[0]
    text = outputs.read_text()
    # These exact host output blocks are intentionally reviewed, not a generic KDL editor.
    for connector in ("DP-1", "HDMI-A-1"):
        start = text.index(f'output "{connector}" {{')
        end = text.index("\n}", start)
        if "hdr " in text[start:end]:
            raise SystemExit(f"HDR settings already exist for {connector}; review them manually.")
        text = text[:end] + '\n    hdr mode="auto"' + text[end:]
    outputs.write_text(text)
    layout = candidate / CHANGED[1]
    text = layout.read_text()
    old = "    geometry-corner-radius 12\n"
    if text.count(old) != 1:
        raise SystemExit("Expected one global window radius 12; review the host layout before migration.")
    layout.write_text(text.replace(old, "    geometry-corner-radius 12 smoothing=0.65\n"))
    rules = candidate / CHANGED[2]
    text = rules.read_text()
    old = "    background-effect {\n        blur true\n    }"
    if text.count(old) != 5:
        raise SystemExit("Expected five existing blur-only rule groups; review the host rules before migration.")
    glass = '''    background-effect {
        blur true
        xray true
        liquid-glass {
            refraction-strength 3
            power-factor 10
            refraction-power 1
            physical-refraction 0
            lens-distortion 0
            glow-weight 0.0001
            edge-lighting 0.2
            fringing 0
            saturation 0.9
            vibrancy 0.2
            adaptive-dim 0.2
            adaptive-boost 0.2
        }
    }'''
    rules.write_text(text.replace(old, glass))
    validate(binary, candidate / "config.kdl")
    files = {str(p.relative_to(stage / "original")): digest(p)
             for p in (stage / "original").rglob("*") if p.is_file()}
    record = {"state": "prepared", "files": files,
              "changes": {name: digest(candidate / name) for name in CHANGED}}
    write_record(stage / "manifest.json", record)
    print(f"Prepared canonical config migration: {stage}")


def replace_file(source, destination):
    # Keep the old file available until its replacement is fully copied.
    fd, temporary = tempfile.mkstemp(prefix=".niri-migrate-", dir=destination.parent)
    os.close(fd)
    try:
        shutil.copy2(source, temporary)
        os.replace(temporary, destination)
    finally:
        Path(temporary).unlink(missing_ok=True)


def write_record(path, record):
    fd, temporary = tempfile.mkstemp(prefix=".manifest-", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as output:
            output.write(json.dumps(record, indent=2) + "\n")
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def apply(config, stage, binary):
    manifest = stage / "manifest.json"
    if not manifest.exists():
        return
    record = json.loads(manifest.read_text())
    if record["state"] == "applied":
        return
    for name, sha in record["changes"].items():
        if name not in CHANGED or digest(stage / "candidate" / name) != sha:
            raise SystemExit("Prepared changes were altered; no files changed.")
    if set(record["changes"]) != set(CHANGED):
        raise SystemExit("Incomplete prepared migration; no files changed.")
    for name in CHANGED:
        if digest(stage / "original" / name) != record["files"][name]:
            raise SystemExit("Original backup was altered; no files changed.")
    if record["state"] == "applying":
        # Recover an interrupted write only if the files still contain our known
        # old/new bytes. Never roll back edits made after that interruption.
        for name in CHANGED:
            if digest(config / name) not in (record["files"][name], record["changes"][name]):
                raise SystemExit("Interrupted migration followed by user edits; manual recovery required.")
        for name in CHANGED:
            replace_file(stage / "original" / name, config / name)
        record["state"] = "prepared"
        write_record(manifest, record)
    if record["state"] != "prepared":
        raise SystemExit("Unknown migration state; no files changed.")

    def check_current():
        stale = [name for name, sha in record["files"].items()
                 if not (config / name).is_file() or digest(config / name) != sha]
        if stale:
            raise SystemExit("Config edited after preparation; no files changed. Reprepare: " + ", ".join(stale))

    check_current()
    validate(binary, stage / "candidate/config.kdl")
    check_current()
    changed = []
    record["state"] = "applying"
    write_record(manifest, record)
    try:
        for name in CHANGED:
            if digest(config / name) != record["files"][name]:
                raise SystemExit(f"Config edited during migration: {name}; prior writes will be rolled back.")
            # Register before replacement so interruption immediately after
            # os.replace cannot leave an untracked partial write.
            changed.append(name)
            replace_file(stage / "candidate" / name, config / name)
        validate(binary, config / "config.kdl")
    except BaseException:
        for name in changed:
            if digest(config / name) not in (record["files"][name], record["changes"][name]):
                raise SystemExit("Migration failed followed by user edits; files preserved, manual recovery required.")
        for name in reversed(changed):
            replace_file(stage / "original" / name, config / name)
        record["state"] = "prepared"
        write_record(manifest, record)
        raise
    # Once the valid config has been committed, a journal-write interruption
    # must not trigger rollback after an "applied" record became visible.
    record["state"] = "applied"
    write_record(manifest, record)
    print(f"Canonical niri config enabled; originals preserved in {stage / 'original'}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "apply"))
    parser.add_argument("--config-dir", type=Path, default=Path.home() / ".config/niri")
    parser.add_argument("--stage-dir", type=Path)
    parser.add_argument("--binary", type=Path, required=True,
                        help="selected accepted executable; do not validate with a stale flat installation")
    args = parser.parse_args()
    config = args.config_dir.resolve()
    stage = args.stage_dir or config / ".pending-personal"
    if args.action == "prepare":
        prepare(config, stage, args.binary)
    else:
        apply(config, stage, args.binary)


if __name__ == "__main__":
    main()
