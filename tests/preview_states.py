#!/usr/bin/env python3
"""Writes the plugin's states as pictures for review: python3 tests/preview_states.py OUT_DIR"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from visual_states import Renderer  # noqa: E402

renderer = Renderer(Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/omatranslate-states"))
names = renderer.render_all(animated=True)
print(f"{len(names)} states → {renderer.out}")
print("\n".join(renderer.qml_errors()))
