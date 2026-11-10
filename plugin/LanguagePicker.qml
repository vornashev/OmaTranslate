import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Language chip that opens a searchable list. Offline-capable languages are marked.
AbstractButton {
    id: root
    property var theme
    property var translate: function(key) { return key }
    property var codes: []
    property var names: []
    property string current: ""
    property string badge: ""
    property string labelOverride: ""
    // Extra search terms per entry, e.g. English names for endonyms.
    property var aliases: []
    property var offlineCodes: []
    signal picked(string code)

    readonly property int currentIndex: Math.max(0, codes.indexOf(current))
    implicitHeight: 34
    implicitWidth: row.implicitWidth + 22
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.name: label.text
    onClicked: popup.opened ? popup.close() : popup.open()

    background: Rectangle {
        radius: root.theme.smallRadius
        color: popup.opened ? root.theme.surface2 : root.hovered ? Qt.tint(root.theme.surface, root.theme.overlay) : root.theme.surface
        border.color: popup.opened || root.visualFocus ? root.theme.accent : "transparent"
    }
    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight
        Row {
            id: row
            x: 12 - root.leftPadding
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8
            Text {
                id: label
                text: root.labelOverride || root.names[root.currentIndex] || ""
                textFormat: Text.PlainText
                font.family: root.theme.fontFamily
                font.pixelSize: 13
                color: root.theme.text
                anchors.verticalCenter: parent.verticalCenter
            }
            Badge { visible: root.badge !== ""; theme: root.theme; text: root.badge; anchors.verticalCenter: parent.verticalCenter }
            Icon { name: "chev"; color: root.theme.muted; anchors.verticalCenter: parent.verticalCenter }
        }
    }

    Popup {
        id: popup
        y: root.height + 6
        width: 290
        padding: 6
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        onOpened: { search.text = ""; search.forceActiveFocus() }
        background: Rectangle {
            radius: root.theme.radius
            color: root.theme.surface
            border.color: root.theme.line
        }
        contentItem: ColumnLayout {
            spacing: 4
            OmaField {
                id: search
                Layout.fillWidth: true
                theme: root.theme
                iconName: "search"
                placeholderText: root.translate("searchLanguage")
                Keys.onDownPressed: list.forceActiveFocus()
                onAccepted: if (list.count > 0) root.choose(list.model[0].code)
            }
            ListView {
                id: list
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(contentHeight, 320)
                clip: true
                keyNavigationEnabled: true
                boundsBehavior: Flickable.StopAtBounds
                model: {
                    const q = search.text.trim().toLowerCase()
                    const out = []
                    for (let i = 0; i < root.codes.length; i++) {
                        const name = String(root.names[i])
                        const alias = String(root.aliases[i] || "")
                        if (!q || name.toLowerCase().indexOf(q) >= 0 || alias.toLowerCase().indexOf(q) >= 0 || root.codes[i].toLowerCase().indexOf(q) === 0)
                            out.push({ code: root.codes[i], name: name })
                    }
                    return out
                }
                delegate: ItemDelegate {
                    id: entry
                    required property var modelData
                    width: ListView.view.width
                    height: 32
                    padding: 0
                    onClicked: root.choose(modelData.code)
                    background: Rectangle {
                        radius: root.theme.smallRadius - 2
                        color: entry.hovered || entry.ListView.isCurrentItem && list.activeFocus ? root.theme.surface2 : "transparent"
                    }
                    contentItem: RowLayout {
                        spacing: 8
                        Item { Layout.leftMargin: 10; implicitWidth: 16; implicitHeight: 16
                            Icon { visible: entry.modelData.code === root.current; name: "check"; color: root.theme.accent } }
                        Text {
                            Layout.fillWidth: true
                            text: entry.modelData.name
                            textFormat: Text.PlainText
                            // Keep right-to-left endonyms (العربية) in the list column.
                            horizontalAlignment: Text.AlignLeft
                            elide: Text.ElideRight
                            font.family: root.theme.fontFamily
                            font.pixelSize: 13
                            color: root.theme.text
                        }
                        Badge { visible: root.offlineCodes.indexOf(entry.modelData.code) >= 0; theme: root.theme; tone: "neutral"; text: root.translate("langOffline") }
                        Text {
                            Layout.rightMargin: 10
                            text: entry.modelData.code === "auto" ? "" : entry.modelData.code.toUpperCase()
                            textFormat: Text.PlainText
                            font.family: root.theme.fontFamily
                            font.pixelSize: 11
                            color: root.theme.faint
                        }
                    }
                    Keys.onReturnPressed: root.choose(modelData.code)
                }
            }
        }
    }
    function choose(code) { popup.close(); if (code !== current) picked(code) }
}
