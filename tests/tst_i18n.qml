import QtQuick
import QtQuick.Controls
import QtTest
import ".."

TestCase {
    id: test
    name: "I18n"
    when: windowShown

    // The UI language resolves in Rust before the engine loads; in this test
    // process no translator exists, so qsTr must pass the English source
    // through byte-for-byte. Translated lookups (real QTranslator retranslation
    // against the committed catalogs) are proven by the src/i18n.rs unit tests.
    Component {
        id: fake
        QtObject {
            property string state: "idle"
            property string message: "Test only"
            property string prompt: ""
            property var sessions: ["Hyprland", "Weston"]
            property int selected: 0
            property int capabilities: 7
            property bool preview: true
            property string accounts: "[]"
            property bool closing: false
            property string themePaths: "[]"
            signal identityChosen(string name)
            function choose_user(name, index) {}
            function choose_manual() {}
            function begin(name, index) {}
            function answer(text) {}
            function cancel() {}
            function shutdown() {}
            function power(index) {}
            function reset_identity() { return true; }
        }
    }
    Component { id: view; Main {} }
    function setup() {
        const backend = createTemporaryObject(fake, test);
        const window = createTemporaryObject(view, null, {backend: backend});
        verify(window);
        window.requestActivate();
        tryCompare(window, "active", true);
        verify(waitForPolish(window));
        return {window: window, backend: backend};
    }
    function midX(item, scope) {
        return item.mapToItem(scope, item.width / 2, item.height / 2).x;
    }
    function fetch(url) {
        const request = new XMLHttpRequest();
        request.open("GET", url, false);
        request.send();
        return request;
    }
    readonly property var languages: ["zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg"]
    // Every qsTr source of Main.qml; i18n/waylight.ts must list them all under
    // the Main context or a user-visible string went unwrapped.
    readonly property var sources: [
        " (preview only)", "…", "Authentication response", "Cancel", "Choose user",
        "Closing after authentication cleanup…",
        "Connecting; controls are not ready yet.", "Continue", "Desktop session",
        "Desktop session · Wayland",
        "Disconnected; user switching and power changes are disabled.",
        "Enter answer", "Greeter — preview", "More…", "More… · Enter a username manually",
        "No", "Password", "Power request in progress; controls are disabled.",
        "PREVIEW · No system changes", "Restart", "Restart this computer?",
        "Selected user",
        "Session start is committed; user switching and power changes are disabled.",
        "Shut down this computer?", "Shut Down", "Sign in", "SIGN IN · Wayland",
        "Sleep", "Submit answer", "Username", "Yes"
    ]

    function test_english_source_strings_unchanged() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        compare(window.title, "Greeter — preview");
        compare(findChild(window, "cornerLabel").text, "PREVIEW · No system changes");
        const session = findChild(window, "session");
        compare(session.Accessible.name, "Desktop session");
        compare(session.ToolTip.text, "Desktop session · Wayland");
        const tiles = findChild(window, "userTiles");
        compare(tiles.itemAt(0).modelData.name, "More…");
        compare(tiles.itemAt(0).Accessible.name, "More… · Enter a username manually");
        compare(tiles.itemAt(0).Accessible.description, "Selected user");
        window.selectedUsername = "someone";
        compare(tiles.itemAt(0).Accessible.description, "Choose user");
        window.selectedUsername = "";
        const username = findChild(window, "username");
        compare(username.placeholderText, "Username");
        compare(username.Accessible.name, "Username");
        const field = findChild(window, "password");
        compare(field.placeholderText, "Password");
        compare(field.Accessible.name, "Authentication response");
        compare(findChild(window, "login").Accessible.name, "Sign in");
        compare(findChild(window, "login").text, "→");
        compare(findChild(window, "answerLogin").Accessible.name, "Submit answer");
        compare(findChild(window, "answerLogin").text, "→");
        compare(findChild(window, "continueControl").text, "Continue");
        compare(findChild(window, "cancel").text, "Cancel");
        const dialog = findChild(window, "confirmPower");
        compare(dialog.title, "Shut down this computer?");
        dialog.action = 1;
        compare(dialog.title, "Restart this computer?");
        dialog.action = 0;
        compare(findChild(dialog, "confirmYes").text, "Yes");
        compare(findChild(dialog, "confirmNo").text, "No");
        // objectNames stay the untranslated model keys; in English the displayed
        // label equals the key, and the preview suffix translates separately.
        const powers = findChild(window, "powerButtons");
        const keys = ["Sleep", "Restart", "Shut Down"];
        for (let i = 0; i < 3; i++) {
            compare(powers.itemAt(i).objectName, keys[i]);
            compare(powers.itemAt(i).text, keys[i]);
            compare(powers.itemAt(i).Accessible.name, keys[i] + " (preview only)");
        }
        // Blocked-state explanations are the locked English sentences.
        const blocked = {
            loading: "Connecting; controls are not ready yet.",
            starting: "Session start is committed; user switching and power changes are disabled.",
            disconnected: "Disconnected; user switching and power changes are disabled.",
            power: "Power request in progress; controls are disabled."
        };
        for (const state in blocked) {
            backend.state = state;
            compare(window.blockedReason, blocked[state]);
        }
        backend.closing = true;
        compare(window.blockedReason, "Closing after authentication cleanup…");
        backend.closing = false;
        backend.state = "idle";
        compare(window.blockedReason, "");
        backend.preview = false;
        compare(field.placeholderText, "Enter answer");
    }

    function test_committed_catalogs_and_source_coverage() {
        // A committed, non-empty .qm for every supported language. The embed
        // into :/qt/qml/Waylight/i18n/ is proven by the Rust unit tests.
        for (const language of languages) {
            const qm = fetch(Qt.resolvedUrl("../i18n/waylight_" + language + ".qm"));
            compare(qm.readyState, 4);
            compare(qm.status, 200, language);
            verify(qm.responseText.length > 0, language);
        }
        // No en catalog: English is the source language.
        const en = fetch(Qt.resolvedUrl("../i18n/waylight_en.qm"));
        verify(en.status !== 200);
        // lupdate must see every qsTr string, in the Main context only.
        const ts = fetch(Qt.resolvedUrl("../i18n/waylight.ts"));
        compare(ts.status, 200);
        verify(ts.responseText.indexOf("<name>Main</name>") >= 0);
        for (const source of sources)
            verify(ts.responseText.indexOf("<source>" + source + "</source>") >= 0, source);
    }

    function test_layout_mirroring_follows_direction() {
        const ctx = setup(); const window = ctx.window;
        const content = window.contentItem;
        verify(waitForPolish(window));
        compare(window.LayoutMirroring.childrenInherit, true);
        // The process runs left-to-right, so the production binding must not
        // mirror yet.
        compare(window.LayoutMirroring.enabled, false);
        compare(window.LayoutMirroring.enabled,
            Qt.application.layoutDirection === Qt.RightToLeft);
        const corner = findChild(window, "cornerLabel");
        const session = findChild(window, "session");
        const clock = findChild(window, "clockColumn");
        verify(midX(corner, content) < window.width / 2);
        verify(midX(session, content) > window.width / 2);
        verify(Math.abs(midX(clock, content) - window.width / 2) < 3);

        // Apply exactly what RightToLeft does through the production binding
        // (QGuiApplication layout direction is read-only from QML).
        window.LayoutMirroring.enabled = true;
        verify(waitForPolish(window));
        verify(midX(corner, content) > window.width / 2);
        verify(midX(session, content) < window.width / 2);
        verify(Math.abs(midX(clock, content) - window.width / 2) < 3);
        const powers = findChild(window, "powerButtons");
        verify(powers.parent);
        verify(Math.abs(midX(powers.parent, content) - window.width / 2) < 3);

        window.LayoutMirroring.enabled = false;
        verify(waitForPolish(window));
        verify(midX(corner, content) < window.width / 2);
        verify(midX(session, content) > window.width / 2);
    }

    function test_power_object_names_survive_labels() {
        const ctx = setup(); const window = ctx.window;
        const powers = findChild(window, "powerButtons");
        const keys = ["Sleep", "Restart", "Shut Down"];
        for (let i = 0; i < 3; i++) {
            compare(powers.itemAt(i).objectName, powers.itemAt(i).modelData.key);
            compare(powers.itemAt(i).modelData.key, keys[i]);
        }
        // The More… tile key drives no objectName, but its untranslated marker
        // stays: tests match the empty username, never the displayed name.
        const tiles = findChild(window, "userTiles");
        compare(tiles.itemAt(0).modelData.username, "");
        compare(tiles.itemAt(0).modelData.key, "more");
    }
}
