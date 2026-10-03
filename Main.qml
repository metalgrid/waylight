pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    width: 1100
    height: 720
    minimumWidth: 640
    minimumHeight: 580
    visible: true
    title: backend.preview ? "Greeter — preview" : "Sign in"
    color: "#182b56"
    font.family: "sans-serif"

    required property var backend
    property bool closingAllowed: false
    readonly property bool busy: backend.state !== "idle"
    readonly property bool answering: backend.state === "secret" || backend.state === "visible"
    readonly property bool acknowledging: backend.state === "info" || backend.state === "error"
    readonly property bool cancellable: !backend.closing && (answering || acknowledging || backend.state === "waiting")
    readonly property bool interactive: !backend.closing && (!busy || cancellable || backend.state === "cancelling")
    readonly property string blockedReason: backend.closing ? "Closing after authentication cleanup…"
        : backend.state === "starting" || backend.state === "handoff" ? "Session start is committed; user switching and power changes are disabled."
        : backend.state === "disconnected" ? "Disconnected; user switching and power changes are disabled."
        : backend.state === "power" ? "Power request in progress; controls are disabled."
        : backend.state === "loading" ? "Connecting; controls are not ready yet." : ""
    // Presentation only: never use this snapshot to authorize an action or retain an answer.
    property var presentation: ({state: backend.state, prompt: backend.prompt, message: backend.message})
    property bool holdingTransition: false
    readonly property string layoutState: holdingTransition ? presentation.state : backend.state
    readonly property bool layoutBusy: layoutState !== "idle"
    readonly property bool layoutAnswering: layoutState === "secret" || layoutState === "visible"
    readonly property bool layoutAcknowledging: layoutState === "info" || layoutState === "error"
    Component.onCompleted: presentCurrent()
    function presentCurrent() {
        holdingTransition = false;
        presentation = {state: backend.state, prompt: backend.prompt, message: backend.message};
    }
    function updatePresentation() {
        if (!backend.closing && (backend.state === "waiting" || backend.state === "cancelling")) {
            if (!holdingTransition && presentation.state !== "waiting" && presentation.state !== "cancelling") {
                holdingTransition = true;
                transitionDelay.start();
            }
        } else {
            transitionDelay.stop();
            presentCurrent();
        }
    }
    Timer {
        id: transitionDelay
        objectName: "transitionDelay"
        interval: 150
        onTriggered: root.presentCurrent()
    }
    property date now: new Date()
    readonly property string selectedSession: session.currentText
    property string selectedUsername: ""
    readonly property var users: JSON.parse(backend.accounts).slice(0, 5)
    readonly property var tiles: users.concat([{username: "", name: "More…", picture: ""}])
    readonly property string loginUsername: selectedUsername || username.text

    function focusIdentity() {
        if (busy || backend.closing || confirmPower.visible) return;
        if (selectedUsername) login.forceActiveFocus();
        else username.forceActiveFocus();
    }
    function focusCurrent() {
        if (confirmPower.visible || backend.closing) return;
        if (answering) password.forceActiveFocus();
        else if (acknowledging) continueControl.forceActiveFocus();
        else focusIdentity();
    }
    function chooseUser(name) {
        if (!interactive || confirmPower.visible) return;
        if (name) backend.choose_user(name, session.currentIndex);
        else backend.choose_manual();
    }

    function submit() {
        if (backend.closing || confirmPower.visible) return;
        if (!busy) {
            if (loginUsername.length > 0 && session.count > 0)
                backend.begin(loginUsername, session.currentIndex);
        } else if (answering || acknowledging) {
            const response = password.text;
            password.clear();
            backend.answer(response);
        }
    }
    function reset() {
        if (confirmPower.visible) { confirmPower.close(); return; }
        if (session.popup.visible) { session.popup.close(); return; }
        password.clear();
        if (cancellable || (interactive && backend.state === "cancelling")) backend.cancel();
    }
    onClosing: function(close) {
        close.accepted = closingAllowed;
        if (!closingAllowed) backend.shutdown();
    }
    Connections {
        target: root.backend
        function onStateChanged() {
            password.clear();
            root.updatePresentation();
            if (!root.interactive) confirmPower.close();
            root.focusCurrent();
        }
        // Backend view updates set text before state. Snapshot only after that batch,
        // so a cleared/next prompt cannot overwrite the previous layout mid-transition.
        function onPromptChanged() { Qt.callLater(root.updatePresentation); }
        function onMessageChanged() { Qt.callLater(root.updatePresentation); }
        function onClosingChanged() {
            root.updatePresentation();
            if (root.backend.closing) { password.clear(); confirmPower.close(); }
        }
        function onIdentityChosen(name) {
            root.selectedUsername = name;
            username.clear();
            password.clear();
            Qt.callLater(root.focusCurrent);
        }
        function onSelectedChanged() { session.currentIndex = root.backend.selected; }
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: root.now = new Date()
    }
    Shortcut {
        sequence: "Escape"
        onActivated: root.reset()
    }

    Image {
        anchors.fill: parent
        source: "background.svg"
        fillMode: Image.PreserveAspectCrop
    }
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0; color: "#18070b20" }
            GradientStop { position: 1; color: "#50070b20" }
        }
    }
    Label {
        anchors { top: parent.top; left: parent.left; margins: 24 }
        text: root.backend.preview ? "PREVIEW · No system changes" : "SIGN IN · Wayland"
        color: "#dce0ed"
        font { pixelSize: 11; letterSpacing: 1.2 }
    }
    ComboBox {
        id: session
        objectName: "session"
        anchors { top: parent.top; right: parent.right; margins: 16 }
        width: 256
        height: 40
        model: root.backend.sessions
        enabled: !root.busy && !root.backend.closing
        Accessible.name: "Desktop session"
        ToolTip.visible: hovered || activeFocus
        ToolTip.text: "Desktop session · Wayland"
        ToolTip.delay: 700
        leftPadding: 14
        rightPadding: 32
        contentItem: Label {
            text: session.currentText
            textFormat: Text.PlainText
            color: "#ecedf5"
            font.pixelSize: 12
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
        indicator: Label {
            x: session.width - width - 14
            anchors.verticalCenter: parent.verticalCenter
            text: "⌄"
            color: "#ecedf5"
        }
        background: Rectangle {
            radius: 20
            color: session.hovered ? "#35ffffff" : "#18ffffff"
            border.color: session.activeFocus ? "#d9e2ff" : "#40ffffff"
        }
        delegate: ItemDelegate {
            id: option
            required property string modelData
            required property int index
            width: session.width - 12
            text: modelData
            highlighted: session.highlightedIndex === index
            contentItem: Label {
                text: option.text
                textFormat: Text.PlainText
                color: "#ecedf5"
                font.pixelSize: 12
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: 8
                color: option.highlighted ? "#35ffffff" : "transparent"
            }
        }
        popup: Popup {
            y: session.height + 6
            width: session.width
            padding: 6
            implicitHeight: contentItem.implicitHeight + topPadding + bottomPadding
            contentItem: ListView {
                implicitHeight: contentHeight
                model: session.popup.visible ? session.delegateModel : null
                currentIndex: session.highlightedIndex
                clip: true
            }
            background: Rectangle {
                radius: 14
                color: "#ed29324c"
                border.color: "#50ffffff"
            }
        }
    }

    Column {
        anchors { top: parent.top; topMargin: root.height * 0.115; horizontalCenter: parent.horizontalCenter }
        spacing: 0
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            text: Qt.formatDateTime(root.now, "dddd, MMMM d")
            color: "#eef0f8"
            font { pixelSize: 21; weight: Font.Medium }
        }
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            text: Qt.formatDateTime(root.now, "hh:mm")
            color: "#f4f1fa"
            font { pixelSize: Math.min(100, root.height * 0.135); weight: Font.DemiBold; letterSpacing: -3 }
        }
    }

    ScrollView {
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: root.height * (root.height < 650 ? 0.33 : 0.37); bottom: parent.bottom; bottomMargin: 100 }
        width: Math.min(580, root.width - 32)
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
        width: parent.width
        spacing: 10

        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 8
            Repeater {
                id: userTiles
                objectName: "userTiles"
                property var incomingTiles: root.tiles
                onIncomingTilesChanged: refresh()
                // Capture focus before replacing the model destroys its delegates.
                function refresh() {
                    let focusedUsername = null;
                    for (let i = 0; i < count; i++) {
                        if (root.interactive && itemAt(i).activeFocus)
                            focusedUsername = model[i].username;
                    }
                    model = incomingTiles;
                    if (focusedUsername === null || !root.interactive || confirmPower.visible) return;
                    for (let i = 0; i < count; i++) {
                        if (model[i].username === focusedUsername) {
                            itemAt(i).forceActiveFocus();
                            return;
                        }
                    }
                    itemAt(count - 1).forceActiveFocus(); // Removed user: More…
                }
                Component.onCompleted: refresh()
                delegate: Button {
                    id: tile
                    required property var modelData
                    required property int index
                    readonly property bool selected: root.selectedUsername === modelData.username
                    readonly property bool hasPicture: picture.status === Image.Ready
                    width: 84
                    height: 100
                    padding: 4
                    enabled: root.interactive && (!modelData.username || session.count > 0)
                    activeFocusOnTab: true
                    Accessible.name: modelData.username ? modelData.name + " · " + modelData.username : "More… · Enter a username manually"
                    Accessible.description: selected ? "Selected user" : "Choose user"
                    ToolTip {
                        id: userTip
                        objectName: "userTip"
                        visible: tile.hovered || tile.activeFocus
                        text: tile.Accessible.name
                        width: Math.min(implicitWidth, root.width - 32)
                        delay: 700
                        contentItem: Text {
                            text: userTip.text
                            textFormat: Text.PlainText
                            font: userTip.font
                            color: userTip.palette.toolTipText
                            wrapMode: Text.Wrap
                        }
                    }
                    onClicked: root.chooseUser(modelData.username)
                    Keys.onReturnPressed: root.chooseUser(modelData.username)
                    Keys.onEnterPressed: root.chooseUser(modelData.username)
                    Keys.onLeftPressed: userTiles.itemAt((index + userTiles.count - 1) % userTiles.count).forceActiveFocus()
                    Keys.onRightPressed: userTiles.itemAt((index + 1) % userTiles.count).forceActiveFocus()
                    Keys.onPressed: function(event) {
                        if (event.key === Qt.Key_Home || event.key === Qt.Key_End) {
                            userTiles.itemAt(event.key === Qt.Key_Home ? 0 : userTiles.count - 1).forceActiveFocus();
                            event.accepted = true;
                        }
                    }
                    contentItem: Column {
                        spacing: 4
                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 56
                            height: 56
                            radius: width / 2
                            color: "#648695"
                            border { color: "#80ffffff"; width: 1 }
                            Rectangle {
                                visible: !tile.hasPicture && !!tile.modelData.username
                                anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: parent.height * 0.205 }
                                width: parent.width * 0.284
                                height: width
                                radius: width / 2
                                color: "#e4e9e7"
                            }
                            Rectangle {
                                visible: !tile.hasPicture && !!tile.modelData.username
                                anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: parent.height * 0.17 }
                                width: parent.width * 0.557
                                height: parent.height * 0.284
                                topLeftRadius: 25
                                topRightRadius: 25
                                bottomLeftRadius: 9
                                bottomRightRadius: 9
                                color: "#e4e9e7"
                            }
                            Image {
                                id: picture
                                objectName: "picture"
                                anchors.fill: parent
                                source: tile.modelData.picture || ""
                                sourceSize.width: 56
                                sourceSize.height: 56
                                fillMode: Image.PreserveAspectFit
                                visible: tile.hasPicture
                            }
                            Label {
                                anchors.centerIn: parent
                                visible: !tile.modelData.username
                                text: "…"
                                color: "white"
                                font.pixelSize: 28
                            }
                        }
                        Label {
                            objectName: "tileName"
                            width: parent.width
                            text: tile.modelData.name
                            textFormat: Text.PlainText
                            color: "white"
                            font.pixelSize: 12
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }
                    background: Rectangle {
                        radius: 12
                        color: tile.selected ? "#35ffffff" : tile.hovered ? "#18ffffff" : "transparent"
                        border.width: tile.activeFocus ? 2 : 1
                        border.color: tile.activeFocus ? "white" : tile.selected ? "#a0ffffff" : "transparent"
                    }
                }
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 8
            ColumnLayout {
                spacing: 8
                Label {
                    visible: !!root.selectedUsername
                    Layout.preferredWidth: 260
                    Layout.preferredHeight: root.layoutBusy ? 20 : 38
                    text: root.selectedUsername
                    textFormat: Text.PlainText
                    color: "white"
                    font.pixelSize: 16
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
                TextField {
                    id: username
                    objectName: "username"
                    Layout.preferredWidth: 260
                    Layout.preferredHeight: 38
                    visible: !root.selectedUsername && !root.layoutBusy
                    placeholderText: "Username"
                    placeholderTextColor: "#d8d9e3"
                    color: "white"
                    enabled: !root.busy && !root.backend.closing
                    maximumLength: 256
                    leftPadding: 13
                    font { pixelSize: 18; weight: Font.DemiBold }
                    Accessible.name: "Username"
                    selectByMouse: true
                    background: Rectangle { radius: 19; color: "#18ffffff"; border.color: username.activeFocus ? "#d9e2ff" : "transparent" }
                    onAccepted: root.submit()
                    onTextEdited: { if (root.backend.reset_identity()) password.clear(); }
                    Component.onCompleted: forceActiveFocus()
                }
                Label {
                    visible: !root.selectedUsername && root.layoutBusy
                    Layout.preferredWidth: 260
                    text: username.text
                    textFormat: Text.PlainText
                    color: "white"
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                }
            }
            Button {
                id: login
                objectName: "login"
                Layout.alignment: Qt.AlignBottom
                Layout.preferredWidth: 38
                Layout.preferredHeight: 38
                visible: !root.layoutBusy
                enabled: !root.busy && !root.backend.closing && root.loginUsername.length > 0 && session.count > 0
                text: "→"
                Accessible.name: "Sign in"
                onClicked: root.submit()
                Keys.onReturnPressed: root.submit()
                Keys.onEnterPressed: root.submit()
                contentItem: Label {
                    text: login.text
                    color: "white"
                    font.pixelSize: 24
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    radius: 19
                    color: login.down ? "#80ffffff" : login.hovered ? "#60ffffff" : "#30ffffff"
                    border.color: login.activeFocus ? "white" : "#70ffffff"
                    opacity: login.enabled ? 1 : 0.4
                }
            }
        }
        ScrollView {
            Layout.fillWidth: true
            objectName: "promptArea"
            Layout.preferredHeight: Math.min(90, promptLabel.implicitHeight + 8)
            visible: root.layoutAnswering || root.layoutAcknowledging
            opacity: root.holdingTransition ? 0 : 1
            contentWidth: availableWidth
            clip: true
            Label {
                id: promptLabel
                objectName: "prompt"
                width: parent.width
                text: root.holdingTransition ? root.presentation.prompt : root.backend.prompt
                textFormat: Text.PlainText
                color: "white"
                wrapMode: Text.Wrap
                horizontalAlignment: Text.AlignHCenter
                Accessible.name: text
                Accessible.ignored: root.holdingTransition
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 8
            visible: root.layoutAnswering
            TextField {
                id: password
                objectName: "password"
                Layout.preferredWidth: 224
                Layout.preferredHeight: 38
                placeholderText: root.backend.preview ? "Password" : "Enter answer"
                placeholderTextColor: "#d8d9e3"
                color: "white"
                echoMode: root.backend.state === "secret" ? TextInput.Password : TextInput.Normal
                maximumLength: 2147483647
                visible: root.layoutAnswering
                enabled: root.answering && !root.backend.closing
                selectByMouse: true
                activeFocusOnTab: true
                font.pixelSize: 14
                leftPadding: 13
                rightPadding: 13
                Accessible.name: root.answering ? root.backend.prompt || "Authentication response" : "Authentication response"
                background: Rectangle {
                    radius: 19
                    color: "#35ffffff"
                    border.color: password.activeFocus ? "#d9e2ff" : "#60ffffff"
                    border.width: password.activeFocus ? 2 : 1
                }
                onAccepted: root.submit()
            }
            Button {
                id: answerLogin
                objectName: "answerLogin"
                Layout.preferredWidth: 38
                Layout.preferredHeight: 38
                enabled: root.answering && !root.backend.closing
                text: "→"
                Accessible.name: "Submit answer"
                onClicked: root.submit()
                Keys.onReturnPressed: root.submit()
                Keys.onEnterPressed: root.submit()
                contentItem: Label {
                    text: answerLogin.text
                    color: "white"
                    font.pixelSize: 24
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    radius: 19
                    color: answerLogin.down ? "#80ffffff" : answerLogin.hovered ? "#60ffffff" : "#30ffffff"
                    border.color: answerLogin.activeFocus ? "white" : "#70ffffff"
                }
            }
        }
        Button {
            id: continueControl
            objectName: "continueControl"
            Layout.alignment: Qt.AlignHCenter
            visible: root.layoutAcknowledging
            enabled: root.acknowledging && !root.backend.closing
            text: "Continue"
            onClicked: root.submit()
            Keys.onReturnPressed: root.submit()
            Keys.onEnterPressed: root.submit()
        }
        ScrollView {
            Layout.fillWidth: true
            objectName: "statusArea"
            Layout.preferredHeight: Math.min(70, statusLabel.implicitHeight + 8)
            visible: statusLabel.text.length > 0
            opacity: root.holdingTransition ? 0 : 1
            contentWidth: availableWidth
            clip: true
            Label {
                id: statusLabel
                objectName: "status"
                width: parent.width
                text: root.holdingTransition ? root.presentation.message
                    : root.blockedReason + (root.blockedReason && root.backend.message ? "\n" : "") + root.backend.message
                textFormat: Text.PlainText
                color: "#e2e4ef"
                font.pixelSize: 12
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                Accessible.role: Accessible.StaticText
                Accessible.ignored: root.holdingTransition
            }
        }
        Button {
            objectName: "cancel"
            Layout.alignment: Qt.AlignHCenter
            visible: root.holdingTransition ? root.layoutAnswering || root.layoutAcknowledging
                : root.cancellable || root.backend.state === "cancelling"
            enabled: root.cancellable || (root.interactive && root.backend.state === "cancelling")
            text: "Cancel"
            onClicked: root.reset()
        }
        }
    }

    Dialog {
        id: confirmPower
        objectName: "confirmPower"
        property int action: 0
        anchors.centerIn: parent
        title: action === 1 ? "Restart this computer?" : "Shut down this computer?"
        modal: true
        focus: true
        standardButtons: Dialog.Yes | Dialog.No
        onAccepted: {
            if (root.interactive && (root.backend.capabilities & (1 << action)) !== 0)
                root.backend.power(action);
        }
        onClosed: Qt.callLater(root.focusCurrent)
    }

    RowLayout {
        anchors { bottom: parent.bottom; bottomMargin: 28; horizontalCenter: parent.horizontalCenter }
        spacing: 30
        Repeater {
            objectName: "powerButtons"
            model: [
                { label: "Sleep", icon: "icons/sleep.svg", action: 0 },
                { label: "Restart", icon: "icons/restart.svg", action: 1 },
                { label: "Shut Down", icon: "icons/power.svg", action: 2 }
            ]
            delegate: Button {
                id: power
                required property var modelData
                objectName: modelData.label
                Layout.preferredWidth: 44
                Layout.preferredHeight: 44
                padding: 10
                text: modelData.label
                enabled: root.interactive && (root.backend.capabilities & (1 << modelData.action)) !== 0
                Accessible.name: text + (root.backend.preview ? " (preview only)" : "")
                ToolTip.visible: hovered || activeFocus
                ToolTip.text: text
                ToolTip.delay: 400
                onClicked: {
                    if (modelData.action === 0) root.backend.power(0);
                    else { confirmPower.action = modelData.action; confirmPower.open(); }
                }
                contentItem: Image {
                    source: power.modelData.icon
                    sourceSize.width: 24
                    sourceSize.height: 24
                    fillMode: Image.PreserveAspectFit
                }
                background: Rectangle {
                    radius: 22
                    color: power.hovered ? "#30ffffff" : "transparent"
                    border.color: power.activeFocus ? "#e2e4ef" : "transparent"
                }
            }
        }
    }
}
