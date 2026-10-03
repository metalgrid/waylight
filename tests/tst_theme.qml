import QtQuick
import QtTest
import ".."

TestCase {
    id: test
    name: "ThemeTokens"

    Component { id: themeComponent; Theme {} }

    // WCAG relative luminance/contrast over QML colors (opaque inputs only).
    function luminance(color) {
        const channel = function(value) {
            return value <= 0.03928 ? value / 12.92 : Math.pow((value + 0.055) / 1.055, 2.4);
        };
        return 0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b);
    }
    function contrastRatio(a, b) {
        const left = luminance(a); const right = luminance(b);
        return (Math.max(left, right) + 0.05) / (Math.min(left, right) + 0.05);
    }
    function fresh() {
        const theme = createTemporaryObject(themeComponent, test);
        verify(theme);
        return theme;
    }

    function test_defaults_match_previous_literals() {
        const theme = fresh();
        compare(String(theme.windowColor), "#182b56");
        compare(String(theme.overlayTop), "#18070b20");
        compare(String(theme.overlayBottom), "#50070b20");
        compare(String(theme.textBar), "#dce0ed");
        compare(String(theme.textPrimary), "#ffffff");
        compare(String(theme.textBright), "#f4f1fa");
        compare(String(theme.textDate), "#eef0f8");
        compare(String(theme.textSecondary), "#e2e4ef");
        compare(String(theme.textPlaceholder), "#d8d9e3");
        compare(String(theme.textCombo), "#ecedf5");
        compare(String(theme.accent), "#d9e2ff");
        compare(String(theme.error), "#ed29324c");
        compare(String(theme.fillHover), "#35ffffff");
        compare(String(theme.fillRest), "#18ffffff");
        compare(String(theme.fillDown), "#80ffffff");
        compare(String(theme.fillAnswer), "#60ffffff");
        compare(String(theme.fillSubtle), "#30ffffff");
        compare(String(theme.borderSelected), "#a0ffffff");
        compare(String(theme.borderField), "#60ffffff");
        compare(String(theme.borderSubtle), "#40ffffff");
        compare(String(theme.borderButton), "#70ffffff");
        compare(String(theme.borderAvatar), "#80ffffff");
        compare(String(theme.avatarFallback), "#648695");
        compare(String(theme.avatarGlyph), "#e4e9e7");
        compare(String(theme.popupBackground), "#ed29324c");
        compare(String(theme.popupBorder), "#50ffffff");
        compare(theme.backgroundMode, "builtin");
        compare(theme.backgroundImage, "");
        compare(String(theme.backgroundColor), "#182b56");
        compare(String(theme.backgroundTop), "#182b56");
        compare(String(theme.backgroundBottom), "#182b56");
        compare(theme.fontFamily, "sans-serif");
        compare(theme.fontScale, 1.0);
        compare(theme.clockTopRatio, 0.115);
        compare(theme.clockDateSize, 21);
        compare(theme.clockTimeCap, 100);
        compare(theme.panelTopRatio, 0.37);
        compare(theme.panelTopRatioShort, 0.33);
        compare(theme.panelShortHeight, 650);
        compare(theme.panelMaxWidth, 580);
        compare(theme.panelBottomMargin, 100);
        compare(theme.tileWidth, 84);
        compare(theme.tileHeight, 100);
        compare(theme.avatarSize, 56);
        compare(theme.tileRadius, 12);
        compare(theme.fieldWidth, 260);
        compare(theme.fieldHeight, 38);
        compare(theme.fieldRadius, 19);
        compare(theme.controlRadius, 20);
        compare(theme.popupRadius, 14);
        compare(theme.optionRadius, 8);
        compare(theme.powerSize, 44);
        compare(theme.powerRadius, 22);
        compare(theme.barMarginLeft, 24);
        compare(theme.barMarginRight, 16);
        compare(theme.bottomBarMargin, 28);
        // The defaults map and the properties must never diverge.
        for (const key in theme.defaultsMap)
            compare(String(theme[key]), String(theme.defaultsMap[key]), "token " + key);
    }

    function test_valid_json_overrides_tokens() {
        const theme = fresh();
        verify(theme.applyJson(JSON.stringify({
            preset: "dusk",
            colors: { accent: "#00ff00" },
            background: { mode: "solid", color: "#102030" },
            font: { family: "serif", scale: 1.25 },
            layout: { tileWidth: 92, panelMaxWidth: 480, clockTopRatio: 0.2 }
        })));
        compare(String(theme.accent), "#00ff00");
        compare(theme.backgroundMode, "solid");
        compare(String(theme.backgroundColor), "#102030");
        compare(theme.fontFamily, "serif");
        compare(theme.fontScale, 1.25);
        compare(theme.tileWidth, 92);
        compare(theme.panelMaxWidth, 480);
        compare(theme.clockTopRatio, 0.2);
        // Untouched tokens keep their defaults.
        compare(theme.tileHeight, 100);
        compare(String(theme.windowColor), "#182b56");
    }

    function test_color_format_validation() {
        const theme = fresh();
        const duskAccent = String(theme.accent);
        verify(theme.applyJson('{"colors":{"accent":"#abc"}}'));
        compare(String(theme.accent), "#aabbcc");
        verify(theme.applyJson('{"colors":{"accent":"#abcd"}}'));
        compare(String(theme.accent), "#ddaabbcc");
        verify(theme.applyJson('{"colors":{"accent":"#aabbccdd"}}'));
        compare(String(theme.accent), "#aabbccdd");
        for (const invalid of ["red", "#12345", "#1234567", "#123456789", " #ffffff", "", 5, null, true]) {
            verify(theme.applyJson(JSON.stringify({colors: {accent: invalid}})));
            compare(String(theme.accent), duskAccent, "invalid " + JSON.stringify(invalid));
        }
    }

    function test_malformed_and_wrong_types_fail_open() {
        const theme = fresh();
        ignoreWarning(/waylight theme/);
        verify(!theme.applyJson("{not json"));
        compare(theme.tileWidth, 84);
        ignoreWarning(/waylight theme/);
        verify(!theme.applyJson("[1, 2]"));
        compare(theme.tileWidth, 84);
        ignoreWarning(/waylight theme/);
        verify(!theme.applyJson("null"));
        compare(theme.tileWidth, 84);
        // Wrong types fall back to that key's default; other keys still apply.
        verify(theme.applyJson('{"layout":{"tileWidth":"92"},"font":{"scale":"big"},'
            + '"colors":{"accent":5},"background":{"mode":9}}'));
        compare(theme.tileWidth, 84);
        compare(theme.fontScale, 1.0);
        compare(String(theme.accent), "#d9e2ff");
        compare(theme.backgroundMode, "builtin");
        verify(theme.applyJson('{"layout":{"tileWidth":90,"nope":"x"},"future":true}'));
        compare(theme.tileWidth, 90);
    }

    function test_unknown_preset_keeps_defaults() {
        const theme = fresh();
        ignoreWarning(/waylight theme/);
        verify(!theme.resetTo("bogus"));
        compare(String(theme.accent), "#d9e2ff");
        compare(theme.backgroundMode, "builtin");
        compare(theme.clockTimeCap, 100);
        // Unknown preset applies the dusk base, then explicit keys still win.
        ignoreWarning(/waylight theme/);
        verify(theme.applyJson('{"preset":"bogus","colors":{"accent":"#00ff00"}}'));
        compare(String(theme.accent), "#00ff00");
        compare(String(theme.windowColor), "#182b56");
        // A non-string preset is a wrong type, not an unknown name: silent default.
        verify(theme.applyJson('{"preset":7}'));
        compare(String(theme.accent), "#d9e2ff");
    }

    function test_clamping() {
        const theme = fresh();
        verify(theme.applyJson('{"layout":{"clockTopRatio":-1,"panelTopRatio":5,'
            + '"panelTopRatioShort":0.9,"tileWidth":0,"panelMaxWidth":9999,'
            + '"avatarSize":0.5,"bottomBarMargin":-3}}'));
        compare(theme.clockTopRatio, 0);
        compare(theme.panelTopRatio, 0.9);
        compare(theme.panelTopRatioShort, 0.9);
        compare(theme.tileWidth, 1);
        compare(theme.panelMaxWidth, 2000);
        compare(theme.avatarSize, 1);
        compare(theme.bottomBarMargin, 1);
        verify(theme.applyJson('{"font":{"scale":9},"layout":{"clockDateSize":2001}}'));
        compare(theme.fontScale, 2);
        compare(theme.clockDateSize, 2000);
        verify(theme.applyJson('{"font":{"scale":0.1}}'));
        compare(theme.fontScale, 0.5);
    }

    function test_preset_switch_changes_accent() {
        const theme = fresh();
        const duskAccent = String(theme.accent);
        verify(theme.resetTo("midnight"));
        const midnightAccent = String(theme.accent);
        verify(midnightAccent !== duskAccent);
        compare(String(theme.textPrimary), "#ffffff");
        compare(theme.backgroundMode, "gradient");
        verify(theme.resetTo("daylight"));
        const daylightAccent = String(theme.accent);
        verify(daylightAccent !== midnightAccent);
        verify(daylightAccent !== duskAccent);
        compare(String(theme.textPrimary), "#151e36");
        // Daylight body text keeps >= 4.5:1 contrast on its own background.
        verify(contrastRatio(theme.textPrimary, theme.backgroundBottom) >= 4.5);
        verify(contrastRatio(theme.textPrimary, theme.backgroundTop) >= 4.5);
        verify(contrastRatio(theme.textSecondary, theme.backgroundBottom) >= 4.5);
        verify(contrastRatio(theme.textBar, theme.backgroundBottom) >= 4.5);
        // Session picker text on both the light window and its light popup.
        verify(contrastRatio(theme.textCombo, theme.backgroundBottom) >= 4.5);
        verify(contrastRatio(theme.textCombo, theme.popupBackground) >= 4.5);
        verify(theme.resetTo("dusk"));
        compare(String(theme.accent), duskAccent);
        compare(String(theme.textPrimary), "#ffffff");
        compare(theme.backgroundMode, "builtin");
        verify(theme.resetTo("midnight"));
        verify(contrastRatio(theme.textPrimary, theme.backgroundBottom) >= 4.5);
    }

    function test_background_modes_and_fallback() {
        const theme = fresh();
        verify(theme.applyJson('{"background":{"mode":"image","image":"/tmp/picture.jpg"}}'));
        compare(theme.backgroundMode, "image");
        compare(theme.backgroundImage, "file:///tmp/picture.jpg");
        verify(theme.applyJson('{"background":{"mode":"wallop"}}'));
        compare(theme.backgroundMode, "builtin");
        verify(theme.applyJson('{"background":{"mode":"gradient","top":"#111111","bottom":"#eeeeee"}}'));
        compare(theme.backgroundMode, "gradient");
        compare(String(theme.backgroundTop), "#111111");
        compare(String(theme.backgroundBottom), "#eeeeee");
        verify(theme.applyJson('{"background":{"top":5}}'));
        compare(String(theme.backgroundTop), "#182b56");
        verify(theme.applyJson('{"background":{"mode":"solid"}}'));
        compare(theme.backgroundMode, "solid");
    }

    function test_load_file_fail_open_and_valid() {
        const theme = fresh();
        verify(!theme.loadFile(""));
        verify(!theme.loadFile("/no/such/waylight-theme-test.json"));
        compare(theme.tileWidth, 84);
        ignoreWarning(/waylight theme/);
        verify(theme.loadFile(Qt.resolvedUrl("fixtures/theme-malformed.json").toString()));
        compare(theme.tileWidth, 84);
        compare(String(theme.accent), "#d9e2ff");
        verify(theme.loadFile(Qt.resolvedUrl("fixtures/theme-valid.json").toString()));
        compare(theme.tileWidth, 92);
        compare(String(theme.accent), "#7fd1b9");
        // The first readable candidate wins even if a later one is better.
        const valid = Qt.resolvedUrl("fixtures/theme-valid.json").toString();
        const malformed = Qt.resolvedUrl("fixtures/theme-malformed.json").toString();
        // Missing candidates warn nothing; the readable file applies.
        verify(theme.loadFirstExisting(JSON.stringify(["/no/such/waylight-theme-test.json", valid])));
        compare(theme.tileWidth, 92);
        ignoreWarning(/waylight theme/);
        verify(theme.loadFirstExisting(JSON.stringify([malformed, valid])));
        compare(theme.tileWidth, 84);
        verify(!theme.loadFirstExisting("not json"));
        verify(!theme.loadFirstExisting(JSON.stringify(["/no/such/waylight-theme-test.json"])));
        verify(!theme.loadFirstExisting("[]"));
        // Parsed but non-array inputs return false without warning.
        verify(!theme.loadFirstExisting("null"));
        verify(!theme.loadFirstExisting("5"));
        verify(!theme.loadFirstExisting('"theme.json"'));
        verify(!theme.loadFirstExisting("{}"));
    }

    function test_background_image_is_local_absolute_only() {
        const theme = fresh();
        // Absolute local paths are the only accepted form; they are normalized
        // like loadFile normalizes its argument.
        verify(theme.applyJson('{"background":{"image":"/usr/share/bg.png"}}'));
        compare(theme.backgroundImage, "file:///usr/share/bg.png");
        // Remote URLs and relative paths keep the default "".
        for (const rejected of ["https://example.com/bg.jpg", "http://example.com/bg.jpg",
                                "file:///already-a-url.png", "pictures/bg.jpg", "./bg.jpg", 5, null]) {
            verify(theme.applyJson(JSON.stringify({background: {image: rejected}})));
            compare(theme.backgroundImage, "", "rejected " + JSON.stringify(rejected));
        }
        // fontFamily shares the string kind and must stay unrestricted.
        verify(theme.applyJson('{"font":{"family":"serif"}}'));
        compare(theme.fontFamily, "serif");
    }

    function test_zero_byte_file_wins_with_defaults() {
        const theme = fresh();
        // A readable zero-byte file exists, so it wins; its content falls back
        // to defaults with the usual warning.
        ignoreWarning(/waylight theme/);
        verify(theme.loadFile(Qt.resolvedUrl("fixtures/theme-empty.json").toString()));
        compare(theme.tileWidth, 84);
        compare(String(theme.accent), "#d9e2ff");
        const empty = Qt.resolvedUrl("fixtures/theme-empty.json").toString();
        const valid = Qt.resolvedUrl("fixtures/theme-valid.json").toString();
        ignoreWarning(/waylight theme/);
        verify(theme.loadFirstExisting(JSON.stringify([empty, valid])));
        compare(theme.tileWidth, 84);
        compare(String(theme.accent), "#d9e2ff");
    }

    function test_oversized_file_is_unreadable() {
        const theme = fresh();
        // Over 1 MiB is treated as unreadable even though the JSON head is
        // valid: the cap bounds the synchronous read.
        ignoreWarning(/waylight theme/);
        verify(!theme.loadFile(Qt.resolvedUrl("fixtures/theme-oversized.json").toString()));
        compare(theme.tileWidth, 84);
        // It also does not win the search order; a later valid file applies.
        const oversized = Qt.resolvedUrl("fixtures/theme-oversized.json").toString();
        const valid = Qt.resolvedUrl("fixtures/theme-valid.json").toString();
        verify(theme.loadFirstExisting(JSON.stringify([oversized, valid])));
        compare(theme.tileWidth, 92);
    }

    function test_preset_then_explicit_keys_order() {
        const first = fresh();
        verify(first.resetTo("daylight"));
        const daylightAccent = String(first.accent);
        // The preset applies first; explicit keys in the file override it.
        const theme = fresh();
        verify(theme.applyJson('{"preset":"daylight","colors":{"accent":"#123456"}}'));
        compare(String(theme.accent), "#123456");
        compare(String(theme.textPrimary), "#151e36");
        compare(theme.backgroundMode, "gradient");
        verify(daylightAccent !== "#123456");
    }

    function test_scaled_font_sizes() {
        const theme = fresh();
        compare(theme.scaled(21), 21);
        compare(theme.scaled(theme.clockTimeCap), 100);
        verify(theme.applyJson('{"font":{"scale":2}}'));
        compare(theme.scaled(21), 42);
        verify(theme.applyJson('{"font":{"scale":0.5}}'));
        compare(theme.scaled(21), 11);
    }
}
