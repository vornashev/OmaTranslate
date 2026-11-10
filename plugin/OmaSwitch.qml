import QtQuick
import QtQuick.Controls

Switch {
    id: control
    property var theme
    padding: 0
    implicitWidth: 34
    implicitHeight: 20
    focusPolicy: Qt.StrongFocus
    indicator: Rectangle {
        implicitWidth: 34
        implicitHeight: 20
        radius: 10
        color: control.checked ? control.theme.accent : control.theme.surface2
        border.color: control.visualFocus ? control.theme.accent : control.checked ? "transparent" : control.theme.line
        Rectangle {
            x: control.checked ? parent.width - width - 3 : 3
            y: 3
            width: 14
            height: 14
            radius: 7
            color: control.checked ? control.theme.onAccent : control.theme.faint
            Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        }
    }
    contentItem: Item {}
}
