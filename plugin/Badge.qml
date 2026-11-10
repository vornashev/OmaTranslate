import QtQuick

Rectangle {
    id: root
    property var theme
    property string text: ""
    property string tone: "accent" // accent | neutral | warn
    implicitWidth: label.implicitWidth + 12
    implicitHeight: label.implicitHeight + 4
    radius: 5
    color: tone === "accent" ? theme.accentSoft : tone === "warn" ? theme.alpha(theme.warn, 0.12) : theme.overlay
    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        textFormat: Text.PlainText
        font.family: root.theme.fontFamily
        font.pixelSize: 10
        color: root.tone === "accent" ? root.theme.accent : root.tone === "warn" ? root.theme.warn : root.theme.muted
    }
}
