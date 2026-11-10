#!/usr/bin/env python3
"""Exercise the real Rust daemon over IPC, including actual Tesseract and PNG rendering.

Only translation inference is replaced by a deterministic protocol fixture. It is
not a measurement of translation quality or real model inference speed.
"""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("OMATRANSLATE_TEST_BINARY", ROOT / "target/debug/omatranslate"))


class Integration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="omatranslate-test-")
        cls.root = Path(cls.temp.name)
        cls.path = str(cls.root / "service.sock")
        cls.stdio = False
        try:
            with socket.socket(socket.AF_UNIX) as probe:
                probe.bind(str(cls.root / "probe.sock"))
        except PermissionError:
            cls.stdio = True
            print("Socket creation denied: using the diagnostic stdio transport; Unix transport is not exercised.")
        worker = cls.root / "worker.py"
        worker.write_text('''import json,sys,time
print(json.dumps({"event":"ready"}),flush=True)
for line in sys.stdin:
 r=json.loads(line)
 if any("slow" in x for x in r["segments"]): time.sleep(0.7)
 if any("fail" in x for x in r["segments"]): answer={"error":"offlineFailed"}
 else: answer={"translations":["Перевод: "+x for x in r["segments"]]}
 print(json.dumps(answer,ensure_ascii=False),flush=True)
''')
        env = dict(os.environ, XDG_CONFIG_HOME=str(cls.root / "config"), XDG_DATA_HOME=str(cls.root / "data"),
                   OMATRANSLATE_SOCKET=cls.path, OMATRANSLATE_WORKER=str(worker), OMATRANSLATE_NO_SHELL="1")
        cls.stderr = (cls.root / "stderr").open("w+")
        cls.daemon = subprocess.Popen([str(BINARY), "serve"] + (["--stdio"] if cls.stdio else []), env=env,
                                     stdin=subprocess.PIPE if cls.stdio else subprocess.DEVNULL,
                                     stdout=subprocess.PIPE if cls.stdio else subprocess.DEVNULL, stderr=cls.stderr, bufsize=0)
        for _ in range(100):
            if cls.stdio or Path(cls.path).exists():
                break
            if cls.daemon.poll() is not None:
                cls.stderr.seek(0)
                raise AssertionError(cls.stderr.read())
            time.sleep(0.02)
        cls.env = env

    @classmethod
    def tearDownClass(cls):
        cls.daemon.terminate()
        cls.daemon.wait(timeout=5)
        cls.stderr.close()
        cls.temp.cleanup()

    def setUp(self):
        if self.stdio:
            self.reader, self.writer = self.daemon.stdout, self.daemon.stdin
        else:
            self.sock = socket.socket(socket.AF_UNIX)
            self.sock.settimeout(10)
            self.sock.connect(self.path)
            self.file = self.sock.makefile("rwb", buffering=0)
            self.reader = self.writer = self.file
        self.request("setSettings", settings=dict(provider="local", source="en", target="ru", ocrLanguages="eng", history=False))
        self.request("dismiss")

    def tearDown(self):
        if not self.stdio:
            self.file.close()
            self.sock.close()

    def request(self, op, **kwargs):
        self.writer.write((json.dumps(dict(v=1, op=op, **kwargs)) + "\n").encode())
        import select
        if not select.select([self.reader], [], [], 20)[0]:
            self.fail("Daemon did not respond")
        return json.loads(self.reader.readline())

    def wait_state(self, predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            state = self.request("status")
            if predicate(state):
                return state
            time.sleep(0.03)
        self.fail("State never reached: " + json.dumps(state))

    def test_translation_and_no_disk_history(self):
        response = self.request("translate", text="Hello world")
        state = self.wait_state(lambda s: s["job"] == response["job"] and s["status"] == "ready")
        self.assertEqual(state["translated"], "Перевод: Hello world")
        self.assertFalse((self.root / "data/omatranslate/history.json").exists())

    def test_newest_request_wins(self):
        self.request("translate", text="slow old request")
        time.sleep(0.05)
        latest = self.request("translate", text="new request")
        state = self.wait_state(lambda s: s["job"] == latest["job"] and s["status"] == "ready")
        self.assertEqual(state["translated"], "Перевод: new request")
        time.sleep(0.8)
        self.assertEqual(self.request("status")["translated"], state["translated"])

    def test_image_ocr_render_export_and_cleanup(self):
        fixture = Path("/tmp/omatranslate-ui/ocr-input.png")
        self.assertTrue(fixture.exists(), "Run ui_smoke.py first")
        ack = self.request("image", path=str(fixture))
        state = self.wait_state(lambda s: s["job"] == ack["job"] and s["status"] in ("ready", "error"))
        self.assertEqual(state["status"], "ready", state)
        self.assertIn("Welcome", state["sourceText"])
        self.assertTrue(state["blocks"])
        rendered = Path(state["translatedImage"])
        import shutil
        shutil.copyfile(rendered, "/tmp/omatranslate-ui/ocr-rendered.png")
        self.assertEqual(rendered.read_bytes()[:8], b"\x89PNG\r\n\x1a\n")
        output = self.root / "export.png"
        self.assertEqual(self.request("save", path=str(output))["event"], "ack")
        self.assertEqual(output.read_bytes(), rendered.read_bytes())
        self.assertEqual(self.request("save", path=str(output))["error"], "fileExists")
        self.assertEqual(self.request("save", path=str(fixture), overwrite=True)["error"], "originalProtected")
        self.assertEqual(self.request("editBlock", index=0, text="x" * 10001)["error"], "inputTooLarge")
        self.assertEqual(self.request("status")["status"], "ready")
        self.request("editBlock", index=0, text="Corrected recognition")
        state = self.wait_state(lambda s: s["status"] == "ready" and s["blocks"][0]["source"] == "Corrected recognition")
        self.assertIn("Corrected recognition", state["blocks"][0]["translated"])
        self.request("dismiss")
        self.assertFalse(rendered.exists())
        self.assertTrue(output.exists())

    def test_errors_preserve_source(self):
        self.request("translate", text="fail this request")
        state = self.wait_state(lambda s: s["status"] == "error")
        self.assertEqual(state["sourceText"], "fail this request")
        self.assertEqual(state["error"], "offlineFailed")

    def test_history_opt_in_and_clear(self):
        self.request("setSettings", settings=dict(history=True))
        self.request("translate", text="Saved phrase")
        state = self.wait_state(lambda s: s["status"] == "ready")
        self.assertEqual(state["history"][0]["source"], "Saved phrase")
        self.request("clearHistory")
        self.assertEqual(self.request("status")["history"], [])

    def test_validation_and_private_settings(self):
        self.assertEqual(self.request("setSettings", settings=dict(provider="unexpected"))["error"], "invalidProvider")
        settings = self.root / "config/omatranslate/settings.json"
        self.assertEqual(settings.stat().st_mode & 0o777, 0o600)
        if not self.stdio:
            self.assertEqual(Path(self.path).stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main(verbosity=2)
