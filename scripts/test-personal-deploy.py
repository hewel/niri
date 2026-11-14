#!/usr/bin/env python3
"""Exercise deployment and migration through their module seam, without root writes."""
from contextlib import contextmanager
import errno
import hashlib
import importlib.util
import os
import json
from pathlib import Path
import shutil
import stat
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

    @contextmanager
    def authorized_deployment(self, reload_failure=None):
        """Run the real publisher under the fixture root; never execute host administration."""
        run = subprocess.run
        def command(args, **kwargs):
            if args == ["/fixture/sudo", "-n", "true"]:
                return subprocess.CompletedProcess(args, 0)
            if args[:3] == ["/fixture/sudo", "-n", "/usr/bin/bash"]:
                d.publish(self.paths, Path(args[-2]), args[-1])
                return subprocess.CompletedProcess(args, 0)
            if args == ["systemctl", "--user", "daemon-reload"]:
                if reload_failure is not None:
                    raise reload_failure
                return subprocess.CompletedProcess(args, 0)
            self.assertTrue(Path(args[0]).is_relative_to(self.base), args)
            self.assertEqual(args[1], "validate")
            return run(args, **kwargs)
        with patch.object(d.tempfile, "tempdir", str(self.base)), \
                patch.object(d.shutil, "which", return_value="/fixture/sudo"), \
                patch.object(d.subprocess, "run", side_effect=command):
            yield d.Deployment(self.paths, lambda stage, sha: d.authorized_publish(self.paths, stage, sha))

    def test_rollback_succeeds_without_a_user_manager(self):
        one = self.install("one")
        two = self.install("two")
        unavailable = subprocess.CalledProcessError(1, ["systemctl", "--user", "daemon-reload"])
        with self.authorized_deployment(unavailable) as deployment:
            self.assertEqual(deployment.rollback(), one)
        self.assertEqual(self.paths.selected(), {"current": one, "previous": two})
        self.assertEqual((self.config / "config.kdl").read_text(), "accepted config\n")

    def test_install_reload_failure_reports_committed_selection_and_retries_safely(self):
        one = self.install("one")
        artifact, accepted = self.artifact("two")
        unavailable = subprocess.CalledProcessError(1, ["systemctl", "--user", "daemon-reload"])
        with self.authorized_deployment(unavailable) as deployment:
            with self.assertRaises(d.SelectionPublishedError) as failure:
                deployment.install(artifact, accepted)
        committed = failure.exception.selection
        self.assertEqual(self.paths.selected(), committed)
        self.assertEqual(committed["previous"], one)
        self.assertEqual(d.digest(self.paths.bundle(committed["current"]) / "niri"), d.digest(artifact))
        with self.authorized_deployment() as deployment:
            self.assertEqual(deployment.install(artifact, accepted), committed["current"])
        self.assertEqual(self.paths.selected(), committed)
        self.assertEqual((self.config / "config.kdl").read_text(), "accepted config\n")

    def test_initial_install_has_complete_immutable_bundle_and_canonical_session(self):
        identifier = self.install("one")
        d.check_bundle(self.paths.bundle(identifier), identifier)
        self.assertEqual(self.paths.selected(), {"current": identifier, "previous": None})
        self.assertEqual((self.config / "config.kdl").read_text(), "accepted config\n")
        service = self.paths.root / d.SYSTEM_FILES["niri-personal.service"]
        self.assertIn("ExecStart=/usr/local/bin/niri-personal-session --exec", service.read_text())

    def test_restrictive_umask_publishes_traversable_build_and_keeps_staging_private(self):
        def validate_private_stage(binary, config):
            self.assertEqual(stat.S_IMODE(binary.parent.parent.stat().st_mode) & 0o077, 0)
            d.validate(binary, config)
        deployment = d.Deployment(
            self.paths, lambda stage, sha: d.publish(self.paths, stage, sha), validate_private_stage)
        old_umask = os.umask(0o077)
        try:
            identifier = deployment.install(*self.artifact("private-umask"))
        finally:
            os.umask(old_umask)
        published = [self.paths.selection, self.paths.bundle(identifier) / "niri"]
        published.extend(self.paths.root / value for value in d.SYSTEM_FILES.values())
        for file in published:
            self.assertTrue(file.is_file())
            directory = file.parent
            while directory.is_relative_to(self.paths.root):
                with self.subTest(directory=directory):
                    self.assertEqual(stat.S_IMODE(directory.stat().st_mode), 0o755)
                directory = directory.parent
        self.assertEqual(self.deploy.status()["selection"]["current"]["bundle"], identifier)

    def test_publication_preserves_existing_system_directory_permissions(self):
        parent = self.paths.root / "usr/local"
        parent.mkdir(parents=True)
        parent.chmod(0o751)
        self.paths.store.mkdir(parents=True)
        self.paths.store.chmod(0o700)
        identifier = self.install("permissions")
        self.assertEqual(stat.S_IMODE(parent.stat().st_mode), 0o751)
        self.assertEqual(stat.S_IMODE(self.paths.store.stat().st_mode), 0o755)
        self.assertEqual(stat.S_IMODE(self.paths.bundle(identifier).stat().st_mode), 0o755)

    def test_store_symlink_does_not_relax_or_publish_into_another_directory(self):
        private = self.base / "private"
        private.mkdir(mode=0o700)
        self.paths.store.parent.mkdir(parents=True)
        self.paths.store.symlink_to(private, target_is_directory=True)
        with self.assertRaises(d.DeploymentError):
            self.install("redirected-store")
        self.assertEqual(stat.S_IMODE(private.stat().st_mode), 0o700)
        self.assertEqual(list(private.iterdir()), [])

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

    def test_observed_acceptance_of_same_runtime_preserves_real_rollback(self):
        one = self.install("one")
        binary, accepted = self.artifact("two")
        record = d.read_json(accepted)
        record.update(source="user-authorized", result="approved-for-daily-trial",
                      user_request="Install for daily trial", scope="Awaiting observation")
        d.write_json(accepted, record)
        trial = self.deploy.install(binary, accepted)
        record.update(source="user-observed", result="passed", scope="User observed this exact build")
        d.write_json(accepted, record)
        observed = self.deploy.install(binary, accepted)
        self.assertNotEqual(observed, trial)
        self.assertEqual(self.paths.selected(), {"current": observed, "previous": one})
        self.assertEqual(self.deploy.status()["selection"]["current"]["source"], "user-observed")
        self.assertEqual(self.deploy.rollback(), one)
        self.assertEqual(d.read_json(self.paths.bundle(trial) / "acceptance.json")["source"], "user-authorized")

    def test_acceptance_update_after_first_install_does_not_invent_rollback(self):
        binary, accepted = self.artifact("first")
        self.deploy.install(binary, accepted)
        record = d.read_json(accepted)
        record["scope"] = "Additional user observation of the same runtime"
        d.write_json(accepted, record)
        current = self.deploy.install(binary, accepted)
        self.assertEqual(self.paths.selected(), {"current": current, "previous": None})
        with self.assertRaises(d.DeploymentError):
            self.deploy.rollback()

    def test_same_runtime_legacy_import_preserves_handoff_without_duplicate_rollback(self):
        previous = self.prepare_legacy()
        binary = self.base / "legacy"
        accepted = self.base / "legacy-observed.json"
        record = d.read_json(previous)
        record["scope"] = "Additional observation of the same legacy executable"
        d.write_json(accepted, record)
        current = self.deploy.install(binary, accepted, previous_acceptance=previous)
        self.assertEqual(self.paths.selected(), {"current": current, "previous": None})
        self.assertEqual(d.digest(self.paths.store / "niri"), record["binary_sha256"])
        self.assert_legacy_exec()

    def test_session_or_migration_helper_change_is_a_new_runtime(self):
        source = self.base / "source"
        (source / "scripts").mkdir(parents=True)
        (source / "docs").symlink_to(SOURCE / "docs", target_is_directory=True)
        for name in ("personal-daily-session.sh", "personal-unify-config.py"):
            shutil.copyfile(SOURCE / "scripts" / name, source / "scripts" / name)
        paths = d.Paths(self.paths.root, self.config, source)
        deployment = d.Deployment(paths, lambda stage, sha: d.publish(paths, stage, sha))
        binary, accepted = self.artifact("same-binary")
        current = deployment.install(binary, accepted)
        for name, replacement in (
            ("personal-daily-session.sh", "#!/bin/sh\nexit 17\n"),
            ("personal-unify-config.py", "#!/usr/bin/python3\nraise SystemExit(17)\n"),
        ):
            with self.subTest(helper=name):
                (source / "scripts" / name).write_text(replacement)
                changed = deployment.install(binary, accepted)
                self.assertEqual(paths.selected(), {"current": changed, "previous": current})
                self.assertEqual(deployment.rollback(), current)
                self.assertEqual(deployment.rollback(), changed)
                current = changed

    def test_acceptance_update_still_checks_current_runtime_integrity(self):
        binary, accepted = self.artifact("one")
        current = self.deploy.install(binary, accepted)
        (self.paths.bundle(current) / "session.sh").write_text("corrupted installed helper")
        record = d.read_json(accepted)
        record["scope"] = "Updated evidence, unchanged accepted runtime"
        d.write_json(accepted, record)
        with self.assertRaises(d.DeploymentError):
            self.deploy.install(binary, accepted)
        self.assertEqual(self.paths.selected(), {"current": current, "previous": None})

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

    def test_explicit_daily_trial_authorization_preserves_evidence_and_rollback(self):
        old = self.install("observed")
        binary, accepted = self.artifact("authorized")
        record = d.read_json(accepted)
        record.update(source="user-authorized", result="approved-for-daily-trial",
                      user_request="Install the latest version",
                      scope="Daily trial requested; physical HDR remains unverified.")
        d.write_json(accepted, record)
        identifier = self.deploy.install(binary, accepted)
        self.assertEqual(d.read_json(self.paths.bundle(identifier) / "acceptance.json"), record)
        status = self.deploy.status()["selection"]
        self.assertEqual(status["current"]["source"], "user-authorized")
        self.assertEqual(status["current"]["result"], "approved-for-daily-trial")
        self.assertEqual(status["previous"]["source"], "user-observed")
        self.assertEqual(self.deploy.rollback(), old)

    def test_daily_trial_authorization_still_requires_request_scope_and_matching_bytes(self):
        binary, accepted = self.artifact("authorized")
        record = d.read_json(accepted)
        record.update(source="user-authorized", result="approved-for-daily-trial",
                      user_request="Install", scope="Daily trial, HDR unverified")
        for invalid in [dict(record, user_request=" "), dict(record, scope=None),
                        dict(record, source="automated"), dict(record, result="passed"),
                        dict(record, binary_sha256="0" * 64)]:
            with self.subTest(record=invalid):
                d.write_json(accepted, invalid)
                with self.assertRaises(d.DeploymentError):
                    self.deploy.install(binary, accepted)
                self.assertIsNone(self.paths.selected())

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
        with self.assertRaises(SystemExit):
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
        with patch.object(m, "validate", fail), self.assertRaises(SystemExit):
            self.apply()
        self.assertIn("user edit", (self.config / "cfg/layout.kdl").read_text())
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applying")

    def test_partial_write_failure_rolls_back(self):
        original = m.replace_file
        interrupted = False
        def fail(source, destination):
            nonlocal interrupted
            if destination == self.config / "cfg/layout.kdl" and not interrupted:
                interrupted = True
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
        with self.assertRaises(SystemExit):
            self.apply()
        self.assertIn("user edit", (self.config / "cfg/layout.kdl").read_text())

    def test_altered_candidate(self):
        (self.stage / "candidate/cfg/layout.kdl").write_text("altered")
        with self.assertRaises(SystemExit):
            self.apply()
        self.assert_originals()

    def test_recursive_stage_refused(self):
        with self.assertRaises(SystemExit):
            m.prepare(self.config, self.config / "nested-stage", Path("unused"))
        self.assertFalse((self.config / "nested-stage").exists())

    def saved_layout(self, radius=29, atomic=False):
        path = self.config / "cfg/layout.kdl"
        content = path.read_bytes().replace(b"geometry-corner-radius 12",
                                           f"geometry-corner-radius {radius}".encode())
        if atomic:
            temporary = self.config / "cfg/editor-save"
            temporary.write_bytes(content)
            os.replace(temporary, path)
        else:
            path.write_bytes(content)
        return content

    def assert_preserved(self, content):
        paths = [self.config / "cfg/layout.kdl", *(self.stage / "recovery").rglob("saved")]
        self.assertTrue(any(path.read_bytes() == content for path in paths))

    def test_user_save_between_recovery_writes_stops_without_losing_data(self):
        self.interrupted()
        exchange = m.replace_file
        saved = []
        def save(source, destination):
            exchange(source, destination)
            if destination == self.config / "cfg/outputs.kdl":
                saved.append(self.saved_layout())
        with patch.object(m, "replace_file", save), self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_preserved(saved[0])
        self.assertEqual((self.config / "cfg/layout.kdl").read_bytes(), saved[0])
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applying")

    def test_user_save_between_rollback_writes_stops_without_losing_data(self):
        exchange = m.replace_file
        rolling_back = False
        saved = []
        def fail(binary, config):
            nonlocal rolling_back
            if config == self.config / "config.kdl":
                rolling_back = True
                raise RuntimeError("final validation failure")
        def save(source, destination):
            exchange(source, destination)
            if rolling_back and destination == self.config / "cfg/rules.kdl":
                saved.append(self.saved_layout())
        with patch.object(m, "validate", fail), patch.object(m, "replace_file", save), \
                self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_preserved(saved[0])
        self.assertEqual((self.config / "cfg/layout.kdl").read_bytes(), saved[0])
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applying")

    def save_at_exchange(self, atomic):
        exchange = m.replace_file
        saved = []
        def save(source, destination):
            if destination == self.config / "cfg/layout.kdl" and not saved:
                saved.append(self.saved_layout(atomic=atomic))
            exchange(source, destination)
        with patch.object(m, "replace_file", save), self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_preserved(saved[0])
        with self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_preserved(saved[0])
        return saved[0]

    def test_in_place_save_after_last_check_is_retained(self):
        self.save_at_exchange(atomic=False)

    def test_atomic_editor_save_after_last_check_is_retained(self):
        self.save_at_exchange(atomic=True)

    def test_atomic_editor_save_after_exchange_remains_live(self):
        exchange = m.replace_file
        saved = []
        def save(source, destination):
            exchange(source, destination)
            if destination == self.config / "cfg/layout.kdl" and not saved:
                saved.append(self.saved_layout(atomic=True))
        with patch.object(m, "replace_file", save), self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assertEqual((self.config / "cfg/layout.kdl").read_bytes(), saved[0])
        with self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assertEqual((self.config / "cfg/layout.kdl").read_bytes(), saved[0])

    def test_old_open_file_descriptor_can_save_after_success(self):
        path = self.config / "cfg/layout.kdl"
        content = path.read_bytes().replace(b"geometry-corner-radius 12", b"geometry-corner-radius 37")
        with path.open("r+b") as editor:
            self.apply()
            editor.seek(0)
            editor.write(content)
            editor.truncate()
            editor.flush()
        self.assert_preserved(content)
        with self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_preserved(content)

    def crash_at_exchange(self, after, edit=False):
        sys.stdout.flush()
        pid = os.fork()
        if pid == 0:
            exchange = m.replace_file
            def interrupted(source, destination):
                if destination == self.config / "cfg/layout.kdl":
                    if edit:
                        self.saved_layout()
                    if not after:
                        os._exit(86)
                    exchange(source, destination)
                    os._exit(86)
                exchange(source, destination)
            with patch.object(m, "replace_file", interrupted):
                self.apply()
            os._exit(99)
        _, status = os.waitpid(pid, 0)
        self.assertEqual(os.waitstatus_to_exitcode(status), 86)

    def assert_migration_completed(self):
        self.apply()
        self.assertEqual(d.read_json(self.stage / "manifest.json")["state"], "applied")
        for name, sha in self.record["changes"].items():
            self.assertEqual(m.digest(self.config / name), sha)

    def test_crash_after_intent_before_exchange_recovers(self):
        self.crash_at_exchange(after=False)
        self.assert_migration_completed()

    def test_crash_after_exchange_and_during_recovery_recovers(self):
        self.crash_at_exchange(after=True)
        self.crash_at_exchange(after=True)
        self.assert_migration_completed()

    def test_crash_after_exchanging_user_save_requires_manual_recovery(self):
        content = (self.config / "cfg/layout.kdl").read_bytes().replace(
            b"geometry-corner-radius 12", b"geometry-corner-radius 29")
        self.crash_at_exchange(after=True, edit=True)
        with self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_preserved(content)

    def test_crash_before_intent_publication_recovers(self):
        sys.stdout.flush()
        pid = os.fork()
        if pid == 0:
            write_record = m.write_record
            def interrupted(path, record):
                if path.name == "intent.json":
                    os._exit(86)
                write_record(path, record)
            with patch.object(m, "write_record", interrupted):
                self.apply()
            os._exit(99)
        _, status = os.waitpid(pid, 0)
        self.assertEqual(os.waitstatus_to_exitcode(status), 86)
        self.assert_originals()
        self.assert_migration_completed()

    def test_unsupported_exchange_preserves_config_and_can_retry(self):
        with patch.object(m, "replace_file", side_effect=OSError(errno.EOPNOTSUPP, "unsupported")), \
                self.assertRaises(m.MigrationConflict):
            self.apply()
        self.assert_originals()
        self.assert_migration_completed()

    def test_saved_conflict_blocks_real_session_before_service_start(self):
        content = self.save_at_exchange(atomic=True)
        home = self.base / "home"
        (home / ".config").mkdir(parents=True)
        (home / ".config/niri").symlink_to(self.config, target_is_directory=True)
        (self.config / ".pending-personal").symlink_to(self.stage, target_is_directory=True)
        bundle = self.base / "bundle"
        bundle.mkdir()
        shutil.copy2(SOURCE / "scripts/personal-unify-config.py", bundle / "unify-config.py")
        binary = bundle / "niri"
        binary.write_text("#!/bin/sh\nexit 99\n")
        binary.chmod(0o755)
        commands = self.base / "commands"
        commands.mkdir()
        systemctl = commands / "systemctl"
        systemctl.write_text(
            '#!/bin/sh\nprintf "%s\\n" "$*" >> "$FIXTURE_SYSTEMCTL_LOG"\n'
            'if [ "$*" = "--user show -p ActiveState --value niri.service niri-personal.service graphical-session.target" ]; then\n'
            '  printf "inactive\\ninactive\\ninactive\\n"\n  exit 0\nfi\nexit 97\n')
        systemctl.chmod(0o755)
        log = self.base / "systemctl.log"
        env = {**os.environ, "HOME": str(home), "NIRI_PERSONAL_BUNDLE": str(bundle),
               "PATH": f"{commands}:/usr/bin:/bin", "FIXTURE_SYSTEMCTL_LOG": str(log),
               "PYTHONDONTWRITEBYTECODE": "1"}
        result = subprocess.run(
            ["/usr/bin/bash", str(SOURCE / "scripts/personal-daily-session.sh"), "--login"],
            env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn(str(self.config / ".pending-personal/recovery"), result.stderr)
        self.assertEqual(log.read_text().splitlines(), [
            "--user show -p ActiveState --value niri.service niri-personal.service graphical-session.target"])
        self.assert_preserved(content)


if __name__ == "__main__":
    unittest.main()
