import QtQuick
import QtQuick.Controls
import QtTest
import ".."

TestCase {
    id: test
    name: "GreeterView"
    when: windowShown

    Component {
        id: fake
        QtObject {
            property string state: "idle"
            property string message: "Test only"
            property string prompt: ""
            property var sessions: ["Hyprland", "Hyprland (uwsm-managed)", "Weston"]
            property int selected: 0
            property int capabilities: 7
            property bool preview: true
            property string accounts: "[]"
            property bool closing: false
            signal identityChosen(string name)
            property int begins: 0
            property int cancels: 0
            property var pending: null
            function execute(intent) {
                if (intent.kind === "user") { begin(intent.name, intent.index); identityChosen(intent.name); }
                else if (intent.kind === "manual") { prompt = ""; message = ""; state = "idle"; identityChosen(""); }
                else { action = intent.index; state = "power"; }
            }
            function request(intent) {
                if (closing || !["idle", "waiting", "secret", "visible", "info", "error", "cancelling"].includes(state)) return;
                if (state === "idle") execute(intent);
                else {
                    pending = intent;
                    if (state !== "cancelling") { cancels++; state = "cancelling"; }
                    message = (intent.kind === "user" ? "Sign in as " + intent.name : intent.kind)
                        + " requested; waiting for authentication cleanup…";
                }
            }
            function completeCleanup() {
                const next = pending; pending = null;
                if (next) execute(next); else state = "idle";
            }
            function choose_user(name, index) { request({kind: "user", name: name, index: index}); }
            function choose_manual() { request({kind: "manual"}); }
            property int identityResets: 0
            function reset_identity() {
                if (closing || state !== "idle") return false;
                identityResets++; prompt = ""; message = "";
                return true;
            }
            property string user: ""
            property int session: -1
            property string response: "unset"
            property int action: -1
            property int answers: 0
            property bool closed: false
            property bool deferPrompt: false
            function deliverPrompt() { message = "No authentication request will be sent."; prompt = "<b>Literal PAM prompt</b>"; state = "secret"; }
            function begin(name, index) {
                begins++; user = name; session = index; message = ""; prompt = ""; state = "waiting";
                if (!deferPrompt) deliverPrompt();
            }
            function answer(text) { response = text; answers++; state = "waiting"; }
            function cancel() { pending = null; if (state !== "cancelling") { cancels++; state = "cancelling"; } }
            function shutdown() { closed = true; closing = true; pending = null; }
            function power(index) { if ((capabilities & (1 << index)) !== 0) request({kind: "power", index: index}); }
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
    function test_username_session_and_empty_secret() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const field = findChild(window, "password"); const login = findChild(window, "login");
        const answer = findChild(window, "answerLogin"); const proceed = findChild(window, "continueControl");
        verify(!field.visible); verify(!answer.visible); verify(!proceed.visible);
        const username = findChild(window, "username"); const session = findChild(window, "session");
        verify(!login.enabled);
        compare(session.count, 3);
        username.text = "sample";
        session.forceActiveFocus(); keyClick(Qt.Key_Space);
        tryCompare(session.popup, "visible", true);
        keyClick(Qt.Key_Down); keyClick(Qt.Key_Return);
        compare(session.currentIndex, 1);
        username.forceActiveFocus(); keyClick(Qt.Key_Return);
        compare(backend.user, "sample"); compare(backend.session, 1);
        verify(!session.enabled); verify(!username.enabled);
        compare(field.echoMode, TextInput.Password);
        tryCompare(field, "activeFocus", true);
        compare(findChild(window, "prompt").textFormat, Text.PlainText);
        verify(!login.visible); verify(answer.visible && answer.enabled); keyClick(Qt.Key_Return);
        compare(backend.response, ""); compare(backend.answers, 1);
        window.submit(); compare(backend.answers, 1);
        backend.state = "visible";
        tryCompare(field, "activeFocus", true);
        compare(field.echoMode, TextInput.Normal);
        field.text = "x".repeat(400);
        keyClick(Qt.Key_Return);
        compare(backend.response.length, 400); compare(field.text, "");
        backend.state = "info";
        tryCompare(proceed, "activeFocus", true);
        verify(proceed.visible); verify(!field.visible); verify(!answer.visible); verify(!login.visible);
        keyClick(Qt.Key_Return); compare(backend.state, "waiting");
        backend.state = "error"; window.submit(); compare(backend.answers, 4);
        keyClick(Qt.Key_Escape); compare(backend.state, "cancelling");
        verify(!login.enabled);
        backend.state = "disconnected"; verify(!login.enabled);
        backend.state = "idle"; tryCompare(username, "activeFocus", true);
        window.close(); verify(backend.closed);
    }
    function test_tiles_manual_late_arrival_identity_and_busy_guards() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const username = findChild(window, "username"); const tiles = findChild(window, "userTiles");
        const field = findChild(window, "password"); const login = findChild(window, "login");
        compare(tiles.count, 1); compare(tiles.itemAt(0).modelData.name, "More…");
        verify(tiles.itemAt(0).selected); verify(!tiles.itemAt(0).hasPicture);
        tryCompare(username, "activeFocus", true);
        username.text = "typed-manually";
        backend.accounts = JSON.stringify([{username: "demo-z", name: "<b>Zed</b>"}]);
        compare(tiles.count, 2); compare(window.selectedUsername, "");
        compare(username.text, "typed-manually"); verify(username.activeFocus);
        compare(findChild(tiles.itemAt(0), "tileName").textFormat, Text.PlainText);
        compare(tiles.itemAt(0).Accessible.name, "<b>Zed</b> · demo-z");
        const tip = findChild(tiles.itemAt(0), "userTip");
        compare(tip.contentItem.textFormat, Text.PlainText);
        compare(tip.text, "<b>Zed</b> · demo-z");
        verify(!tiles.itemAt(0).hasPicture);
        tiles.itemAt(0).forceActiveFocus(); keyClick(Qt.Key_Space);
        compare(window.selectedUsername, "demo-z"); verify(!username.visible);
        compare(backend.state, "secret"); compare(backend.user, "demo-z"); compare(backend.begins, 1);
        compare(username.text, "");
        verify(tiles.itemAt(0).selected);
        tryCompare(field, "activeFocus", true);
        backend.accounts = JSON.stringify([
            {username: "demo-a", name: "Alex"}, {username: "demo-z", name: "<b>Zed</b>"}
        ]);
        compare(window.selectedUsername, "demo-z"); verify(tiles.itemAt(1).selected); verify(field.activeFocus);
        for (let i = 0; i < tiles.count; i++) verify(tiles.itemAt(i).enabled);
        field.text = "sample answer";
        backend.accounts = JSON.stringify([{username: "demo-z", name: "Zed"}]);
        compare(field.text, "sample answer"); verify(field.activeFocus); compare(backend.state, "secret");
        window.chooseUser(""); compare(window.selectedUsername, "demo-z"); compare(backend.state, "cancelling");
        verify(tiles.itemAt(1).enabled); compare(backend.cancels, 1);
        verify(!username.activeFocus); verify(!login.activeFocus);
        keyClick(Qt.Key_Escape); compare(backend.pending, null);
        backend.completeCleanup(); tryCompare(login, "activeFocus", true);
        tiles.itemAt(1).forceActiveFocus(); keyClick(Qt.Key_Return);
        compare(window.selectedUsername, ""); verify(username.visible); tryCompare(username, "activeFocus", true);
        compare(field.text, ""); compare(backend.prompt, "");
        backend.message = "stale failure"; keyClick(Qt.Key_X); compare(backend.message, "");
        compare(username.text, "x");
    }
    function test_tile_focus_survives_discovery_reorder_and_removal() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const tiles = findChild(window, "userTiles"); const username = findChild(window, "username");
        tiles.itemAt(0).forceActiveFocus();
        verify(tiles.itemAt(0).activeFocus);
        backend.accounts = JSON.stringify([{username: "demo-a", name: "Alex"}]);
        compare(tiles.count, 2); verify(tiles.itemAt(1).activeFocus);
        keyClick(Qt.Key_Return);
        tryCompare(username, "activeFocus", true); verify(username.visible);
        compare(window.selectedUsername, ""); compare(backend.user, ""); compare(backend.state, "idle");

        tiles.itemAt(0).forceActiveFocus();
        backend.accounts = JSON.stringify([
            {username: "demo-b", name: "Blair"}, {username: "demo-a", name: "Alex"}
        ]);
        verify(tiles.itemAt(1).activeFocus); compare(tiles.itemAt(1).modelData.username, "demo-a");
        backend.accounts = JSON.stringify([{username: "demo-b", name: "Blair"}]);
        verify(tiles.itemAt(1).activeFocus); compare(tiles.itemAt(1).modelData.username, "");
        compare(window.selectedUsername, ""); compare(backend.user, ""); compare(backend.begins, 0);

        const session = findChild(window, "session");
        session.forceActiveFocus();
        backend.accounts = "[]";
        verify(session.activeFocus); compare(backend.state, "idle");
    }
    function demoAccounts() {
        return ["alex", "sam", "lee", "robin", "jules"].map(name => ({
            username: "demo-" + name,
            name: name[0].toUpperCase() + name.slice(1) + " (demo)",
            picture: Qt.resolvedUrl("../icons/demo-" + name + ".png").toString()
        }));
    }
    function test_five_tile_cap_keyboard_navigation_click_and_image_fallback() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const tiles = findChild(window, "userTiles");
        mouseClick(tiles.itemAt(0)); // More… also works without discovery.
        tryCompare(findChild(window, "username"), "activeFocus", true);
        const accounts = demoAccounts();
        accounts.push({username: "sixth", name: "Not a tile"});
        backend.accounts = JSON.stringify(accounts);
        compare(tiles.count, 6); compare(tiles.itemAt(5).modelData.name, "More…");
        // Repeater delegates exist before Row/ancestor layouts position them.
        verify(waitForPolish(window));
        for (let i = 0; i < 5; i++) {
            tryCompare(tiles.itemAt(i), "hasPicture", true);
            compare(findChild(tiles.itemAt(i), "picture").sourceSize.width, 56);
        }
        tiles.itemAt(0).forceActiveFocus(); keyClick(Qt.Key_Right);
        verify(tiles.itemAt(1).activeFocus); compare(window.selectedUsername, "");
        keyClick(Qt.Key_Left); verify(tiles.itemAt(0).activeFocus);
        keyClick(Qt.Key_Left); verify(tiles.itemAt(5).activeFocus);
        keyClick(Qt.Key_Home); verify(tiles.itemAt(0).activeFocus);
        keyClick(Qt.Key_End); verify(tiles.itemAt(5).activeFocus);
        keyClick(Qt.Key_Home); keyClick(Qt.Key_Tab); verify(tiles.itemAt(1).activeFocus);
        keyClick(Qt.Key_Return); compare(window.selectedUsername, "demo-sam"); compare(backend.user, "demo-sam"); compare(backend.begins, 1);
        // Selection changes identity/status visibility; settle the whole window before hit testing.
        verify(waitForPolish(window));
        mouseClick(tiles.itemAt(2)); compare(window.selectedUsername, "demo-sam"); compare(backend.state, "cancelling");
        compare(backend.cancels, 1); backend.completeCleanup(); compare(window.selectedUsername, "demo-lee"); compare(backend.begins, 2);
        verify(waitForPolish(window));
        mouseClick(tiles.itemAt(5)); compare(window.selectedUsername, "demo-lee"); backend.completeCleanup();
        tryCompare(findChild(window, "username"), "activeFocus", true);
        findChild(window, "username").text = "sixth"; window.submit(); compare(backend.user, "sixth");
        backend.state = "idle";
        accounts[0].picture = "data:image/png;base64,bm90IHBuZw==";
        accounts[1].picture = "";
        ignoreWarning(/.*Error decoding: data:image\/png;base64,bm90IHBuZw==: Unsupported image format/);
        backend.accounts = JSON.stringify(accounts);
        tryCompare(findChild(tiles.itemAt(0), "picture"), "status", Image.Error);
        verify(!tiles.itemAt(0).hasPicture); verify(!tiles.itemAt(1).hasPicture);
    }
    function test_real_prompt_labels_and_control_visibility() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const field = findChild(window, "password");
        const answer = findChild(window, "answerLogin"); const login = findChild(window, "login");
        const proceed = findChild(window, "continueControl");
        backend.preview = false;
        backend.prompt = "<b>Recovery code or token PIN — not necessarily a password</b>";
        for (const state of ["secret", "visible", "info", "error", "waiting", "cancelling", "starting", "handoff", "disconnected", "idle"]) {
            backend.state = state;
            if (window.holdingTransition) expireTransition(window);
            const answering = state === "secret" || state === "visible";
            compare(field.visible, answering); compare(answer.visible, answering);
            const tiles = findChild(window, "userTiles");
            const interactive = ["idle", "secret", "visible", "info", "error", "waiting", "cancelling"].includes(state);
            for (let i = 0; i < tiles.count; i++) compare(tiles.itemAt(i).enabled, interactive);
            if (!interactive) { window.chooseUser("blocked"); compare(window.selectedUsername, ""); verify(window.blockedReason.length > 0); }
            compare(login.visible, state === "idle");
            compare(proceed.visible, state === "info" || state === "error");
            compare(field.placeholderText, "Enter answer");
            compare(findChild(window, "prompt").text, backend.prompt);
            compare(findChild(window, "prompt").textFormat, Text.PlainText);
        }
        backend.state = "disconnected";
        backend.message = "Session start outcome is unknown. No retry or rollback was attempted.";
        verify(findChild(window, "status").text.includes(backend.message));
        backend.preview = true; backend.prompt = "Password"; backend.state = "secret";
        compare(field.placeholderText, "Password");
    }
    // Drive the single-shot boundary explicitly: no wall-clock sleeps/races.
    function freezeTransition(window) {
        const timer = findChild(window, "transitionDelay");
        compare(timer.interval, 150); verify(timer.running); verify(window.holdingTransition);
        timer.stop();
    }
    function expireTransition(window) {
        const timer = findChild(window, "transitionDelay");
        timer.stop(); timer.triggered();
        verify(!window.holdingTransition);
    }
    function test_transition_presentation_data() {
        return [{tag: "preview", preview: true}, {tag: "real", preview: false}];
    }
    function test_transition_presentation(data) {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        backend.preview = data.preview;
        backend.accounts = JSON.stringify(demoAccounts());
        const tiles = findChild(window, "userTiles");
        const field = findChild(window, "password"); const answer = findChild(window, "answerLogin");
        const prompt = findChild(window, "prompt"); const status = findChild(window, "status");
        backend.deferPrompt = true;
        verify(waitForPolish(window)); mouseClick(tiles.itemAt(0));
        freezeTransition(window);
        compare(backend.state, "waiting"); compare(window.selectedUsername, "demo-alex");
        verify(!field.visible); verify(findChild(window, "login").visible);
        verify(!findChild(window, "login").enabled); verify(!findChild(window, "session").enabled);
        window.submit(); compare(backend.begins, 1);
        backend.deliverPrompt(); verify(!window.holdingTransition);
        verify(!findChild(window, "transitionDelay").running);
        tryCompare(field, "activeFocus", true);
        verify(waitForPolish(window));
        const before = field.mapToItem(window.contentItem, 0, 0);
        const cancelBefore = findChild(window, "cancel").mapToItem(window.contentItem, 0, 0);
        field.text = "synthetic secret";
        mouseClick(tiles.itemAt(1)); freezeTransition(window);
        compare(field.text, ""); verify(field.visible && !field.enabled); verify(answer.visible && !answer.enabled);
        compare(window.selectedUsername, "demo-alex");
        verify(prompt.Accessible.ignored && status.Accessible.ignored);
        compare(findChild(window, "promptArea").opacity, 0); compare(findChild(window, "statusArea").opacity, 0);
        verify(!status.text.includes("cleanup"));
        window.submit(); compare(backend.answers, 0);
        // Real backend clears text before delivering waiting, then updates identity.
        backend.completeCleanup(); compare(backend.state, "waiting");
        compare(window.selectedUsername, "demo-sam"); verify(window.holdingTransition);
        verify(waitForPolish(window));
        compare(field.mapToItem(window.contentItem, 0, 0), before);
        compare(findChild(window, "cancel").mapToItem(window.contentItem, 0, 0), cancelBefore);
        backend.prompt = "New user challenge"; backend.message = ""; backend.state = "visible";
        verify(!window.holdingTransition); verify(!findChild(window, "transitionDelay").running);
        compare(prompt.text, "New user challenge"); compare(findChild(window, "promptArea").opacity, 1);
        tryCompare(field, "activeFocus", true); compare(field.text, "");
        verify(!status.text.includes("cleanup"));
        // A fresh transition gets one deadline; replacement intentions do not postpone it.
        window.chooseUser("demo-lee"); freezeTransition(window);
        window.chooseUser("demo-robin"); verify(!findChild(window, "transitionDelay").running);
        verify(window.holdingTransition); verify(field.visible); verify(!status.text.includes("cleanup"));
        expireTransition(window);
        verify(!field.visible); verify(!answer.visible);
        verify(status.text.includes("Sign in as demo-robin requested")); compare(findChild(window, "statusArea").opacity, 1);
        window.chooseUser("demo-jules"); verify(!window.holdingTransition);
        verify(status.text.includes("Sign in as demo-jules requested"));
        keyClick(Qt.Key_Escape); compare(backend.pending, null); compare(field.text, "");
        backend.completeCleanup(); compare(backend.state, "idle");
    }
    function test_transition_terminal_states_are_immediate() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const field = findChild(window, "password"); const status = findChild(window, "status");
        for (const state of ["idle", "error", "disconnected", "starting", "handoff", "power", "closing"]) {
            backend.closing = false; backend.deliverPrompt();
            field.text = "synthetic";
            window.chooseUser("sample"); freezeTransition(window);
            backend.message = "Failure/outcome must be visible now";
            if (state === "closing") backend.closing = true;
            else backend.state = state;
            verify(!window.holdingTransition); verify(!findChild(window, "transitionDelay").running);
            verify(status.text.includes(backend.message)); compare(findChild(window, "statusArea").opacity, 1);
            compare(field.text, ""); verify(!field.visible);
            if (["disconnected", "starting", "handoff", "power", "closing"].includes(state)) {
                verify(!window.interactive); window.chooseUser("blocked");
                compare(backend.begins, 0);
            }
        }
    }
    function test_power_capabilities_confirmations_and_escape() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const powers = findChild(window, "powerButtons");
        const dialog = findChild(window, "confirmPower");
        compare(powers.count, 3);
        backend.capabilities = 0;
        for (let i = 0; i < 3; i++) verify(!powers.itemAt(i).enabled);
        backend.capabilities = 7;
        mouseClick(powers.itemAt(1)); tryCompare(dialog, "visible", true);
        compare(backend.action, -1);
        keyClick(Qt.Key_Escape); tryCompare(dialog, "visible", false);
        compare(backend.state, "idle"); compare(backend.action, -1);
        mouseClick(powers.itemAt(2)); tryCompare(dialog, "visible", true);
        verify(waitForPolish(window));
        mouseClick(dialog.standardButton(Dialog.Yes)); compare(backend.action, 2);
        backend.state = "idle"; verify(waitForPolish(window));
        mouseClick(powers.itemAt(0)); compare(backend.action, 0);
        backend.state = "waiting";
        for (let i = 0; i < 3; i++) verify(powers.itemAt(i).enabled);
        backend.action = -1; verify(waitForPolish(window)); mouseClick(powers.itemAt(0));
        compare(backend.state, "cancelling"); compare(backend.action, -1); verify(!dialog.visible);
        backend.completeCleanup(); compare(backend.action, 0);
        backend.state = "starting";
        keyClick(Qt.Key_Escape); compare(backend.state, "starting");
        backend.state = "handoff";
        for (let i = 0; i < 3; i++) verify(!powers.itemAt(i).enabled);
    }
    function test_tile_activation_once_data() {
        return [{tag: "mouse", key: 0}, {tag: "Space", key: Qt.Key_Space},
            {tag: "Return", key: Qt.Key_Return}, {tag: "Enter", key: Qt.Key_Enter}];
    }
    function test_tile_activation_once(data) {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        backend.accounts = JSON.stringify(demoAccounts());
        const tiles = findChild(window, "userTiles");
        verify(waitForPolish(window));
        const session = findChild(window, "session"); session.currentIndex = 2;
        tiles.itemAt(0).forceActiveFocus();
        if (data.key) keyClick(data.key); else mouseClick(tiles.itemAt(0));
        compare(backend.begins, 1); compare(backend.cancels, 0); compare(backend.session, 2);
        compare(window.selectedUsername, "demo-alex");
        tryCompare(findChild(window, "password"), "activeFocus", true);
        window.chooseUser("demo-sam"); window.chooseUser("demo-lee");
        compare(backend.cancels, 1); compare(backend.begins, 1); compare(window.selectedUsername, "demo-alex");
        backend.completeCleanup(); compare(backend.begins, 2); compare(window.selectedUsername, "demo-lee");
        window.chooseUser(""); compare(backend.begins, 2); compare(window.selectedUsername, "demo-lee");
        backend.completeCleanup(); tryCompare(findChild(window, "username"), "activeFocus", true);
        compare(window.selectedUsername, ""); compare(backend.begins, 2);
    }
    function test_modal_preserves_answer_focus_and_revalidates() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        const powers = findChild(window, "powerButtons"); const dialog = findChild(window, "confirmPower");
        const field = findChild(window, "password");
        window.chooseUser("sample"); tryCompare(field, "activeFocus", true);
        field.text = "synthetic unchanged answer"; verify(waitForPolish(window));
        mouseClick(powers.itemAt(1)); tryCompare(dialog, "visible", true);
        compare(backend.state, "secret"); compare(backend.cancels, 0); compare(field.text, "synthetic unchanged answer");
        verify(!field.activeFocus); verify(waitForPolish(window));
        mouseClick(dialog.standardButton(Dialog.No)); tryCompare(dialog, "visible", false);
        tryCompare(field, "activeFocus", true); compare(field.text, "synthetic unchanged answer"); compare(backend.cancels, 0);
        mouseClick(powers.itemAt(2)); tryCompare(dialog, "visible", true);
        backend.state = "waiting"; backend.prompt = "New literal challenge"; backend.state = "visible";
        verify(!field.activeFocus); verify(dialog.activeFocus);
        keyClick(Qt.Key_Escape); tryCompare(dialog, "visible", false); tryCompare(field, "activeFocus", true);
        compare(backend.state, "visible"); compare(backend.cancels, 0);
        field.text = "another synthetic answer";
        mouseClick(powers.itemAt(2)); tryCompare(dialog, "visible", true);
        backend.capabilities = 0; verify(waitForPolish(window)); mouseClick(dialog.standardButton(Dialog.Yes));
        compare(backend.cancels, 0); compare(field.text, "another synthetic answer");
        backend.capabilities = 7;
        mouseClick(powers.itemAt(2)); tryCompare(dialog, "visible", true);
        verify(waitForPolish(window)); mouseClick(dialog.standardButton(Dialog.Yes));
        compare(backend.state, "cancelling"); compare(backend.action, -1); compare(backend.cancels, 1); compare(field.text, "");
        backend.completeCleanup(); compare(backend.state, "power"); compare(backend.action, 2);
        for (const state of ["starting", "handoff", "disconnected", "power"]) {
            backend.state = "secret"; verify(waitForPolish(window));
            mouseClick(powers.itemAt(1)); tryCompare(dialog, "visible", true);
            backend.state = state; tryCompare(dialog, "visible", false);
            verify(!window.interactive); verify(window.blockedReason.length > 0);
        }
        backend.state = "secret"; verify(waitForPolish(window));
        mouseClick(powers.itemAt(1)); tryCompare(dialog, "visible", true);
        backend.shutdown(); tryCompare(dialog, "visible", false);
        verify(!window.interactive); verify(!field.enabled);
    }
    function test_late_discovery_preserves_tile_focus_during_prompt() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        window.chooseUser("sample");
        const tiles = findChild(window, "userTiles"); const field = findChild(window, "password");
        tryCompare(field, "activeFocus", true); field.text = "synthetic";
        backend.accounts = JSON.stringify([{username: "a", name: "A"}]);
        verify(field.activeFocus); compare(field.text, "synthetic");
        tiles.itemAt(0).forceActiveFocus();
        backend.accounts = JSON.stringify([{username: "b", name: "B"}, {username: "a", name: "A"}]);
        verify(tiles.itemAt(1).activeFocus); compare(backend.begins, 1); compare(field.text, "synthetic");
        backend.accounts = JSON.stringify([{username: "b", name: "B"}]);
        verify(tiles.itemAt(1).activeFocus); compare(tiles.itemAt(1).modelData.username, "");
        compare(window.selectedUsername, "sample");
    }
    function test_application_images_data() {
        return [{tag: "normal", width: 1100, height: 720}, {tag: "minimum", width: 640, height: 580}];
    }
    function test_application_images(data) {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        window.width = data.width; window.height = data.height;
        backend.message = "Preview only. Use sample text, never real credentials.";
        backend.accounts = JSON.stringify(demoAccounts());
        const tiles = findChild(window, "userTiles");
        compare(tiles.count, 6);
        verify(waitForPolish(window));
        for (let i = 0; i < tiles.count; i++) {
            const tile = tiles.itemAt(i);
            if (i < 5) tryCompare(tile, "hasPicture", true);
            const corner = tile.mapToItem(window.contentItem, tile.width, tile.height);
            verify(corner.x <= window.width && corner.y < window.height - 100);
            verify(tile.mapToItem(window.contentItem, 0, 0).x >= 0);
        }
        waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/manual-" + data.tag + ".png");
        tiles.itemAt(0).forceActiveFocus();
        waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/focus-" + data.tag + ".png");
        mouseClick(tiles.itemAt(0));
        tryCompare(findChild(window, "password"), "activeFocus", true);
        waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/selected-" + data.tag + ".png");
        backend.prompt = "Password";
        backend.message = "No authentication request will be sent.";
        backend.state = "secret";
        tryCompare(findChild(window, "password"), "activeFocus", true);
        waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/password-" + data.tag + ".png");
        const dialog = findChild(window, "confirmPower");
        const powers = findChild(window, "powerButtons");
        verify(waitForPolish(window)); mouseClick(powers.itemAt(1)); tryCompare(dialog, "visible", true);
        verify(waitForPolish(window)); waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/modal-" + data.tag + ".png");
        dialog.reject(); tryCompare(dialog, "visible", false);
        window.chooseUser("demo-sam");
        backend.message = "Sign in as demo-sam requested; waiting for authentication cleanup…";
        freezeTransition(window);
        verify(waitForPolish(window)); waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-transition-checks/switch-fast-" + data.tag + ".png");
        expireTransition(window);
        verify(waitForPolish(window)); waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-transition-checks/switch-slow-" + data.tag + ".png");
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/switch-wait-" + data.tag + ".png");
        compare(window.selectedUsername, "demo-alex"); backend.completeCleanup();
        tryCompare(findChild(window, "password"), "activeFocus", true);
        verify(waitForPolish(window)); waitForRendering(window.contentItem);
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/switched-" + data.tag + ".png");
        const selectedCancel = findChild(window, "cancel");
        verify(selectedCancel.mapToItem(window.contentItem, 0, selectedCancel.height).y <= window.height - 100);
        backend.state = "idle"; window.chooseUser("");
        findChild(window, "username").text = "demo-manual";
        backend.prompt = "Password"; backend.message = "No authentication request will be sent."; backend.state = "secret";
        tryCompare(findChild(window, "password"), "activeFocus", true);
        waitForRendering(window.contentItem);
        const cancel = findChild(window, "cancel");
        grabImage(window.contentItem).save("/tmp/waylight-login-ux-checks/password-manual-" + data.tag + ".png");
        const bottom = cancel.mapToItem(window.contentItem, 0, cancel.height).y;
        verify(bottom <= window.height - 100, "Cancel clipped at " + bottom);
    }
    function test_long_plain_prompt_and_no_sessions() {
        const ctx = setup(); const window = ctx.window; const backend = ctx.backend;
        findChild(window, "username").text = "sample";
        backend.sessions = [];
        verify(!findChild(window, "login").enabled);
        backend.prompt = "<b>Not HTML</b> " + "long prompt ".repeat(100);
        backend.state = "visible";
        compare(findChild(window, "prompt").text, backend.prompt);
        const field = findChild(window, "password");
        field.text = "sample";
        keyClick(Qt.Key_Escape);
        compare(field.text, ""); compare(backend.state, "cancelling");
    }
}
