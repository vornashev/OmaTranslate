import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Ui
import qs.Commons as Commons

Item {
    id: root
    property var shell: null
    property bool opened: false
    property var currentState: ({})
    property string transportError: ""
    property int lastHide: 0
    property bool receivedState: false
    // Counts start attempts that never launched: the plugin was added without its native backend.
    property int failedStarts: 0
    // The repository's installer sits next to plugin/ when the plugin was added from the marketplace.
    readonly property string installer: decodeURIComponent(String(Qt.resolvedUrl("../install.sh")).replace(/^file:\/\//, ""))
    readonly property string binary: Quickshell.env("OMATRANSLATE_BINARY") || "omatranslate"
    readonly property string socketPath: Quickshell.env("OMATRANSLATE_SOCKET") ||
        (Quickshell.env("XDG_RUNTIME_DIR") + "/omatranslate-" + (Quickshell.env("WAYLAND_DISPLAY") || "default").replace(/[^A-Za-z0-9-]/g, "_").slice(0,32) + ".sock")
    Commons.Translations { id: strings; directory: Qt.resolvedUrl("i18n") }
    function open(payloadJson) {
        opened = true
        try { view.settingsOpen = JSON.parse(payloadJson || "{}").settings === true } catch(e) {}
        if (!socket.connected) socket.connected = true
        Qt.callLater(function() { view.focusInput() })
    }
    function close() { opened = false; view.clearInput(); send({op:"dismiss"}) }
    function toggle() { if (opened) close(); else open("{}") }
    function requestClose() { if (shell && shell.hide) shell.hide("vornashev.omatranslate"); else close() }
    function send(request) {
        if (!socket.connected) { if (transportError !== "backendMissing") transportError = "serviceUnavailable"; return }
        request.v = 1
        socket.write(JSON.stringify(request) + "\n")
        socket.flush()
    }
    Process {
        id: daemon; command: [root.binary, "serve"]; running: true
        onStarted: { root.failedStarts = 0; if (root.transportError === "backendMissing") root.transportError = "" }
    }
    Timer {
        interval: 1000; repeat: true; running: !socket.connected
        onTriggered: {
            socket.connected = true
            if (!daemon.running) {
                if (++root.failedStarts >= 2) root.transportError = "backendMissing"
                daemon.running = true
            }
        }
    }
    Socket {
        id: socket
        path: root.socketPath
        connected: true
        onConnectionStateChanged: if (connected) { root.transportError = ""; root.send({op:"subscribe"}) }
        parser: SplitParser {
            splitMarker: "\n"
            onRead: function(data) {
                try {
                    const message = JSON.parse(data)
                    if (message.event === "state") {
                        if (root.receivedState && message.hideSerial !== root.lastHide) root.opened = false
                        root.lastHide = message.hideSerial
                        root.receivedState = true
                        root.currentState = message
                    } else if (message.event === "error") root.transportError = message.error
                } catch(e) { root.transportError = "invalidResponse" }
            }
        }
    }
    OverlayWindow {
        id: panel
        shown: root.opened
        WlrLayershell.namespace: "omatranslate"
        MouseArea { anchors.fill: parent; onClicked: root.requestClose() }
        TranslatorView {
            id: view
            anchors.horizontalCenter: parent.horizontalCenter
            y: Math.max(40, Commons.Style.gapsOut + 24)
            width: Math.min(imageMode || settingsOpen ? 1040 : 640, panel.width - 32)
            height: Math.min(implicitHeight, panel.height - y - 24)
            stateData: root.currentState
            connectionError: root.transportError
            installer: root.installer
            backgroundColor: Commons.Color.menu.background
            textColor: Commons.Color.menu.text
            accentColor: Commons.Color.accent
            borderColor: Commons.Color.menu.border
            dangerColor: Commons.Color.urgent
            cornerRadius: Commons.Style.cornerRadius
            fontFamily: Commons.Style.font.menuFamily
            translate: function(key) { return strings.t(key) }
            onCommand: function(request) { if (root.transportError !== "backendMissing") root.transportError = ""; root.send(request) }
            onDismiss: root.requestClose()
        }
    }
}
