import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Settings with a section list on the left. Every change is sent at once; there is no "Apply".
Item {
    id: root
    property var stateData: ({})
    property var translate: function(key) { return key }
    property var theme
    property string section: "general"
    readonly property var settings: stateData.settings || ({})
    readonly property var sections: [
        {id: "general", icon: "sliders", label: t("secGeneral")},
        {id: "services", icon: "globe", label: t("secServices")},
        {id: "models", icon: "cpu", label: t("secModels")},
        {id: "images", icon: "image", label: t("secImages")},
        {id: "shortcuts", icon: "keyboard", label: t("secShortcuts")}
    ]
    signal command(var request)
    function t(key) { return translate(key) }
    function patch(value) { command({op:"setSettings",settings:value}) }
    readonly property var languages: stateData.languages || []
    function languageName(code) {
        for (let i = 0; i < languages.length; i++) if (languages[i].code === code) return languages[i].name
        return code
    }
    function ocrName(code) {
        for (let i = 0; i < languages.length; i++) if (languages[i].ocr === code) return languages[i].name
        return code
    }
    readonly property var ocrSelected: String(settings.ocrLanguages || "eng").split("+")
    function toggleOcr(code) {
        const next = ocrSelected.filter(function(c) { return c !== code })
        if (next.length === ocrSelected.length) next.push(code)
        if (next.length > 0) patch({ocrLanguages: next.join("+")})
    }
    // Tesseract data for the translation target that isn't installed yet.
    readonly property string missingOcr: {
        for (let i = 0; i < languages.length; i++)
            if (languages[i].code === settings.target) {
                const code = languages[i].ocr
                const installed = stateData.ocrInstalled || []
                return installed.length && installed.indexOf(code) < 0 ? code : ""
            }
        return ""
    }

    component Title: Text {
        textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 18; font.weight: Font.DemiBold
        color: root.theme.text
    }
    component Hint: Text {
        Layout.fillWidth: true
        textFormat: Text.PlainText; wrapMode: Text.Wrap; font.family: root.theme.fontFamily; font.pixelSize: 11
        lineHeight: 1.35; color: root.theme.muted
    }
    component Label13: Text {
        textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 13; color: root.theme.text
    }
    // A setting row: title, explanation, control on the right, divider below.
    component OptionRow: Item {
        id: option
        property string title: ""
        property string description: ""
        default property alias control: slot.data
        Layout.fillWidth: true
        implicitHeight: Math.max(texts.implicitHeight, slot.implicitHeight) + 26
        ColumnLayout {
            id: texts
            anchors { left: parent.left; right: slot.left; rightMargin: 16; verticalCenter: parent.verticalCenter }
            spacing: 3
            Label13 { Layout.fillWidth: true; text: option.title; wrapMode: Text.Wrap }
            Hint { visible: option.description !== ""; text: option.description }
        }
        Row {
            id: slot
            anchors { right: parent.right; verticalCenter: parent.verticalCenter }
            spacing: 10
        }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: root.theme.line }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.preferredWidth: 220; Layout.fillHeight: true
            color: root.theme.surface
            // Rounded only in the card's bottom-left corner.
            radius: Math.max(0, root.theme.radius - 1)
            Rectangle { width: parent.width; height: parent.radius; color: root.theme.surface }
            Rectangle { anchors.right: parent.right; width: parent.radius; height: parent.height; color: root.theme.surface }
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: root.theme.line }
            ColumnLayout {
                anchors { fill: parent; margins: 10 }
                spacing: 2
                Repeater {
                    model: root.sections
                    AbstractButton {
                        id: navItem
                        required property var modelData
                        readonly property bool current: root.section === modelData.id
                        Layout.fillWidth: true
                        implicitHeight: 34
                        hoverEnabled: true
                        focusPolicy: Qt.StrongFocus
                        Accessible.name: modelData.label
                        onClicked: root.section = modelData.id
                        background: Rectangle {
                            radius: root.theme.smallRadius - 1
                            color: navItem.current ? root.theme.surface2 : navItem.hovered ? root.theme.overlay : "transparent"
                            border.color: navItem.visualFocus ? root.theme.accent : "transparent"
                        }
                        contentItem: Item {
                            Row {
                                x: 10 - navItem.leftPadding; anchors.verticalCenter: parent.verticalCenter
                                spacing: 10
                                Icon { name: navItem.modelData.icon; color: navItem.current ? root.theme.accent : root.theme.muted; anchors.verticalCenter: parent.verticalCenter }
                                Text {
                                    text: navItem.modelData.label; textFormat: Text.PlainText
                                    font.family: root.theme.fontFamily; font.pixelSize: 13; font.weight: navItem.current ? Font.DemiBold : Font.Normal
                                    color: navItem.current ? root.theme.text : root.theme.muted
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                        }
                    }
                }
                Item { Layout.fillHeight: true }
                Text { text: "OmaTranslate"; textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 11; color: root.theme.faint }
            }
        }

        ScrollView {
            id: scroller
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: scroller.availableWidth
                spacing: 0
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 28; Layout.topMargin: 22; Layout.bottomMargin: 22
                    spacing: 14

                    // ── General ──
                    ColumnLayout {
                        visible: root.section === "general"
                        Layout.fillWidth: true; spacing: 4
                        Title { text: root.t("secGeneral") }
                        Hint { text: root.t("generalDesc"); Layout.bottomMargin: 8 }
                        OptionRow {
                            title: root.t("autoTranslate"); description: root.t("autoTranslateDesc")
                            Segmented {
                                visible: root.settings.autoTranslate === true
                                theme: root.theme; itemHeight: 24
                                readonly property var values: [150, 250, 500]
                                model: ["150", "250", "500 ms"]
                                currentIndex: Math.max(0, values.indexOf(root.settings.debounceMs || 250))
                                onActivated: function(index) { root.patch({debounceMs: values[index]}) }
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            OmaSwitch { theme: root.theme; checked: root.settings.autoTranslate === true; onToggled: root.patch({autoTranslate: checked}); anchors.verticalCenter: parent.verticalCenter }
                        }
                        OptionRow {
                            title: root.t("myLanguageTitle"); description: root.t("myLanguageDesc")
                            LanguagePicker {
                                theme: root.theme; translate: root.translate
                                codes: root.languages.map(function(l) { return l.code })
                                names: root.languages.map(function(l) { return l.name })
                                aliases: root.languages.map(function(l) { return l.english })
                                current: root.settings.target || "en"
                                onPicked: function(code) { root.patch({target: code}) }
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                        OptionRow {
                            id: secondRow
                            readonly property string lastName: root.t("lastLanguage").arg(root.languageName(root.settings.lastLanguage || "en"))
                            title: root.t("autoSwapTitle").arg(root.languageName(root.settings.target))
                                .arg(root.settings.secondLanguage === "last" ? lastName : root.languageName(root.settings.secondLanguage))
                            description: root.t("autoSwapDesc")
                            LanguagePicker {
                                theme: root.theme; translate: root.translate
                                // "last" follows the language you worked with most recently; a code pins one.
                                codes: ["last"].concat(root.languages.map(function(l) { return l.code }))
                                names: [secondRow.lastName].concat(root.languages.map(function(l) { return l.name }))
                                aliases: [""].concat(root.languages.map(function(l) { return l.english }))
                                current: root.settings.secondLanguage || "last"
                                onPicked: function(code) { root.patch({secondLanguage: code}) }
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            OmaSwitch { theme: root.theme; checked: root.settings.autoSwap === true; onToggled: root.patch({autoSwap: checked}); anchors.verticalCenter: parent.verticalCenter }
                        }
                        OptionRow {
                            title: root.t("saveHistoryTitle"); description: root.t("saveHistoryDesc")
                            OmaSwitch { theme: root.theme; checked: root.settings.history === true; onToggled: root.patch({history: checked}) }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true; Layout.topMargin: 14
                            spacing: 4
                            Label13 { text: root.t("glossaryTitle") }
                            Hint { text: root.t("glossaryDesc") }
                            Rectangle {
                                Layout.fillWidth: true; Layout.topMargin: 6
                                implicitHeight: Math.max(76, glossary.implicitHeight + 16)
                                radius: root.theme.smallRadius
                                color: root.theme.background
                                border.color: glossary.activeFocus ? root.theme.accent : root.theme.line
                                TextEdit {
                                    id: glossary
                                    objectName: "glossaryInput"
                                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8; leftMargin: 12 }
                                    text: (root.settings.glossary || []).join("\n")
                                    textFormat: TextEdit.PlainText; wrapMode: TextEdit.Wrap; selectByMouse: true
                                    font.family: root.theme.fontFamily; font.pixelSize: 12; color: root.theme.text
                                    selectionColor: root.theme.accent; selectedTextColor: root.theme.onAccent
                                    readonly property var terms: text.split(/[\n,]/).map(function(t) { return t.trim() }).filter(function(t) { return t !== "" })
                                    onActiveFocusChanged: if (!activeFocus && JSON.stringify(terms) !== JSON.stringify(root.settings.glossary || [])) root.patch({glossary: terms})
                                    Text {
                                        visible: !glossary.text && !glossary.activeFocus
                                        text: "Omarchy\nHyprland\nClaude Code"; textFormat: Text.PlainText
                                        font: glossary.font; color: root.theme.faint
                                    }
                                }
                            }
                        }
                    }

                    // ── Services ──
                    ColumnLayout {
                        visible: root.section === "services"
                        Layout.fillWidth: true; spacing: 10
                        Title { text: root.t("servicesTitle") }
                        Hint { text: root.t("providersHelp"); Layout.bottomMargin: 4 }
                        Repeater {
                            model: [
                                {id: "google", icon: "globe", name: "Google", desc: root.t("googleDesc")},
                                {id: "mymemory", icon: "globe", name: "MyMemory", desc: root.t("mymemoryDesc")},
                                {id: "libre", icon: "server", name: "LibreTranslate", desc: root.t("libreDesc")},
                                {id: "local", icon: "cpu", name: root.t("offline"), desc: root.t("offlineDesc")}
                            ]
                            Rectangle {
                                id: card
                                required property var modelData
                                readonly property bool selected: (root.settings.provider || "google") === modelData.id
                                Layout.fillWidth: true
                                implicitHeight: cardRow.implicitHeight + 28
                                radius: root.theme.smallRadius + 2
                                color: selected ? root.theme.accentTint : cardArea.containsMouse ? root.theme.overlay : "transparent"
                                border.color: selected ? root.theme.accent : root.theme.line
                                MouseArea { id: cardArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.patch({provider: card.modelData.id}) }
                                RowLayout {
                                    id: cardRow
                                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14; leftMargin: 16; rightMargin: 16 }
                                    spacing: 14
                                    Rectangle {
                                        Layout.alignment: Qt.AlignTop
                                        implicitWidth: 18; implicitHeight: 18; radius: 9; color: "transparent"
                                        border.width: 1.5; border.color: card.selected ? root.theme.accent : root.theme.faint
                                        Rectangle { visible: card.selected; anchors.centerIn: parent; width: 8; height: 8; radius: 4; color: root.theme.accent }
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true; spacing: 6
                                        Row {
                                            spacing: 8
                                            Icon { name: card.modelData.icon; color: card.selected ? root.theme.accent : root.theme.muted; anchors.verticalCenter: parent.verticalCenter }
                                            Text { text: card.modelData.name; textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 14; font.weight: Font.DemiBold; color: root.theme.text }
                                            Badge { visible: card.modelData.id === "local"; theme: root.theme; tone: "neutral"; text: "EN ↔ RU"; anchors.verticalCenter: parent.verticalCenter }
                                        }
                                        Hint { text: card.modelData.desc; font.pixelSize: 12 }
                                        RowLayout {
                                            visible: card.modelData.id === "libre"
                                            Layout.fillWidth: true; spacing: 8
                                            OmaField {
                                                Layout.fillWidth: true; theme: root.theme
                                                text: root.settings.libreUrl || "http://localhost:5000"; placeholderText: root.t("serverUrl")
                                                onEditingFinished: if (text !== root.settings.libreUrl) root.patch({libreUrl: text})
                                            }
                                            OmaField {
                                                Layout.preferredWidth: 190; theme: root.theme
                                                text: root.settings.libreKey || ""; placeholderText: root.t("apiKey"); echoMode: TextInput.Password
                                                onEditingFinished: if (text !== (root.settings.libreKey || "")) root.patch({libreKey: text})
                                            }
                                        }
                                        Row {
                                            visible: card.modelData.id === "local"
                                            spacing: 10
                                            Text {
                                                text: root.t("modelsCount").arg((root.stateData.models || []).filter(function(m) { return m.installed }).length).arg((root.stateData.models || []).length)
                                                textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 11; color: root.theme.faint
                                                anchors.verticalCenter: parent.verticalCenter
                                            }
                                            OmaButton { theme: root.theme; kind: "ghost"; small: true; text: root.t("manageModels") + " →"; onClicked: root.section = "models" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Offline models ──
                    ColumnLayout {
                        visible: root.section === "models"
                        Layout.fillWidth: true; spacing: 10
                        Title { text: root.t("secModels") }
                        Hint { text: root.t("modelsDesc"); Layout.bottomMargin: 4 }
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: engineRow.implicitHeight + 20
                            radius: root.theme.smallRadius
                            color: root.stateData.offlineEngine ? root.theme.accentTint : root.theme.alpha(root.theme.warn, 0.08)
                            border.color: root.stateData.offlineEngine ? root.theme.accentLine : root.theme.alpha(root.theme.warn, 0.3)
                            RowLayout {
                                id: engineRow
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 14 }
                                spacing: 10
                                Icon { name: root.stateData.offlineEngine ? "check" : "alert"; color: root.stateData.offlineEngine ? root.theme.accent : root.theme.warn }
                                Text {
                                    Layout.fillWidth: true; wrapMode: Text.Wrap
                                    text: root.stateData.offlineEngine ? root.t("engineReady") : root.t("engineMissingHelp")
                                    textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 12; color: root.theme.text
                                }
                            }
                        }
                        Repeater {
                            model: root.stateData.models || []
                            Rectangle {
                                id: modelCard
                                required property var modelData
                                readonly property bool downloading: !!root.stateData.download && root.stateData.download.pair === modelData.pair
                                readonly property var download: root.stateData.download || ({})
                                Layout.fillWidth: true
                                implicitHeight: modelColumn.implicitHeight + 28
                                radius: root.theme.smallRadius + 2
                                color: "transparent"; border.color: root.theme.line
                                ColumnLayout {
                                    id: modelColumn
                                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14; leftMargin: 16; rightMargin: 16 }
                                    spacing: 10
                                    RowLayout {
                                        Layout.fillWidth: true; spacing: 12
                                        Rectangle {
                                            implicitWidth: 34; implicitHeight: 34; radius: root.theme.smallRadius; color: root.theme.surface
                                            Icon { anchors.centerIn: parent; name: "cpu"; color: modelCard.modelData.installed ? root.theme.accent : root.theme.muted }
                                        }
                                        ColumnLayout {
                                            Layout.fillWidth: true; spacing: 3
                                            Text { Layout.fillWidth: true; text: String(modelCard.modelData.pair).split("_").map(root.languageName).join(" → "); textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 14; font.weight: Font.DemiBold; color: root.theme.text }
                                            Text {
                                                text: modelCard.modelData.installed ? root.t("installed") + " · Argos " + modelCard.modelData.version
                                                    : modelCard.downloading ? root.t("downloading") : root.t("notInstalled")
                                                textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 11; color: root.theme.muted
                                            }
                                        }
                                        OmaButton {
                                            visible: !modelCard.downloading
                                            theme: root.theme; small: true
                                            kind: modelCard.modelData.installed ? "danger" : "subtle"
                                            iconName: modelCard.modelData.installed ? "trash" : "download"
                                            text: modelCard.modelData.installed ? root.t("remove") : root.t("download")
                                            enabled: !root.stateData.download
                                            onClicked: root.command({op: modelCard.modelData.installed ? "removeModel" : "downloadModel", pair: modelCard.modelData.pair})
                                        }
                                        OmaButton { visible: modelCard.downloading; theme: root.theme; small: true; kind: "outline"; text: root.t("cancel"); onClicked: root.command({op:"cancelDownload"}) }
                                    }
                                    RowLayout {
                                        visible: modelCard.downloading
                                        Layout.fillWidth: true; spacing: 10
                                        ProgressBar {
                                            id: progress
                                            Layout.fillWidth: true
                                            from: 0; to: modelCard.download.total || 1; value: modelCard.download.received || 0
                                            indeterminate: !modelCard.download.total
                                            background: Rectangle { implicitHeight: 4; radius: 2; color: root.theme.surface2 }
                                            contentItem: Item {
                                                implicitHeight: 4
                                                Rectangle { width: progress.visualPosition * parent.width; height: 4; radius: 2; color: root.theme.accent; visible: !progress.indeterminate }
                                            }
                                        }
                                        Text {
                                            text: ((modelCard.download.received || 0) / 1048576).toFixed(1) + " / " + (modelCard.download.total ? (modelCard.download.total / 1048576).toFixed(1) : "…") + " MB"
                                            textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 11; color: root.theme.muted
                                        }
                                    }
                                }
                            }
                        }
                        OptionRow {
                            title: root.t("keepModelsWarm"); description: root.t("modelHelp")
                            OmaSwitch { theme: root.theme; checked: root.settings.keepModelsWarm === true; onToggled: root.patch({keepModelsWarm: checked}) }
                        }
                    }

                    // ── Images ──
                    ColumnLayout {
                        visible: root.section === "images"
                        Layout.fillWidth: true; spacing: 4
                        Title { text: root.t("secImages") }
                        Hint { text: root.t("imagesDesc"); Layout.bottomMargin: 8 }
                        OptionRow {
                            title: root.t("ocrLanguage"); description: root.t("ocrDesc")
                            Flow {
                                width: 300; spacing: 6
                                anchors.verticalCenter: parent.verticalCenter
                                layoutDirection: Qt.RightToLeft
                                Repeater {
                                    model: ((root.stateData.ocrInstalled && root.stateData.ocrInstalled.length) ? root.stateData.ocrInstalled : root.ocrSelected).slice().reverse()
                                    OmaButton {
                                        required property string modelData
                                        theme: root.theme; small: true
                                        kind: root.ocrSelected.indexOf(modelData) >= 0 ? "primary" : "outline"
                                        text: root.ocrName(modelData)
                                        tip: modelData
                                        onClicked: root.toggleOcr(modelData)
                                    }
                                }
                            }
                        }
                        Hint {
                            visible: root.missingOcr !== ""
                            text: root.t("ocrMissingHelp").arg(root.languageName(root.settings.target)).arg("sudo pacman -S tesseract-data-" + root.missingOcr)
                        }
                        OptionRow {
                            title: root.t("minimumFont"); description: root.t("minFontDesc")
                            Slider {
                                id: fontSlider
                                width: 180; from: 10; to: 28; stepSize: 1; snapMode: Slider.SnapAlways
                                value: root.settings.minFontSize || 12
                                onMoved: root.patch({minFontSize: value})
                                anchors.verticalCenter: parent.verticalCenter
                                background: Rectangle {
                                    x: fontSlider.leftPadding; y: fontSlider.topPadding + fontSlider.availableHeight / 2 - 2
                                    width: fontSlider.availableWidth; height: 4; radius: 2; color: root.theme.surface2
                                    Rectangle { width: fontSlider.visualPosition * parent.width; height: 4; radius: 2; color: root.theme.accent }
                                }
                                handle: Rectangle {
                                    x: fontSlider.leftPadding + fontSlider.visualPosition * (fontSlider.availableWidth - width)
                                    y: fontSlider.topPadding + fontSlider.availableHeight / 2 - 7
                                    width: 14; height: 14; radius: 7; color: root.theme.accent
                                    border.color: fontSlider.visualFocus ? root.theme.text : "transparent"
                                }
                            }
                            Text { text: Math.round(fontSlider.value) + " px"; textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 12; color: root.theme.text; anchors.verticalCenter: parent.verticalCenter }
                        }
                        Rectangle {
                            Layout.fillWidth: true; Layout.topMargin: 10
                            implicitHeight: privacyRow.implicitHeight + 20
                            radius: root.theme.smallRadius; color: root.theme.surface
                            RowLayout {
                                id: privacyRow
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 14 }
                                spacing: 10
                                Icon { name: "shield"; color: root.theme.accent }
                                Text { Layout.fillWidth: true; wrapMode: Text.Wrap; text: root.t("imagesPrivacy"); textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 12; color: root.theme.text }
                            }
                        }
                    }

                    // ── Shortcuts ──
                    ColumnLayout {
                        visible: root.section === "shortcuts"
                        Layout.fillWidth: true; spacing: 10
                        Title { text: root.t("secShortcuts") }
                        Hint { text: root.t("shortcutsDesc"); Layout.bottomMargin: 4 }
                        Repeater {
                            model: [
                                {key: "selectionKey", title: root.t("scSelection"), desc: root.t("scSelectionDesc"), fallback: "SUPER + U"},
                                {key: "clipboardKey", title: root.t("scClipboard"), desc: root.t("scClipboardDesc"), fallback: "SUPER + SHIFT + U"},
                                {key: "screenshotKey", title: root.t("scRegion"), desc: root.t("scRegionDesc"), fallback: "SUPER + ALT + U"},
                                {key: "replaceKey", title: root.t("scReplace"), desc: root.t("scReplaceDesc"), fallback: "SUPER + CTRL + U"}
                            ]
                            Rectangle {
                                id: shortcut
                                required property var modelData
                                property bool recording: false
                                Layout.fillWidth: true
                                implicitHeight: shortcutRow.implicitHeight + 24
                                radius: root.theme.smallRadius + 2
                                color: recording ? root.theme.accentTint : "transparent"
                                border.color: recording ? root.theme.accent : root.theme.line
                                function record(event) {
                                    if (event.key === Qt.Key_Escape) { recording = false; event.accepted = true; return }
                                    const modifierKeys = [Qt.Key_Shift, Qt.Key_Control, Qt.Key_Alt, Qt.Key_Meta, Qt.Key_Super_L, Qt.Key_Super_R, Qt.Key_AltGr]
                                    event.accepted = true
                                    if (modifierKeys.indexOf(event.key) >= 0) return
                                    const parts = []
                                    if (event.modifiers & Qt.MetaModifier) parts.push("SUPER")
                                    if (event.modifiers & Qt.ControlModifier) parts.push("CTRL")
                                    if (event.modifiers & Qt.AltModifier) parts.push("ALT")
                                    if (event.modifiers & Qt.ShiftModifier) parts.push("SHIFT")
                                    if (parts.length === 0) return
                                    let name = ""
                                    if (event.key >= Qt.Key_A && event.key <= Qt.Key_Z) name = String.fromCharCode(event.key)
                                    else if (event.key >= Qt.Key_0 && event.key <= Qt.Key_9) name = String.fromCharCode(event.key)
                                    else if (event.key >= Qt.Key_F1 && event.key <= Qt.Key_F12) name = "F" + (event.key - Qt.Key_F1 + 1)
                                    else name = (event.text || "").toUpperCase()
                                    if (!name) return
                                    parts.push(name)
                                    const value = {}
                                    value[modelData.key] = parts.join(" + ")
                                    root.patch(value)
                                    root.command({op: "applyShortcuts"})
                                    recording = false
                                }
                                RowLayout {
                                    id: shortcutRow
                                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 16; rightMargin: 12 }
                                    spacing: 14
                                    ColumnLayout {
                                        Layout.fillWidth: true; spacing: 3
                                        Text { text: shortcut.modelData.title; textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 13; font.weight: Font.DemiBold; color: root.theme.text }
                                        Hint { text: shortcut.modelData.desc }
                                    }
                                    Keycaps { visible: !shortcut.recording; theme: root.theme; fontSize: 11; keys: root.settings[shortcut.modelData.key] || shortcut.modelData.fallback }
                                    OmaButton {
                                        visible: !shortcut.recording
                                        theme: root.theme; kind: "ghost"; small: true; iconName: "pencil"; tip: root.t("change")
                                        onClicked: { shortcut.recording = true; recorder.forceActiveFocus() }
                                    }
                                    Rectangle {
                                        visible: shortcut.recording
                                        implicitWidth: recorderRow.implicitWidth + 24; implicitHeight: 34
                                        radius: root.theme.smallRadius; color: root.theme.background; border.color: root.theme.accent
                                        Row {
                                            id: recorderRow
                                            anchors.centerIn: parent; spacing: 8
                                            Rectangle { width: 7; height: 7; radius: 4; color: root.theme.danger; anchors.verticalCenter: parent.verticalCenter }
                                            Text { text: root.t("pressShortcut"); textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 12; color: root.theme.text; anchors.verticalCenter: parent.verticalCenter }
                                            Keycaps { theme: root.theme; keys: ["Esc"]; anchors.verticalCenter: parent.verticalCenter }
                                        }
                                        Item {
                                            id: recorder
                                            anchors.fill: parent
                                            focus: shortcut.recording
                                            Keys.onPressed: function(event) { shortcut.record(event) }
                                            onActiveFocusChanged: if (!activeFocus) shortcut.recording = false
                                        }
                                    }
                                }
                            }
                        }
                        RowLayout {
                            spacing: 8
                            Icon { name: "info"; color: root.theme.faint }
                            Text { text: root.t("shortcutsNote"); textFormat: Text.PlainText; font.family: root.theme.fontFamily; font.pixelSize: 11; color: root.theme.faint }
                        }
                    }
                }
            }
        }
    }
}
