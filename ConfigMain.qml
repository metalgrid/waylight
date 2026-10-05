pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The config editor window. Edits live in memory until Apply sends them to
// waylight-configd (SetTheme/SetLanguage); Revert re-reads the daemon. The
// live preview instantiates the greeter's own Main.qml — it is an
// ApplicationWindow, so a separate top-level window is the correct embedding —
// with a mock backend and the draft theme applied in memory through
// Theme.qml's applyJson API. The token tables mirror Theme.qml; the greeter's
// fail-open validation stays the single source of truth (this file never
// rejects unknown keys, it only rounds-trips them through Theme).
ApplicationWindow {
    id: root
    width: 1040
    height: 720
    minimumWidth: 880
    minimumHeight: 560
    visible: true
    title: qsTr("Waylight configuration")
    LayoutMirroring.enabled: Qt.application.layoutDirection === Qt.RightToLeft
    LayoutMirroring.childrenInherit: true

    required property var backend

    // --- token tables, mirroring Theme.qml ---------------------------------
    // Colors emitted into the "colors" section (background colors belong to
    // the background section below).
    readonly property var _colorSection: [
        "windowColor", "overlayTop", "overlayBottom", "textBar", "textPrimary",
        "textBright", "textDate", "textSecondary", "textPlaceholder",
        "textCombo", "accent", "error", "fillHover", "fillRest", "fillDown",
        "fillAnswer", "fillSubtle", "borderSelected", "borderField",
        "borderSubtle", "borderButton", "borderAvatar", "avatarFallback",
        "avatarGlyph", "popupBackground", "popupBorder"
    ]
    // Every color token, for kind detection (Theme._colorKeys).
    readonly property var _colorTokens: root._colorSection.concat([
        "backgroundColor", "backgroundTop", "backgroundBottom"
    ])
    readonly property var _backgroundMap: ({
        mode: "backgroundMode",
        image: "backgroundImage",
        color: "backgroundColor",
        top: "backgroundTop",
        bottom: "backgroundBottom"
    })
    readonly property var _fontMap: ({family: "fontFamily", scale: "fontScale"})
    readonly property var _ratioTokens: ["clockTopRatio", "panelTopRatio", "panelTopRatioShort"]
    readonly property var _sizeTokens: [
        "clockDateSize", "clockTimeCap", "panelShortHeight", "panelMaxWidth",
        "panelBottomMargin", "tileWidth", "tileHeight", "avatarSize",
        "tileRadius", "fieldWidth", "fieldHeight", "fieldRadius",
        "controlRadius", "popupRadius", "optionRadius", "powerSize",
        "powerRadius", "barMarginLeft", "barMarginRight", "bottomBarMargin"
    ]
    readonly property var _allTokens: (function () {
        const list = root._colorTokens.slice();
        for (const key in root._backgroundMap) list.push(root._backgroundMap[key]);
        for (const key in root._fontMap) list.push(root._fontMap[key]);
        return list.concat(root._ratioTokens, root._sizeTokens);
    })()
    readonly property var _presets: ["dusk", "midnight", "daylight"]
    readonly property var _backgroundModes: ["builtin", "image", "solid", "gradient"]
    // Native names, deliberately untranslated; the display language of the
    // login screen itself is what the user picks here.
    readonly property var languages: [
        {code: "en", name: "English"},
        {code: "zh", name: "简体中文"},
        {code: "hi", name: "हिन्दी"},
        {code: "es", name: "Español"},
        {code: "fr", name: "Français"},
        {code: "ar", name: "العربية"},
        {code: "bn", name: "বাংলা"},
        {code: "pt", name: "Português (Brasil)"},
        {code: "ru", name: "Русский"},
        {code: "ur", name: "اردو"},
        {code: "bg", name: "Български"}
    ]
    readonly property string _colorPattern: "^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"

    // --- draft state --------------------------------------------------------
    property var tokens: ({})
    property string languageCode: "en"
    property string loadedLanguage: "en"
    property string rawText: ""
    property bool showPreview: false
    property string localStatus: ""
    // Suppresses editor echo while a daemon load rebinds the draft.
    property bool loading: false

    // The same fail-open Theme component the greeter uses: one instance
    // normalizes daemon files, presets and raw JSON into concrete tokens.
    Theme {
        id: normalizer
    }

    function tokenValue(token) {
        const value = normalizer[token];
        if (root._colorTokens.includes(token)) return String(value);
        if (typeof value === "number") return value;
        return String(value);
    }

    function tokensFromJson(text) {
        normalizer.applyJson(text);
        const map = {};
        for (const token of root._allTokens) map[token] = root.tokenValue(token);
        return map;
    }

    function serializeTheme() {
        const out = {colors: {}, background: {}, font: {}, layout: {}};
        for (const token of root._colorSection) out.colors[token] = root.tokens[token];
        for (const key in root._backgroundMap) out.background[key] = root.tokens[root._backgroundMap[key]];
        for (const key in root._fontMap) out.font[key] = root.tokens[root._fontMap[key]];
        for (const token of root._ratioTokens) out.layout[token] = root.tokens[token];
        for (const token of root._sizeTokens) out.layout[token] = root.tokens[token];
        return JSON.stringify(out, null, 2);
    }

    function effectiveThemeText() {
        return tabBar.currentIndex === 1 ? root.rawText : root.serializeTheme();
    }

    // Mirrors the daemon's structural validation so the raw editor can warn
    // before a round trip is wasted; values stay the greeter's business.
    function structuralProblem(text) {
        let data;
        try {
            data = JSON.parse(text);
        } catch (error) {
            return qsTr("Not valid JSON.");
        }
        if (data === null || typeof data !== "object" || Array.isArray(data))
            return qsTr("Not a JSON object.");
        for (const key in data)
            if (!["colors", "background", "font", "layout"].includes(key))
                return qsTr("Unknown section '%1'; the daemon allows colors, background, font, layout.").arg(key);
        for (const key of ["colors", "background", "font", "layout"])
            if (key in data && (typeof data[key] !== "object" || data[key] === null || Array.isArray(data[key])))
                return qsTr("Section '%1' must be an object.").arg(key);
        return "";
    }

    function languageFromConfig(text) {
        try {
            const data = JSON.parse(text);
            if (data !== null && typeof data === "object" && typeof data.language === "string")
                return data.language;
        } catch (error) { /* fail open to English */ }
        return "en";
    }

    function loadFromDaemon() {
        root.loading = true;
        // An empty daemon reply means "no file yet": defaults, no warning.
        const theme = typeof root.backend.theme === "string" ? root.backend.theme : "";
        const config = typeof root.backend.config === "string" ? root.backend.config : "";
        root.tokens = root.tokensFromJson(theme === "" ? "{}" : theme);
        root.languageCode = root.languageFromConfig(config === "" ? "{}" : config);
        root.loadedLanguage = root.languageCode;
        root.rawText = root.serializeTheme();
        root.loading = false;
        root.localStatus = "";
        root.applyToPreview();
    }

    function setToken(key, value) {
        if (root.loading) return;
        const next = Object.assign({}, root.tokens);
        next[key] = value;
        root.tokens = next;
    }

    function applyPreset(name) {
        if (root.loading) return;
        root.tokens = root.tokensFromJson(JSON.stringify({preset: name}));
    }

    function apply() {
        if (tabBar.currentIndex === 1) {
            const problem = root.structuralProblem(root.rawText);
            if (problem !== "") {
                root.localStatus = problem;
                return;
            }
        }
        const language = root.languageCode;
        const languageChanged = language !== root.loadedLanguage;
        root.localStatus = qsTr("Applying…");
        root.backend.applyTheme(root.effectiveThemeText());
        if (languageChanged)
            root.backend.applyLanguage(language);
    }

    function revert() {
        root.localStatus = qsTr("Reverting…");
        root.backend.reload();
    }

    Component.onCompleted: root.backend.reload()
    onTokensChanged: previewDebounce.restart()

    Connections {
        target: root.backend
        function onThemeChanged() { root.loadFromDaemon(); }
        function onConfigChanged() { root.loadFromDaemon(); }
        function onApplied() { root.backend.reload(); }
        // A terminal daemon reply (ready or error) ends the local
        // "Applying…"/"Reverting…" placeholder so the daemon's own message —
        // in particular a validation failure — becomes visible.
        function onStatusChanged() {
            if (root.backend.status !== "busy") root.localStatus = "";
        }
    }

    // --- live preview -------------------------------------------------------
    // Mock backend with the exact property/function surface of the real
    // greeter Backend that Main.qml consumes; nothing is polled and no
    // greetd interaction can happen.
    QtObject {
        id: previewBackend
        property string state: "idle"
        property string message: ""
        property string prompt: ""
        property var sessions: []
        property int selected: 0
        property int capabilities: 0
        property bool preview: true
        property string accounts: "[]"
        property bool closing: false
        property string themePaths: "[]"
        // Same surface as the real Backend: the resolved UI language drives
        // the preview clock's locale-sensitive rendering.
        property string uiLanguage: "en"
        signal identityChosen(string name)
        function begin(name, index) {}
        function answer(text) {}
        function cancel() {}
        function choose_user(name, index) {}
        function choose_manual() {}
        function power(index) {}
        function reset_identity() { return true; }
        function shutdown() {}
    }

    Main {
        id: preview
        backend: previewBackend
        // Main.qml defaults to visible; the draft editor decides.
        visible: root.showPreview
        onClosing: function (close) {
            close.accepted = false;
            root.showPreview = false;
        }
    }

    function previewThemeObject() {
        // ApplicationWindow routes non-visual children (the internal Theme)
        // into contentData, not data.
        for (let i = 0; i < preview.contentData.length; ++i) {
            const candidate = preview.contentData[i];
            if (candidate !== null && candidate.objectName === "theme") return candidate;
        }
        return null;
    }

    function applyToPreview() {
        if (!root.showPreview) return;
        const themeObject = root.previewThemeObject();
        if (themeObject !== null) themeObject.applyJson(root.effectiveThemeText());
    }

    Timer {
        id: previewDebounce
        interval: 150
        onTriggered: root.applyToPreview()
    }

    // --- window chrome -------------------------------------------------------
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            Label {
                text: qsTr("Waylight configuration")
                font.bold: true
                font.pixelSize: 16
            }
            Label {
                objectName: "statusLabel"
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                text: root.localStatus !== "" ? root.localStatus
                    : typeof root.backend.message === "string" ? root.backend.message : ""
                color: root.localStatus !== "" || root.backend.status === "error" ? "firebrick" : palette.text
            }
            Button {
                checkable: true
                checked: root.showPreview
                text: qsTr("Preview")
                Accessible.name: qsTr("Toggle the live greeter preview")
                onToggled: {
                    root.showPreview = checked;
                    if (checked) root.applyToPreview();
                }
            }
            Button {
                text: qsTr("Revert")
                Accessible.name: qsTr("Discard the draft and reload from the daemon")
                onClicked: root.revert()
            }
            Button {
                text: qsTr("Apply")
                enabled: typeof root.backend.status === "string" && root.backend.status !== "busy"
                highlighted: true
                Accessible.name: qsTr("Apply theme and language through the daemon")
                onClicked: root.apply()
            }
        }

        TabBar {
            id: tabBar
            Layout.fillWidth: true
            TabButton { objectName: "structuredTab"; text: qsTr("Structured") }
            TabButton { objectName: "rawTab"; text: qsTr("Raw JSON") }
            onCurrentIndexChanged: {
                if (currentIndex === 1) {
                    root.rawText = root.serializeTheme();
                } else if (root.structuralProblem(root.rawText) === "") {
                    // Carry accepted raw edits back into the structured draft.
                    root.tokens = root.tokensFromJson(root.rawText);
                }
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            // ---- structured page ----
            ScrollView {
                contentWidth: availableWidth
                ColumnLayout {
                    width: root.width > 900 ? root.width - 60 : 840
                    spacing: 10

                    GroupBox {
                        Layout.fillWidth: true
                        title: qsTr("Preset and language")
                        GridLayout {
                            anchors.fill: parent
                            columns: 2
                            columnSpacing: 16
                            rowSpacing: 8
                            Label { text: qsTr("Preset") }
                            ComboBox {
                                id: presetBox
                                Layout.minimumWidth: 240
                                model: root._presets
                                displayText: qsTr("Custom (keep current tokens)")
                                onActivated: function (index) {
                                    root.applyPreset(root._presets[index]);
                                    displayText = qsTr("Preset: %1").arg(root._presets[index]);
                                }
                                Accessible.name: qsTr("Built-in theme preset")
                            }
                            Label { text: qsTr("Language") }
                            ComboBox {
                                id: languageBox
                                Layout.minimumWidth: 240
                                model: root.languages
                                textRole: "name"
                                onActivated: function (index) {
                                    root.languageCode = root.languages[index].code;
                                }
                                currentIndex: {
                                    for (let i = 0; i < root.languages.length; ++i)
                                        if (root.languages[i].code === root.languageCode) return i;
                                    return 0;
                                }
                                Accessible.name: qsTr("Login screen language")
                            }
                            Label {
                                Layout.columnSpan: 2
                                color: palette.placeholderText
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                                text: qsTr("Presets expand into concrete tokens; the daemon stores no preset key. Language changes apply after the greeter restarts.")
                            }
                        }
                    }

                    GroupBox {
                        Layout.fillWidth: true
                        title: qsTr("Appearance")
                        GridLayout {
                            anchors.fill: parent
                            columns: 2
                            columnSpacing: 16
                            rowSpacing: 8

                            Label { text: qsTr("Accent color") }
                            RowLayout {
                                spacing: 8
                                TextField {
                                    id: accentField
                                    Layout.preferredWidth: 120
                                    text: typeof root.tokens.accent === "string" ? root.tokens.accent : ""
                                    onTextEdited: {
                                        if (new RegExp(root._colorPattern).test(accentField.text))
                                            root.setToken("accent", accentField.text);
                                    }
                                    Accessible.name: qsTr("Accent color as #rrggbb")
                                }
                                Rectangle {
                                    Layout.preferredWidth: 24
                                    Layout.preferredHeight: 24
                                    radius: 6
                                    color: accentField.text.match(new RegExp(root._colorPattern)) ? accentField.text : "transparent"
                                    border.color: palette.mid
                                }
                                Label {
                                    visible: accentField.text !== "" && !accentField.text.match(new RegExp(root._colorPattern))
                                    color: "firebrick"
                                    text: qsTr("Use #rgb, #rgba, #rrggbb or #aarrggbb")
                                }
                            }

                            Label { text: qsTr("Background") }
                            ComboBox {
                                id: backgroundModeBox
                                Layout.minimumWidth: 160
                                model: root._backgroundModes
                                currentIndex: {
                                    const mode = root.tokens.backgroundMode;
                                    return typeof mode === "string" && root._backgroundModes.includes(mode)
                                        ? root._backgroundModes.indexOf(mode) : 0;
                                }
                                onActivated: function (index) {
                                    root.setToken("backgroundMode", root._backgroundModes[index]);
                                }
                                Accessible.name: qsTr("Background mode")
                            }

                            Label { text: qsTr("Background image") }
                            TextField {
                                id: backgroundField
                                Layout.fillWidth: true
                                enabled: root.tokens.backgroundMode === "image"
                                placeholderText: qsTr("Absolute path to a local image file")
                                text: typeof root.tokens.backgroundImage === "string"
                                    ? (root.tokens.backgroundImage.startsWith("file://")
                                        ? root.tokens.backgroundImage.substring("file://".length) : root.tokens.backgroundImage)
                                    : ""
                                onTextEdited: {
                                    const value = backgroundField.text;
                                    if (value.startsWith("/")) root.setToken("backgroundImage", value);
                                }
                                Accessible.name: qsTr("Background image path")
                            }

                            Label { text: qsTr("Font family") }
                            TextField {
                                id: fontFamilyField
                                Layout.preferredWidth: 220
                                text: typeof root.tokens.fontFamily === "string" ? root.tokens.fontFamily : ""
                                onTextEdited: root.setToken("fontFamily", fontFamilyField.text)
                                Accessible.name: qsTr("Font family")
                            }

                            Label { text: qsTr("Font scale") }
                            RowLayout {
                                spacing: 8
                                Slider {
                                    id: fontScaleSlider
                                    Layout.preferredWidth: 200
                                    from: 0.5; to: 2.0; stepSize: 0.05
                                    value: typeof root.tokens.fontScale === "number" ? root.tokens.fontScale : 1
                                    onMoved: root.setToken("fontScale", value)
                                    Accessible.name: qsTr("Font scale from 0.5 to 2.0")
                                }
                                Label { text: fontScaleSlider.value.toFixed(2) }
                            }
                        }
                    }

                    GroupBox {
                        Layout.fillWidth: true
                        title: qsTr("Layout")
                        GridLayout {
                            anchors.fill: parent
                            columns: 2
                            columnSpacing: 16
                            rowSpacing: 8

                            Label { text: qsTr("Clock top ratio") }
                            RowLayout {
                                spacing: 8
                                Slider {
                                    id: clockRatioSlider
                                    Layout.preferredWidth: 200
                                    from: 0; to: 0.9; stepSize: 0.005
                                    value: typeof root.tokens.clockTopRatio === "number" ? root.tokens.clockTopRatio : 0.115
                                    onMoved: root.setToken("clockTopRatio", value)
                                    Accessible.name: qsTr("Clock position as a fraction of window height")
                                }
                                Label { text: clockRatioSlider.value.toFixed(3) }
                            }

                            Label { text: qsTr("Panel top ratio") }
                            RowLayout {
                                spacing: 8
                                Slider {
                                    id: panelRatioSlider
                                    Layout.preferredWidth: 200
                                    from: 0; to: 0.9; stepSize: 0.005
                                    value: typeof root.tokens.panelTopRatio === "number" ? root.tokens.panelTopRatio : 0.37
                                    onMoved: root.setToken("panelTopRatio", value)
                                    Accessible.name: qsTr("Panel position as a fraction of window height")
                                }
                                Label { text: panelRatioSlider.value.toFixed(3) }
                            }

                            Label { text: qsTr("Panel maximum width") }
                            SpinBox {
                                from: 1; to: 2000; editable: true
                                value: typeof root.tokens.panelMaxWidth === "number" ? root.tokens.panelMaxWidth : 580
                                onValueModified: root.setToken("panelMaxWidth", value)
                                Accessible.name: qsTr("Panel maximum width in pixels")
                            }

                            Label { text: qsTr("User tile width") }
                            SpinBox {
                                from: 1; to: 2000; editable: true
                                value: typeof root.tokens.tileWidth === "number" ? root.tokens.tileWidth : 84
                                onValueModified: root.setToken("tileWidth", value)
                                Accessible.name: qsTr("User tile width in pixels")
                            }

                            Label { text: qsTr("User tile height") }
                            SpinBox {
                                from: 1; to: 2000; editable: true
                                value: typeof root.tokens.tileHeight === "number" ? root.tokens.tileHeight : 100
                                onValueModified: root.setToken("tileHeight", value)
                                Accessible.name: qsTr("User tile height in pixels")
                            }

                            Label { text: qsTr("Avatar size") }
                            SpinBox {
                                from: 1; to: 2000; editable: true
                                value: typeof root.tokens.avatarSize === "number" ? root.tokens.avatarSize : 56
                                onValueModified: root.setToken("avatarSize", value)
                                Accessible.name: qsTr("Avatar size in pixels")
                            }

                            Label {
                                Layout.columnSpan: 2
                                color: palette.placeholderText
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                                text: qsTr("Every value is clamped the same way as in the greeter: ratios 0–0.9, sizes 1–2000 px, font scale 0.5–2.0. All other tokens keep their loaded values and round-trip untouched.")
                            }
                        }
                    }
                }
            }

            // ---- raw JSON page ----
            ColumnLayout {
                spacing: 6
                Label {
                    visible: root.structuralProblem(root.rawText) !== ""
                    color: "firebrick"
                    text: root.structuralProblem(root.rawText)
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                }
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    TextArea {
                        id: rawArea
                        font.family: "monospace"
                        wrapMode: TextArea.Wrap
                        text: root.rawText
                        onTextChanged: if (!root.loading && text !== root.rawText) root.rawText = text
                        Accessible.name: qsTr("Raw theme JSON")
                    }
                }
                Label {
                    color: palette.placeholderText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    text: qsTr("The daemon accepts the colors, background, font and layout sections (up to 1,000,000 bytes); the greeter ignores unknown keys and clamps values.")
                }
            }
        }
    }
}
