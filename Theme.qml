import QtQuick

// Presentation tokens for Main.qml. Defaults reproduce the previous Main.qml
// literals exactly, so the stock look is pixel-identical. Every load path is
// fail-open: a missing or broken theme never prevents startup, unknown keys
// are ignored, invalid values fall back to their default, and numbers are
// clamped (ratios 0-0.9, sizes 1-2000 px, font.scale 0.5-2.0).
QtObject {
    id: theme

    // Single source of truth for the "dusk" defaults (see .pi/THEME-PLAN.md).
    readonly property var defaultsMap: ({
        windowColor: "#182b56",
        overlayTop: "#18070b20",
        overlayBottom: "#50070b20",
        textBar: "#dce0ed",
        textPrimary: "#ffffff",
        textBright: "#f4f1fa",
        textDate: "#eef0f8",
        textSecondary: "#e2e4ef",
        textPlaceholder: "#d8d9e3",
        textCombo: "#ecedf5",
        accent: "#d9e2ff",
        error: "#ed29324c",
        fillHover: "#35ffffff",
        fillRest: "#18ffffff",
        fillDown: "#80ffffff",
        fillAnswer: "#60ffffff",
        fillSubtle: "#30ffffff",
        borderSelected: "#a0ffffff",
        borderField: "#60ffffff",
        borderSubtle: "#40ffffff",
        borderButton: "#70ffffff",
        borderAvatar: "#80ffffff",
        avatarFallback: "#648695",
        avatarGlyph: "#e4e9e7",
        popupBackground: "#ed29324c",
        popupBorder: "#50ffffff",
        backgroundMode: "builtin",
        backgroundImage: "",
        backgroundColor: "#182b56",
        backgroundTop: "#182b56",
        backgroundBottom: "#182b56",
        fontFamily: "sans-serif",
        fontScale: 1.0,
        clockTopRatio: 0.115,
        clockDateSize: 21,
        clockTimeCap: 100,
        panelTopRatio: 0.37,
        panelTopRatioShort: 0.33,
        panelShortHeight: 650,
        panelMaxWidth: 580,
        panelBottomMargin: 100,
        tileWidth: 84,
        tileHeight: 100,
        avatarSize: 56,
        tileRadius: 12,
        fieldWidth: 260,
        fieldHeight: 38,
        fieldRadius: 19,
        controlRadius: 20,
        popupRadius: 14,
        optionRadius: 8,
        powerSize: 44,
        powerRadius: 22,
        barMarginLeft: 24,
        barMarginRight: 16,
        bottomBarMargin: 28
    })

    // colors
    property color windowColor: defaultsMap.windowColor
    property color overlayTop: defaultsMap.overlayTop
    property color overlayBottom: defaultsMap.overlayBottom
    property color textBar: defaultsMap.textBar
    property color textPrimary: defaultsMap.textPrimary
    property color textBright: defaultsMap.textBright
    property color textDate: defaultsMap.textDate
    property color textSecondary: defaultsMap.textSecondary
    property color textPlaceholder: defaultsMap.textPlaceholder
    // Session picker text (closed control and its popup). Not in the locked
    // token table, but required for the daylight preset's contrast goal; the
    // default is the previous literal, so the stock look is unchanged.
    property color textCombo: defaultsMap.textCombo
    property color accent: defaultsMap.accent
    property color error: defaultsMap.error
    property color fillHover: defaultsMap.fillHover
    property color fillRest: defaultsMap.fillRest
    property color fillDown: defaultsMap.fillDown
    property color fillAnswer: defaultsMap.fillAnswer
    property color fillSubtle: defaultsMap.fillSubtle
    property color borderSelected: defaultsMap.borderSelected
    property color borderField: defaultsMap.borderField
    property color borderSubtle: defaultsMap.borderSubtle
    property color borderButton: defaultsMap.borderButton
    property color borderAvatar: defaultsMap.borderAvatar
    property color avatarFallback: defaultsMap.avatarFallback
    property color avatarGlyph: defaultsMap.avatarGlyph
    property color popupBackground: defaultsMap.popupBackground
    property color popupBorder: defaultsMap.popupBorder
    // background: "builtin" (embedded SVG), "image" (local file,
    // PreserveAspectCrop), "solid", or "gradient" (top to bottom)
    property string backgroundMode: defaultsMap.backgroundMode
    property string backgroundImage: defaultsMap.backgroundImage
    property color backgroundColor: defaultsMap.backgroundColor
    property color backgroundTop: defaultsMap.backgroundTop
    property color backgroundBottom: defaultsMap.backgroundBottom
    // font: scale is applied to every font.pixelSize and the clock cap
    property string fontFamily: defaultsMap.fontFamily
    property real fontScale: defaultsMap.fontScale
    // layout
    property real clockTopRatio: defaultsMap.clockTopRatio
    property int clockDateSize: defaultsMap.clockDateSize
    property int clockTimeCap: defaultsMap.clockTimeCap
    property real panelTopRatio: defaultsMap.panelTopRatio
    property real panelTopRatioShort: defaultsMap.panelTopRatioShort
    property int panelShortHeight: defaultsMap.panelShortHeight
    property int panelMaxWidth: defaultsMap.panelMaxWidth
    property int panelBottomMargin: defaultsMap.panelBottomMargin
    property int tileWidth: defaultsMap.tileWidth
    property int tileHeight: defaultsMap.tileHeight
    property int avatarSize: defaultsMap.avatarSize
    property int tileRadius: defaultsMap.tileRadius
    property int fieldWidth: defaultsMap.fieldWidth
    property int fieldHeight: defaultsMap.fieldHeight
    property int fieldRadius: defaultsMap.fieldRadius
    property int controlRadius: defaultsMap.controlRadius
    property int popupRadius: defaultsMap.popupRadius
    property int optionRadius: defaultsMap.optionRadius
    property int powerSize: defaultsMap.powerSize
    property int powerRadius: defaultsMap.powerRadius
    property int barMarginLeft: defaultsMap.barMarginLeft
    property int barMarginRight: defaultsMap.barMarginRight
    property int bottomBarMargin: defaultsMap.bottomBarMargin

    // Built-in presets as overrides on top of the dusk defaults. Values pass
    // through the same validation as theme files. Daylight keeps dark text on
    // its own light gradient and light popup (>= 4.5:1 for body text). The
    // embedded white SVG artwork cannot be recolored by tokens; see README.
    readonly property var _presets: ({
        dusk: {},
        midnight: {
            windowColor: "#060a14",
            overlayTop: "#0e060b20",
            overlayBottom: "#2e060b20",
            textBar: "#b7bfd6",
            textBright: "#f2f5ff",
            textDate: "#d9dfef",
            textSecondary: "#c6cde2",
            textPlaceholder: "#9aa3bd",
            accent: "#a7bcff",
            fillHover: "#26ffffff",
            fillRest: "#10ffffff",
            fillDown: "#66ffffff",
            fillAnswer: "#52ffffff",
            fillSubtle: "#24ffffff",
            borderSelected: "#96ffffff",
            borderField: "#59ffffff",
            borderSubtle: "#38ffffff",
            borderButton: "#66ffffff",
            borderAvatar: "#66ffffff",
            avatarFallback: "#3a4562",
            avatarGlyph: "#cdd5e8",
            popupBackground: "#101a30",
            backgroundMode: "gradient",
            backgroundColor: "#0a1020",
            backgroundTop: "#04060d",
            backgroundBottom: "#0c1322"
        },
        daylight: {
            windowColor: "#dfe6f4",
            overlayTop: "#12ffffff",
            overlayBottom: "#38ffffff",
            textBar: "#2c3550",
            textPrimary: "#151e36",
            textBright: "#0b1226",
            textDate: "#222c48",
            textSecondary: "#39445f",
            textPlaceholder: "#4d5875",
            accent: "#2b57c8",
            error: "#b3261e",
            fillHover: "#30152038",
            fillRest: "#16152038",
            fillDown: "#66152038",
            fillAnswer: "#52152038",
            fillSubtle: "#26152038",
            borderSelected: "#8c2b57c8",
            borderField: "#59152038",
            borderSubtle: "#3a152038",
            borderButton: "#66152038",
            borderAvatar: "#66152038",
            avatarFallback: "#93a5c4",
            avatarGlyph: "#ffffff",
            textCombo: "#39445f",
            popupBackground: "#f7f9ff",
            popupBorder: "#662b57c8",
            backgroundMode: "gradient",
            backgroundColor: "#dfe6f4",
            backgroundTop: "#f3f6fd",
            backgroundBottom: "#d3ddec"
        }
    })

    readonly property var _colorKeys: [
        "windowColor", "overlayTop", "overlayBottom", "textBar", "textPrimary",
        "textBright", "textDate", "textSecondary", "textPlaceholder",
        "textCombo", "accent", "error", "fillHover", "fillRest", "fillDown",
        "fillAnswer", "fillSubtle", "borderSelected", "borderField",
        "borderSubtle", "borderButton", "borderAvatar", "avatarFallback",
        "avatarGlyph", "popupBackground", "popupBorder", "backgroundColor",
        "backgroundTop", "backgroundBottom"
    ]
    readonly property var _ratioKeys: [
        "clockTopRatio", "panelTopRatio", "panelTopRatioShort"
    ]
    readonly property var _sizeKeys: [
        "clockDateSize", "clockTimeCap", "panelShortHeight", "panelMaxWidth",
        "panelBottomMargin", "tileWidth", "tileHeight", "avatarSize",
        "tileRadius", "fieldWidth", "fieldHeight", "fieldRadius",
        "controlRadius", "popupRadius", "optionRadius", "powerSize",
        "powerRadius", "barMarginLeft", "barMarginRight", "bottomBarMargin"
    ]
    readonly property var _layoutKeys: _ratioKeys.concat(_sizeKeys)
    readonly property var _backgroundKeys: {
        "mode": "backgroundMode",
        "image": "backgroundImage",
        "color": "backgroundColor",
        "top": "backgroundTop",
        "bottom": "backgroundBottom"
    }
    readonly property var _fontKeys: {
        "family": "fontFamily",
        "scale": "fontScale"
    }
    readonly property var _backgroundModes: ["builtin", "image", "solid", "gradient"]

    // Named font size: every font.pixelSize and the clock cap use this.
    function scaled(size) {
        return Math.round(size * theme.fontScale);
    }

    // Applies a complete theme definition. Returns true when the text parsed;
    // on parse failure the theme resets to the dusk defaults.
    function applyJson(text) {
        let data;
        try {
            data = JSON.parse(text);
        } catch (error) {
            console.warn("waylight theme: not valid JSON; using defaults");
            theme.resetTo("dusk");
            return false;
        }
        if (data === null || typeof data !== "object" || Array.isArray(data)) {
            console.warn("waylight theme: not a JSON object; using defaults");
            theme.resetTo("dusk");
            return false;
        }
        theme.resetTo(typeof data.preset === "string" ? data.preset : "dusk");
        theme.applyMap(Object.assign(
            theme._known(data.colors, theme._colorKeys),
            theme._known(data.background, theme._backgroundKeys),
            theme._known(data.font, theme._fontKeys),
            theme._known(data.layout, theme._layoutKeys)));
        return true;
    }

    // Reads one theme file. True when the file was read (even if its content
    // fell back to defaults, including a zero-byte file); false when it could
    // not be read at all (missing, or over the 1 MiB cap below).
    function loadFile(path) {
        if (typeof path !== "string" || path === "")
            return false;
        const url = path.startsWith("/") ? "file://" + path : path;
        const request = new XMLHttpRequest();
        request.open("GET", url, false);
        request.send();
        if (request.readyState !== 4
            || !(request.status === 200
                 || (request.status === 0 && request.getAllResponseHeaders() !== "")))
            return false;
        // Bound the synchronous read: an oversized file is treated as
        // unreadable and never applied. Body length is deliberately not part
        // of the acceptance check: a readable 0-byte file must win.
        if (request.responseText.length > 1000000) {
            console.warn("waylight theme: file over 1 MiB; ignored");
            return false;
        }
        theme.applyJson(request.responseText);
        return true;
    }

    // Loads the first readable candidate from a JSON array string of paths.
    // The first existing file wins even if its content is malformed.
    function loadFirstExisting(pathsJson) {
        if (typeof pathsJson !== "string" || pathsJson === "")
            return false;
        let paths;
        try {
            paths = JSON.parse(pathsJson);
        } catch (error) {
            return false;
        }
        if (!Array.isArray(paths))
            return false;
        for (const path of paths) {
            if (theme.loadFile(path))
                return true;
        }
        return false;
    }

    // Applies a built-in preset by name. Unknown names keep the dusk defaults.
    function resetTo(presetName) {
        const known = typeof presetName === "string" && presetName in theme._presets;
        if (!known)
            console.warn("waylight theme: unknown preset " + String(presetName)
                + "; using dusk");
        theme.applyMap(theme.defaultsMap);
        if (known)
            theme.applyMap(theme._presets[presetName]);
        return known;
    }

    // Validates and assigns a map of {propertyName: value}. Unknown property
    // names and invalid values are ignored; numbers are clamped.
    function applyMap(map) {
        if (map === null || typeof map !== "object")
            return;
        for (const propertyName in map)
            theme._assignToken(propertyName, map[propertyName], theme._kindOf(propertyName));
    }

    function _kindOf(propertyName) {
        if (theme._colorKeys.includes(propertyName))
            return "color";
        if (theme._ratioKeys.includes(propertyName))
            return "ratio";
        if (theme._sizeKeys.includes(propertyName))
            return "size";
        if (propertyName === "fontScale")
            return "scale";
        if (propertyName === "backgroundMode")
            return "mode";
        if (propertyName === "backgroundImage" || propertyName === "fontFamily")
            return "string";
        return "unknown";
    }

    function _assignToken(propertyName, value, kind) {
        if (kind === "color") {
            if (theme._isColor(value))
                theme[propertyName] = theme._colorValue(value);
        } else if (kind === "ratio") {
            if (theme._isNumber(value))
                theme[propertyName] = Math.min(0.9, Math.max(0, value));
        } else if (kind === "size") {
            if (theme._isNumber(value))
                theme[propertyName] = Math.min(2000, Math.max(1, value));
        } else if (kind === "scale") {
            if (theme._isNumber(value))
                theme[propertyName] = Math.min(2, Math.max(0.5, value));
        } else if (kind === "mode") {
            if (typeof value === "string" && theme._backgroundModes.includes(value))
                theme[propertyName] = value;
        } else if (kind === "string") {
            if (propertyName === "backgroundImage") {
                // Local-only: absolute paths are stored normalized like
                // loadFile; anything else (remote URLs, relative paths,
                // non-strings, and the "" default itself) resolves to the
                // default "".
                theme[propertyName] = typeof value === "string" && value.startsWith("/")
                    ? "file://" + value : "";
            } else if (typeof value === "string") {
                theme[propertyName] = value;
            }
        }
        // Unknown kinds keep the current token.
    }

    function _isColor(value) {
        return typeof value === "string"
            && /^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i.test(value);
    }

    // QColor has no #rgba form; expand #rgb and #rgba so plan-compliant input
    // is representable (#rgba digits are r, g, b, a).
    function _colorValue(value) {
        const digits = value.substring(1);
        if (digits.length === 3)
            return "#" + digits[0] + digits[0] + digits[1] + digits[1] + digits[2] + digits[2];
        if (digits.length === 4)
            return "#" + digits[3] + digits[3] + digits[0] + digits[0]
                + digits[1] + digits[1] + digits[2] + digits[2];
        return value;
    }

    function _isNumber(value) {
        return typeof value === "number" && Number.isFinite(value);
    }

    // Keeps only the known keys of a theme section, renamed to property names.
    function _known(section, keys) {
        const known = {};
        if (section === null || typeof section !== "object" || Array.isArray(section))
            return known;
        if (Array.isArray(keys)) {
            for (const key of keys) {
                if (key in section)
                    known[key] = section[key];
            }
        } else {
            for (const key in keys) {
                if (key in section)
                    known[keys[key]] = section[key];
            }
        }
        return known;
    }
}
