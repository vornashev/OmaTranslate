#!/usr/bin/env python3
"""Visual regression: renders the plugin's states and compares them with tests/visual/*.png.

    python3 tests/visual_regression.py            # compare
    python3 tests/visual_regression.py --update   # accept the current rendering as the reference

Differences are written to $OMATRANSLATE_TEST_OUTPUT/visual (default /tmp/omatranslate-ui/visual):
the rendering, and a diff image with changed pixels in red.
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from visual_states import ROOT, Renderer  # noqa: E402
from PySide6.QtGui import QColor, QImage  # noqa: E402

REFERENCE = ROOT / "tests/visual"
OUT = Path(os.environ.get("OMATRANSLATE_TEST_OUTPUT", "/tmp/omatranslate-ui")) / "visual"
# The software renderer is deterministic, so the thresholds are tight: a pixel counts
# as changed when a channel moves by more than CHANNEL_TOLERANCE; a state fails
# when more than MAX_CHANGED of its pixels change.
CHANNEL_TOLERANCE = 6
MAX_CHANGED = 0.001


def compare(actual_path: Path, reference_path: Path, diff_path: Path):
    actual, reference = QImage(str(actual_path)), QImage(str(reference_path))
    if actual.size() != reference.size():
        return f"size {actual.width()}×{actual.height()} instead of {reference.width()}×{reference.height()}"
    actual = actual.convertToFormat(QImage.Format_RGB32)
    reference = reference.convertToFormat(QImage.Format_RGB32)
    width, height = actual.width(), actual.height()
    try:
        import numpy

        def pixels(image):
            return numpy.frombuffer(image.constBits(), numpy.uint8).reshape(height, image.bytesPerLine())[:, :width * 4].reshape(height, width, 4)[..., :3].astype(int)

        mask = (numpy.abs(pixels(actual) - pixels(reference)).max(axis=2) > CHANNEL_TOLERANCE)
        changed = int(mask.sum())
        spots = zip(*numpy.nonzero(mask)) if changed else []
    except ImportError:  # slower, without numpy
        spots = [(y, x) for y in range(height) for x in range(width)
                 if max(abs(((actual.pixel(x, y) >> s) & 255) - ((reference.pixel(x, y) >> s) & 255)) for s in (0, 8, 16)) > CHANNEL_TOLERANCE]
        changed = len(spots)
    diff = reference.copy()
    for y, x in spots:
        diff.setPixelColor(int(x), int(y), QColor("#ff2d2d"))
    share = changed / (width * height)
    if share > MAX_CHANGED:
        diff.save(str(diff_path))
        return f"{share:.2%} of pixels changed"
    return None


def main():
    update = "--update" in sys.argv
    renderer = Renderer(OUT)
    if not renderer.font_available():
        print(json.dumps({"visual": "skipped", "reason": "font JetBrainsMono Nerd Font is not installed"}))
        return 0
    names = renderer.render_all(animated=False)
    errors = renderer.qml_errors()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    if update:
        REFERENCE.mkdir(parents=True, exist_ok=True)
        for name in names:
            (REFERENCE / f"{name}.png").write_bytes((OUT / f"{name}.png").read_bytes())
        print(json.dumps({"visual": "updated", "states": len(names), "reference": str(REFERENCE)}))
        return 0
    failures = {}
    for name in names:
        reference = REFERENCE / f"{name}.png"
        if not reference.exists():
            failures[name] = "no reference image (run with --update)"
            continue
        problem = compare(OUT / f"{name}.png", reference, OUT / f"{name}.diff.png")
        if problem:
            failures[name] = problem
    if failures:
        for name, problem in failures.items():
            print(f"{name}: {problem}  →  {OUT / (name + '.png')}", file=sys.stderr)
        print("If the change is intended, run: python3 tests/visual_regression.py --update", file=sys.stderr)
        return 1
    print(json.dumps({"visual": "passed", "states": len(names)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
