pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import ".."

// Tests for the config GUI editor. The Waylight module's C++ types (real
// ConfigBackend) only exist inside the binaries, so ConfigMain is instantiated
// with a mock exposing the exact same property/function surface.
TestCase {
    id: test
    name: "ConfigEditor"
    when: windowShown

    readonly property string daemonTheme: JSON.stringify({
        colors: {accent: "#102030"},
        background: {mode: "solid"},
        font: {scale: 1.25},
        layout: {panelMaxWidth: 600}
    })

    Component {
        id: fakeBackend
        QtObject {
            // ConfigBackend surface
            property string theme: ""
            property string config: ""
            property string themePath: "/etc/waylight/theme.json"
            property string configPath: "/etc/waylight/waylight.json"
            property string status: "ready"
            property string message: ""
            signal applied()
            // What the daemon would hand out and what it received.
            property string storedTheme: test.daemonTheme
            property string storedConfig: JSON.stringify({language: "es"})
            property string appliedTheme: ""
            property var appliedLanguages: []
            // Mirrors a daemon-side refusal: the request flips status to busy
            // and the Failed reply is delivered only by deliverFailure(), so
            // the test can observe the intermediate "Applying…" state.
            property bool failNext: false
            property string pendingFailure: ""
            function deliverFailure() {
                status = "error";
                message = pendingFailure;
                pendingFailure = "";
            }
            function reload() {
                // Mirror the daemon's GetAll: both properties change together.
                // Assign through empty first so the changed signals always fire
                // even when the stored values are unchanged.
                theme = "";
                config = "";
                theme = storedTheme;
                config = storedConfig;
            }
            function applyTheme(json) {
                status = "busy";
                if (failNext) {
                    pendingFailure = "SetTheme failed: dev.waylight.Config1.Error.InvalidTheme: theme has unknown section 'preset'";
                    return;
                }
                appliedTheme = json;
                status = "ready";
                message = "Saved. Restart the greeter (or reboot to the login screen) to apply.";
                applied();
            }
            function applyLanguage(code) {
                status = "busy";
                if (failNext) {
                    pendingFailure = "SetLanguage failed: dev.waylight.Config1.Error.InvalidLanguage: unsupported language 'de'";
                    return;
                }
                appliedLanguages = appliedLanguages.concat([code]);
                status = "ready";
                message = "Saved. Restart the greeter (or reboot to the login screen) to apply.";
                applied();
            }
        }
    }

    Component {
        id: editor
        ConfigMain {}
    }

    function setup() {
        const backend = createTemporaryObject(fakeBackend, test);
        const window = createTemporaryObject(editor, null, {backend: backend});
        verify(window);
        window.requestActivate();
        tryCompare(window, "active", true);
        // The Component.onCompleted reload() must have pulled daemon state.
        compare(window.tokens.accent, "#102030");
        return {window: window, backend: backend};
    }

    function test_loads_daemon_state_into_editors() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        compare(window.languageCode, "es");
        compare(window.loadedLanguage, "es");
        compare(window.tokens.backgroundMode, "solid");
        compare(window.tokens.fontScale, 1.25);
        compare(window.tokens.panelMaxWidth, 600);
        compare(backend.themePath, "/etc/waylight/theme.json");
        // The raw editor starts from the same draft.
        verify(window.rawText.includes("panelMaxWidth"));
    }

    function test_serialization_emits_only_known_sections() {
        const ctx = setup(); const window = ctx.window;
        const data = JSON.parse(window.serializeTheme());
        const keys = Object.keys(data).sort();
        compare(keys, ["background", "colors", "font", "layout"]);
        compare(data.colors.accent, "#102030");
        compare(data.background.mode, "solid");
        // Every serialized value round-trips through the greeter's Theme.
        verify(window.structuralProblem(window.serializeTheme()) === "");
    }

    function test_preset_expands_into_tokens_without_preset_key() {
        const ctx = setup(); const window = ctx.window;
        window.applyPreset("midnight");
        const data = JSON.parse(window.serializeTheme());
        verify(!("preset" in data));
        compare(data.colors.windowColor, "#060a14");
        // A unknown preset name keeps the dusk defaults (fail-open like Theme).
        window.applyPreset("solarized");
        compare(JSON.parse(window.serializeTheme()).colors.windowColor, "#182b56");
    }

    function test_values_clamp_like_the_greeter() {
        const ctx = setup(); const window = ctx.window;
        window.tokens = window.tokensFromJson(JSON.stringify({
            layout: {tileWidth: 99999, clockTopRatio: 12},
            font: {scale: 9},
            colors: {accent: "not-a-color"}
        }));
        compare(window.tokens.tileWidth, 2000);
        compare(window.tokens.clockTopRatio, 0.9);
        compare(window.tokens.fontScale, 2);
        compare(window.tokens.accent, "#d9e2ff"); // dusk default
    }

    function test_structural_problems_mirror_the_daemon() {
        const ctx = setup(); const window = ctx.window;
        verify(window.structuralProblem("not json") !== "");
        verify(window.structuralProblem("[]") !== "");
        verify(window.structuralProblem(JSON.stringify({preset: "dusk"})).includes("Unknown section"));
        verify(window.structuralProblem(JSON.stringify({colors: 5})).includes("must be an object"));
        verify(window.structuralProblem(window.serializeTheme()) === "");
    }

    function test_apply_sends_theme_and_changed_language() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        window.languageCode = "fr";
        window.apply();
        compare(backend.appliedTheme, window.serializeTheme());
        compare(backend.appliedLanguages, ["fr"]);
        tryCompare(backend, "theme", backend.storedTheme); // applied() → reload()
        compare(window.languageCode, "es"); // synced back from the daemon
    }

    function test_apply_uses_raw_tab_content_and_warns_on_garbage() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        // Drive the real tab bar to the raw JSON page.
        const rawTab = findChild(window, "rawTab");
        verify(rawTab);
        rawTab.checked = true;
        tryCompare(window, "rawText", window.serializeTheme());
        // Edit the raw draft there and apply it.
        window.rawText = JSON.stringify({colors: {accent: "#abcdef"}});
        window.apply();
        compare(JSON.parse(backend.appliedTheme).colors.accent, "#abcdef");
        compare(backend.appliedLanguages, []);
        // Invalid raw JSON is refused locally: nothing reaches the daemon.
        backend.appliedTheme = "";
        window.rawText = "{broken";
        window.apply();
        compare(backend.appliedTheme, "");
    }

    function test_revert_reloads_from_daemon() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        window.applyPreset("daylight");
        verify(window.tokens.accent !== "#102030");
        backend.storedTheme = JSON.stringify({colors: {accent: "#456789"}});
        window.revert();
        tryCompare(window.tokens, "accent", "#456789");
    }

    function test_preview_applies_the_draft_in_memory() {
        const ctx = setup(); const window = ctx.window;
        const themeObject = window.previewThemeObject();
        verify(themeObject !== null);
        compare(themeObject.objectName, "theme");
        // The preview greeter runs against the mock, never the real Backend.
        verify(window.showPreview === false);
        window.showPreview = true;
        window.setToken("accent", "#112233");
        // The preview debounce is 150 ms.
        wait(300);
        compare(String(themeObject.accent), "#112233");
        // The daemon file on disk is never touched by previewing.
        compare(ctx.backend.appliedTheme, "");
    }

    function test_daemon_failure_surfaces_message_and_clears_applying() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const label = findChild(window, "statusLabel");
        verify(label);
        // Language stays as loaded, so Apply sends exactly one daemon call.
        window.languageCode = window.loadedLanguage;
        backend.failNext = true;
        window.apply();
        // While the daemon works, the placeholder is all the user sees.
        compare(backend.status, "busy");
        compare(window.localStatus, "Applying…");
        compare(label.text, "Applying…");
        // The daemon refuses (e.g. polkit denial or invalid theme): the Failed
        // reply must clear the placeholder and surface the daemon message.
        backend.deliverFailure();
        tryCompare(window, "localStatus", "");
        compare(label.text, backend.message);
        verify(label.text.includes("InvalidTheme"), label.text);
        compare(label.color, "#b22222"); // firebrick, resolved
        // A later successful apply recovers to a normal status line.
        backend.failNext = false;
        window.apply();
        tryCompare(label, "text", backend.message);
        verify(label.text.includes("Saved"), label.text);
        compare(window.localStatus, "");
    }

    function test_language_picker_lists_native_names() {
        const ctx = setup(); const window = ctx.window;
        compare(window.languages.length, 11);
        const codes = window.languages.map(function (entry) { return entry.code; });
        for (const code of ["en", "zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg"])
            verify(codes.includes(code));
        compare(window.languages[4].name, "Français");
        compare(window.languages[9].name, "اردو");
    }
}
