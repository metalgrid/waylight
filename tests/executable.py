#!/usr/bin/env python3
"""Isolated fake greetd + headless Cage. Never executes received session commands.
Private Wayland and accessibility buses; no host input or real credentials.
"""
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time


def inside(root, executable, keyboard):
    # dbus-run-session supplies a NEW bus. Never inspect the host accessibility tree.
    os.environ["AT_SPI_BUS_ADDRESS"] = os.environ["DBUS_SESSION_BUS_ADDRESS"]
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi, GLib

    root = Path(root)
    registry = subprocess.Popen(["/usr/lib/at-spi2-registryd"])
    app = subprocess.Popen([executable, "--state-dir", os.environ["WAYLIGHT_TEST_STATE"]], cwd="/")
    keys = None
    try:
        keys = subprocess.Popen([keyboard], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        assert keys.stdout.readline().strip() == "ready", "private keyboard failed to initialize"

        def key(code):
            keys.stdin.write(str(code) + "\n")
            keys.stdin.flush()
            assert keys.stdout.readline().strip() == "sent", f"private keyboard failed for evdev {code}"

        def nodes(node):
            node.clear_cache()
            yield node
            for index in range(node.get_child_count()):
                yield from nodes(node.get_child_at_index(index))

        def ready(name, role, prompt=None, text=None):
            # A Wayland roundtrip only acknowledges the compositor, not Qt input
            # processing. Observe the real UI via its standard accessibility API.
            end = time.monotonic() + 5
            observed = []
            active = False
            while time.monotonic() < end:
                assert app.poll() is None, f"greeter exited while waiting for {name!r}"
                try:
                    tree = list(nodes(Atspi.get_desktop(0)))
                    observed = []
                    prompt_ready = prompt is None
                    target_ready = False
                    active = False
                    for node in tree:
                        states = node.get_state_set()
                        if not states.contains(Atspi.StateType.SHOWING):
                            continue
                        label = node.get_name()
                        if node.get_role() == Atspi.Role.FRAME and states.contains(Atspi.StateType.ACTIVE):
                            active = True
                        if node.get_role() == Atspi.Role.LABEL and label == prompt:
                            prompt_ready = True
                        if states.contains(Atspi.StateType.FOCUSED):
                            observed.append((label, node.get_role_name()))
                        if ((name is None or label == name) and node.get_role() == role
                                and states.contains(Atspi.StateType.FOCUSED)
                                and states.contains(Atspi.StateType.ENABLED)):
                            # Only read the synthetic username, never a response.
                            target_ready = text is None or Atspi.Text.get_text(node, 0, -1) == text
                    if active and prompt_ready and target_ready:
                        print(f"READY {name!r}, prompt={prompt!r}, synthetic username={text!r}", flush=True)
                        return
                except GLib.Error as error:
                    observed = [str(error)]
                time.sleep(0.02)
            raise AssertionError(f"UI not ready: name={name!r}, prompt={prompt!r}, window_active={active}, focused={observed!r}")

        ready("Username", Atspi.Role.TEXT, text="")
        # Synthetic username only. All prompt responses are deliberately empty.
        for code in [31, 30, 50, 25, 38, 18]:  # sample (evdev US)
            key(code)
        ready("Username", Atspi.Role.TEXT, text="sample")
        key(28)  # One submit, never retry authentication or StartSession.
        previous = ""
        end = time.monotonic() + 15
        while app.poll() is None and time.monotonic() < end:
            marker = root / "step"
            step = marker.read_text() if marker.exists() else ""
            failure = root / "failure"
            assert not failure.exists(), failure.read_text() if failure.exists() else ""
            if step and step != previous:
                previous = step
                if step == "shutdown":
                    app.terminate()
                else:
                    kind = "secret" if step == "shutdown-prompt" else step
                    prompt = f"Synthetic {kind} <b>plain text</b>"
                    if kind in ("secret", "visible"):
                        role = Atspi.Role.PASSWORD_TEXT if kind == "secret" else Atspi.Role.TEXT
                        # Qt redacts the accessible name of password fields.
                        ready(None if kind == "secret" else prompt, role, prompt=prompt)
                    else:
                        ready("Continue", Atspi.Role.PUSH_BUTTON, prompt=prompt)
                    if step == "shutdown-prompt":
                        app.terminate()
                    else:
                        key(28)
            time.sleep(0.02)
        assert app.poll() is not None, f"greeter did not exit; last fake-server step={previous!r}"
        (root / "app-exit").write_text(str(app.wait()))
        return 0  # Cage's clean exit is checked separately from greeter outcome.
    finally:
        if app.poll() is None:
            app.kill()
            app.wait()
        if keys:
            if keys.poll() is None:
                keys.stdin.close()
            keys.wait(timeout=3)
        registry.terminate()
        registry.wait(timeout=3)


def frame(connection):
    def exact(size):
        result = b""
        while len(result) < size:
            chunk = connection.recv(size - len(result))
            assert chunk, "unexpected greeter EOF"
            result += chunk
        return result
    size = struct.unpack("=I", exact(4))[0]
    assert 0 < size <= 65536, f"invalid fake-greetd request frame size: {size}"
    return json.loads(exact(size))


def send(connection, value):
    body = json.dumps(value).encode()
    data = struct.pack("=I", len(body)) + body
    # Exercise partial frame/header handling through the executable too.
    for fragment in [data[:2], data[2:7], data[7:]]:
        connection.sendall(fragment)
        time.sleep(0.01)


def main():
    project = Path(__file__).resolve().parent.parent
    executable = project / "target/debug/waylight-greeter"
    assert executable.exists(), "run cargo build --locked first"
    evidence = Path(os.environ.get("WAYLIGHT_EVIDENCE", "/tmp/waylight-checks"))
    evidence.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="waylight-test-") as temporary:
        root = Path(temporary)
        copied = root / "greeter"
        shutil.copy2(executable, copied)
        keyboard = root / "keyboard"
        flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "wayland-client", "xkbcommon"], text=True).split()
        subprocess.run(["cc", "-Wall", "-Wextra", "-Werror", str(project / "tests/keyboard.c"), "-o", str(keyboard), *flags], check=True)
        env = os.environ.copy()
        env.update(QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software", DBUS_SYSTEM_BUS_ADDRESS=f"unix:path={root}/no-system-bus", HOME=str(root), XDG_CACHE_HOME=str(root / "cache"), XDG_CONFIG_HOME=str(root / "config"))
        env.pop("GREETD_SOCK", None)
        for args in [[], ["--unknown"], ["--help", "--unknown"], ["--preview", "--preview"], ["--preview", "--state-dir", str(root)]]:
            result = subprocess.run([str(copied), *args], env=env, cwd="/", capture_output=True, timeout=5)
            assert result.returncode == 2
        bad_qml = subprocess.run([str(copied), "--preview"], env=env | {"QT_QUICK_CONTROLS_STYLE": "WaylightMissingTestStyle"}, cwd="/", capture_output=True, timeout=5)
        assert bad_qml.returncode == 1, "QML load failure did not fail closed"
        listener = socket.socket(socket.AF_UNIX)
        listener.bind(str(root / "preview.sock"))
        listener.listen()
        listener.settimeout(0.1)
        env["GREETD_SOCK"] = str(root / "preview.sock")
        bus = socket.socket(socket.AF_UNIX)
        bus.bind(str(root / "no-system-bus"))
        bus.listen()
        bus.settimeout(0.1)
        preview = subprocess.Popen([str(copied), "--preview"], env=env, cwd="/", stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        time.sleep(0.5)
        try:
            listener.accept()
            raise AssertionError("preview connected to greetd")
        except socket.timeout:
            pass
        try:
            bus.accept()
            raise AssertionError("preview connected to the system bus")
        except socket.timeout:
            pass
        bus.close()
        (root / "no-system-bus").unlink()
        preview.terminate()
        stdout, stderr = preview.communicate(timeout=5)
        assert preview.returncode == 1 and not stderr, stderr.decode()
        assert not list(root.rglob("qmlcache")), "preview wrote a QML disk cache"
        assert not list(root.rglob("session")), "preview wrote session state"
        # Fontconfig may maintain its ordinary font cache, not greeter state.
        listener.close()
        print("PASS CLI/QML-load failures, embedded offscreen preview, no greetd/system-bus connection/session state/QML cache")

        # Headless Cage cannot inherit host key events, modifiers, or focus loss.
        # Both the input helper and AT-SPI observer see only this private case.
        for mode in ["ack", "nonfatal-state", "lost-start-reply", "shutdown-prompt"]:
            case = root / mode
            case.mkdir()
            runtime = case / "runtime"
            runtime.mkdir(mode=0o700)
            state = case / "state"
            if mode != "nonfatal-state":
                state.mkdir(mode=0o700)
            if mode == "ack":
                (state / "session").write_text("weston.desktop")
                (state / "session").chmod(0o600)
            server = socket.socket(socket.AF_UNIX)
            server.bind(str(case / "greetd.sock"))
            server.listen()
            server.settimeout(15)
            errors = []
            seen = []
            def daemon():
                stage = "accept"
                try:
                    connection, _ = server.accept()
                    with connection:
                        connection.settimeout(10)
                        stage = "create_session"
                        request = frame(connection)
                        assert request == {"type": "create_session", "username": "sample"}, f"{mode}: expected synthetic create_session(sample), got {request!r}"
                        seen.append(request["type"])
                        for kind in ["secret", "visible", "info", "error"]:
                            stage = f"{kind} response"
                            send(connection, {"type": "auth_message", "auth_message_type": kind, "auth_message": f"Synthetic {kind} <b>plain text</b>"})
                            (case / "step").write_text("shutdown-prompt" if mode == "shutdown-prompt" else kind)
                            request = frame(connection)
                            seen.append(request["type"])
                            if mode == "shutdown-prompt":
                                assert request == {"type": "cancel_session"}, f"{mode}: expected cancel_session, got {request!r}"
                                send(connection, {"type": "success"})
                                return
                            assert request == {"type": "post_auth_message_response", "response": "" if kind in ("secret", "visible") else None}, f"{mode}: unexpected synthetic {kind} reply: {request!r}"
                        stage = "start_session"
                        send(connection, {"type": "success"})
                        request = frame(connection)
                        seen.append(request["type"])
                        assert request["type"] == "start_session", f"{mode}: expected start_session, got {request!r}"
                        assert len(request["cmd"]) == 1 and request["cmd"][0].startswith("'/usr/"), f"{mode}: invalid trusted session command: {request!r}"
                        assert request["env"] and all(value.startswith(("XDG_SESSION_", "XDG_CURRENT_DESKTOP=")) for value in request["env"]), f"{mode}: invalid session environment: {request!r}"
                        if mode == "ack":
                            assert request["cmd"] == ["'/usr/bin/weston'"], f"{mode}: stored Weston session not selected: {request!r}"
                            assert request["env"] == ["XDG_SESSION_TYPE=wayland", "XDG_SESSION_DESKTOP=weston"], f"{mode}: incorrect Weston environment: {request!r}"
                        if mode == "lost-start-reply":
                            (case / "step").write_text("shutdown")
                            return
                        send(connection, {"type": "success"})
                except BaseException as error:
                    detail = f"{mode}/{stage}: {type(error).__name__}: {error}; request types so far={seen!r}"
                    errors.append(detail)
                    (case / "failure").write_text(detail)
            thread = threading.Thread(target=daemon)
            thread.start()
            # QML_XHR_ALLOW_FILE_READ is deliberately NOT set: without it the
            # greeter's theme XHR fails and the defaults apply, so headless
            # Cage runs stay deterministic and cannot couple to a developer's
            # ~/.config or /etc/waylight/theme.json.
            child_env = env | {
                "QT_QPA_PLATFORM": "wayland", "WLR_BACKENDS": "headless", "WLR_RENDERER": "pixman",
                "QT_LINUX_ACCESSIBILITY_ALWAYS_ON": "1", "XDG_CURRENT_DESKTOP": "",
                "XDG_RUNTIME_DIR": str(runtime),
                "WAYLIGHT_TEST_RUNTIME": str(runtime), "WAYLIGHT_TEST_STATE": str(state),
                "GREETD_SOCK": str(case / "greetd.sock"),
            }
            with (evidence / f"cage-{mode}.txt").open("w") as log:
                result = subprocess.run(["dbus-run-session", "--", "cage", "--", sys.executable, str(Path(__file__).resolve()), "--inside", str(case), str(copied), str(keyboard)], env=child_env, cwd="/", stdout=log, stderr=log, timeout=35)
            thread.join(timeout=2)
            server.close()
            assert not thread.is_alive(), f"{mode}: fake server did not finish; request types={seen!r}"
            assert not errors, "\n".join(errors)
            assert result.returncode == 0, f"Cage failed in {mode}; see evidence"
            expected = 0 if mode in ("ack", "nonfatal-state") else 1
            assert (case / "app-exit").read_text() == str(expected), f"{mode}: expected greeter exit={expected}; see cage-{mode}.txt"
            if mode == "ack":
                assert (state / "session").read_text() == "weston.desktop"
            else:
                assert not (state / "session").exists()
            print(f"PASS headless Cage {mode}: greeter={expected}, Cage=0, requests={seen}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--inside":
        sys.exit(inside(*sys.argv[2:]))
    main()
