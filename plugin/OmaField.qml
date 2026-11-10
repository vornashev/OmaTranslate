import QtQuick
import QtQuick.Controls

TextField {
    id: field
    property var theme
    property string iconName: ""
    property color fieldColor: theme.background
    implicitHeight: 34
    leftPadding: iconName ? 34 : 12
    rightPadding: 12
    font.family: theme.fontFamily
    font.pixelSize: 12
    color: theme.text
    placeholderTextColor: theme.faint
    selectionColor: theme.accent
    selectedTextColor: theme.onAccent
    selectByMouse: true
    background: Rectangle {
        radius: field.theme.smallRadius
        color: field.fieldColor
        border.color: field.activeFocus ? field.theme.accent : field.theme.line
        Icon {
            visible: field.iconName !== ""
            name: field.iconName
            color: field.theme.muted
            x: 11
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
