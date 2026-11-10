import QtQuick
import QtQuick.Controls

// Button from the design file: kind = primary | subtle | ghost | outline | danger.
// With an icon and no text it becomes a square icon button; `tip` names it for tooltips and screen readers.
Button {
    id: button
    property var theme
    property string kind: "subtle"
    property string iconName: ""
    property bool small: false
    property string tip: ""
    readonly property bool iconOnly: text === "" && iconName !== ""
    readonly property color ink: kind === "primary" ? theme.onAccent
        : kind === "danger" ? theme.danger
        : kind === "ghost" ? (hovered ? theme.text : theme.muted) : theme.text
    implicitHeight: small ? 28 : 34
    implicitWidth: iconOnly ? implicitHeight : contentItem.implicitWidth + leftPadding + rightPadding
    leftPadding: iconOnly ? 0 : (iconName ? 10 : 13)
    rightPadding: iconOnly ? 0 : 13
    topPadding: 0
    bottomPadding: 0
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    font.family: theme.fontFamily
    font.pixelSize: small ? 12 : 13
    font.weight: kind === "primary" ? Font.DemiBold : Font.Normal
    Accessible.name: tip || text
    ToolTip.visible: tip !== "" && hovered
    ToolTip.text: tip
    ToolTip.delay: 600

    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight
        opacity: button.enabled ? 1 : 0.4
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 7
            Icon {
                visible: button.iconName !== ""
                name: button.iconName
                color: button.kind === "subtle" && !button.iconOnly ? button.theme.muted : button.ink
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                visible: button.text !== ""
                text: button.text
                textFormat: Text.PlainText
                font: button.font
                color: button.ink
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }
    background: Rectangle {
        radius: button.theme.smallRadius
        opacity: button.enabled ? 1 : 0.5
        color: {
            const t = button.theme
            if (button.kind === "primary") return button.down ? Qt.darker(t.accent, 1.1) : button.hovered ? Qt.lighter(t.accent, 1.06) : t.accent
            if (button.kind === "subtle") return button.down ? t.line : button.hovered ? Qt.tint(t.surface2, t.alpha(t.text, 0.04)) : t.surface2
            if (button.kind === "danger") return button.hovered ? t.dangerTint : "transparent"
            return button.down ? t.surface2 : button.hovered ? t.overlay : "transparent"
        }
        border.width: 1
        border.color: button.visualFocus ? button.theme.accent : button.kind === "outline" ? button.theme.line : "transparent"
    }
}
