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
from PySide6.QtCore import QUrl, QObject, QTimer, Qt, QPoint, QPointF, QMimeData, qInstallMessageHandler
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
        commons = imports / "qs/Commons"
        commons.mkdir(parents=True)
        (commons / "qmldir").write_text("module qs.Commons\nTranslations 1.0 Translations.qml\n")
        english = (ROOT / "plugin/i18n/en.json").read_text()
        (commons / "Translations.qml").write_text(f'''import QtQuick
QtObject {{ property var directory: ""; readonly property var catalog: {english}
    function t(key) {{ return catalog[key] || key }} }}
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
        tooltip = widget.findChild(QObject, "translateBarButton").property("tooltipText")
        assert tooltip.startswith("OmaTranslate\n") and all(w in tooltip for w in ("Left", "Right", "Middle")), tooltip
        window.grabWindow().save(str(OUT / "bar-harness.png"))
        window.close()
        widget.setParentItem(None)
        del widget, component, window, engine


def check_source_paste(view, window, source, commands):
    clipboard = QGuiApplication.clipboard()
    # The shell supplies the Wayland offer. Use Qt's offscreen clipboard here.
    def paste_offer():
        view.pasteSource(clipboard.mimeData().formats(), 1)
    view.sourcePasteRequested.connect(paste_offer)
    source.setProperty("text", "keep old text")
    source.select(5, 8)
    clipboard.setText("new")
    QTest.keyClick(window, Qt.Key_V, Qt.ControlModifier)
    assert source.property("text") == "keep new text"
    data = QMimeData()
    data.setText("image caption")
    data.setData("image/png", b"image payload")
    clipboard.setMimeData(data)
    commands.clear()
    QTest.keyClick(window, Qt.Key_V, Qt.ControlModifier)
    QTest.qWait(320)
    assert commands == [{"op": "clipboard"}], commands
    assert source.property("text") == "keep new text"
    view.sourcePasteRequested.disconnect(paste_offer)
    clipboard.clear()


def check_canvas(view, window, commands):
    canvas = view.findChild(QObject, "imageCanvas")
    picture = view.findChild(QObject, "previewImage")
    def fragment_point():
        return picture.mapToScene(QPointF(picture.width() * 0.4, picture.height() * 0.19)).toPoint()
    anchor = fragment_point()
    commands.clear()
    for _ in range(5):
        QTest.wheelEvent(window, anchor, QPoint(0, 120))
    assert abs(view.property("zoom") - 1.25 ** 5) < 0.001
    assert abs(fragment_point().x() - anchor.x()) <= 2, "zoom moved the label away from the pointer"
    start = fragment_point()
    old_x, old_y = (canvas.property(p) for p in ("contentX", "contentY"))
    QTest.mousePress(window, Qt.LeftButton, Qt.NoModifier, start)
    for step in range(1, 6):
        QTest.mouseMove(window, start - QPoint(12 * step, 9 * step), 15)
    QTest.mouseRelease(window, Qt.LeftButton, Qt.NoModifier, start - QPoint(60, 45))
    assert canvas.property("contentX") > old_x + 10
    assert canvas.property("contentY") > old_y + 10
    assert view.property("selectedBlock") == -1, "drag selected a fragment"
    QTest.wheelEvent(window, anchor, QPoint(0, -120))
    assert abs(view.property("zoom") - 1.25 ** 4) < 0.001
    QTest.mouseClick(window, Qt.LeftButton, Qt.NoModifier, fragment_point())
    assert view.property("selectedBlock") == 0
    assert not commands, commands


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
    check_source_paste(view, window, text, commands)
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
    state["translatedImage"] = str(OUT / "image.png")
    view.setProperty("stateData", state)
    view.focusInput()
    QTest.keyPress(window, Qt.Key_Space)
    assert view.property("peeking")
    QTest.keyRelease(window, Qt.Key_Space)
    assert not view.property("peeking") and not view.property("original")
    check_canvas(view, window, commands)
    check_bar()
    errors = [m for m in messages if any(s in m for s in ("ReferenceError", "TypeError", "Unable to assign", "Binding loop", "Cannot assign", "is not a type"))]
    assert not errors, errors
    print(json.dumps({"qml": "passed", "autoTranslate": "passed", "plainText": "passed", "barClicksWithHostFixture": "passed", "screenshots": str(OUT)}))
    window.close()


if __name__ == "__main__":
    run()
