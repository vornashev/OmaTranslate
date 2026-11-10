"""Verify durable integration and recovery from a temporarily unavailable shell."""
from contextlib import redirect_stdout
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installer", ROOT / "bin/integrate.py")
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class Shell:
    def __init__(self, config):
        self.config = config
        self.root = config / "omarchy/plugins"
        self.calls = []
        self.pending = 0
        self.waits = 0
        self.config_error = ""

    def run(self, args, **kwargs):
        self.calls.append(args)
        if args == ["omarchy-shell", "shell", "rescanPlugins"]:
            self.pending = 2
            return ""
        if args == ["omarchy-shell", "shell", "listPlugins"]:
            if self.pending:
                self.pending -= 1
                self.waits += 1
                raise installer.ShellUnavailable("omarchy-shell is not responding")
            config = json.loads((self.config / "omarchy/shell.json").read_text())
            entries = [entry for entries in config["bar"]["layout"].values() for entry in entries]
            enabled = {entry["id"] for entry in entries + config.get("plugins", [])}
            return json.dumps([dict(id=p.parent.name, enabled=p.parent.name in enabled,
                                   kinds=json.loads(p.read_text())["kinds"])
                               for p in self.root.glob("*/manifest.json")])
        if args == ["omarchy", "restart", "shell"]:
            self.pending = 0
            return ""
        if args[:3] == ["omarchy", "plugin", "validate"]:
            assert json.loads((Path(args[3]) / "manifest.json").read_text())["id"] == installer.PLUGIN_ID
            return ""
        if args == ["hyprctl", "binds", "-j"]:
            return "[]"
        if args == ["hyprctl", "reload"]:
            return "ok"
        if args == ["hyprctl", "-j", "configerrors"]:
            return json.dumps([self.config_error] if self.config_error else [])
        raise AssertionError(args)


class InstallerLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="omatranslate-installer-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.config = self.base / "config"
        self.root = self.config / "omarchy/plugins"
        self.legacy = self.root / installer.LEGACY_ID
        self.current = self.root / installer.PLUGIN_ID
        self.root.mkdir(parents=True)
        installer.copy_payload(ROOT, self.legacy)
        manifest = json.loads((self.legacy / "manifest.json").read_text())
        manifest["id"] = installer.LEGACY_ID
        (self.legacy / "manifest.json").write_text(json.dumps(manifest))
        (self.legacy / "local-note").write_text("Preserve the user's old copy")
        self.shell_config = self.config / "omarchy/shell.json"
        self.original_config = {"version": 1, "bar": {"position": "top", "layout": {
            "left": [{"id": "other.left", "setting": "keep"}], "center": [{"id": "omarchy.clock"}],
            "right": [{"id": "other.right"}]}},
            "plugins": [{"id": "other.overlay"}, {"id": installer.LEGACY_ID, "locale": "ru"}],
            "idle": {"lock": 300}}
        self.shell_config.write_text(json.dumps(self.original_config))
        self.bindings = self.config / "hypr/bindings.lua"
        self.bindings.parent.mkdir()
        self.original = 'o.bind("SUPER + A", "User action", "my-app")\n'
        self.bindings.write_text(self.original)
        self.settings = self.config / "omatranslate/settings.json"
        self.settings.parent.mkdir()
        self.settings.write_text('{"provider":"local"}')
        self.shell = Shell(self.config)
        self.addCleanup(patch.stopall)
        patch.dict(os.environ, XDG_CONFIG_HOME=str(self.config), XDG_STATE_HOME=str(self.base / "state")).start()
        patch.object(installer, "run", self.shell.run).start()
        patch.object(installer.time, "sleep", return_value=None).start()
        self.stop = patch.object(installer, "stop_backend").start()

    def install(self, *args):
        with redirect_stdout(io.StringIO()):
            installer.main(list(args))

    def test_transient_ipc_failure_keeps_migration_and_repeated_install_preserves_placement(self):
        self.install()
        self.assertEqual(self.shell.waits, 2)
        self.assertFalse(self.legacy.exists())
        self.assertTrue(self.current.is_dir())
        self.assertEqual(json.loads((self.current / "manifest.json").read_text())["author"], "vornashev")
        backups = list((self.base / "state/omatranslate/backups").iterdir())
        self.assertEqual((backups[0] / installer.LEGACY_ID / "local-note").read_text(), "Preserve the user's old copy")
        self.assertEqual(json.loads((backups[0] / "shell.json").read_text()), self.original_config)
        self.assertEqual(self.settings.read_text(), '{"provider":"local"}')
        config = json.loads(self.shell_config.read_text())
        self.assertEqual(config["plugins"], [{"id": "other.overlay"}])
        self.assertEqual(config["bar"]["layout"]["right"][-1], {"id": installer.PLUGIN_ID, "locale": "ru"})
        config["bar"]["layout"]["left"].insert(0, config["bar"]["layout"]["right"].pop())
        self.shell_config.write_text(json.dumps(config))
        self.install()
        self.assertEqual(json.loads(self.shell_config.read_text()), config)
        self.assertEqual(self.bindings.read_text().count(installer.START), 1)
        self.assertIn(self.original, self.bindings.read_text())
        self.assertEqual(self.stop.call_count, 2)
        self.assertIn(["hyprctl", "-j", "configerrors"], self.shell.calls)
        self.assertEqual(self.shell.calls.count(["omarchy-shell", "shell", "rescanPlugins"]), 2)
        self.assertNotIn(["omarchy", "restart", "shell"], self.shell.calls)

    def test_changed_qml_of_a_loaded_plugin_restarts_the_shell_instead_of_rescan(self):
        self.install()
        rescans = self.shell.calls.count(["omarchy-shell", "shell", "rescanPlugins"])
        # An older interface is installed; the shell caches its QML types.
        stale = next((self.current / "plugin").glob("*.qml"))
        stale.write_text(stale.read_text() + "\n// old\n")
        self.install()
        self.assertEqual(self.shell.calls.count(["omarchy", "restart", "shell"]), 1)
        self.assertEqual(self.shell.calls.count(["omarchy-shell", "shell", "rescanPlugins"]), rescans)

    def test_shell_recovery_restarts_once_and_never_repeats_rescan(self):
        with patch.object(installer, "wait_for_plugin", side_effect=[installer.ShellUnavailable("timeout"), None]):
            self.install()
        self.assertEqual(self.shell.calls.count(["omarchy", "restart", "shell"]), 1)
        self.assertEqual(self.shell.calls.count(["omarchy-shell", "shell", "rescanPlugins"]), 1)
        self.assertTrue(self.current.exists())

    def test_rescan_timeout_does_not_resubmit_a_possibly_completed_mutation(self):
        original_run = self.shell.run

        def timeout_after_rescan(args, **kwargs):
            result = original_run(args, **kwargs)
            if args[-1] == "rescanPlugins":
                raise installer.ShellUnavailable("omarchy-shell is not responding")
            return result

        with patch.object(installer, "run", side_effect=timeout_after_rescan):
            self.install()
        self.assertEqual(self.shell.calls.count(["omarchy-shell", "shell", "rescanPlugins"]), 1)
        self.assertNotIn(["omarchy", "restart", "shell"], self.shell.calls)
        self.assertTrue(self.current.exists())

    def test_permanent_shell_failure_keeps_installed_files_and_bar_entry(self):
        with patch.object(installer, "wait_for_plugin", side_effect=installer.ShellUnavailable("timeout")), \
             self.assertRaisesRegex(installer.ActivationPending, "Saved configuration was kept"):
            self.install()
        self.assertTrue(self.current.exists())
        self.assertFalse(self.legacy.exists())
        self.assertIn(installer.START, self.bindings.read_text())
        config = json.loads(self.shell_config.read_text())
        self.assertEqual(config["bar"]["layout"]["right"][-1]["id"], installer.PLUGIN_ID)
        self.assertEqual(self.shell.calls.count(["omarchy", "restart", "shell"]), 1)
        self.assertEqual(self.shell.calls.count(["omarchy-shell", "shell", "rescanPlugins"]), 1)

    def test_hyprland_failure_rolls_back_files_and_bindings_without_shell_ipc(self):
        self.shell.config_error = "bindings.lua: invalid configuration"
        with self.assertRaisesRegex(installer.IntegrationError, "invalid configuration"):
            self.install()
        self.assertFalse(self.current.exists())
        self.assertTrue(self.legacy.exists())
        self.assertEqual(self.bindings.read_text(), self.original)
        self.assertEqual(json.loads(self.shell_config.read_text()), self.original_config)
        self.assertFalse(any(call[0] == "omarchy-shell" for call in self.shell.calls))
        self.stop.assert_not_called()

    def test_invalid_shell_json_restores_files_and_shortcuts_without_overwriting_it(self):
        self.shell_config.write_text("invalid JSON")
        with self.assertRaises(ValueError):
            self.install()
        self.assertEqual(self.shell_config.read_text(), "invalid JSON")
        self.assertEqual(self.bindings.read_text(), self.original)
        self.assertFalse(self.current.exists())
        self.assertTrue(self.legacy.exists())

    def test_update_failure_restores_existing_current_plugin(self):
        installer.copy_payload(ROOT, self.current)
        (self.current / "local-note").write_text("existing current version")
        self.shell.config_error = "invalid configuration"
        with self.assertRaises(installer.IntegrationError):
            self.install()
        self.assertEqual((self.current / "local-note").read_text(), "existing current version")

    def test_marketplace_checkout_and_local_qml_are_preserved(self):
        installer.copy_payload(ROOT, self.current)
        (self.current / ".git").mkdir()
        (self.current / ".git/HEAD").write_text("ref: refs/heads/main\n")
        qml = self.current / "plugin/Translate.qml"
        qml.write_text(qml.read_text() + "\n// User customization\n")
        original = qml.read_text()
        self.install()
        self.assertEqual((self.current / ".git/HEAD").read_text(), "ref: refs/heads/main\n")
        self.assertEqual(qml.read_text(), original)

    def test_remove_cleans_both_ids_and_preserves_user_bindings(self):
        self.install()
        self.install("--remove")
        self.assertFalse(self.current.exists())
        self.assertFalse(self.legacy.exists())
        config = json.loads(self.shell_config.read_text())
        expected = self.original_config.copy()
        expected["plugins"] = [{"id": "other.overlay"}]
        self.assertEqual(config, expected)
        self.assertNotIn(installer.START, self.bindings.read_text())
        self.assertIn(self.original, self.bindings.read_text())
        self.assertEqual(self.settings.read_text(), '{"provider":"local"}')


class ShellConfigTests(unittest.TestCase):
    def test_existing_placement_wins_and_duplicate_legacy_entries_are_removed(self):
        config = {"version": 1, "bar": {"layout": {
            "left": [{"id": installer.LEGACY_ID}, {"id": "other.widget"}],
            "tray": [{"id": "other.tray"}, {"id": installer.PLUGIN_ID, "locale": "ru"}]}},
            "plugins": [{"id": installer.PLUGIN_ID}, {"id": "other.overlay"}],
            "disabledPlugins": [installer.PLUGIN_ID, installer.LEGACY_ID, "other.disabled"]}
        updated = installer.configured_shell(config)
        self.assertEqual(updated["bar"]["layout"]["tray"], config["bar"]["layout"]["tray"])
        self.assertEqual(updated["bar"]["layout"]["left"], [{"id": "other.widget"}])
        self.assertEqual(updated["plugins"], [{"id": "other.overlay"}])
        self.assertEqual(updated["disabledPlugins"], ["other.disabled"])
        self.assertEqual(installer.configured_shell(updated), updated)
        self.assertEqual(config["bar"]["layout"]["left"][0]["id"], installer.LEGACY_ID)

    def test_legacy_widget_migrates_in_place_with_settings(self):
        config = {"version": 1, "bar": {"layout": {"centerEnd": [
            {"id": "before"}, {"id": installer.LEGACY_ID, "locale": "ru"}, {"id": "after"}]}}}
        updated = installer.configured_shell(config)
        self.assertEqual(updated["bar"]["layout"]["centerEnd"], [
            {"id": "before"}, {"id": installer.PLUGIN_ID, "locale": "ru"}, {"id": "after"}])
        self.assertNotIn("right", updated["bar"]["layout"])

    def test_invalid_config_shapes_are_not_silently_replaced(self):
        for config in ([], {}, {"version": 1, "bar": []}, {"version": 1, "bar": {"layout": {"right": {}}}},
                       {"version": 1, "plugins": {}}, {"version": 1, "disabledPlugins": {}}):
            with self.subTest(config=config), self.assertRaises(installer.IntegrationError):
                installer.configured_shell(config)

    def test_missing_user_config_copies_stock_layout_before_adding_widget(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            stock = base / "stock/config/omarchy/shell.json"
            stock.parent.mkdir(parents=True)
            original = {"version": 1, "bar": {"layout": {"left": [{"id": "omarchy.menu"}],
                        "right": [{"id": "omarchy.audio"}]}}}
            stock.write_text(json.dumps(original))
            with patch.dict(os.environ, OMARCHY_PATH=str(base / "stock")):
                installer.save_shell_config(base / "config", base / "backup")
            saved = json.loads((base / "config/omarchy/shell.json").read_text())
            self.assertEqual(saved["bar"]["layout"]["left"], original["bar"]["layout"]["left"])
            self.assertEqual(saved["bar"]["layout"]["right"], [{"id": "omarchy.audio"}, {"id": installer.PLUGIN_ID}])

    def test_readiness_timeout_reports_last_ipc_error(self):
        with patch.object(installer, "plugins", side_effect=installer.ShellUnavailable("shell is not responding")), \
             self.assertRaisesRegex(installer.ShellUnavailable, "not responding"):
            installer.wait_for_plugin(installer.PLUGIN_ID, timeout=0)


class InstallerDiagnosticsTests(unittest.TestCase):
    def test_shell_timeout_is_extended_and_classified_as_transient(self):
        result = subprocess.CompletedProcess([], 1, "", "omarchy-shell is not responding")
        with patch.object(installer.subprocess, "run", return_value=result) as command, \
             self.assertRaises(installer.ShellUnavailable):
            installer.run(["omarchy-shell", "shell", "listPlugins"])
        self.assertEqual(command.call_args.kwargs["env"]["OMARCHY_SHELL_IPC_TIMEOUT"], "8s")

    def test_pending_activation_is_distinct_from_file_installation_failure(self):
        with tempfile.TemporaryDirectory() as temporary, patch.dict(os.environ, XDG_STATE_HOME=temporary), \
             patch.object(installer, "main", side_effect=installer.ActivationPending("Files installed; shell unavailable")), \
             redirect_stdout(io.StringIO()), patch("sys.stderr", new=io.StringIO()):
            self.assertEqual(installer.cli(), 1)
            report = json.loads((Path(temporary) / "omatranslate/last-integration.json").read_text())
        self.assertEqual(report["status"], "activation-pending")
        self.assertIn("Files installed", report["error"])

    def test_command_error_includes_stderr_and_stdout(self):
        with self.assertRaises(installer.CommandError) as raised:
            installer.run([sys.executable, "-c", "import sys; print('details'); print('shell failure', file=sys.stderr); sys.exit(7)"])
        self.assertIn("exit 7", str(raised.exception))
        self.assertIn("shell failure", str(raised.exception))
        self.assertIn("details", str(raised.exception))

    def test_cli_shows_error_without_traceback(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            (base / "hypr").mkdir()
            (base / "hypr/bindings.lua").write_text("")
            command = base / "hyprctl"
            command.write_text("#!/bin/sh\necho 'compositor is unavailable' >&2\nexit 1\n")
            command.chmod(0o755)
            result = subprocess.run([sys.executable, str(ROOT / "bin/integrate.py")],
                                    env=dict(os.environ, XDG_CONFIG_HOME=str(base), XDG_STATE_HOME=str(base / "state"), PATH=str(base)),
                                    text=True, capture_output=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("compositor is unavailable", result.stderr)
            self.assertNotIn("Traceback", result.stderr)
            report = base / "state/omatranslate/last-integration.json"
            diagnostic = json.loads(report.read_text())
            self.assertEqual(diagnostic["status"], "failed")
            self.assertIn("compositor is unavailable", diagnostic["error"])
            self.assertEqual(report.stat().st_mode & 0o777, 0o600)

    def test_json_config_errors_require_an_array_and_preserve_real_errors(self):
        for response, expected in [("[]\n", []), ('["", "  "]', []), ('["bindings.lua: invalid binding"]', ["bindings.lua: invalid binding"])]:
            with self.subTest(response=response), patch.object(installer, "run", return_value=response):
                self.assertEqual(installer.config_errors(), expected)
        for response in ("unknown request", '{}', '[42]'):
            with self.subTest(response=response), patch.object(installer, "run", return_value=response), self.assertRaises(installer.IntegrationError):
                installer.config_errors()


@unittest.skipUnless(shutil.which("omarchy"), "needs the real omarchy plugin validator (not on CI)")
class InstallerPayloadTests(unittest.TestCase):
    def test_makepkg_symlinks_are_not_part_of_runtime_validation(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source"
            installer.copy_payload(ROOT, source)
            (source / "build").mkdir()
            (source / "build/source.tar.gz").symlink_to(ROOT / "README.md")
            whole_repo = subprocess.run(["omarchy", "plugin", "validate", str(source)], capture_output=True, text=True)
            self.assertNotEqual(whole_repo.returncode, 0)
            self.assertIn("symlinks", whole_repo.stderr)
            installer.validate_payload(source)

    def test_actual_runtime_symlinks_are_still_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source"
            installer.copy_payload(ROOT, source)
            entry = source / "plugin/Translate.qml"
            entry.unlink()
            entry.symlink_to(ROOT / "plugin/Translate.qml")
            with self.assertRaisesRegex(installer.CommandError, "symlinks"):
                installer.validate_payload(source)


class BackendUpgradeTests(unittest.TestCase):
    def setUp(self):
        self.addCleanup(patch.stopall)
        patch.object(installer.Path, "exists", return_value=True).start()
        self.connection = patch.object(installer.socket, "socket").start().return_value.__enter__.return_value
        self.connection.getsockopt.return_value = struct.pack("3i", 12345, os.geteuid(), os.getegid())
        self.open_pid = patch.object(installer.os, "pidfd_open", return_value=91).start()
        self.close_pid = patch.object(installer.os, "close").start()
        self.send_signal = patch.object(installer.signal, "pidfd_send_signal").start()
        patch.object(installer.os, "readlink", return_value="/usr/bin/omatranslate (deleted)").start()
        self.cmdline = patch.object(installer.Path, "read_bytes", return_value=b"/usr/bin/omatranslate\0serve\0").start()

    def test_upgrade_stops_only_verified_service_via_pidfd(self):
        installer.stop_backend()
        self.open_pid.assert_called_once_with(12345)
        self.send_signal.assert_called_once_with(91, signal.SIGTERM)
        self.close_pid.assert_called_once_with(91)

    def test_foreign_socket_is_never_signalled(self):
        self.connection.getsockopt.return_value = struct.pack("3i", 12345, os.geteuid() + 1, os.getegid())
        with self.assertRaises(installer.IntegrationError):
            installer.stop_backend()
        self.open_pid.assert_not_called()
        self.send_signal.assert_not_called()

    def test_non_service_process_is_never_signalled(self):
        self.cmdline.return_value = b"/usr/bin/omatranslate\0translate\0user text\0"
        with self.assertRaises(installer.IntegrationError):
            installer.stop_backend()
        self.send_signal.assert_not_called()
        self.close_pid.assert_called_once_with(91)


if __name__ == "__main__":
    unittest.main()
