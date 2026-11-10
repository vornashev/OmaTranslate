import QtQuick
import qs.Ui
import qs.Commons as Commons

BarWidget {
    id: root
    moduleName: "vornashev.omatranslate"
    implicitWidth: button.implicitWidth
    implicitHeight: button.implicitHeight
    Commons.Translations { id: strings; directory: Qt.resolvedUrl("i18n") }

    WidgetButton {
        id: button
        objectName: "translateBarButton"
        anchors.fill: parent
        bar: root.bar
        // nf-md-translate: a glyph from the bar's own icon font, centred like the other bar icons.
        text: String.fromCodePoint(0xF05CA)
        // Each mouse button does something different; the tooltip is where people discover that.
        tooltipText: "OmaTranslate\n" + strings.t("barTooltip")
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
