import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Pane {
    id: root
    property var stateData: ({})
    property string connectionError: ""
    property string installer: ""
    property color backgroundColor: "#13171a"
    property color textColor: "#e3e7e8"
    property color accentColor: "#aac8ad"
    property color borderColor: "#2a3236"
    property color dangerColor: "#e9a69a"
    property int cornerRadius: 12
    property string fontFamily: "sans-serif"
    property var translate: function(key) { return key }
    property bool settingsOpen: false
    property bool historyOpen: false
    property bool original: false
    property real zoom: 1
    property int selectedBlock: -1
    property bool syncing: false
    property var settings: stateData.settings || ({provider:"google",source:"auto",target:"en",secondLanguage:"en",autoSwap:true,autoTranslate:true,debounceMs:250,minFontSize:12,ocrLanguages:"eng"})
    // The job whose image the user closed. Image mode stays off for it even if the
    // backend still reports mode "image" (older backends kept it after dismiss).
    property int closedImageJob: -1
    readonly property bool imageMode: stateData.mode === "image" && (stateData.job || 0) !== closedImageJob
    readonly property bool busy: ["working","capturing","ocr","translating"].indexOf(stateData.status) >= 0
    readonly property var providers: ["google","mymemory","libre","local"]
    readonly property var providerNames: ["Google","MyMemory","LibreTranslate",t("offline")]
    readonly property string providerName: providerNames[Math.max(0, providers.indexOf(settings.provider))]
    // The backend's catalog, with the user's two languages pinned on top.
    readonly property var catalog: {
        const all = stateData.languages && stateData.languages.length ? stateData.languages : [{code:"en",name:"English",english:"English"}]
        const pinned = [settings.target, settings.secondLanguage]
        const top = all.filter(function(l) { return pinned.indexOf(l.code) >= 0 })
        return top.concat(all.filter(function(l) { return pinned.indexOf(l.code) < 0 }))
    }
    readonly property var languageCodes: ["auto"].concat(catalog.map(function(l) { return l.code }))
    readonly property var languageNames: [t("autoDetect")].concat(catalog.map(function(l) { return l.name }))
    readonly property var languageAliases: [""].concat(catalog.map(function(l) { return l.english || "" }))
    // Languages with an installed offline model in either direction.
    readonly property var offlineCodes: {
        const out = []
        ;(stateData.models || []).forEach(function(m) {
            if (m.installed) String(m.pair).split("_").forEach(function(c) { if (out.indexOf(c) < 0) out.push(c) })
        })
        return out
    }
    readonly property var history: stateData.history || []
    // empty | waiting | loading | error | done
    readonly property string textState: connectionError === "backendMissing" ? "setup"
        : connectionError ? "loading"
        : stateData.error ? "error"
        : busy ? "loading"
        : stateData.translated ? "done"
        : source.text.length > 0 ? "waiting" : "empty"
    signal command(var request)
    signal dismiss()

    function t(key) { return translate(key) }
    function send(request) { command(request) }
    function focusInput() { if (!imageMode && !settingsOpen && !historyOpen) source.forceActiveFocus(); else root.forceActiveFocus() }
    function clearInput() { debounce.stop(); syncing = true; source.text = ""; syncing = false }
    function patch(values, repeat) {
        debounce.stop()
        send({op:"setSettings",settings:values})
        if (repeat && (stateData.sourceText || source.text)) {
            if (imageMode) send({op:"retranslate"})
            else send({op:"translate",text:source.text})
        }
    }
    function submit() { debounce.stop(); if (imageMode) send({op:"retranslate"}); else send({op:"translate",text:source.text}) }
    function fileUrl(path) { return path ? "file://" + encodeURI(path).replace(/#/g,"%23").replace(/\?/g,"%3F") : "" }
    function message(code) { const translated = t(code); return translated === code ? t("operationFailed") : translated }
    function languageName(code) { const i = languageCodes.indexOf(code); return i > 0 ? languageNames[i] : "" }
    function copyText(value) { clipboardHelper.text = value; clipboardHelper.selectAll(); clipboardHelper.copy(); clipboardHelper.text = "" }
    readonly property var blocks: stateData.blocks || []
    // Leaves image mode: the backend forgets the image and returns to text.
    function closeImage() { closedImageJob = stateData.job || 0; clearInput(); send({op:"dismiss"}); Qt.callLater(focusInput) }
    function restore(entry) {
        historyOpen = false
        syncing = true; source.text = entry.source; syncing = false
        send({op:"translate", text: entry.source})
        Qt.callLater(focusInput)
    }

    onStateDataChanged: {
        syncing = true
        if (stateData.sourceKind !== "input" || !source.activeFocus) source.text = stateData.sourceText || ""
        syncing = false
    }
    onImageModeChanged: { original = false; zoom = 1; selectedBlock = -1 }
    onSettingsOpenChanged: if (settingsOpen) historyOpen = false

    OmaTheme {
        id: ui
        background: root.backgroundColor
        text: root.textColor
        accent: root.accentColor
        border: root.borderColor
        urgent: root.dangerColor
        radius: root.cornerRadius
        fontFamily: root.fontFamily
    }

    padding: 0
    font.family: fontFamily
    palette.window: ui.surface
    palette.windowText: ui.text
    palette.base: ui.background
    palette.text: ui.text
    palette.button: ui.surface2
    palette.buttonText: ui.text
    palette.highlight: ui.accent
    palette.highlightedText: ui.onAccent
    palette.placeholderText: ui.faint
    palette.toolTipBase: ui.surface2
    palette.toolTipText: ui.text
    background: Rectangle { color: ui.background; border.color: root.borderColor; radius: root.cornerRadius }
    clip: true
    implicitHeight: settingsOpen ? 660 : imageMode && !historyOpen ? 700 : historyOpen ? (historyList.visible ? 560 : 420) : Math.min(650, content.implicitHeight)
    focus: true
    Keys.onEscapePressed: { if (settingsOpen) settingsOpen = false; else if (historyOpen) historyOpen = false; else dismiss() }
    Shortcut { sequence: "Ctrl+Return"; enabled: root.visible; onActivated: root.submit() }
    Shortcut { sequence: "Ctrl+Shift+C"; enabled: root.visible; onActivated: root.send({op:"copy",kind:"text"}) }
    Timer { id: debounce; interval: root.settings.debounceMs || 250; onTriggered: root.send({op:"translate",text:source.text}) }
    TextEdit { id: clipboardHelper; visible: false; textFormat: TextEdit.PlainText }
    // Consume clicks inside the card so the shell's outside-click surface cannot dismiss it.
    MouseArea { anchors.fill: parent; onPressed: function(mouse) { mouse.accepted = true } }

    ColumnLayout {
        id: content
        anchors.fill: parent
        spacing: 0

        // ── Top bar ──────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 10
            Layout.leftMargin: root.settingsOpen ? 10 : 14
            spacing: 10
            OmaButton {
                visible: root.settingsOpen
                theme: ui; kind: "ghost"; iconName: "back"; tip: root.t("back")
                onClicked: root.settingsOpen = false
            }
            Text {
                visible: root.settingsOpen
                text: root.t("settings"); textFormat: Text.PlainText
                font.family: root.fontFamily; font.pixelSize: 14; font.weight: Font.DemiBold; color: ui.text
            }
            Rectangle {
                visible: !root.settingsOpen
                implicitWidth: 28; implicitHeight: 28; radius: Math.min(7, root.cornerRadius)
                color: ui.accentSoft
                Icon { anchors.centerIn: parent; name: "translate"; color: ui.accent }
            }
            Segmented {
                visible: !root.settingsOpen
                theme: ui
                model: [root.t("tabTranslate"), root.t("tabHistory")]
                counts: ["", root.history.length > 0 ? root.history.length : ""]
                currentIndex: root.historyOpen ? 1 : 0
                onActivated: function(index) { root.historyOpen = index === 1; if (index === 0) Qt.callLater(root.focusInput) }
            }
            // Image context with a way back to text translation.
            Rectangle {
                visible: root.imageMode && !root.settingsOpen && !root.historyOpen
                implicitWidth: imageChip.implicitWidth + 8; implicitHeight: 26
                radius: ui.smallRadius - 2; color: ui.overlay
                Row {
                    id: imageChip
                    anchors.verticalCenter: parent.verticalCenter
                    x: 10
                    spacing: 6
                    Icon { name: "image"; color: ui.muted; anchors.verticalCenter: parent.verticalCenter }
                    Text {
                        text: root.t("imageBadge") + (root.stateData.imageWidth ? " · " + root.stateData.imageWidth + " × " + root.stateData.imageHeight : "")
                        textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.muted
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    OmaButton {
                        theme: ui; kind: "ghost"; small: true; iconName: "close"; tip: root.t("closeImage")
                        implicitHeight: 22
                        onClicked: root.closeImage()
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
            }
            Item { Layout.fillWidth: true }
            Text {
                visible: root.settingsOpen
                text: root.t("savedInstantly"); textFormat: Text.PlainText
                font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint
            }
            OmaButton {
                visible: !root.settingsOpen
                theme: ui; kind: "ghost"; iconName: "sliders"; tip: root.t("settings")
                onClicked: root.settingsOpen = true
            }
            OmaButton { theme: ui; kind: "ghost"; iconName: "close"; tip: root.t("close"); onClicked: root.dismiss() }
        }
        Rectangle { visible: root.settingsOpen; Layout.fillWidth: true; implicitHeight: 1; color: ui.line }

        // ── Languages ────────────────────────────────────────────
        RowLayout {
            visible: !root.settingsOpen && !root.historyOpen
            Layout.fillWidth: true
            Layout.leftMargin: 14; Layout.rightMargin: 14; Layout.bottomMargin: 10; Layout.topMargin: 2
            spacing: 6
            LanguagePicker {
                id: sourceLanguage
                theme: ui; translate: root.translate
                codes: root.languageCodes; names: root.languageNames; aliases: root.languageAliases
                offlineCodes: root.offlineCodes
                current: root.settings.source || "auto"
                readonly property bool detected: current === "auto" && !!root.stateData.sourceText && !!root.languageName(root.stateData.sourceLanguage)
                labelOverride: detected ? root.languageName(root.stateData.sourceLanguage) : ""
                badge: detected ? root.t("badgeAuto") : ""
                onPicked: function(code) { root.patch({source: code}, true) }
            }
            OmaButton {
                theme: ui; kind: "ghost"; iconName: "swap"; tip: root.t("swap")
                onClicked: root.patch({source:root.settings.target,target:root.stateData.sourceLanguage||root.settings.secondLanguage||"en",autoSwap:false},true)
            }
            LanguagePicker {
                theme: ui; translate: root.translate
                codes: root.languageCodes.slice(1); names: root.languageNames.slice(1); aliases: root.languageAliases.slice(1)
                offlineCodes: root.offlineCodes
                current: root.settings.target || "en"
                onPicked: function(code) { root.patch({target: code}, true) }
            }
            Item { Layout.fillWidth: true }
            Segmented {
                visible: root.imageMode
                theme: ui; itemHeight: 28
                model: [root.t("showTranslation"), root.t("showOriginal")]
                currentIndex: root.original ? 1 : 0
                onActivated: function(index) { root.original = index === 1 }
            }
            Rectangle {
                visible: root.imageMode
                implicitWidth: zoomRow.implicitWidth + 6; implicitHeight: 34
                radius: ui.smallRadius + 1; color: ui.surface
                Row {
                    id: zoomRow
                    anchors.centerIn: parent
                    OmaButton { theme: ui; kind: "ghost"; small: true; iconName: "minus"; tip: root.t("zoomOut"); onClicked: root.zoom = Math.max(0.25, root.zoom / 1.25) }
                    Text {
                        width: 46; height: 28; text: Math.round(root.zoom * 100) + "%"; textFormat: Text.PlainText
                        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
                        font.family: root.fontFamily; font.pixelSize: 12; color: ui.text
                    }
                    OmaButton { theme: ui; kind: "ghost"; small: true; iconName: "plus"; tip: root.t("zoomIn"); onClicked: root.zoom = Math.min(5, root.zoom * 1.25) }
                    OmaButton { theme: ui; kind: "ghost"; small: true; iconName: "fit"; tip: root.t("fit"); onClicked: root.zoom = 1 }
                }
            }
        }

        // ── Text mode ────────────────────────────────────────────
        ColumnLayout {
            visible: !root.settingsOpen && !root.historyOpen && !root.imageMode
            Layout.fillWidth: true
            Layout.leftMargin: 14; Layout.rightMargin: 14; Layout.bottomMargin: 14
            spacing: 8

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: sourceColumn.implicitHeight + 18
                radius: ui.smallRadius + 2
                color: ui.surface
                border.color: source.activeFocus ? ui.accentLine : "transparent"
                ColumnLayout {
                    id: sourceColumn
                    anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 14; rightMargin: 8; topMargin: 10 }
                    spacing: 4
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.rightMargin: 6
                        Layout.preferredHeight: Math.max(66, Math.min(150, source.contentHeight + 8))
                        clip: true
                        TextArea {
                            id: source; objectName: "sourceInput"
                            placeholderText: root.t("inputPlaceholder"); placeholderTextColor: ui.faint
                            color: ui.text; selectionColor: ui.accent; selectedTextColor: ui.onAccent
                            font.pixelSize: 15; font.family: root.fontFamily; wrapMode: TextEdit.Wrap; textFormat: TextEdit.PlainText
                            background: null; padding: 0; topPadding: 2; selectByMouse: true
                            onTextChanged: if (!root.syncing && activeFocus && root.settings.autoTranslate) debounce.restart()
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        OmaButton { theme: ui; kind: "ghost"; small: true; iconName: "paste"; text: root.t("paste"); onClicked: root.send({op:"clipboard"}) }
                        OmaButton { theme: ui; kind: "ghost"; small: true; iconName: "region"; text: root.t("captureRegion"); onClicked: root.send({op:"screenshot"}) }
                        Item { Layout.fillWidth: true }
                        Text {
                            visible: source.length > 0
                            text: source.length.toLocaleString(Qt.locale(), "f", 0); textFormat: Text.PlainText
                            font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint
                        }
                        OmaButton {
                            visible: source.length > 0
                            theme: ui; kind: "ghost"; small: true; iconName: "close"; tip: root.t("clear")
                            onClicked: { root.clearInput(); source.forceActiveFocus() }
                        }
                    }
                }
            }

            // Empty: teach the global shortcuts instead of an empty box.
            Rectangle {
                visible: root.textState === "empty"
                Layout.fillWidth: true
                implicitHeight: ways.implicitHeight + 22
                radius: ui.smallRadius + 2
                color: "transparent"
                border.color: ui.line
                ColumnLayout {
                    id: ways
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12; leftMargin: 14 }
                    spacing: 2
                    Text { text: root.t("onboardingTitle4"); textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint; Layout.bottomMargin: 4 }
                    Repeater {
                        model: [
                            {icon: "cursor", title: root.t("waySelection"), hint: root.t("waySelectionHint"), keys: root.settings.selectionKey || "SUPER + U"},
                            {icon: "paste", title: root.t("wayClipboard"), hint: root.t("wayClipboardHint"), keys: root.settings.clipboardKey || "SUPER + SHIFT + U"},
                            {icon: "region", title: root.t("wayRegion"), hint: root.t("wayRegionHint"), keys: root.settings.screenshotKey || "SUPER + ALT + U"},
                            {icon: "swap", title: root.t("wayReplace"), hint: root.t("wayReplaceHint"), keys: root.settings.replaceKey || "SUPER + CTRL + U"}
                        ]
                        RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Layout.topMargin: 4; Layout.bottomMargin: 4
                            spacing: 12
                            Rectangle {
                                implicitWidth: 30; implicitHeight: 30; radius: ui.smallRadius; color: ui.surface
                                Icon { anchors.centerIn: parent; name: parent.parent.modelData.icon; color: ui.accent }
                            }
                            ColumnLayout {
                                spacing: 2
                                Text { text: parent.parent.modelData.title; textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 13; color: ui.text }
                                Text { text: parent.parent.modelData.hint; textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.muted }
                            }
                            Item { Layout.fillWidth: true }
                            Keycaps { theme: ui; keys: parent.modelData.keys }
                        }
                    }
                }
            }

            // Loading: skeleton of the coming text, so the window does not jump.
            Rectangle {
                id: loadingPanel
                visible: root.textState === "loading"
                Layout.fillWidth: true
                implicitHeight: loadingColumn.implicitHeight + 24
                radius: ui.smallRadius + 2
                color: ui.accentTint
                border.color: ui.accentLine
                clip: true
                Rectangle {
                    id: sweep
                    y: 0; height: 2; width: parent.width * 0.35; color: ui.accent
                    SequentialAnimation on x {
                        running: loadingPanel.visible && root.visible
                        loops: Animation.Infinite
                        NumberAnimation { from: -sweep.width; to: sweep.parent.width; duration: 1100; easing.type: Easing.InOutQuad }
                    }
                }
                ColumnLayout {
                    id: loadingColumn
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12; leftMargin: 14; topMargin: 16 }
                    spacing: 10
                    Repeater {
                        model: [0.8, 0.62, 0.36]
                        Rectangle { required property var modelData; implicitWidth: loadingColumn.width * modelData; implicitHeight: 13; radius: 5; color: ui.overlay }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.topMargin: 4
                        Text {
                            text: root.connectionError ? root.message(root.connectionError)
                                : root.stateData.status === "translating" ? root.t("translatingVia").arg(root.providerName)
                                : root.t(root.stateData.status || "working")
                            textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.muted
                        }
                        Item { Layout.fillWidth: true }
                        OmaButton { theme: ui; kind: "primary"; iconName: "copy"; text: root.t("copy"); enabled: false }
                    }
                }
            }

            // Setup: the plugin was added without its native backend; show the one command that fixes it.
            Rectangle {
                visible: root.textState === "setup"
                Layout.fillWidth: true
                implicitHeight: setupColumn.implicitHeight + 24
                radius: ui.smallRadius + 2
                color: ui.surface
                border.color: ui.line
                ColumnLayout {
                    id: setupColumn
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12; leftMargin: 14 }
                    spacing: 10
                    RowLayout {
                        spacing: 10
                        Icon { name: "alert"; color: ui.warn }
                        Text {
                            Layout.fillWidth: true
                            text: root.t("backendMissingTitle")
                            textFormat: Text.PlainText; wrapMode: Text.Wrap
                            font.family: root.fontFamily; font.pixelSize: 14; font.weight: Font.DemiBold; color: ui.text
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.t("backendMissing")
                        textFormat: Text.PlainText; wrapMode: Text.Wrap
                        font.family: root.fontFamily; font.pixelSize: 12; lineHeight: 1.4; color: ui.muted
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: installCommand.implicitHeight + 16
                        radius: ui.smallRadius
                        color: ui.canvas
                        TextEdit {
                            id: installCommand
                            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 10; rightMargin: 10 }
                            text: root.installer
                            readOnly: true; selectByMouse: true; wrapMode: TextEdit.WrapAnywhere
                            font.family: root.fontFamily; font.pixelSize: 12; color: ui.text; selectionColor: ui.accentSoft
                        }
                    }
                    OmaButton {
                        id: copyCommand
                        property bool done: false
                        theme: ui; small: true; kind: "primary"; iconName: "copy"
                        text: done ? root.t("copied") : root.t("copyCommand")
                        onClicked: { root.copyText(root.installer); done = true; copiedReset.restart() }
                        Timer { id: copiedReset; interval: 1600; onTriggered: copyCommand.done = false }
                    }
                }
            }

            // Error: what happened and the ways out, right where the result was expected.
            Rectangle {
                visible: root.textState === "error"
                Layout.fillWidth: true
                implicitHeight: errorColumn.implicitHeight + 24
                radius: ui.smallRadius + 2
                color: ui.dangerTint
                border.color: ui.dangerLine
                ColumnLayout {
                    id: errorColumn
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12; leftMargin: 14 }
                    spacing: 8
                    RowLayout {
                        spacing: 10
                        Icon { name: "alert"; color: ui.danger }
                        Text {
                            text: root.stateData.error === "networkError" ? root.t("providerDown").arg(root.providerName) : root.t("translationFailed")
                            textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 14; font.weight: Font.DemiBold; color: ui.danger
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.message(root.stateData.error || "")
                        textFormat: Text.PlainText; wrapMode: Text.Wrap
                        font.family: root.fontFamily; font.pixelSize: 12; lineHeight: 1.4; color: ui.muted
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 6
                        OmaButton { theme: ui; small: true; iconName: "retry"; text: root.t("retryShort"); onClicked: root.submit() }
                        Repeater {
                            model: ["google", "mymemory", "local"].filter(function(p) { return p !== root.settings.provider }).slice(0, 2)
                            OmaButton {
                                required property var modelData
                                theme: ui; small: true; kind: "outline"
                                iconName: modelData === "local" ? "cpu" : ""
                                text: modelData === "local" ? root.t("offline") : root.t("useProvider").arg(root.providerNames[root.providers.indexOf(modelData)])
                                onClicked: root.patch({provider: modelData}, true)
                            }
                        }
                    }
                }
            }

            // Result.
            Rectangle {
                visible: root.textState === "done" || root.textState === "waiting"
                Layout.fillWidth: true
                implicitHeight: resultColumn.implicitHeight + 22
                radius: ui.smallRadius + 2
                color: ui.accentTint
                border.color: ui.accentLine
                ColumnLayout {
                    id: resultColumn
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10; leftMargin: 14; topMargin: 12 }
                    spacing: 8
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.max(30, Math.min(220, result.contentHeight + 4))
                        clip: true
                        TextArea {
                            id: result; objectName: "translationResult"
                            text: root.stateData.translated || ""
                            placeholderText: root.t("resultPlaceholder"); placeholderTextColor: ui.faint
                            color: ui.text; selectionColor: ui.accent; selectedTextColor: ui.onAccent
                            font.pixelSize: 18; font.family: root.fontFamily; wrapMode: TextEdit.Wrap; textFormat: TextEdit.PlainText
                            readOnly: true; selectByMouse: true; background: null; padding: 0
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Text {
                            text: root.stateData.elapsedMs && root.stateData.translated ? root.providerName + " · " + root.stateData.elapsedMs + " ms" : ""
                            textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint
                        }
                        Item { Layout.fillWidth: true }
                        OmaButton { theme: ui; kind: "ghost"; iconName: "retry"; tip: root.t("retry"); enabled: source.length > 0; onClicked: root.submit() }
                        OmaButton { theme: ui; kind: "primary"; iconName: "copy"; text: root.t("copy"); enabled: !!root.stateData.translated; onClicked: root.send({op:"copy",kind:"text"}) }
                    }
                }
            }
        }

        // ── Image mode ───────────────────────────────────────────
        RowLayout {
            visible: root.imageMode && !root.settingsOpen && !root.historyOpen
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.leftMargin: 14; Layout.rightMargin: 14; Layout.bottomMargin: 12
            spacing: 12
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true
                color: ui.canvas; radius: ui.smallRadius + 2; border.color: ui.line; clip: true
                Flickable {
                    id: canvas; anchors.fill: parent; anchors.margins: 16; clip: true
                    contentWidth: Math.max(width, picture.width); contentHeight: Math.max(height, picture.height)
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.horizontal: ScrollBar {}
                    ScrollBar.vertical: ScrollBar {}
                    Image {
                        id: picture
                        readonly property real fitScale: Math.min(canvas.width / Math.max(1, root.stateData.imageWidth || 1), canvas.height / Math.max(1, root.stateData.imageHeight || 1))
                        readonly property real k: fitScale * root.zoom
                        width: (root.stateData.imageWidth || 1) * k
                        height: (root.stateData.imageHeight || 1) * k
                        x: Math.max(0, (canvas.width - width) / 2); y: Math.max(0, (canvas.height - height) / 2)
                        source: root.fileUrl(root.original || !root.stateData.translatedImage ? root.stateData.originalImage : root.stateData.translatedImage)
                        cache: false; asynchronous: true; fillMode: Image.PreserveAspectFit
                        Repeater {
                            model: root.blocks
                            Item {
                                id: blockArea
                                required property var modelData
                                required property int index
                                readonly property bool selected: root.selectedBlock === index
                                x: modelData.x * picture.k; y: modelData.y * picture.k
                                width: modelData.width * picture.k; height: modelData.height * picture.k
                                // Only the selected or hovered fragment is marked, so markers never cover the picture.
                                readonly property bool marked: selected || blockHover.containsMouse
                                Rectangle {
                                    anchors.fill: parent; anchors.margins: -3; radius: 3
                                    color: blockHover.containsMouse && !blockArea.selected ? ui.alpha(ui.accent, 0.12) : "transparent"
                                    border.width: blockArea.marked ? (blockArea.selected ? 2 : 1) : 0; border.color: ui.accent
                                }
                                Rectangle {
                                    visible: blockArea.marked
                                    x: -3 - width / 2; y: -3 - height / 2
                                    width: 18; height: 18; radius: 9
                                    color: ui.accent
                                    Text { anchors.centerIn: parent; text: blockArea.index + 1; font.family: root.fontFamily; font.pixelSize: 10; font.bold: true; color: ui.onAccent }
                                }
                                MouseArea {
                                    id: blockHover
                                    anchors.fill: parent; anchors.margins: -3; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                    onClicked: { root.selectedBlock = blockArea.index; fragments.positionViewAtIndex(blockArea.index, ListView.Contain) }
                                }
                            }
                        }
                    }
                }
                // Recognising: dim the picture and say what is happening.
                Rectangle {
                    anchors.fill: parent
                    visible: root.busy
                    color: ui.alpha(ui.canvas, 0.75)
                    Rectangle {
                        anchors.centerIn: parent
                        width: busyColumn.implicitWidth + 56; height: busyColumn.implicitHeight + 40
                        radius: root.cornerRadius; color: ui.surface; border.color: ui.line
                    }
                    ColumnLayout {
                        id: busyColumn
                        anchors.centerIn: parent
                        spacing: 12
                        Rectangle {
                            Layout.alignment: Qt.AlignHCenter
                            implicitWidth: 160; implicitHeight: 3; radius: 2; color: ui.surface2; clip: true
                            Rectangle {
                                id: imageSweep
                                width: 56; height: 3; radius: 2; color: ui.accent
                                SequentialAnimation on x {
                                    running: root.busy && root.imageMode && root.visible
                                    loops: Animation.Infinite
                                    NumberAnimation { from: -56; to: 160; duration: 1100; easing.type: Easing.InOutQuad }
                                }
                            }
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: root.t(root.stateData.status || "working"); textFormat: Text.PlainText
                            font.family: root.fontFamily; font.pixelSize: 13; color: ui.text
                        }
                    }
                }
                // Nothing to show: what happened and the ways out, on a card so it reads over the picture.
                Rectangle {
                    anchors.fill: parent
                    visible: emptyState.visible && !!root.stateData.originalImage
                    color: ui.alpha(ui.canvas, 0.75)
                }
                Rectangle {
                    anchors.centerIn: parent
                    visible: emptyState.visible
                    width: emptyState.width + 48; height: emptyState.implicitHeight + 40
                    radius: root.cornerRadius
                    color: ui.surface; border.color: ui.line
                }
                ColumnLayout {
                    id: emptyState
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 96, 400)
                    visible: !root.busy && (root.blocks.length === 0 || !root.stateData.originalImage)
                    spacing: 10
                    Icon {
                        Layout.alignment: Qt.AlignHCenter
                        name: root.stateData.error ? "alert" : "image"; size: 28
                        color: root.stateData.error ? ui.danger : ui.faint
                    }
                    Text {
                        Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                        text: root.stateData.error && root.stateData.error !== "noText" ? root.message(root.stateData.error) : root.t("imageEmpty")
                        textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 14; color: ui.text
                    }
                    Text {
                        visible: !root.stateData.error || root.stateData.error === "noText"
                        Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                        text: root.t("imageEmptyHint"); textFormat: Text.PlainText
                        font.family: root.fontFamily; font.pixelSize: 12; color: ui.muted
                    }
                    Row {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.topMargin: 6
                        spacing: 8
                        OmaButton { theme: ui; kind: "primary"; small: true; iconName: "region"; text: root.t("captureAgain"); onClicked: root.send({op:"screenshot"}) }
                        OmaButton { theme: ui; kind: "outline"; small: true; iconName: "back"; text: root.t("backToText"); onClicked: root.closeImage() }
                    }
                }
            }
            ColumnLayout {
                visible: root.blocks.length > 0
                Layout.preferredWidth: 300; Layout.maximumWidth: 300; Layout.fillWidth: false; Layout.fillHeight: true
                spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8
                    Text { text: root.t("fragments"); textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 12; font.weight: Font.DemiBold; color: ui.text }
                    Text { text: root.blocks.length; textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint }
                    Item { Layout.fillWidth: true }
                }
                ListView {
                    id: fragments
                    Layout.fillWidth: true; Layout.fillHeight: true
                    clip: true; spacing: 8
                    boundsBehavior: Flickable.StopAtBounds
                    model: root.blocks
                    ScrollBar.vertical: ScrollBar {}
                    delegate: Rectangle {
                        id: card
                        required property var modelData
                        required property int index
                        property bool editing: false
                        readonly property bool selected: root.selectedBlock === index
                        width: ListView.view.width - 6
                        implicitHeight: cardColumn.implicitHeight + 22
                        radius: ui.smallRadius
                        color: ui.surface
                        border.color: selected || editing ? ui.accent : "transparent"
                        MouseArea { anchors.fill: parent; onClicked: root.selectedBlock = card.index }
                        ColumnLayout {
                            id: cardColumn
                            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10; leftMargin: 12; topMargin: 10 }
                            spacing: 7
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                Rectangle {
                                    implicitWidth: 18; implicitHeight: 18; radius: 9
                                    color: card.selected || card.editing ? ui.accent : ui.surface2
                                    Text { anchors.centerIn: parent; text: card.index + 1; font.family: root.fontFamily; font.pixelSize: 10; font.bold: true; color: card.selected || card.editing ? ui.onAccent : ui.muted }
                                }
                                Badge { visible: !!card.modelData.overflow; theme: ui; tone: "warn"; text: root.t("fullTextHere") }
                                Item { Layout.fillWidth: true }
                                OmaButton {
                                    visible: !card.editing
                                    theme: ui; kind: "ghost"; small: true; iconName: "pencil"; tip: root.t("editFragment")
                                    onClicked: { card.editing = true; root.selectedBlock = card.index; editor.text = card.modelData.source; editor.forceActiveFocus() }
                                }
                            }
                            Text {
                                visible: !card.editing
                                Layout.fillWidth: true
                                text: card.modelData.source; textFormat: Text.PlainText; wrapMode: Text.Wrap
                                font.family: root.fontFamily; font.pixelSize: 11; lineHeight: 1.35; color: ui.muted
                            }
                            TextEdit {
                                visible: !card.editing
                                Layout.fillWidth: true
                                text: card.modelData.translated; textFormat: TextEdit.PlainText; wrapMode: TextEdit.Wrap; readOnly: true; selectByMouse: true
                                font.family: root.fontFamily; font.pixelSize: 13; color: ui.text
                                selectionColor: ui.accent; selectedTextColor: ui.onAccent
                            }
                            Rectangle {
                                visible: card.editing
                                Layout.fillWidth: true
                                implicitHeight: editor.implicitHeight + 16
                                radius: ui.smallRadius - 2; color: ui.background; border.color: ui.accentLine
                                TextEdit {
                                    id: editor
                                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8; leftMargin: 10 }
                                    textFormat: TextEdit.PlainText; wrapMode: TextEdit.Wrap; selectByMouse: true
                                    font.family: root.fontFamily; font.pixelSize: 12; color: ui.text
                                    selectionColor: ui.accent; selectedTextColor: ui.onAccent
                                    Keys.onEscapePressed: card.editing = false
                                }
                            }
                            RowLayout {
                                visible: card.editing
                                spacing: 6
                                OmaButton {
                                    theme: ui; kind: "primary"; small: true; text: root.t("translateEdit")
                                    onClicked: { root.send({op:"editBlock",index:card.index,text:editor.text}); card.editing = false }
                                }
                                OmaButton { theme: ui; kind: "ghost"; small: true; text: root.t("cancel"); onClicked: card.editing = false }
                            }
                        }
                    }
                }
            }
        }

        // ── History ──────────────────────────────────────────────
        ColumnLayout {
            visible: root.historyOpen && !root.settingsOpen
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.leftMargin: 14; Layout.rightMargin: 14; Layout.bottomMargin: 10; Layout.topMargin: 2
            spacing: 6
            OmaField {
                id: historySearch
                Layout.fillWidth: true
                visible: root.settings.history === true && root.history.length > 0
                theme: ui; iconName: "search"; fieldColor: ui.surface
                placeholderText: root.t("historySearch")
            }
            ListView {
                id: historyList
                visible: root.settings.history === true && count > 0
                Layout.fillWidth: true; Layout.fillHeight: true
                clip: true; spacing: 2
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}
                model: {
                    const q = historySearch.text.trim().toLowerCase()
                    return root.history.filter(function(e) {
                        return !q || String(e.source).toLowerCase().indexOf(q) >= 0 || String(e.translated).toLowerCase().indexOf(q) >= 0
                    })
                }
                delegate: Rectangle {
                    id: entry
                    required property var modelData
                    width: ListView.view.width - 6
                    implicitHeight: entryColumn.implicitHeight + 20
                    radius: ui.smallRadius
                    color: entryArea.containsMouse ? ui.surface2 : "transparent"
                    MouseArea { id: entryArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.restore(entry.modelData) }
                    ColumnLayout {
                        id: entryColumn
                        anchors { left: parent.left; right: actions.left; top: parent.top; margins: 10; leftMargin: 12 }
                        spacing: 4
                        Badge {
                            visible: !!entry.modelData.provider
                            theme: ui; tone: "neutral"
                            text: root.providerNames[Math.max(0, root.providers.indexOf(entry.modelData.provider))]
                        }
                        Text { Layout.fillWidth: true; text: entry.modelData.source; textFormat: Text.PlainText; elide: Text.ElideRight; maximumLineCount: 2; wrapMode: Text.Wrap; font.family: root.fontFamily; font.pixelSize: 12; color: ui.muted }
                        Text { Layout.fillWidth: true; text: entry.modelData.translated; textFormat: Text.PlainText; wrapMode: Text.Wrap; font.family: root.fontFamily; font.pixelSize: 13; lineHeight: 1.35; color: ui.text }
                    }
                    Row {
                        id: actions
                        anchors { right: parent.right; top: parent.top; margins: 8 }
                        spacing: 2
                        opacity: entryArea.containsMouse || hovered ? 1 : 0
                        readonly property bool hovered: restoreButton.hovered || copyButton.hovered
                        OmaButton { id: restoreButton; theme: ui; kind: "ghost"; small: true; iconName: "arrow"; tip: root.t("restore"); onClicked: root.restore(entry.modelData) }
                        OmaButton { id: copyButton; theme: ui; kind: "ghost"; small: true; iconName: "copy"; tip: root.t("copy"); onClicked: root.copyText(entry.modelData.translated) }
                    }
                }
            }
            ColumnLayout {
                visible: !historyList.visible
                Layout.fillWidth: true; Layout.fillHeight: true
                spacing: 12
                Item { Layout.fillHeight: true; Layout.fillWidth: true }
                Icon { Layout.alignment: Qt.AlignHCenter; name: "history"; size: 28; color: ui.faint }
                Text {
                    Layout.alignment: Qt.AlignHCenter; Layout.maximumWidth: 380
                    horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                    text: root.settings.history !== true ? root.t("historyOff") : root.history.length > 0 ? root.t("historyNothingFound") : root.t("historyEmpty")
                    textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 13; color: ui.muted
                }
                OmaButton {
                    visible: root.settings.history !== true
                    Layout.alignment: Qt.AlignHCenter
                    theme: ui; text: root.t("enableHistory")
                    onClicked: root.patch({history: true})
                }
                Item { Layout.fillHeight: true }
            }
        }

        SettingsView {
            id: settingsView
            visible: root.settingsOpen
            Layout.fillWidth: true; Layout.fillHeight: true
            stateData: root.stateData; translate: root.translate; theme: ui
            onCommand: function(request) { root.send(request) }
        }

        Item { visible: !root.settingsOpen && root.historyOpen === false && !root.imageMode; Layout.fillHeight: true }

        // ── Status bar ───────────────────────────────────────────
        Rectangle {
            visible: !root.settingsOpen
            Layout.fillWidth: true
            implicitHeight: statusRow.implicitHeight + 16
            color: ui.surface
            // Rounded like the card at the bottom, square at the top.
            radius: Math.max(0, root.cornerRadius - 1)
            Rectangle { width: parent.width; height: parent.radius; color: ui.surface }
            Rectangle { width: parent.width; height: 1; color: ui.line }
            RowLayout {
                id: statusRow
                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 10; rightMargin: 12 }
                spacing: 12

                AbstractButton {
                    id: providerButton
                    visible: !root.historyOpen
                    implicitWidth: providerRow.implicitWidth + 12; implicitHeight: 26
                    hoverEnabled: true
                    Accessible.name: root.t("provider") + ": " + root.providerName
                    onClicked: providerMenu.open()
                    background: Rectangle { radius: ui.smallRadius - 2; color: providerButton.hovered || providerMenu.opened ? ui.overlay : "transparent" }
                    contentItem: Item {
                        Row {
                            id: providerRow
                            anchors.centerIn: parent
                            spacing: 7
                            Rectangle {
                                width: 7; height: 7; radius: 4; anchors.verticalCenter: parent.verticalCenter
                                color: root.stateData.error || root.connectionError ? ui.danger : ui.accent
                            }
                            Text { text: root.providerName; textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 12; color: ui.muted; anchors.verticalCenter: parent.verticalCenter }
                            Icon { name: "chev"; color: ui.faint; anchors.verticalCenter: parent.verticalCenter }
                        }
                    }
                    Popup {
                        id: providerMenu
                        objectName: "providerMenu"
                        y: -height - 8
                        x: -2
                        width: 300
                        padding: 6
                        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
                        background: Rectangle { radius: root.cornerRadius; color: ui.surface; border.color: ui.line }
                        contentItem: Column {
                            spacing: 2
                            Repeater {
                                model: root.providers
                                AbstractButton {
                                    id: providerItem
                                    required property var modelData
                                    required property int index
                                    readonly property bool current: root.settings.provider === modelData
                                    readonly property bool missingModels: modelData === "local" && !(root.stateData.models || []).some(function(m) { return m.installed })
                                    width: 288; height: 44
                                    hoverEnabled: true
                                    Accessible.name: root.providerNames[index]
                                    onClicked: { providerMenu.close(); if (!current) root.patch({provider: modelData}, true) }
                                    background: Rectangle { radius: ui.smallRadius - 1; color: providerItem.hovered ? ui.surface2 : "transparent" }
                                    contentItem: Item {
                                        Item {
                                            id: checkSlot
                                            x: 10; width: 16; height: 16; anchors.verticalCenter: parent.verticalCenter
                                            Icon { visible: providerItem.current; name: "check"; color: ui.accent }
                                        }
                                        Column {
                                            anchors { left: checkSlot.right; leftMargin: 10; right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                                            spacing: 2
                                            Text {
                                                text: root.providerNames[providerItem.index]; textFormat: Text.PlainText
                                                font.family: root.fontFamily; font.pixelSize: 13
                                                font.weight: providerItem.current ? Font.DemiBold : Font.Normal
                                                color: ui.text
                                            }
                                            Text {
                                                width: parent.width; elide: Text.ElideRight
                                                text: providerItem.missingModels ? root.t("hintLocalMissing") : root.t("hint_" + providerItem.modelData)
                                                textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11
                                                color: providerItem.missingModels ? ui.warn : ui.faint
                                            }
                                        }
                                    }
                                }
                            }
                            Rectangle { width: 288; height: 1; color: ui.line }
                            AbstractButton {
                                id: configureItem
                                width: 288; height: 34
                                hoverEnabled: true
                                onClicked: { providerMenu.close(); settingsView.section = "services"; root.settingsOpen = true }
                                background: Rectangle { radius: ui.smallRadius - 1; color: configureItem.hovered ? ui.surface2 : "transparent" }
                                contentItem: Item {
                                    Row {
                                        x: 10; anchors.verticalCenter: parent.verticalCenter; spacing: 10
                                        Icon { name: "sliders"; color: ui.muted; anchors.verticalCenter: parent.verticalCenter }
                                        Text { text: root.t("configureServices"); textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 12; color: ui.muted; anchors.verticalCenter: parent.verticalCenter }
                                    }
                                }
                            }
                        }
                    }
                }
                Icon { visible: root.historyOpen || root.imageMode; name: "shield"; color: ui.faint }
                Text {
                    visible: root.historyOpen || root.imageMode
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    text: root.historyOpen
                        ? (root.history.length > 0 ? root.t("historyCount").arg(root.history.length) : root.t("historyLocal"))
                        : root.t("imagePrivacy") + " · OCR " + (root.settings.ocrLanguages || "eng") + (root.stateData.elapsedMs ? " · " + root.stateData.elapsedMs + " ms" : "")
                    textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint
                }
                // Notices ("Скопировано") everywhere; errors outside text mode, where no error card exists.
                Row {
                    readonly property string code: root.stateData.notice || ((root.imageMode && root.blocks.length > 0) || root.historyOpen ? (root.stateData.error || root.connectionError) : "")
                    readonly property bool bad: !root.stateData.notice
                    visible: code !== ""
                    spacing: 6
                    Layout.maximumWidth: 340
                    Icon { name: parent.bad ? "alert" : "check"; color: parent.bad ? ui.danger : ui.accent; anchors.verticalCenter: parent.verticalCenter }
                    Text {
                        width: Math.min(implicitWidth, 316)
                        elide: Text.ElideRight
                        text: root.message(parent.code); textFormat: Text.PlainText
                        font.family: root.fontFamily; font.pixelSize: 11; color: parent.bad ? ui.danger : ui.accent
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
                Item { visible: !root.historyOpen && !root.imageMode; Layout.fillWidth: true }
                Row {
                    visible: !root.historyOpen && !root.imageMode && root.width > 560
                    spacing: 12
                    Repeater {
                        model: [{keys: ["Ctrl", "Enter"], label: root.t("hintTranslate")}, {keys: ["Ctrl", "Shift", "C"], label: root.t("hintCopy")}, {keys: ["Esc"], label: root.t("hintClose")}]
                        Row {
                            required property var modelData
                            spacing: 6
                            Keycaps { theme: ui; keys: parent.modelData.keys }
                            Text { text: parent.modelData.label; textFormat: Text.PlainText; font.family: root.fontFamily; font.pixelSize: 11; color: ui.faint; anchors.verticalCenter: parent.verticalCenter }
                        }
                    }
                }
                OmaButton {
                    visible: root.imageMode && !root.historyOpen
                    theme: ui; kind: "ghost"; small: true; iconName: "download"; text: "PNG"
                    enabled: !!root.stateData.translatedImage; onClicked: root.send({op:"save"})
                }
                OmaButton {
                    visible: root.imageMode && !root.historyOpen
                    theme: ui; small: true; iconName: "copy"; text: root.t("copyTextShort")
                    enabled: !!root.stateData.translated && root.blocks.length > 0 && !root.busy; onClicked: root.send({op:"copy",kind:"text"})
                }
                OmaButton {
                    visible: root.imageMode && !root.historyOpen
                    theme: ui; kind: "primary"; small: true; iconName: "image"; text: root.t("copyImage")
                    enabled: !!root.stateData.translatedImage; onClicked: root.send({op:"copy",kind:"image"})
                }
                OmaButton {
                    id: clearHistory
                    property bool armed: false
                    visible: root.historyOpen && root.history.length > 0
                    theme: ui; kind: "danger"; small: true; iconName: "trash"
                    text: armed ? root.t("clearConfirm") : root.t("clearHistory")
                    onClicked: { if (armed) { root.send({op:"clearHistory"}); armed = false } else { armed = true; disarm.restart() } }
                    Timer { id: disarm; interval: 3000; onTriggered: clearHistory.armed = false }
                }
            }
        }
    }
}
