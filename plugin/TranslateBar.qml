import QtQuick
import qs.Ui

BarWidget {
    id: root
    moduleName: "vornashev.omatranslate"
    implicitWidth: button.implicitWidth
    implicitHeight: button.implicitHeight

    WidgetButton {
        id: button
        objectName: "translateBarButton"
        anchors.fill: parent
        bar: root.bar
        // nf-md-translate: a glyph from the bar's own icon font, centred like the other bar icons.
        text: String.fromCodePoint(0xF05CA)
        tooltipText: "OmaTranslate"
        onPressed: function(mouseButton) {
            if (!root.bar) return
            if (mouseButton === Qt.RightButton)
                root.bar.run("omatranslate settings")
            else if (mouseButton === Qt.MiddleButton)
                root.bar.run("omatranslate clipboard")
            else if (mouseButton === Qt.LeftButton)
                root.bar.run("omatranslate open")
        }
    }
}
