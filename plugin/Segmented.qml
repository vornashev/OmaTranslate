import QtQuick

// Segmented control and tab strip: one visible current choice.
Rectangle {
    id: root
    property var theme
    property var model: []
    property int currentIndex: 0
    property var counts: []
    property int itemHeight: 26
    signal activated(int index)
    implicitWidth: row.implicitWidth + 6
    implicitHeight: itemHeight + 6
    radius: theme.smallRadius + 1
    color: theme.surface
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 2
        Repeater {
            model: root.model
            Rectangle {
                id: item
                required property var modelData
                required property int index
                readonly property bool current: index === root.currentIndex
                implicitWidth: content.implicitWidth + 24
                implicitHeight: root.itemHeight
                radius: root.theme.smallRadius - 2
                color: current ? root.theme.surface2 : area.containsMouse ? root.theme.overlay : "transparent"
                border.width: area.activeFocus ? 1 : 0
                border.color: root.theme.accent
                Accessible.role: Accessible.PageTab
                Accessible.name: String(modelData)
                Row {
                    id: content
                    anchors.centerIn: parent
                    spacing: 6
                    Text {
                        text: String(item.modelData)
                        textFormat: Text.PlainText
                        font.family: root.theme.fontFamily
                        font.pixelSize: 12
                        font.weight: item.current ? Font.DemiBold : Font.Normal
                        color: item.current ? root.theme.text : root.theme.muted
                    }
                    Text {
                        visible: text !== ""
                        text: root.counts[item.index] !== undefined && root.counts[item.index] !== "" ? String(root.counts[item.index]) : ""
                        textFormat: Text.PlainText
                        font.family: root.theme.fontFamily
                        font.pixelSize: 10
                        color: root.theme.faint
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
                MouseArea {
                    id: area
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    activeFocusOnTab: true
                    // The owner sets currentIndex from its state; assigning it here would break that binding.
                    function activate() { root.activated(item.index) }
                    onClicked: activate()
                    Keys.onSpacePressed: activate()
                    Keys.onReturnPressed: activate()
                }
            }
        }
    }
}
