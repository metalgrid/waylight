import QtQuick
import Waylight

Main {
    id: app
    backend: Backend { id: controller }
    Timer { interval: 16; running: true; repeat: true; onTriggered: controller.poll() }
    Connections {
        target: controller
        function onFinished() { app.closingAllowed = true; Qt.quit(); }
    }
}
