#!/usr/bin/env python3
"""Exercise deployment and migration through their module seam, without root writes."""
import hashlib
import importlib.util
import os
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent.parent


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, SOURCE / "scripts" / file)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


d = load("deployment", "personal-deploy.py")
m = load("migration", "personal-unify-config.py")
launcher = load("launcher", "../docs/personal/deployment/niri-personal-launch.py")


class DeploymentTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="niri-deploy-test-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.config = self.base / "config"
        self.config.mkdir()
        (self.config / "config.kdl").write_text("accepted config\n")
        self.paths = d.Paths(self.base / "system", self.config, SOURCE)
        self.deploy = d.Deployment(self.paths, lambda stage, plan_sha: d.publish(self.paths, stage, plan_sha))

    def artifact(self, name, rejected="never-reject-this"):
        binary = self.base / name
        binary.write_text(f'#!/usr/bin/python3\nfrom pathlib import Path\nimport sys\n# {name}\nraise SystemExit(1 if {rejected!r} in Path(sys.argv[3]).read_text() else 0)\n')
        binary.chmod(0o755)
        accepted = self.base / f"{name}.json"
        d.write_json(accepted, {"source": "user-observed", "result": "passed", "binary_sha256": d.digest(binary), "scope": "test fixture"})
        return binary, accepted

    def install(self, name, **kwargs):
        return self.deploy.install(*self.artifact(name, **kwargs))

    def test_initial_install_has_complete_immutable_bundle_and_canonical_session(self):
        identifier = self.install("one")
        d.check_bundle(self.paths.bundle(identifier), identifier)
        self.assertEqual(self.paths.selected(), {"current": identifier, "previous": None})
        self.assertEqual((self.config / "config.kdl").read_text(), "accepted config\n")
        service = self.paths.root / d.SYSTEM_FILES["niri-personal.service"]
        self.assertIn("ExecStart=/usr/local/bin/niri-personal-session --exec", service.read_text())

    def test_upgrade_and_rollback_preserve_edited_config(self):
        one = self.install("one")
        (self.config / "config.kdl").write_text("user customization\n")
        two = self.install("two")
        self.assertEqual(self.paths.selected(), {"current": two, "previous": one})
        self.assertEqual(self.deploy.rollback(), one)
        self.assertEqual(self.paths.selected(), {"current": one, "previous": two})
        self.assertEqual((self.config / "config.kdl").read_text(), "user customization\n")

    def test_repeated_install_does_not_lose_rollback(self):
        one = self.install("one")
        args = self.artifact("two")
        two = self.deploy.install(*args)
        self.assertEqual(self.deploy.install(*args), two)
        self.assertEqual(self.paths.selected(), {"current": two, "previous": one})
        self.assertEqual(len(list((self.paths.store / "releases").iterdir())), 2)

    def test_denied_privilege_retries_without_migration_or_config_changes(self):
        pending = self.config / ".pending-personal"
        (pending / "candidate").mkdir(parents=True)
        d.write_json(pending / "manifest.json", {"state": "prepared"})
        (pending / "candidate/config.kdl").write_text("pending custom config")
        before = {str(p.relative_to(self.config)): p.read_bytes() for p in self.config.rglob("*") if p.is_file()}
        def denied(stage, plan_sha):
            raise PermissionError("authentication cancelled")
        failing = d.Deployment(self.paths, denied)
        args = self.artifact("one", rejected="accepted config")
        with self.assertRaises(PermissionError):
            failing.install(*args)
        self.assertIsNone(self.paths.selected())
        self.deploy.install(*args)
        self.assertEqual(before, {str(p.relative_to(self.config)): p.read_bytes() for p in self.config.rglob("*") if p.is_file()})

    def test_completed_migration_uses_current_config_and_is_never_reapplied(self):
        pending = self.config / ".pending-personal"
        (pending / "candidate").mkdir(parents=True)
        d.write_json(pending / "manifest.json", {"state": "applied"})
        (pending / "candidate/config.kdl").write_text("obsolete candidate")
        self.install("one", rejected="obsolete")
        self.install("two", rejected="obsolete")
        self.assertEqual(d.read_json(pending / "manifest.json"), {"state": "applied"})

    def test_incompatible_rollback_does_not_select_or_restore_config(self):
        self.install("one", rejected="new syntax")
        two = self.install("two")
        (self.config / "config.kdl").write_text("new syntax")
        with self.assertRaises(subprocess.CalledProcessError):
            self.deploy.rollback()
        self.assertEqual(self.paths.selected()["current"], two)
        self.assertEqual((self.config / "config.kdl").read_text(), "new syntax")

    def test_login_pins_one_bundle_across_later_selection(self):
        one = self.install("one")
        with patch.object(sys, "argv", ["niri-personal-session", "--login"]), patch.dict(os.environ, clear=False), patch.object(os, "execv", side_effect=InterruptedError("exec intercepted")) as execute:
            with self.assertRaises(InterruptedError):
                launcher.main(self.paths.store)
            pinned = os.environ["NIRI_PERSONAL_BUNDLE"]
            self.assertEqual(execute.call_args.args[0], self.paths.bundle(one) / "session.sh")
            self.install("two")
            with patch.object(sys, "argv", ["niri-personal-session", "--exec"]):
                with self.assertRaises(InterruptedError):
                    launcher.main(self.paths.store)
            self.assertEqual(execute.call_args.args[0], Path(pinned) / "niri")
            self.assertEqual(Path(pinned), self.paths.bundle(one))

    def test_wrong_or_unobserved_acceptance_rejected_before_publishing(self):
        binary, accepted = self.artifact("one")
        binary.write_text(binary.read_text() + "# unaccepted rebuild\n")
        with self.assertRaises(d.DeploymentError):
            self.deploy.install(binary, accepted)
        record = d.read_json(accepted)
        record.update(binary_sha256=d.digest(binary), source="automated")
        d.write_json(accepted, record)
        with self.assertRaises(d.DeploymentError):
            self.deploy.install(binary, accepted)
        self.assertIsNone(self.paths.selected())

    def test_bootstrap_imports_flat_binary_only_with_matching_acceptance(self):
        old, accepted = self.artifact("old")
        self.paths.store.mkdir(parents=True)
        shutil.copy2(old, self.paths.store / "niri")
        new = self.artifact("new")
        with self.assertRaises(d.DeploymentError):
            self.deploy.install(*new)
        identifier = self.deploy.install(*new, previous_acceptance=accepted)
        previous = self.paths.selected()["previous"]
        self.assertNotEqual(identifier, previous)
        self.assertEqual(d.digest(self.paths.bundle(previous) / "niri"), d.digest(old))

    def test_interrupted_publication_leaves_selection_and_retries(self):
        one = self.install("one")
        two = self.artifact("two")
        original = d.atomic_copy
        def interrupted(source, destination, mode):
            if destination == self.paths.selection:
                raise InterruptedError("after bundle publication, before selection")
            return original(source, destination, mode)
        with patch.object(d, "atomic_copy", interrupted), self.assertRaises(InterruptedError):
            self.deploy.install(*two)
        self.assertEqual(self.paths.selected()["current"], one)
        identifier = self.deploy.install(*two)
        self.assertEqual(self.paths.selected(), {"current": identifier, "previous": one})

    def test_retry_removes_only_own_abandoned_copies(self):
        self.install("one")
        abandoned = self.paths.store / "releases/.incoming-interrupted"
        abandoned.mkdir()
        (abandoned / "partial").write_text("incomplete")
        entries = self.paths.store / ".entry-interrupted"
        entries.mkdir()
        partial = self.paths.selection.parent / ".niri-personal-install-interrupted"
        partial.write_text("partial selection")
        unrelated = self.paths.store / "unrelated"
        unrelated.write_text("keep")
        self.install("two")
        self.assertFalse(abandoned.exists())
        self.assertFalse(entries.exists())
        self.assertFalse(partial.exists())
        self.assertEqual(unrelated.read_text(), "keep")

    def test_concurrent_selection_requires_fresh_retry(self):
        self.install("one")
        args = self.artifact("two")
        def changed(stage, plan_sha):
            d.write_json(self.paths.selection, {"current": "a" * 64, "previous": None})
            d.publish(self.paths, stage, plan_sha)
        with self.assertRaisesRegex(d.DeploymentError, "changed during preparation"):
            d.Deployment(self.paths, changed).install(*args)

    def test_validation_uses_snapshot_if_artifact_and_acceptance_are_replaced(self):
        artifact, accepted = self.artifact("original")
        initial = d.read_json(accepted)
        replacement, replacement_acceptance = self.artifact("replacement")
        validated = []
        def swap(binary, config):
            validated.append(d.digest(binary))
            self.assertNotEqual(binary, artifact)
            shutil.copyfile(replacement, artifact)
            shutil.copyfile(replacement_acceptance, accepted)
            d.validate(binary, config)
        deployment = d.Deployment(self.paths, lambda stage, plan_sha: d.publish(self.paths, stage, plan_sha), swap)
        identifier = deployment.install(artifact, accepted)
        installed = self.paths.bundle(identifier)
        self.assertEqual(validated, [initial["binary_sha256"]])
        self.assertEqual(d.digest(installed / "niri"), initial["binary_sha256"])
        self.assertEqual(d.read_json(installed / "acceptance.json"), initial)

    def test_plan_replacement_during_authentication_is_rejected(self):
        one = self.install("one")
        args = self.artifact("two")
        def rewrite_plan(stage, initial_plan_sha):
            plan = d.read_json(stage / "plan.json")
            plan["selection"]["previous"] = None
            d.write_json(stage / "plan.json", plan)
            d.publish(self.paths, stage, initial_plan_sha)
        with self.assertRaisesRegex(d.DeploymentError, "plan changed during authentication"):
            d.Deployment(self.paths, rewrite_plan).install(*args)
        self.assertEqual(self.paths.selected()["current"], one)

    def prepare_legacy(self):
        binary, accepted = self.artifact("legacy")
        self.paths.store.mkdir(parents=True)
        shutil.copy2(binary, self.paths.store / "niri")
        wrapper = self.paths.root / "usr/local/bin/niri-personal-session"
        wrapper.parent.mkdir(parents=True)
        wrapper.write_text("#!/bin/sh\n# retained legacy wrapper\n")
        wrapper.chmod(0o755)
        return accepted

    def assert_legacy_exec(self):
        with patch.object(sys, "argv", ["niri-personal-session", "--exec"]), patch.dict(os.environ, {}, clear=True), patch.object(os, "execv", side_effect=InterruptedError("exec intercepted")) as execute:
            with self.assertRaises(InterruptedError):
                launcher.main(self.paths.store)
            self.assertEqual(execute.call_args.args[0], self.paths.store / "niri")

    def test_old_wrapper_handoff_uses_accepted_flat_binary_after_later_updates(self):
        previous = self.prepare_legacy()
        self.deploy.install(*self.artifact("one"), previous_acceptance=previous)
        legacy_pin = (self.paths.store / "legacy-session.json").read_bytes()
        self.assert_legacy_exec()
        self.install("two")
        self.assert_legacy_exec()
        self.assertEqual((self.paths.store / "legacy-session.json").read_bytes(), legacy_pin)
        (self.paths.store / "niri").write_text("replaced flat executable")
        with patch.object(sys, "argv", ["niri-personal-session", "--exec"]), patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(SystemExit, "Retained legacy session changed"):
                launcher.main(self.paths.store)

    def test_missing_pin_without_legacy_context_is_refused(self):
        self.install("one")
        with patch.object(sys, "argv", ["niri-personal-session", "--exec"]), patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(SystemExit, "Missing personal build pin"):
                launcher.main(self.paths.store)

    def test_interrupted_bootstrap_retains_legacy_handoff_and_retries(self):
        previous = self.prepare_legacy()
        args = self.artifact("one")
        original = d.atomic_copy
        def interrupt(source, destination, mode):
            if destination == self.paths.selection:
                raise InterruptedError("selection not published")
            original(source, destination, mode)
        with patch.object(d, "atomic_copy", interrupt), self.assertRaises(InterruptedError):
            self.deploy.install(*args, previous_acceptance=previous)
        self.assertIsNone(self.paths.selected())
        self.assert_legacy_exec()
        self.deploy.install(*args, previous_acceptance=previous)
        self.assert_legacy_exec()

    def test_modified_staged_bytes_do_not_replace_selection(self):
        one = self.install("one")
        args = self.artifact("two")
        def tamper(stage, plan_sha):
            (stage / "candidate/niri").write_text("changed after acceptance")
            d.publish(self.paths, stage, plan_sha)
        with self.assertRaisesRegex(d.DeploymentError, "content changed"):
            d.Deployment(self.paths, tamper).install(*args)
        self.assertEqual(self.paths.selected()["current"], one)


class MigrationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="niri-migration-test-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.config = self.base / "config"
        (self.config / "cfg").mkdir(parents=True)
        (self.config / "config.kdl").write_text('include "cfg/outputs.kdl"\ninclude "cfg/layout.kdl"\ninclude "cfg/rules.kdl"\n')
        (self.config / "cfg/outputs.kdl").write_text('output "DP-1" {\n}\noutput "HDMI-A-1" {\n}\n')
        (self.config / "cfg/layout.kdl").write_text('window-rule {\n    geometry-corner-radius 12\n}\n')
        (self.config / "cfg/rules.kdl").write_text('window-rule {\n    background-effect {\n        blur true\n    }\n}\n' * 5)
        self.stage = self.base / "stage"
        self.validator = patch.object(m, "validate", lambda binary, config: None)
        self.validator.start()
        self.addCleanup(self.validator.stop)
        m.prepare(self.config, self.stage, Path("unused"))
        self.before = {name: m.digest(self.config / name) for name in m.CHANGED}
        self.record = d.read_json(self.stage / "manifest.json")

    def apply(self):
        m.apply(self.config, self.stage, Path("unused"))

    def edit(self, text="// user edit\n"):
        path = self.config / "cfg/layout.kdl"
        path.write_text(text + path.read_text())

    def assert_originals(self):
        self.assertEqual(self.before, {name: m.digest(self.config / name) for name in m.CHANGED})

    def test_success_and_idempotence(self):
        self.apply()
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applied")
        self.edit()
        edited = m.digest(self.config / "cfg/layout.kdl")
        self.apply()
        self.assertEqual(m.digest(self.config / "cfg/layout.kdl"), edited)

    def test_stale_user_edit(self):
        self.edit()
        with self.assertRaisesRegex(SystemExit, "edited after preparation"):
            self.apply()
        self.assertIn("user edit", (self.config / "cfg/layout.kdl").read_text())

    def test_edit_during_validation(self):
        with patch.object(m, "validate", lambda *args: self.edit()), self.assertRaises(SystemExit):
            self.apply()
        self.assertIn("user edit", (self.config / "cfg/layout.kdl").read_text())
        self.assertEqual(m.digest(self.config / "cfg/rules.kdl"), self.before["cfg/rules.kdl"])

    def test_installed_validation_failure(self):
        def fail(binary, config):
            if config == self.config / "config.kdl":
                raise RuntimeError("final validation failure")
        with patch.object(m, "validate", fail), self.assertRaises(RuntimeError):
            self.apply()
        self.assert_originals()

    def test_edit_during_failed_validation_preserved(self):
        def fail(binary, config):
            if config == self.config / "config.kdl":
                self.edit()
                raise RuntimeError("final validation failure")
        with patch.object(m, "validate", fail), self.assertRaisesRegex(SystemExit, "user edits"):
            self.apply()
        self.assertIn("user edit", (self.config / "cfg/layout.kdl").read_text())
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applying")

    def test_partial_write_failure_rolls_back(self):
        original = m.replace_file
        def fail(source, destination):
            if source == self.stage / "candidate/cfg/layout.kdl":
                raise KeyboardInterrupt("second write interrupted")
            original(source, destination)
        with patch.object(m, "replace_file", fail), self.assertRaises(KeyboardInterrupt):
            self.apply()
        self.assert_originals()

    def interrupted(self):
        self.record["state"] = "applying"
        m.write_record(self.stage / "manifest.json", self.record)
        shutil.copy2(self.stage / "candidate/cfg/outputs.kdl", self.config / "cfg/outputs.kdl")

    def test_interrupted_recovery(self):
        self.interrupted()
        self.apply()
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applied")

    def test_interrupted_user_edit(self):
        self.interrupted()
        self.edit()
        with self.assertRaisesRegex(SystemExit, "manual recovery"):
            self.apply()
        self.assertIn("user edit", (self.config / "cfg/layout.kdl").read_text())

    def test_altered_candidate(self):
        (self.stage / "candidate/cfg/layout.kdl").write_text("altered")
        with self.assertRaisesRegex(SystemExit, "altered"):
            self.apply()
        self.assert_originals()

    def test_recursive_stage_refused(self):
        with self.assertRaises(SystemExit):
            m.prepare(self.config, self.config / "nested-stage", Path("unused"))
        self.assertFalse((self.config / "nested-stage").exists())


if __name__ == "__main__":
    unittest.main()
