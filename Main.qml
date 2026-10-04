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
    title: backend.preview ? qsTr("Greeter — preview") : qsTr("Sign in")
    color: theme.windowColor
    font.family: theme.fontFamily
    // Presentation direction follows the UI language resolved in Rust before
    // the engine loads (ar/ur mirror); anchored edges swap, centers stay put.
    LayoutMirroring.enabled: Qt.application.layoutDirection === Qt.RightToLeft
    LayoutMirroring.childrenInherit: true

    required property var backend
    property bool closingAllowed: false
    readonly property bool busy: backend.state !== "idle"
    readonly property bool answering: backend.state === "secret" || backend.state === "visible"
    readonly property bool acknowledging: backend.state === "info" || backend.state === "error"
    readonly property bool cancellable: !backend.closing && (answering || acknowledging || backend.state === "waiting")
    readonly property bool interactive: !backend.closing && (!busy || cancellable || backend.state === "cancelling")
    readonly property string blockedReason: backend.closing ? qsTr("Closing after authentication cleanup…")
        : backend.state === "starting" || backend.state === "handoff" ? qsTr("Session start is committed; user switching and power changes are disabled.")
        : backend.state === "disconnected" ? qsTr("Disconnected; user switching and power changes are disabled.")
        : backend.state === "power" ? qsTr("Power request in progress; controls are disabled.")
        : backend.state === "loading" ? qsTr("Connecting; controls are not ready yet.") : ""
    // Presentation only: never use this snapshot to authorize an action or retain an answer.
    property var presentation: ({state: backend.state, prompt: backend.prompt, message: backend.message})
    property bool holdingTransition: false
    readonly property string layoutState: holdingTransition ? presentation.state : backend.state
    readonly property bool layoutBusy: layoutState !== "idle"
    readonly property bool layoutAnswering: layoutState === "secret" || layoutState === "visible"
    readonly property bool layoutAcknowledging: layoutState === "info" || layoutState === "error"
    Component.onCompleted: {
        // Fail-open: a missing or broken theme never prevents startup.
        theme.loadFirstExisting(typeof backend.themePaths === "string" ? backend.themePaths : "[]");
        presentCurrent();
    }
    Theme {
        id: theme
        objectName: "theme"
    }
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
    // key stays untranslated (tests and the Cage harness match it); only the
    // displayed name is translated.
    readonly property var tiles: users.concat([{username: "", key: "more", name: qsTr("More…"), picture: ""}])
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
        visible: theme.backgroundMode === "builtin"
        anchors.fill: parent
        source: "background.svg"
        fillMode: Image.PreserveAspectCrop
    }
    Image {
        visible: theme.backgroundMode === "image"
        anchors.fill: parent
        source: theme.backgroundImage
        fillMode: Image.PreserveAspectCrop
    }
    Rectangle {
        visible: theme.backgroundMode === "solid"
        anchors.fill: parent
        color: theme.backgroundColor
    }
    Rectangle {
        visible: theme.backgroundMode === "gradient"
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0; color: theme.backgroundTop }
            GradientStop { position: 1; color: theme.backgroundBottom }
        }
    }
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0; color: theme.overlayTop }
            GradientStop { position: 1; color: theme.overlayBottom }
        }
    }
    Label {
        objectName: "cornerLabel"
        anchors { top: parent.top; left: parent.left; margins: theme.barMarginLeft }
        text: root.backend.preview ? qsTr("PREVIEW · No system changes") : qsTr("SIGN IN · Wayland")
        color: theme.textBar
        font { pixelSize: theme.scaled(11); letterSpacing: 1.2 }
    }
    ComboBox {
        id: session
        objectName: "session"
        anchors { top: parent.top; right: parent.right; margins: theme.barMarginRight }
        width: 256
        height: 40
        model: root.backend.sessions
        enabled: !root.busy && !root.backend.closing
        Accessible.name: qsTr("Desktop session")
        ToolTip.visible: hovered || activeFocus
        ToolTip.text: qsTr("Desktop session · Wayland")
        ToolTip.delay: 700
        leftPadding: 14
        rightPadding: 32
        contentItem: Label {
            text: session.currentText
            textFormat: Text.PlainText
            color: theme.textCombo
            font.pixelSize: theme.scaled(12)
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
        indicator: Label {
            x: session.width - width - 14
            anchors.verticalCenter: parent.verticalCenter
            text: "⌄"
            color: theme.textCombo
        }
        background: Rectangle {
            radius: theme.controlRadius
            color: session.hovered ? theme.fillHover : theme.fillRest
            border.color: session.activeFocus ? theme.accent : theme.borderSubtle
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
                color: theme.textCombo
                font.pixelSize: theme.scaled(12)
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: theme.optionRadius
                color: option.highlighted ? theme.fillHover : "transparent"
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
                radius: theme.popupRadius
                color: theme.popupBackground
                border.color: theme.popupBorder
            }
        }
    }

    Column {
        objectName: "clockColumn"
        anchors { top: parent.top; topMargin: root.height * theme.clockTopRatio; horizontalCenter: parent.horizontalCenter }
        spacing: 0
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            text: Qt.formatDateTime(root.now, "dddd, MMMM d")
            color: theme.textDate
            font { pixelSize: theme.scaled(theme.clockDateSize); weight: Font.Medium }
        }
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            text: Qt.formatDateTime(root.now, "hh:mm")
            color: theme.textBright
            font { pixelSize: Math.min(theme.scaled(theme.clockTimeCap), root.height * 0.135); weight: Font.DemiBold; letterSpacing: -3 }
        }
    }

    ScrollView {
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: root.height * (root.height < theme.panelShortHeight ? theme.panelTopRatioShort : theme.panelTopRatio); bottom: parent.bottom; bottomMargin: theme.panelBottomMargin }
        width: Math.min(theme.panelMaxWidth, root.width - 32)
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
                    width: theme.tileWidth
                    height: theme.tileHeight
                    padding: 4
                    enabled: root.interactive && (!modelData.username || session.count > 0)
                    activeFocusOnTab: true
                    Accessible.name: modelData.username ? modelData.name + " · " + modelData.username : qsTr("More… · Enter a username manually")
                    Accessible.description: selected ? qsTr("Selected user") : qsTr("Choose user")
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
                            width: theme.avatarSize
                            height: theme.avatarSize
                            radius: width / 2
                            color: theme.avatarFallback
                            border { color: theme.borderAvatar; width: 1 }
                            Rectangle {
                                visible: !tile.hasPicture && !!tile.modelData.username
                                anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: parent.height * 0.205 }
                                width: parent.width * 0.284
                                height: width
                                radius: width / 2
                                color: theme.avatarGlyph
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
                                color: theme.avatarGlyph
                            }
                            Image {
                                id: picture
                                objectName: "picture"
                                anchors.fill: parent
                                source: tile.modelData.picture || ""
                                sourceSize.width: theme.avatarSize
                                sourceSize.height: theme.avatarSize
                                fillMode: Image.PreserveAspectFit
                                visible: tile.hasPicture
                            }
                            Label {
                                anchors.centerIn: parent
                                visible: !tile.modelData.username
                                text: qsTr("…")
                                color: theme.textPrimary
                                font.pixelSize: theme.scaled(28)
                            }
                        }
                        Label {
                            objectName: "tileName"
                            width: parent.width
                            text: tile.modelData.name
                            textFormat: Text.PlainText
                            color: theme.textPrimary
                            font.pixelSize: theme.scaled(12)
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }
                    background: Rectangle {
                        radius: theme.tileRadius
                        color: tile.selected ? theme.fillHover : tile.hovered ? theme.fillRest : "transparent"
                        border.width: tile.activeFocus ? 2 : 1
                        border.color: tile.activeFocus ? theme.textPrimary : tile.selected ? theme.borderSelected : "transparent"
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
                    Layout.preferredWidth: theme.fieldWidth
                    Layout.preferredHeight: root.layoutBusy ? 20 : theme.fieldHeight
                    text: root.selectedUsername
                    textFormat: Text.PlainText
                    color: theme.textPrimary
                    font.pixelSize: theme.scaled(16)
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
                TextField {
                    id: username
                    objectName: "username"
                    Layout.preferredWidth: theme.fieldWidth
                    Layout.preferredHeight: theme.fieldHeight
                    visible: !root.selectedUsername && !root.layoutBusy
                    placeholderText: qsTr("Username")
                    placeholderTextColor: theme.textPlaceholder
                    color: theme.textPrimary
                    enabled: !root.busy && !root.backend.closing
                    maximumLength: 256
                    leftPadding: 13
                    font { pixelSize: theme.scaled(18); weight: Font.DemiBold }
                    Accessible.name: qsTr("Username")
                    selectByMouse: true
                    background: Rectangle { radius: theme.fieldRadius; color: theme.fillRest; border.color: username.activeFocus ? theme.accent : "transparent" }
                    onAccepted: root.submit()
                    onTextEdited: { if (root.backend.reset_identity()) password.clear(); }
                    Component.onCompleted: forceActiveFocus()
                }
                Label {
                    visible: !root.selectedUsername && root.layoutBusy
                    Layout.preferredWidth: theme.fieldWidth
                    text: username.text
                    textFormat: Text.PlainText
                    color: theme.textPrimary
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                }
            }
            Button {
                id: login
                objectName: "login"
                Layout.alignment: Qt.AlignBottom
                Layout.preferredWidth: theme.fieldHeight
                Layout.preferredHeight: theme.fieldHeight
                visible: !root.layoutBusy
                enabled: !root.busy && !root.backend.closing && root.loginUsername.length > 0 && session.count > 0
                text: "→"
                Accessible.name: qsTr("Sign in")
                onClicked: root.submit()
                Keys.onReturnPressed: root.submit()
                Keys.onEnterPressed: root.submit()
                contentItem: Label {
                    text: login.text
                    color: theme.textPrimary
                    font.pixelSize: theme.scaled(24)
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    radius: theme.fieldRadius
                    color: login.down ? theme.fillDown : login.hovered ? theme.fillAnswer : theme.fillSubtle
                    border.color: login.activeFocus ? theme.textPrimary : theme.borderButton
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
                color: theme.textPrimary
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
                Layout.preferredHeight: theme.fieldHeight
                placeholderText: root.backend.preview ? qsTr("Password") : qsTr("Enter answer")
                placeholderTextColor: theme.textPlaceholder
                color: theme.textPrimary
                echoMode: root.backend.state === "secret" ? TextInput.Password : TextInput.Normal
                maximumLength: 2147483647
                visible: root.layoutAnswering
                enabled: root.answering && !root.backend.closing
                selectByMouse: true
                activeFocusOnTab: true
                font.pixelSize: theme.scaled(14)
                leftPadding: 13
                rightPadding: 13
                Accessible.name: root.answering ? root.backend.prompt || qsTr("Authentication response") : qsTr("Authentication response")
                background: Rectangle {
                    radius: theme.fieldRadius
                    color: theme.fillHover
                    border.color: password.activeFocus ? theme.accent : theme.borderField
                    border.width: password.activeFocus ? 2 : 1
                }
                onAccepted: root.submit()
            }
            Button {
                id: answerLogin
                objectName: "answerLogin"
                Layout.preferredWidth: theme.fieldHeight
                Layout.preferredHeight: theme.fieldHeight
                enabled: root.answering && !root.backend.closing
                text: "→"
                Accessible.name: qsTr("Submit answer")
                onClicked: root.submit()
                Keys.onReturnPressed: root.submit()
                Keys.onEnterPressed: root.submit()
                contentItem: Label {
                    text: answerLogin.text
                    color: theme.textPrimary
                    font.pixelSize: theme.scaled(24)
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    radius: theme.fieldRadius
                    color: answerLogin.down ? theme.fillDown : answerLogin.hovered ? theme.fillAnswer : theme.fillSubtle
                    border.color: answerLogin.activeFocus ? theme.textPrimary : theme.borderButton
                }
            }
        }
        Button {
            id: continueControl
            objectName: "continueControl"
            Layout.alignment: Qt.AlignHCenter
            visible: root.layoutAcknowledging
            enabled: root.acknowledging && !root.backend.closing
            text: qsTr("Continue")
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
                color: theme.textSecondary
                font.pixelSize: theme.scaled(12)
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
            text: qsTr("Cancel")
            onClicked: root.reset()
        }
        }
    }

    Dialog {
        id: confirmPower
        objectName: "confirmPower"
        property int action: 0
        anchors.centerIn: parent
        title: action === 1 ? qsTr("Restart this computer?") : qsTr("Shut down this computer?")
        modal: true
        focus: true
        // Explicit buttons: standardButtons' built-in texts do not translate.
        // Tests match the objectNames, never the displayed text.
        footer: DialogButtonBox {
            Button {
                objectName: "confirmNo"
                text: qsTr("No")
                DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
                onClicked: confirmPower.reject()
            }
            Button {
                objectName: "confirmYes"
                text: qsTr("Yes")
                DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
                onClicked: confirmPower.accept()
            }
        }
        onAccepted: {
            if (root.interactive && (root.backend.capabilities & (1 << action)) !== 0)
                root.backend.power(action);
        }
        onClosed: Qt.callLater(root.focusCurrent)
    }

    RowLayout {
        anchors { bottom: parent.bottom; bottomMargin: theme.bottomBarMargin; horizontalCenter: parent.horizontalCenter }
        spacing: 30
        Repeater {
            objectName: "powerButtons"
            // key stays untranslated so tests and the Cage harness keep matching
            // the objectName; only the displayed label/tooltip translates.
            model: [
                { key: "Sleep", label: qsTr("Sleep"), icon: "icons/sleep.svg", action: 0 },
                { key: "Restart", label: qsTr("Restart"), icon: "icons/restart.svg", action: 1 },
                { key: "Shut Down", label: qsTr("Shut Down"), icon: "icons/power.svg", action: 2 }
            ]
            delegate: Button {
                id: power
                required property var modelData
                objectName: modelData.key
                Layout.preferredWidth: theme.powerSize
                Layout.preferredHeight: theme.powerSize
                padding: 10
                text: modelData.label
                enabled: root.interactive && (root.backend.capabilities & (1 << modelData.action)) !== 0
                Accessible.name: text + (root.backend.preview ? qsTr(" (preview only)") : "")
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
                    radius: theme.powerRadius
                    color: power.hovered ? theme.fillSubtle : "transparent"
                    border.color: power.activeFocus ? theme.textSecondary : "transparent"
                }
            }
        }
    }
}
