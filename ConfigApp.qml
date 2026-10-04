pragma ComponentBehavior: Bound

import QtQuick
import Waylight

// Entry point of the config GUI. Thin on purpose: all behavior lives in
// ConfigMain so tests can instantiate the editor with a mocked backend.
// The ConfigBackend bridge talks to waylight-configd on the system bus and
// is drained by the poll Timer, mirroring the greeter's App.qml.
ConfigMain {
    id: app
    backend: ConfigBackend { id: daemon }
    Timer { interval: 16; running: true; repeat: true; onTriggered: daemon.poll() }
}
