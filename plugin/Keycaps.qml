import QtQuick

// A shortcut drawn as keys: "SUPER + SHIFT + U" or ["Ctrl", "Enter"].
Row {
    id: root
    property var theme
    property var keys: []
    property int fontSize: 10
    readonly property var parts: typeof keys === "string"
        ? keys.split("+").map(function(k) { return k.trim() }).filter(function(k) { return k !== "" })
        : keys
    function pretty(k) {
        const names = { "SUPER": "Super", "SHIFT": "Shift", "CTRL": "Ctrl", "CONTROL": "Ctrl", "ALT": "Alt", "RETURN": "Enter", "ESCAPE": "Esc" }
        return names[k.toUpperCase()] || k
    }
    spacing: 3
    Repeater {
        model: root.parts
        Rectangle {
            required property var modelData
            implicitWidth: Math.max(19, label.implicitWidth + 10)
            implicitHeight: 19
            radius: 4
            color: root.theme.surface2
            border.color: root.theme.line
            Text {
                id: label
                anchors.centerIn: parent
                text: root.pretty(String(parent.modelData))
                textFormat: Text.PlainText
                font.family: root.theme.fontFamily
                font.pixelSize: root.fontSize
                color: root.theme.muted
            }
        }
    }
}
