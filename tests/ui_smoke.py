#!/usr/bin/env python3
"""Load the production QML and exercise editing without touching the desktop."""
import json
import os
from pathlib import Path
import sys
import tempfile

os.environ["QT_QPA_PLATFORM"] = "offscreen"
os.environ["QT_QUICK_BACKEND"] = "software"
os.environ["QT_QPA_PLATFORMTHEME"] = ""
os.environ["QT_STYLE_OVERRIDE"] = ""
os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
from PySide6.QtCore import QUrl, QObject, QTimer, Qt, QPoint, qInstallMessageHandler
from PySide6.QtGui import QGuiApplication, QImage, QPainter, QFont, QColor
from PySide6.QtQml import QQmlEngine, QQmlComponent, QQmlExpression
from PySide6.QtQuick import QQuickWindow
from PySide6.QtTest import QTest
sys.path.insert(0, str(Path(__file__).resolve().parent))
import visual_states  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(os.environ.get("OMATRANSLATE_TEST_OUTPUT", "/tmp/omatranslate-ui"))
OUT.mkdir(parents=True, exist_ok=True)
messages = []


def message_handler(kind, context, message):
    messages.append(message)
    print(message, file=sys.stderr)


def check_bar():
    # Quickshell's compiled runtime is unavailable to PySide. Only the host's
    # two base types are replaced; the real plugin QML receives real mouse events.
    with tempfile.TemporaryDirectory(prefix="omatranslate-bar-host-") as temporary:
        imports = Path(temporary)
        ui = imports / "qs/Ui"
        ui.mkdir(parents=True)
        (ui / "qmldir").write_text("module qs.Ui\nBarWidget 1.0 BarWidget.qml\nWidgetButton 1.0 WidgetButton.qml\n")
        (ui / "BarWidget.qml").write_text('''import QtQuick
Item { property QtObject bar: null; property string moduleName: "" }
''')
        (ui / "WidgetButton.qml").write_text('''import QtQuick
Item {
    id: button
    property var bar: null
    property string text: ""
    property string tooltipText: ""
    signal pressed(int mouseButton)
    implicitWidth: label.implicitWidth + 20
    implicitHeight: 32
    Text { id: label; anchors.centerIn: parent; text: button.text; textFormat: Text.PlainText; color: "#dde5de" }
    MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
        onPressed: function(mouse) { button.pressed(mouse.button) } }
}
''')
        engine = QQmlEngine()
        engine.addImportPath(str(imports))
        component = QQmlComponent(engine)
        component.setData(f'''import QtQuick
import "{(ROOT / 'plugin').as_uri()}"
TranslateBar {{
    id: widget
    signal invoked(string command)
    bar: QtObject {{ function run(command) {{ widget.invoked(command) }} }}
}}'''.encode(), QUrl.fromLocalFile(str(imports / "BarPreview.qml")))
        assert not component.isError(), [e.toString() for e in component.errors()]
        widget = component.create()
        assert widget is not None
        window = QQuickWindow()
        window.resize(140, 60)
        window.setColor(QColor("#171e22"))
        widget.setParentItem(window.contentItem())
        widget.setX(24)
        widget.setY(14)
        widget.setWidth(widget.implicitWidth())
        widget.setHeight(widget.implicitHeight())
        calls = []
        widget.invoked.connect(calls.append)
        window.show()
        QTest.qWait(80)
        for button in (Qt.LeftButton, Qt.RightButton, Qt.MiddleButton):
            QTest.mouseClick(window, button, Qt.NoModifier, QPoint(32, 26))
        assert calls == ["omatranslate open", "omatranslate settings", "omatranslate clipboard"], calls
        window.grabWindow().save(str(OUT / "bar-harness.png"))
        window.close()
        widget.setParentItem(None)
        del widget, component, window, engine


def run():
    qInstallMessageHandler(message_handler)
    app = QGuiApplication(sys.argv)
    engine = QQmlEngine()
    # The actual shared QML is used both by Quickshell and this harness.
    catalog = json.loads((ROOT / "plugin/i18n/ru.json").read_text())
    engine.rootContext().setContextProperty("catalog", catalog)
    wrapper = QQmlComponent(engine)
    source = f'''import QtQuick
import "{(ROOT / 'plugin').as_uri()}"
TranslatorView {{ translate: function(key) {{ return catalog[key] || key }} }}'''
    wrapper.setData(source.encode(), QUrl.fromLocalFile(str(OUT / "preview.qml")))
    if wrapper.isError():
        raise AssertionError("\n".join(e.toString() for e in wrapper.errors()))
    view = wrapper.create()
    assert view is not None
    window = QQuickWindow()
    window.setColor(QColor("#0e1316"))
    window.resize(720, 500)
    view.setParentItem(window.contentItem())
    view.setX(40)
    view.setY(40)
    view.setWidth(640)
    view.setHeight(420)
    settings = dict(provider="google", source="auto", target="ru", autoTranslate=True, debounceMs=250,
                    autoSwap=True, secondLanguage="en", history=False, minFontSize=12, ocrLanguages="eng+rus")
    state = dict(job=1, mode="text", status="ready", sourceKind="selection",
                 sourceText="Small steps make a big difference.", translated="Маленькие шаги многое меняют.",
                 elapsedMs=184, settings=settings, languages=visual_states.LANGUAGES, models=[dict(pair="en_ru", installed=False, version="1.9"),
                                                         dict(pair="ru_en", installed=False, version="1.9")])
    view.setProperty("stateData", state)
    window.show()
    window.requestActivate()
    QTest.qWait(300)
    window.grabWindow().save(str(OUT / "text.png"))
    commands = []
    view.command.connect(lambda value: commands.append(value.toVariant() if hasattr(value, "toVariant") else value))
    text = view.findChild(QObject, "sourceInput")
    assert text is not None
    text.forceActiveFocus()
    text.setProperty("text", "A new phrase")
    QTest.qWait(400)
    assert any(c.get("op") == "translate" and c.get("text") == "A new phrase" for c in commands), commands
    result = view.findChild(QObject, "translationResult")
    assert result is not None
    state.update(sourceKind="input", translated='<img src="https://invalid.example/x">')
    view.setProperty("stateData", state)
    assert QQmlExpression(engine.rootContext(), result, "textFormat === 0").evaluate()[0]
    assert text.property("text") == "A new phrase", "A response overwrote in-progress typing"
    window.resize(1120, 780)
    view.setWidth(1040)
    view.setHeight(700)
    view.setProperty("settingsOpen", True)
    QTest.qWait(200)
    window.grabWindow().save(str(OUT / "settings.png"))
    # A synthetic screenshot doubles as a real OCR fixture for the backend tests.
    image = QImage(900, 340, QImage.Format_RGB32)
    image.fill(QColor("#f4f4f1"))
    painter = QPainter(image)
    painter.setPen(QColor("#172026"))
    font = QFont("sans-serif", 26)
    painter.setFont(font)
    painter.drawText(50, 80, "Welcome to your workspace")
    font.setPointSize(18)
    painter.setFont(font)
    painter.drawText(50, 150, "Keep your projects organized.")
    painter.drawText(50, 200, "Save your changes before closing this window.")
    painter.end()
    image.save(str(OUT / "ocr-input.png"))
    state.update(mode="image", status="ready", originalImage=str(OUT / "ocr-input.png"), translatedImage="",
                 imageWidth=900, imageHeight=340,
                 blocks=[dict(x=50, y=45, width=550, height=45, source="Welcome to your workspace",
                              translated="Добро пожаловать в рабочее пространство", overflow=True)])
    view.setProperty("stateData", state)
    view.setProperty("settingsOpen", False)
    QTest.qWait(200)
    window.grabWindow().save(str(OUT / "image.png"))
    check_bar()
    errors = [m for m in messages if any(s in m for s in ("ReferenceError", "TypeError", "Unable to assign", "Binding loop", "Cannot assign", "is not a type"))]
    assert not errors, errors
    print(json.dumps({"qml": "passed", "autoTranslate": "passed", "plainText": "passed", "barClicksWithHostFixture": "passed", "screenshots": str(OUT)}))
    window.close()


if __name__ == "__main__":
    run()
