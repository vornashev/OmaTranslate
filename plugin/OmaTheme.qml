import QtQuick

// Derives every surface of the translator from the four colours an Omarchy theme
// gives a menu, so dark and light themes both work without a palette of our own.
QtObject {
    property color background: "#13171a"
    property color text: "#e3e7e8"
    property color accent: "#aac8ad"
    property color border: "#2a3236"
    property color urgent: "#e9a69a"
    property int radius: 10
    property string fontFamily: "sans-serif"

    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }

    readonly property bool light: (0.299 * background.r + 0.587 * background.g + 0.114 * background.b) > 0.5
    readonly property color surface: Qt.tint(background, alpha(text, 0.035))
    readonly property color surface2: Qt.tint(background, alpha(text, 0.08))
    readonly property color line: Qt.tint(background, alpha(text, 0.12))
    readonly property color muted: alpha(text, 0.62)
    readonly property color faint: alpha(text, 0.40)
    readonly property color onAccent: background
    readonly property color accentSoft: alpha(accent, 0.15)
    readonly property color accentTint: alpha(accent, 0.05)
    readonly property color accentLine: alpha(accent, 0.25)
    readonly property color danger: Qt.tint(urgent, light ? Qt.rgba(0, 0, 0, 0.25) : Qt.rgba(1, 1, 1, 0.25))
    readonly property color dangerTint: alpha(danger, 0.07)
    readonly property color dangerLine: alpha(danger, 0.28)
    readonly property color warn: light ? "#94661a" : "#e2c58f"
    readonly property color overlay: alpha(text, 0.06)
    readonly property color canvas: Qt.tint(background, Qt.rgba(0, 0, 0, light ? 0.06 : 0.3))
    readonly property int smallRadius: Math.max(0, Math.min(8, radius))
}
