#!/usr/bin/env python3
"""Select an explicitly accepted personal build for the next login."""

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


FILES = ("niri", "session.sh", "unify-config.py", "acceptance.json")
SYSTEM_FILES = {
    "niri-personal-launch.py": "usr/local/bin/niri-personal-session",
    "niri-personal.service": "usr/local/lib/systemd/user/niri-personal.service",
    "niri-personal-shutdown.target": "usr/local/lib/systemd/user/niri-personal-shutdown.target",
    "niri-personal.desktop": "usr/share/wayland-sessions/niri-personal.desktop",
}


class DeploymentError(Exception):
    pass


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    encoded = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
    path.write_bytes(encoded)
    return hashlib.sha256(encoded).hexdigest()


def key(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def acceptance(path, binary):
    record = read_json(path)
    observed = record.get("result") == "passed" and record.get("source") == "user-observed"
    authorized = (
        record.get("result") == "approved-for-daily-trial"
        and record.get("source") == "user-authorized"
        and isinstance(record.get("user_request"), str)
        and bool(record["user_request"].strip())
        and isinstance(record.get("scope"), str)
        and bool(record["scope"].strip())
    )
    if not (observed or authorized):
        raise DeploymentError("Recorded user-observed acceptance or explicit user authorization is required.")
    if record.get("binary_sha256") != digest(binary):
        raise DeploymentError("The artifact does not match the accepted SHA-256; verify this build first.")
    return record


@dataclass(frozen=True)
class Paths:
    # Alternate paths are a private test seam, never command-line arguments.
    root: Path
    config: Path
    source: Path

    @property
    def store(self):
        return self.root / "usr/local/lib/niri-personal"

    @property
    def selection(self):
        return self.store / "selection.json"

    def selected(self):
        return read_json(self.selection) if self.selection.exists() else None

    def bundle(self, identifier):
        if not isinstance(identifier, str) or len(identifier) != 64 or any(c not in "0123456789abcdef" for c in identifier):
            raise DeploymentError("Invalid installed build identifier.")
        return self.store / "releases" / identifier


def validate(binary, config):
    subprocess.run([str(binary), "validate", "-c", str(config)], check=True)


def validation_config(paths):
    pending = paths.config / ".pending-personal"
    if not pending.exists():
        return paths.config / "config.kdl"
    manifest = pending / "manifest.json"
    if not manifest.exists():
        raise DeploymentError("Pending migration is incomplete; inspect .pending-personal before retrying.")
    state = read_json(manifest).get("state")
    if state == "applied":
        return paths.config / "config.kdl"
    if state == "prepared":
        return pending / "candidate/config.kdl"
    raise DeploymentError("Interrupted migration requires recovery at login before selecting another build.")


def make_bundle(destination, binary, accepted_record, source):
    destination.mkdir()
    for name, original in {
        "niri": binary,
        "session.sh": source / "scripts/personal-daily-session.sh",
        "unify-config.py": source / "scripts/personal-unify-config.py",
    }.items():
        shutil.copyfile(original, destination / name)
        (destination / name).chmod(0o755)
    write_json(destination / "acceptance.json", accepted_record)
    acceptance(destination / "acceptance.json", destination / "niri")
    manifest = {"schema": 1, "files": {name: digest(destination / name) for name in FILES}}
    write_json(destination / "manifest.json", manifest)
    return key(manifest)


def check_bundle(directory, identifier):
    manifest = read_json(directory / "manifest.json")
    if manifest.get("schema") != 1 or set(manifest.get("files", {})) != set(FILES) or key(manifest) != identifier:
        raise DeploymentError("Invalid build manifest.")
    for name, sha in manifest["files"].items():
        path = directory / name
        if path.is_symlink() or not path.is_file() or digest(path) != sha:
            raise DeploymentError(f"Build content changed: {name}")
    acceptance(directory / "acceptance.json", directory / "niri")


class Deployment:
    """Own acceptance, config compatibility, immutable preparation and selection."""

    def __init__(self, paths, publish, validator=validate):
        self.paths = paths
        self.publish = publish
        self.validate = validator

    def status(self):
        selection = self.paths.selected()
        if not selection:
            flat = self.paths.store / "niri"
            return {"layout": "legacy-flat" if flat.is_file() else "not-installed",
                    "installed_binary_sha256": digest(flat) if flat.is_file() else None,
                    "selection": None}
        builds = {}
        for role, identifier in selection.items():
            if identifier is None:
                builds[role] = None
                continue
            record = read_json(self.paths.bundle(identifier) / "acceptance.json")
            builds[role] = {"bundle": identifier, "binary_sha256": record["binary_sha256"],
                            "source": record["source"], "result": record["result"],
                            "accepted_scope": record.get("scope")}
        return {"layout": "immutable-bundles", "selection": builds}

    def install(self, artifact, accepted, previous_acceptance=None):
        accepted_record = acceptance(accepted, artifact)
        previous_record = read_json(previous_acceptance) if previous_acceptance else accepted_record
        current = self.paths.selected()
        config = validation_config(self.paths)
        # Staging is disposable: denied authentication leaves neither a pending
        # install marker nor changed config. The user's migration journal is separate.
        with tempfile.TemporaryDirectory(prefix="niri-deploy-") as temporary:
            stage = Path(temporary)
            identifier = make_bundle(stage / "candidate", artifact, accepted_record, self.paths.source)
            self.validate(stage / "candidate/niri", config)
            check_bundle(stage / "candidate", identifier)
            bundles = {identifier: "candidate"}
            previous = current["current"] if current else None
            if current and previous == identifier:
                previous = current.get("previous")
            legacy_sha = None
            if not current and (self.paths.store / "niri").is_file():
                old_binary = self.paths.store / "niri"
                old = make_bundle(stage / "legacy", old_binary, previous_record, self.paths.source)
                self.validate(stage / "legacy/niri", config)
                check_bundle(stage / "legacy", old)
                legacy_sha = previous_record["binary_sha256"]
                if old != identifier:
                    bundles[old] = "legacy"
                    previous = old
            for name in SYSTEM_FILES:
                shutil.copyfile(self.paths.source / "docs/personal/deployment" / name, stage / name)
            plan = {"schema": 1, "expected": current, "selection": {"current": identifier, "previous": previous},
                    "bundles": bundles, "legacy_binary_sha256": legacy_sha,
                    "system_files": {name: digest(stage / name) for name in SYSTEM_FILES}}
            plan_sha = write_json(stage / "plan.json", plan)
            self.publish(stage, plan_sha)
        return identifier

    def rollback(self):
        current = self.paths.selected()
        if not current or not current.get("previous"):
            raise DeploymentError("No preceding accepted build is recorded.")
        target = current["previous"]
        bundle = self.paths.bundle(target)
        check_bundle(bundle, target)
        self.validate(bundle / "niri", validation_config(self.paths))
        with tempfile.TemporaryDirectory(prefix="niri-deploy-") as temporary:
            stage = Path(temporary)
            plan_sha = write_json(stage / "plan.json", {"schema": 1, "expected": current,
                       "selection": {"current": target, "previous": current["current"]},
                       "bundles": {}, "system_files": {}})
            self.publish(stage, plan_sha)
        return target


@contextmanager
def exclusive_store(store):
    store.mkdir(parents=True, exist_ok=True)
    with (store / ".deploy.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def atomic_copy(source, destination, mode):
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=".niri-personal-install-", dir=destination.parent)
    os.close(fd)
    try:
        shutil.copyfile(source, temporary)
        os.chmod(temporary, mode)
        os.replace(temporary, destination)
    finally:
        Path(temporary).unlink(missing_ok=True)


def publish(paths, stage, plan_sha):
    """Privileged adapter: fixed personal paths; never runs a supplied executable."""
    # Copy user-owned input before examining it. Every bundle and helper is checked
    # against its manifest after copying and before making a selection visible.
    plan_bytes = (stage / "plan.json").read_bytes()
    if hashlib.sha256(plan_bytes).hexdigest() != plan_sha:
        raise DeploymentError("Deployment plan changed during authentication; retry.")
    plan = json.loads(plan_bytes)
    with exclusive_store(paths.store):
        if plan.get("schema") != 1 or paths.selected() != plan.get("expected"):
            raise DeploymentError("The selected build changed during preparation; retry from the current state.")
        selection = plan["selection"]
        if set(selection) != {"current", "previous"}:
            raise DeploymentError("Invalid selection record.")
        paths.bundle(selection["current"])
        if selection["previous"] is not None:
            paths.bundle(selection["previous"])
        system_files = plan["system_files"]
        if system_files and set(system_files) != set(SYSTEM_FILES):
            raise DeploymentError("Incomplete personal session entry.")
        releases = paths.store / "releases"
        releases.mkdir(exist_ok=True)
        # Recover only this adapter's abandoned copies, under its exclusive lock.
        for directory, pattern in ((releases, ".incoming-*"), (paths.store, ".entry-*")):
            for stale in directory.glob(pattern):
                if stale.is_dir() and not stale.is_symlink():
                    shutil.rmtree(stale)
        parents = {paths.store, *((paths.root / value).parent for value in SYSTEM_FILES.values())}
        for directory in parents:
            for stale in directory.glob(".niri-personal-install-*"):
                if stale.is_file() and not stale.is_symlink():
                    stale.unlink()
        for identifier, source in plan["bundles"].items():
            destination = paths.bundle(identifier)
            if source not in ("candidate", "legacy"):
                raise DeploymentError("Invalid staged build path.")
            if destination.exists():
                check_bundle(destination, identifier)
                continue
            with tempfile.TemporaryDirectory(prefix=".incoming-", dir=releases) as temporary:
                incoming = Path(temporary) / "bundle"
                incoming.mkdir()
                for name in (*FILES, "manifest.json"):
                    original = stage / source / name
                    if original.is_symlink() or not original.is_file():
                        raise DeploymentError("Staged build must contain regular files.")
                    shutil.copyfile(original, incoming / name)
                    (incoming / name).chmod(0o755 if name in FILES[:3] else 0o644)
                check_bundle(incoming, identifier)
                os.rename(incoming, destination)
        for identifier in (selection["current"], selection["previous"]):
            if identifier:
                check_bundle(paths.bundle(identifier), identifier)
        # Keep the previous flat session usable if first-time bootstrap is interrupted
        # before selection.json appears. Future deployments share these stable entry files.
        legacy = paths.root / "usr/local/bin/niri-personal-session"
        legacy_pin = None
        if not paths.selection.exists() and plan.get("legacy_binary_sha256") and legacy.is_file():
            flat_sha = digest(paths.store / "niri")
            if flat_sha != plan["legacy_binary_sha256"]:
                raise DeploymentError("The flat executable changed during bootstrap; retry.")
            if not (paths.store / "legacy-session.sh").exists():
                atomic_copy(legacy, paths.store / "legacy-session.sh", 0o755)
            legacy_pin = {"binary_sha256": flat_sha,
                          "wrapper_sha256": digest(paths.store / "legacy-session.sh")}
        with tempfile.TemporaryDirectory(prefix=".entry-", dir=paths.store) as temporary:
            copied = Path(temporary)
            for name, expected in system_files.items():
                shutil.copyfile(stage / name, copied / name)
                if digest(copied / name) != expected:
                    raise DeploymentError("Session entry changed during authentication; retry.")
            if legacy_pin:
                write_json(copied / "legacy-session.json", legacy_pin)
                atomic_copy(copied / "legacy-session.json", paths.store / "legacy-session.json", 0o644)
            for name in system_files:
                atomic_copy(copied / name, paths.root / SYSTEM_FILES[name], 0o755 if name.endswith(".py") else 0o644)
            write_json(copied / "selection.json", selection)
            atomic_copy(copied / "selection.json", paths.selection, 0o644)


def authorized_publish(paths, stage, plan_sha):
    helper = paths.source / "scripts/personal-install-system.sh"
    sudo = shutil.which("sudo")
    if sudo and subprocess.run([sudo, "-n", "true"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
        command = [sudo, "-n"]
    elif shutil.which("pkexec"):
        command = ["pkexec"]
    else:
        raise DeploymentError("Administrator authentication requires sudo or pkexec.")
    subprocess.run([*command, "/usr/bin/bash", str(helper), str(stage), plan_sha], check=True)
    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)


def main():
    source = Path(__file__).resolve().parent.parent
    paths = Paths(Path("/"), Path.home() / ".config/niri", source)
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="action", required=True)
    install = actions.add_parser("install", help="install or retry one explicitly accepted artifact")
    install.add_argument("--artifact", type=Path, required=True)
    install.add_argument("--acceptance", type=Path, required=True)
    install.add_argument("--previous-acceptance", type=Path, help="acceptance of the existing flat installation when its binary differs")
    actions.add_parser("rollback", help="select the preceding accepted build after checking current config")
    actions.add_parser("status", help="show the next-login and rollback build records")
    # Invoked only by the fixed-path privileged shell adapter; no alternate roots.
    if len(os.sys.argv) == 4 and os.sys.argv[1] == "_publish":
        if os.geteuid() != 0:
            raise DeploymentError("Publishing requires the authorized installer.")
        publish(paths, Path(os.sys.argv[2]), os.sys.argv[3])
        return
    args = parser.parse_args()
    deployment = Deployment(paths, lambda stage, plan_sha: authorized_publish(paths, stage, plan_sha))
    if args.action == "status":
        print(json.dumps(deployment.status(), indent=2))
        return
    if args.action == "install":
        identifier = deployment.install(args.artifact.resolve(), args.acceptance.resolve(), args.previous_acceptance)
    else:
        identifier = deployment.rollback()
    print(f"Selected accepted build {identifier} for the next Niri HDR + Glass login. The running desktop continues.")


if __name__ == "__main__":
    try:
        main()
    except (DeploymentError, OSError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Daily deployment failed: {error}")
