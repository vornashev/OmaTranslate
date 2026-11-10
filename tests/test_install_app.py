"""Verify one-command orchestration without invoking sudo or changing the desktop."""
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "bin"))
import install_app as app
import package_app as package


class PackageCacheTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="omatranslate-cache-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "install.sh").write_text("installer version one")
        self.inputs = patch.object(package, "INPUTS", ("install.sh",))
        self.inputs.start()
        self.addCleanup(self.inputs.stop)
        self.directory = package.build_directory(self.root)
        self.directory.mkdir(parents=True)
        self.artifact = self.directory / "omatranslate-test.pkg.tar.zst"
        self.artifact.write_bytes(b"native package")
        package.record_packages([self.artifact], package.fingerprint(self.root), self.root)

    def test_reuse_requires_matching_sources_and_artifact(self):
        self.assertEqual(package.cached_packages(self.root), [self.artifact])
        (self.root / "install.sh").write_text("updated installer")
        self.assertIsNone(package.cached_packages(self.root))

    def test_corrupt_package_and_path_escape_are_rejected(self):
        self.artifact.write_bytes(b"corrupted")
        self.assertIsNone(package.cached_packages(self.root))
        record = self.directory / "artifact.json"
        data = json.loads(record.read_text())
        data["packages"][0]["name"] = "../outside.pkg.tar.zst"
        record.write_text(json.dumps(data))
        self.assertIsNone(package.cached_packages(self.root))

    def test_marketplace_builds_are_outside_watched_plugin(self):
        config = self.root / "config"
        checkout = config / "omarchy/plugins/vornashev.omatranslate"
        cache = self.root / "cache"
        with patch.dict(os.environ, XDG_CONFIG_HOME=str(config), XDG_CACHE_HOME=str(cache)):
            directory = package.build_directory(checkout)
        self.assertEqual(directory, cache / "omatranslate/package")
        self.assertFalse(directory.is_relative_to(checkout))


class InstallationFlowTests(unittest.TestCase):
    def setUp(self):
        self.addCleanup(patch.stopall)
        self.calls = []
        self.requirements = {"depends": ["cairo", "tesseract-data-rus"], "makedepends": ["rust", "cmake"]}
        patch.object(package, "dependencies", return_value=self.requirements).start()
        self.cached = patch.object(package, "cached_packages", return_value=[Path("/tmp/current.pkg.tar.zst")]).start()
        self.build = patch.object(package, "build", side_effect=self.build_package).start()
        self.missing = patch.object(app, "missing_dependencies", return_value=[]).start()
        self.run = patch.object(app, "run", side_effect=lambda args, **kwargs: self.calls.append([str(a) for a in args])).start()
        self.args = SimpleNamespace(rebuild=False, offline_engine=False, uninstall=False, repair=False)

    def build_package(self, **kwargs):
        self.calls.append(["build"])
        return [Path("/tmp/current.pkg.tar.zst")]

    def execute(self):
        with redirect_stdout(io.StringIO()):
            app.execute(self.args)

    def test_ready_package_does_not_require_compiler(self):
        self.execute()
        self.missing.assert_called_once_with(self.requirements["depends"])
        self.build.assert_called_once_with(reuse=True)
        self.assertEqual(self.calls, [["sudo", "-v"], ["build"],
                                     ["sudo", "pacman", "-U", "/tmp/current.pkg.tar.zst"],
                                     [sys.executable, str(ROOT / "bin/integrate.py")]])

    def test_fresh_build_installs_dependencies_first(self):
        self.cached.return_value = None
        self.missing.return_value = ["rust", "tesseract-data-rus"]
        self.args.offline_engine = True
        self.execute()
        requested = self.missing.call_args.args[0]
        self.assertIn("base-devel", requested)
        self.assertIn("rust", requested)
        self.assertIn("python-pip", requested)
        self.assertEqual(self.calls[1], ["sudo", "pacman", "-S", "--needed", "rust", "tesseract-data-rus"])
        self.assertLess(self.calls.index(["build"]), len(self.calls) - 1)
        self.assertEqual(self.calls[-2], ["bash", str(ROOT / "bin/setup-offline")])

    def test_package_failure_never_enables_plugin(self):
        def fail_install(args, **kwargs):
            if "-U" in args:
                raise subprocess.CalledProcessError(1, args)
        self.run.side_effect = fail_install
        with self.assertRaises(subprocess.CalledProcessError):
            self.execute()
        self.assertFalse(any("integrate.py" in str(a) for c in self.run.call_args_list for a in c.args[0]))

    def test_no_terminal_hands_off_exact_arguments_once(self):
        with patch.object(app, "terminal_available", return_value=False), \
             patch.object(app.shutil, "which", return_value="/usr/bin/omarchy-launch-terminal"), \
             patch.object(app, "write_report"), patch.object(app, "wait_for_result") as wait, \
             patch.dict(os.environ, {"OMATRANSLATE_INSTALL_TERMINAL": ""}), redirect_stdout(io.StringIO()):
            self.assertFalse(app.ensure_terminal(["--offline-engine"], run_id="a" * 32))
        wait.assert_called_once_with("a" * 32)
        self.assertEqual(self.calls, [["omarchy-launch-terminal", "env", "OMATRANSLATE_INSTALL_TERMINAL=1",
                                      "OMATRANSLATE_INSTALL_RUN=" + "a" * 32, "bash",
                                      str(ROOT / "install.sh"), "--offline-engine"]])

    def test_failed_terminal_handoff_does_not_recurse(self):
        with patch.object(app, "terminal_available", return_value=False), \
             patch.dict(os.environ, OMATRANSLATE_INSTALL_TERMINAL="1"), self.assertRaisesRegex(RuntimeError, "desktop terminal"):
            app.ensure_terminal([])
        self.run.assert_not_called()

    def test_root_invocation_is_rejected_before_any_change(self):
        with patch.object(app.os, "geteuid", return_value=0), self.assertRaisesRegex(RuntimeError, "without sudo"):
            app.main([])
        self.run.assert_not_called()

    def test_uninstall_removes_integration_before_package(self):
        self.args.uninstall = True
        with patch.object(app.subprocess, "run", return_value=SimpleNamespace(returncode=0)):
            self.execute()
        self.assertEqual(self.calls, [[sys.executable, str(ROOT / "bin/integrate.py"), "--remove"],
                                     ["sudo", "pacman", "-R", "omatranslate"]])

    def test_repair_skips_sudo_dependencies_and_compilation(self):
        self.args.repair = True
        self.execute()
        self.assertEqual(self.calls, [[sys.executable, str(ROOT / "bin/integrate.py")]])
        self.build.assert_not_called()
        self.missing.assert_not_called()

    def test_repair_does_not_need_a_terminal_or_package_manager(self):
        with patch.object(app.os, "geteuid", return_value=1000), \
             patch.object(app.shutil, "which", side_effect=lambda command: None if command in ("pacman", "makepkg") else "/usr/bin/" + command), \
             patch.object(app, "ensure_terminal") as terminal, redirect_stdout(io.StringIO()):
            self.assertTrue(app.main(["--repair"]))
        terminal.assert_not_called()
        self.assertEqual(self.calls, [[sys.executable, str(ROOT / "bin/integrate.py")]])

    def test_integration_failure_surfaces_saved_diagnostic(self):
        with tempfile.TemporaryDirectory() as temporary, patch.dict(os.environ, XDG_STATE_HOME=temporary):
            def fail(args, **kwargs):
                report = Path(temporary) / "omatranslate/last-integration.json"
                report.parent.mkdir()
                report.write_text(json.dumps({"error": "Installed, but the shell is not responding"}))
                raise subprocess.CalledProcessError(1, args)
            self.run.side_effect = fail
            with patch.object(app.time, "time_ns", return_value=0), self.assertRaisesRegex(RuntimeError, "shell is not responding"):
                app.integrate()


class InstallationResultTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="omatranslate-result-")
        self.addCleanup(self.temp.cleanup)
        self.environment = patch.dict(os.environ, XDG_STATE_HOME=self.temp.name, OMATRANSLATE_INSTALL_TERMINAL="")
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.run_id = "b" * 32

    def test_completed_result_is_private_and_read_back(self):
        app.write_report(self.run_id, "success")
        with redirect_stdout(io.StringIO()):
            app.wait_for_result(self.run_id, timeout=0)
        self.assertEqual(app.report_path(self.run_id).stat().st_mode & 0o777, 0o600)
        latest = Path(self.temp.name) / "omatranslate/last-install.json"
        self.assertEqual(json.loads(latest.read_text())["status"], "success")

    def test_terminal_exit_zero_does_not_hide_installation_failure(self):
        def launched(args):
            app.write_report(self.run_id, "failed", "Hyprland configuration errors: invalid binding")
            return SimpleNamespace(returncode=0)
        with patch.object(app, "terminal_available", return_value=False), \
             patch.object(app.shutil, "which", return_value="/usr/bin/omarchy-launch-terminal"), \
             patch.object(app, "run", side_effect=launched), redirect_stdout(io.StringIO()), \
             self.assertRaisesRegex(RuntimeError, "invalid binding"):
            app.ensure_terminal([], run_id=self.run_id)

    def test_closed_terminal_without_a_result_is_not_success(self):
        app.write_report(self.run_id, "running")
        with patch.object(app.os, "kill", side_effect=ProcessLookupError), self.assertRaisesRegex(RuntimeError, "without completing"):
            app.wait_for_result(self.run_id, timeout=0)

    def test_unstarted_installer_is_not_success(self):
        app.write_report(self.run_id, "starting")
        with self.assertRaisesRegex(RuntimeError, "did not start"):
            app.wait_for_result(self.run_id, timeout=0)


if __name__ == "__main__":
    unittest.main()
