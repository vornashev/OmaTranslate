"""Render the real plugin QML in every designed state, in a dark and a light theme.

Shared by tests/visual_regression.py (compares with tests/visual/*.png) and
tests/preview_states.py (writes the pictures for review).
"""
import json
import os
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.setdefault("QT_QUICK_BACKEND", "software")
os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
os.environ["QT_QPA_PLATFORMTHEME"] = ""
from PySide6.QtCore import QObject, QPoint, QSize, Qt, QUrl, qInstallMessageHandler  # noqa: E402
from PySide6.QtGui import QColor, QFontDatabase, QGuiApplication  # noqa: E402
from PySide6.QtQml import QQmlComponent, QQmlEngine  # noqa: E402
from PySide6.QtQuick import QQuickWindow  # noqa: E402
from PySide6.QtTest import QTest  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/fixtures/visual"
FONT = "JetBrainsMono Nerd Font"
THEMES = {
    "dark": dict(background="#13171a", text="#e3e7e8", accent="#aac8ad", border="#2a3236", page="#0b0e10"),
    "light": dict(background="#f7f6f2", text="#1e2326", accent="#4a7a57", border="#d9d7ce", page="#dcdbd3"),
}


def language_catalog():
    """The backend's catalog (src/languages.rs), as the state carries it."""
    import re
    rows = re.findall(r'\("([^"]+)", "[a-z]*", "([a-z_]+)", "([^"]+)", "([^"]+)"\)', (ROOT / "src/languages.rs").read_text())
    return [dict(code=c, ocr=o, name=n, english=e) for c, o, n, e in rows]


LANGUAGES = language_catalog()
SETTINGS = dict(provider="google", source="auto", target="ru", autoTranslate=True, debounceMs=250, autoSwap=True, secondLanguage="en",
                history=True, minFontSize=12, ocrLanguages="eng+rus", selectionKey="SUPER + U",
                clipboardKey="SUPER + SHIFT + U", screenshotKey="SUPER + ALT + U", replaceKey="SUPER + CTRL + U",
                glossary=[])
BASE = dict(job=1, mode="text", status="ready", sourceKind="selection", sourceLanguage="en", settings=SETTINGS,
            languages=LANGUAGES, ocrInstalled=["eng", "rus"],
            models=[dict(pair="en_ru", installed=True, version="1.9"), dict(pair="ru_en", installed=False, version="1.9")],
            offlineEngine=True, download=dict(pair="ru_en", received=40265318, total=65011712),
            history=[dict(source="Small steps make a big difference.", translated="Маленькие шаги многое меняют.", provider="google"),
                     dict(source="The build failed because the lockfile is out of date.", translated="Сборка не удалась: lock-файл устарел.", provider="mymemory"),
                     dict(source="Спасибо, что посмотрел пул-реквест!", translated="Thanks for reviewing the pull request!", provider="google")])
DONE = dict(BASE, sourceText="Small steps make a big difference.", translated="Маленькие шаги многое меняют.", elapsedMs=184)


def image_state():
    state = json.loads((FIXTURES / "image-state.json").read_text())
    state.update(originalImage=str(FIXTURES / state["originalImage"]), translatedImage=str(FIXTURES / state["translatedImage"]),
                 settings=SETTINGS, history=BASE["history"], models=BASE["models"],
                 languages=LANGUAGES, ocrInstalled=BASE["ocrInstalled"])
    return state


class Renderer:
    def __init__(self, out: Path):
        self.out = out
        out.mkdir(parents=True, exist_ok=True)
        self.app = QGuiApplication.instance() or QGuiApplication([])
        self.messages = []
        qInstallMessageHandler(lambda kind, context, message: self.messages.append(message))
        self.catalog = json.loads((ROOT / "plugin/i18n/ru.json").read_text())
        self.keep = []

    def font_available(self):
        return FONT in QFontDatabase.families()

    def view(self, state, w=640, h=None, theme="dark", extra=0, **props):
        engine = QQmlEngine()
        t = THEMES[theme]
        component = QQmlComponent(engine)
        # Embed the catalog as a JS literal: a Python dict passed as a context
        # property isn't indexable from QML, which left every string as its key.
        component.setData(f'''import QtQuick
import "{(ROOT / 'plugin').as_uri()}"
TranslatorView {{ readonly property var catalog: {json.dumps(self.catalog, ensure_ascii=False)}
 translate: function(key) {{ return catalog[key] || key }}
 backgroundColor: "{t['background']}"; textColor: "{t['text']}"; accentColor: "{t['accent']}"; borderColor: "{t['border']}"
 fontFamily: "{FONT}" }}'''.encode(), QUrl.fromLocalFile(str(self.out / "view.qml")))
        assert not component.isError(), [e.toString() for e in component.errors()]
        view = component.create()
        self.keep.append((engine, component))
        window = QQuickWindow()
        window.setColor(QColor(t["page"]))
        view.setParentItem(window.contentItem())
        view.setX(30)
        view.setY(30)
        for key, value in props.items():
            view.setProperty(key, value)
        view.setProperty("stateData", state)
        view.setWidth(w)
        QTest.qWait(50)
        height = h or view.implicitHeight()
        view.setHeight(height)
        # Pin the size: an offscreen window sometimes keeps its default size after resize().
        size = QSize(w + 60, int(height) + 60 + extra)
        window.setMinimumSize(size)
        window.setMaximumSize(size)
        window.resize(size)
        window.show()
        for _ in range(100):
            if window.size() == size:
                break
            QTest.qWait(20)
        QTest.qWait(250)
        return view, window

    def save(self, view, window, name):
        image = window.grabWindow()
        assert image.size() == window.size(), f"{name}: grabbed {image.size()} for a {window.size()} window"
        image.save(str(self.out / f"{name}.png"))
        window.close()
        return name

    def render_all(self, animated=True):
        """Renders every state; `animated=False` leaves out frames that change over time."""
        names = []
        shot = lambda name, state, **kw: names.append(self.save(*self.view(state, **kw), name))  # noqa: E731
        for theme in ("dark", "light"):
            shot(f"{theme}-01-done", DONE, theme=theme)
        shot("dark-02-empty", dict(BASE, status="idle"))
        if animated:
            shot("dark-03-loading", dict(BASE, status="translating", sourceText="The quick brown fox jumps over the lazy dog."))
        shot("dark-04-error", dict(BASE, status="error", error="networkError", sourceText="Small steps make a big difference."))
        view, window = self.view(DONE, extra=200)
        QTest.mouseClick(window, Qt.LeftButton, Qt.NoModifier, QPoint(330, 112))
        QTest.qWait(200)
        names.append(self.save(view, window, "dark-05-picker"))
        shot("dark-06-history", DONE, h=560, historyOpen=True)
        shot("light-06-history", DONE, h=560, historyOpen=True, theme="light")
        shot("dark-06-history-empty", dict(DONE, history=[]), h=420, historyOpen=True)
        image = image_state()
        shot("dark-07-image", image, w=1040, h=700)
        view, window = self.view(image, w=1040, h=700, theme="light")
        view.setProperty("selectedBlock", 5)
        QTest.qWait(150)
        names.append(self.save(view, window, "light-07-image"))
        if animated:
            shot("dark-07-image-busy", dict(image, status="ocr", blocks=[], translatedImage=""), w=1040, h=700)
        shot("dark-07-image-empty", dict(image, status="error", error="noText", blocks=[], translatedImage=""), w=1040, h=700)
        for section in ("general", "services", "models", "images", "shortcuts"):
            view, window = self.view(DONE, w=1040, h=660, settingsOpen=True)
            settings = [c for c in view.findChildren(QObject) if c.metaObject().className().startswith("SettingsView")][0]
            settings.setProperty("section", section)
            QTest.qWait(150)
            names.append(self.save(view, window, f"dark-08-settings-{section}"))
        shot("light-08-settings", DONE, w=1040, h=660, settingsOpen=True, theme="light")
        view, window = self.view(DONE)
        view.findChild(QObject, "providerMenu").setProperty("visible", True)
        QTest.qWait(250)
        names.append(self.save(view, window, "dark-09-provider"))
        return names

    def qml_errors(self):
        return sorted({m for m in self.messages if any(s in m for s in ("TypeError", "ReferenceError", "Binding loop", "Cannot assign", "Unable to assign", "is not a type"))})
