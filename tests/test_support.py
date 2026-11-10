import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


integration = module(ROOT / "bin/integrate.py", "integration_installer")
offline = module(ROOT / "worker/offline.py", "offline_worker")


class InstallerTests(unittest.TestCase):
    def test_replacing_managed_block_preserves_user_config(self):
        original = 'o.bind("SUPER + A", "User action", "my-app")\n'
        merged = original + integration.managed_block(integration.DEFAULTS)
        self.assertEqual(integration.without_block(merged), original)
        self.assertEqual(integration.without_block(original), original)

    def test_conflicts_are_detected_without_overwriting(self):
        binds = [dict(modmask=64, key="U", description="Existing action", arg="other-app")]
        self.assertEqual(integration.conflicts(integration.DEFAULTS, binds), ["Existing action"])
        binds[0]["arg"] = "omatranslate selection"
        self.assertEqual(integration.conflicts(integration.DEFAULTS, binds), [])

    def test_lua_binds_from_our_block_are_not_conflicts(self):
        # Hyprland's Lua config reports binds as "__lua" with a numeric arg.
        owned = integration.owned_bindings(integration.managed_block(integration.DEFAULTS))
        ours = [dict(modmask=64, key="U", dispatcher="__lua", arg="405", description="Translate selection")]
        self.assertEqual(integration.conflicts(integration.DEFAULTS, ours, owned), [])
        foreign = [dict(modmask=64, key="U", dispatcher="__lua", arg="12", description="Other app")]
        self.assertEqual(integration.conflicts(integration.DEFAULTS, foreign, owned), ["Other app"])
        self.assertEqual(integration.owned_bindings('o.bind("SUPER + U", "Translate selection", "x")\n'), set())

    def test_injection_and_duplicate_shortcuts_rejected(self):
        with self.assertRaises(ValueError):
            integration.managed_block(dict(selectionKey='SUPER + U"); os.execute("bad")'))
        settings = dict(integration.DEFAULTS, clipboardKey="SUPER + U")
        self.assertTrue(integration.conflicts(settings, []))


class ModelIntegrityTests(unittest.TestCase):
    def test_corruption_is_detected_before_inference(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "model").mkdir()
            model = root / "model/model.bin"
            model.write_bytes(b"model data")
            (root / "integrity.json").write_text(json.dumps({"files": {"model/model.bin": hashlib.sha256(b"model data").hexdigest()}}))
            offline.verify(root)
            model.write_bytes(b"corrupted")
            with self.assertRaises(offline.OfflineError):
                offline.verify(root)

    def test_integrity_manifest_cannot_escape_model_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "integrity.json").write_text(json.dumps({"files": {"model/model.bin": "x", "../outside": "x"}}))
            with self.assertRaises(offline.OfflineError):
                offline.verify(root)


if __name__ == "__main__":
    unittest.main()
