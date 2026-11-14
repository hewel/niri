#!/usr/bin/env python3
"""Prepare/apply a checked, one-time migration to the host's existing niri config."""

import argparse
import ctypes
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
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
    """Atomically exchange entries; source retains the actual displaced inode."""
    rename = getattr(ctypes.CDLL(None, use_errno=True), "renameat2", None)
    if rename is None:
        raise OSError(errno.ENOSYS, "renameat2(RENAME_EXCHANGE) is required")
    rename.argtypes = (ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint)
    rename.restype = ctypes.c_int
    if rename(-100, os.fsencode(source), -100, os.fsencode(destination), 2):
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error), str(destination))


class MigrationConflict(SystemExit):
    pass


def conflict(stage, detail):
    raise MigrationConflict(
        f"{detail}; user edits and migration evidence preserved, manual recovery required. "
        f"Inspect {stage / 'recovery'}, {stage / 'original'} and {stage / 'candidate'} "
        "alongside the live config. Preserve the whole stage before reconciling or repreparing."
    )


def identity(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode):
        raise ValueError(f"Not a regular configuration file: {path}")
    return [info.st_dev, info.st_ino]


def check_recovery(stage):
    # An intent becomes visible before exchange. An orphan without an intent
    # cannot have been exchanged and is retained, not used or cleaned up.
    for intent in sorted((stage / "recovery").glob("*/intent.json")):
        try:
            operation = json.loads(intent.read_text())
            saved = intent.parent / "saved"
            if identity(saved) == operation["replacement_inode"]:
                allowed = (operation["replacement_sha256"],)
            else:
                allowed = operation["allowed"]
            if digest(saved) not in allowed:
                conflict(stage, f"Saved configuration changed at {saved}")
        except (OSError, ValueError, KeyError, TypeError) as error:
            conflict(stage, f"Uncertain recovery record {intent}: {error}")


def preserve_replace(stage, source, destination, allowed):
    check_recovery(stage)
    try:
        identity(destination)
        if digest(destination) not in allowed:
            conflict(stage, f"Configuration changed before replacement: {destination}")
        recovery = stage / "recovery"
        recovery.mkdir(mode=0o700, exist_ok=True)
        operation = Path(tempfile.mkdtemp(prefix="replace-", dir=recovery))
        saved = operation / "saved"
        shutil.copy2(source, saved)
        replacement_sha = digest(saved)
        write_record(operation / "intent.json", {
            "target": str(destination.absolute()),
            "replacement_inode": identity(saved),
            "replacement_sha256": replacement_sha,
            "allowed": list(allowed),
        })
        replace_file(saved, destination)
        # Never discard saved, even after success: an editor may still hold its
        # inode open and save to it after these checks.
        check_recovery(stage)
        identity(destination)
        if digest(destination) != replacement_sha:
            conflict(stage, f"Configuration changed during replacement: {destination}")
    except (OSError, ValueError) as error:
        conflict(stage, f"Cannot safely exchange configuration {destination}: {error}")


def write_record(path, record):
    fd, temporary = tempfile.mkstemp(prefix=".manifest-", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as output:
            output.write(json.dumps(record, indent=2) + "\n")
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def apply(config, stage, binary):
    if not stage.exists():
        return
    # This serializes migration journals only. Editors need not take this lock;
    # preserving displaced inodes, not the lock, protects their data.
    with (stage / ".apply.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        _apply(config, stage, binary)


def _apply(config, stage, binary):
    manifest = stage / "manifest.json"
    if not manifest.exists():
        return
    record = json.loads(manifest.read_text())
    check_recovery(stage)
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
                conflict(stage, "Interrupted migration followed by user edits")
        for name in CHANGED:
            preserve_replace(stage, stage / "original" / name, config / name,
                             (record["files"][name], record["changes"][name]))
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
            # Register before exchange so interruption immediately afterward
            # cannot leave a partial write outside synchronous rollback.
            changed.append(name)
            preserve_replace(stage, stage / "candidate" / name, config / name,
                             (record["files"][name],))
        validate(binary, config / "config.kdl")
        check_recovery(stage)
        for name in CHANGED:
            if digest(config / name) != record["changes"][name]:
                conflict(stage, f"Configuration changed after replacement: {config / name}")
    except MigrationConflict:
        raise
    except BaseException:
        check_recovery(stage)
        for name in changed:
            if digest(config / name) not in (record["files"][name], record["changes"][name]):
                conflict(stage, "Migration failed followed by user edits")
        for name in reversed(changed):
            preserve_replace(stage, stage / "original" / name, config / name,
                             (record["files"][name], record["changes"][name]))
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
