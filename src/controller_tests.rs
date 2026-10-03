use super::*;
use greetd_ipc::ErrorType;

async fn receive(s: &mut UnixStream) -> Request {
    let mut h = [0; 4];
    tokio::time::timeout(Duration::from_secs(3), s.read_exact(&mut h))
        .await
        .unwrap()
        .unwrap();
    let mut b = vec![0; u32::from_ne_bytes(h) as usize];
    tokio::time::timeout(Duration::from_secs(3), s.read_exact(&mut b))
        .await
        .unwrap()
        .unwrap();
    serde_json::from_slice(&b).unwrap()
}
async fn reply(s: &mut UnixStream, response: Response) {
    let b = serde_json::to_vec(&response).unwrap();
    s.write_all(&(b.len() as u32).to_ne_bytes()).await.unwrap();
    s.write_all(&b).await.unwrap();
}
async fn prompt_token(events: &std_mpsc::Receiver<Event>) -> u32 {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(Event::View { token, .. }) = events.try_recv()
                && token > 0
            {
                return token;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("prompt deadline")
}
fn prompt(kind: AuthMessageType) -> Response {
    Response::AuthMessage {
        auth_message_type: kind,
        auth_message: "arbitrary <b>prompt</b>".into(),
    }
}
fn session() -> Session {
    Session {
        id: "test.desktop".into(),
        name: "Test".into(),
        command: "'/usr/bin/true'".into(),
        env: vec!["XDG_SESSION_TYPE=wayland".into()],
    }
}
fn error() -> Response {
    Response::Error {
        error_type: ErrorType::AuthError,
        description: "test failure".into(),
    }
}
// Construct the actual CXX-Qt backend: these checks exercise its production
// request/poll/dispatch methods, not a second implementation of the pending slot.
fn backend_fixture() -> (
    cxx::UniquePtr<crate::backend::ffi::Backend>,
    mpsc::Receiver<Command>,
    std_mpsc::Sender<Event>,
) {
    let (commands, rx) = mpsc::channel(16);
    let (tx, events) = std_mpsc::channel();
    *crate::backend::BRIDGE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap() = Some(crate::backend::Bridge {
        commands,
        events,
        preview: true,
    });
    let mut backend = crate::backend::ffi::backend_make_unique();
    tx.send(Event::Sessions {
        names: vec!["One".into(), "Two".into()],
        selected: 0,
    })
    .unwrap();
    tx.send(Event::Capabilities(7)).unwrap();
    view(&tx, "idle", "", "", 0);
    backend.pin_mut().poll();
    (backend, rx, tx)
}

#[tokio::test]
async fn production_backend_intentions_and_socket_cleanup_barriers() {
    // One sequential test owns the global constructor bridge; no host bus or
    // power implementation is reachable. Capture the real dispatched commands.
    for state in [
        "waiting",
        "secret",
        "visible",
        "info",
        "error",
        "cancelling",
    ] {
        let (mut backend, mut rx, tx) = backend_fixture();
        let identities = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let received = identities.clone();
        let _identity = backend.pin_mut().on_identity_chosen(move |_, name| {
            received.lock().unwrap().push(name.to_string());
        });
        view(&tx, state, "", "prompt", 1);
        backend.pin_mut().poll();
        backend.pin_mut().choose_user("first".into(), 0);
        backend.pin_mut().choose_manual();
        backend.pin_mut().power(1);
        backend.pin_mut().choose_user("latest".into(), 1);
        backend.pin_mut().choose_user("bad\0name".into(), 0);
        backend.pin_mut().choose_user("invalid-session".into(), 5);
        backend.pin_mut().power(8);
        // A controller-originated cancelling view already represents cleanup.
        if state != "cancelling" {
            assert!(matches!(rx.try_recv(), Ok(Command::Cancel)));
        }
        assert!(rx.try_recv().is_err());
        assert!(identities.lock().unwrap().is_empty());
        view(&tx, "cancelling", "generic controller message", "", 0);
        backend.pin_mut().poll();
        assert!(backend.message().to_string().contains("latest requested"));
        let states = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = states.clone();
        let _state = backend.pin_mut().on_state_changed(move |backend| {
            observed.lock().unwrap().push(backend.state().to_string());
        });
        view(&tx, "idle", "cleanup done", "", 0);
        backend.pin_mut().poll();
        assert_eq!(*states.lock().unwrap(), ["waiting"]); // No interactive idle flash.
        assert_eq!(backend.state().to_string(), "waiting");
        assert!(
            matches!(rx.try_recv(), Ok(Command::Begin { username, session: 1 }) if username == "latest")
        );
        assert_eq!(*identities.lock().unwrap(), ["latest"]);
        assert!(rx.try_recv().is_err());
    }
    for stop in [
        "cancel",
        "shutdown",
        "starting",
        "handoff",
        "disconnected",
        "exit",
        "channel-loss",
        "send-loss",
        "revoked",
    ] {
        let (mut backend, mut rx, tx) = backend_fixture();
        view(&tx, "secret", "", "prompt", 1);
        backend.pin_mut().poll();
        backend.pin_mut().power(2);
        assert!(matches!(rx.try_recv(), Ok(Command::Cancel)));
        match stop {
            "cancel" => backend.pin_mut().cancel(),
            "shutdown" => {
                backend.pin_mut().shutdown();
                assert!(*backend.closing());
                assert!(matches!(rx.try_recv(), Ok(Command::Shutdown)));
            }
            "exit" => {
                tx.send(Event::Exit).unwrap();
            }
            "channel-loss" => {
                drop(tx);
                backend.pin_mut().poll();
                assert!(*backend.closing());
                assert!(rx.try_recv().is_err());
                continue;
            }
            "send-loss" => {
                rx.close();
            }
            "revoked" => {
                tx.send(Event::Capabilities(0)).unwrap();
            }
            state => view(&tx, state, "", "", 0),
        }
        backend.pin_mut().poll();
        view(&tx, "idle", "cleanup done or start rejected", "", 0);
        backend.pin_mut().poll();
        assert!(rx.try_recv().is_err(), "released an intention after {stop}");
        if stop == "send-loss" {
            assert_eq!(backend.state().to_string(), "disconnected");
        }
    }
    // Idle/manual execution and revalidation do not manufacture authentication.
    let (mut backend, mut rx, tx) = backend_fixture();
    backend.pin_mut().choose_manual();
    assert_eq!(backend.state().to_string(), "idle");
    assert!(rx.try_recv().is_err());
    for state in ["loading", "power", "starting", "handoff", "disconnected"] {
        view(&tx, state, "", "", 0);
        backend.pin_mut().poll();
        backend.pin_mut().choose_user("blocked".into(), 0);
        backend.pin_mut().choose_manual();
        backend.pin_mut().power(0);
        assert!(rx.try_recv().is_err());
    }

    for mode in [
        "switch-success",
        "switch-error",
        "power",
        "manual",
        "eof",
        "timeout",
        "start-rejected",
    ] {
        let (mut backend, mut rx, tx) = backend_fixture();
        let (mut client, mut server) = UnixStream::pair().unwrap();
        backend.pin_mut().choose_user("original".into(), 0);
        let Command::Begin { username, .. } = rx.recv().await.unwrap() else {
            panic!("missing begin")
        };
        let actor = async {
            let result = attempt(
                &mut client,
                username,
                &session(),
                &mut rx,
                &tx,
                &mut 0,
                Duration::from_millis(150),
            )
            .await;
            match result {
                Attempt::Idle(message) => view(&tx, "idle", &message, "", 0),
                Attempt::Disconnected(message) => view(&tx, "disconnected", &message, "", 0),
                _ => panic!("unexpected attempt outcome"),
            }
            // Return the live socket/receiver to inspect what the actual backend
            // releases after polling the real controller's cleanup result.
        };
        let ui_and_daemon = async {
            assert!(
                matches!(receive(&mut server).await, Request::CreateSession { username } if username == "original")
            );
            if mode == "start-rejected" {
                reply(&mut server, Response::Success).await;
                assert!(matches!(
                    receive(&mut server).await,
                    Request::StartSession { .. }
                ));
                // UI has not polled the starting event yet: request loses the race.
                backend.pin_mut().choose_user("must-not-replay".into(), 1);
                backend.pin_mut().poll();
                assert_eq!(backend.state().to_string(), "starting");
                reply(&mut server, error()).await;
            } else {
                // Withhold even the create reply: cancellation cannot skip it.
                if mode == "power" {
                    backend.pin_mut().power(0);
                } else if mode == "manual" {
                    backend.pin_mut().choose_manual();
                } else {
                    backend.pin_mut().choose_user("replacement".into(), 1);
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
                let mut byte = [0];
                assert_eq!(
                    server.try_read(&mut byte).unwrap_err().kind(),
                    std::io::ErrorKind::WouldBlock
                );
                reply(&mut server, prompt(AuthMessageType::Secret)).await;
            }
            assert!(matches!(receive(&mut server).await, Request::CancelSession));
            backend.pin_mut().poll();
            assert_eq!(backend.state().to_string(), "cancelling");
            // There is no new create/power dispatch before cancellation reply.
            tokio::time::sleep(Duration::from_millis(5)).await;
            let mut byte = [0];
            assert_eq!(
                server.try_read(&mut byte).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            if mode == "eof" {
                server.shutdown().await.unwrap();
            } else if mode == "timeout" {
                tokio::time::sleep(Duration::from_millis(200)).await;
            } else {
                reply(
                    &mut server,
                    if mode == "switch-error" {
                        error()
                    } else {
                        Response::Success
                    },
                )
                .await;
            }
        };
        tokio::join!(actor, ui_and_daemon);
        assert!(
            rx.try_recv().is_err(),
            "action sent before backend observed cleanup"
        );
        backend.pin_mut().poll();
        match mode {
            "switch-success" | "switch-error" => {
                let Command::Begin {
                    username,
                    session: chosen,
                } = rx.try_recv().unwrap()
                else {
                    panic!("missing replacement")
                };
                assert_eq!(username, "replacement");
                assert_eq!(chosen, 1);
                let selected = session();
                let mut token = 1;
                let next = attempt(
                    &mut client,
                    username,
                    &selected,
                    &mut rx,
                    &tx,
                    &mut token,
                    Duration::from_secs(1),
                );
                let daemon = async {
                    assert!(
                        matches!(receive(&mut server).await, Request::CreateSession { username } if username == "replacement")
                    );
                    reply(&mut server, error()).await;
                    assert!(matches!(receive(&mut server).await, Request::CancelSession));
                    reply(&mut server, Response::Success).await;
                };
                let (result, _) = tokio::join!(next, daemon);
                assert!(matches!(result, Attempt::Idle(_)));
            }
            "power" => {
                assert!(matches!(rx.try_recv(), Ok(Command::Power(0))));
            }
            "manual" | "start-rejected" => {
                assert_eq!(backend.state().to_string(), "idle");
                assert!(rx.try_recv().is_err());
            }
            _ => {
                assert_eq!(backend.state().to_string(), "disconnected");
                assert!(rx.try_recv().is_err());
            }
        }
    }
}

#[tokio::test]
async fn preview_uses_synthetic_accounts_and_password_prompt() {
    let (commands, rx) = mpsc::channel(16);
    let (tx, events) = std_mpsc::channel();
    commands
        .send(Command::Begin {
            username: "demo-alex".into(),
            session: 0,
        })
        .await
        .unwrap();
    commands.send(Command::Shutdown).await.unwrap();
    assert!(
        !run(
            Config {
                preview: true,
                socket: "/unused".into(),
                state: None
            },
            rx,
            tx
        )
        .await
    );
    let events: Vec<_> = events.try_iter().collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Accounts(users) if *users == accounts::preview()))
    );
    assert!(events.iter().any(
        |event| matches!(event, Event::View { state: "secret", prompt, .. } if prompt == "Password")
    ));
}

#[tokio::test]
async fn prompts_empty_answers_stale_tokens_and_acknowledged_launch() {
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let (commands, mut rx) = mpsc::channel(16);
    let (tx, events) = std_mpsc::channel();
    let daemon = async {
        assert!(
            matches!(receive(&mut server).await, Request::CreateSession { username } if username == "sample")
        );
        for (i, kind) in [
            AuthMessageType::Secret,
            AuthMessageType::Visible,
            AuthMessageType::Info,
            AuthMessageType::Error,
        ]
        .into_iter()
        .enumerate()
        {
            reply(&mut server, prompt(kind)).await;
            let token = prompt_token(&events).await;
            commands
                .send(Command::Answer {
                    token: token + 100,
                    text: "stale".into(),
                })
                .await
                .unwrap();
            if i == 0 {
                commands
                    .send(Command::Answer {
                        token,
                        text: "\u{1}".repeat(2000),
                    })
                    .await
                    .unwrap();
                assert_eq!(prompt_token(&events).await, token); // Encoded frame rejected without abandoning PAM.
            }
            commands
                .send(Command::Answer {
                    token,
                    text: String::new(),
                })
                .await
                .unwrap();
            match receive(&mut server).await {
                Request::PostAuthMessageResponse { response } => {
                    assert_eq!(response, if i < 2 { Some(String::new()) } else { None })
                }
                _ => panic!("wrong request"),
            }
        }
        reply(&mut server, Response::Success).await;
        assert!(
            matches!(receive(&mut server).await, Request::StartSession { cmd, env } if cmd == ["'/usr/bin/true'"] && env == ["XDG_SESSION_TYPE=wayland"])
        );
        reply(&mut server, Response::Success).await;
    };
    let mut token = 0;
    let s = session();
    let (result, _) = tokio::join!(
        attempt(
            &mut client,
            "sample".into(),
            &s,
            &mut rx,
            &tx,
            &mut token,
            Duration::from_secs(2)
        ),
        daemon
    );
    assert!(matches!(result, Attempt::Launched));
}
#[test]
fn request_limit_bounds_greetd_internal_datagrams() {
    // Exact 0.10.3 default externally-tagged worker enum wire shapes.
    for response in [None, Some(String::new()), Some("\"\\\n\u{1}é".into())] {
        let internal =
            serde_json::to_vec(&serde_json::json!({"PamResponse": {"resp": response}})).unwrap();
        let public = encode_request(&Request::PostAuthMessageResponse { response }).unwrap();
        assert_eq!(public.len(), internal.len() + 24);
    }
    let cmd = vec!["'/usr/bin/true'".to_owned()];
    let env = vec!["XDG_CURRENT_DESKTOP=Test".to_owned()];
    let internal =
        serde_json::to_vec(&serde_json::json!({"Args": {"env": env, "cmd": cmd}})).unwrap();
    let public = encode_request(&Request::StartSession { cmd, env }).unwrap();
    assert_eq!(public.len(), internal.len() + 14);
    let boundary = Request::PostAuthMessageResponse {
        response: Some("a".repeat(MAX_REQUEST - 51)),
    };
    assert_eq!(encode_request(&boundary).unwrap().len(), MAX_REQUEST);
    assert!(
        encode_request(&Request::PostAuthMessageResponse {
            response: Some("a".repeat(MAX_REQUEST - 50))
        })
        .is_err()
    );
}

#[tokio::test]
async fn invalid_answers_keep_prompt_and_send_only_corrected_response() {
    for kind in [AuthMessageType::Secret, AuthMessageType::Visible] {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let (commands, mut rx) = mpsc::channel(16);
        let (tx, events) = std_mpsc::channel();
        let state = if matches!(kind, AuthMessageType::Secret) {
            "secret"
        } else {
            "visible"
        };
        let daemon = async {
            assert!(matches!(
                receive(&mut server).await,
                Request::CreateSession { .. }
            ));
            reply(&mut server, prompt(kind)).await;
            let token = prompt_token(&events).await;
            for text in [
                "a".repeat(MAX_REQUEST),
                "\u{1}".repeat(2000),
                "sample\0suffix".into(),
            ] {
                commands
                    .send(Command::Answer { token, text })
                    .await
                    .unwrap();
                tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        if let Ok(Event::View {
                            state: actual,
                            message,
                            prompt,
                            token: actual_token,
                        }) = events.try_recv()
                        {
                            assert_eq!(actual, state);
                            assert_eq!(actual_token, token);
                            assert_eq!(prompt, "arbitrary <b>prompt</b>");
                            assert!(message.contains("Nothing was sent."));
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                let mut byte = [0];
                assert_eq!(
                    server.try_read(&mut byte).unwrap_err().kind(),
                    std::io::ErrorKind::WouldBlock
                );
            }
            commands
                .send(Command::Answer {
                    token,
                    text: "corrected".into(),
                })
                .await
                .unwrap();
            assert!(
                matches!(receive(&mut server).await, Request::PostAuthMessageResponse { response: Some(text) } if text == "corrected")
            );
            reply(&mut server, Response::Success).await;
            assert!(matches!(
                receive(&mut server).await,
                Request::StartSession { .. }
            ));
            reply(&mut server, Response::Success).await;
        };
        let mut token = 0;
        let s = session();
        let (result, _) = tokio::join!(
            attempt(
                &mut client,
                "sample".into(),
                &s,
                &mut rx,
                &tx,
                &mut token,
                Duration::from_secs(2)
            ),
            daemon
        );
        assert!(matches!(result, Attempt::Launched));
    }
}

#[tokio::test]
async fn invalid_session_is_rejected_before_authentication_or_start_write() {
    let mut bad_sessions = Vec::new();
    for command in [
        "a".repeat(MAX_REQUEST),
        "sample\0suffix".into(),
        String::new(),
    ] {
        bad_sessions.push(Session {
            command,
            ..session()
        });
    }
    for env in [
        vec![format!(
            "XDG_CURRENT_DESKTOP={}",
            "\"".repeat(MAX_REQUEST / 2)
        )],
        vec!["KEY=sample\0suffix".into()],
        vec!["=empty-key".into()],
    ] {
        bad_sessions.push(Session { env, ..session() });
    }
    for s in bad_sessions {
        let (mut client, server) = UnixStream::pair().unwrap();
        let (_commands, mut rx) = mpsc::channel(16);
        let (tx, events) = std_mpsc::channel();
        let mut token = 0;
        let result = attempt(
            &mut client,
            "sample".into(),
            &s,
            &mut rx,
            &tx,
            &mut token,
            Duration::from_secs(2),
        )
        .await;
        assert!(
            matches!(result, Attempt::Idle(message) if message.starts_with("Cannot use selected session:"))
        );
        assert_eq!(token, 0);
        assert!(events.try_recv().is_err()); // Never announces the irreversible start boundary.
        let mut byte = [0];
        assert_eq!(
            server.try_read(&mut byte).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[tokio::test]
async fn cancel_preserves_fragmented_reply_then_cleanup_barrier_and_retry() {
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let (commands, mut rx) = mpsc::channel(16);
    let (tx, _) = std_mpsc::channel();
    let daemon = async {
        receive(&mut server).await;
        let b = serde_json::to_vec(&Response::Success).unwrap();
        let h = (b.len() as u32).to_ne_bytes();
        server.write_all(&h[..2]).await.unwrap();
        commands.send(Command::Cancel).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.write_all(&h[2..]).await.unwrap();
        server.write_all(&b[..3]).await.unwrap();
        tokio::task::yield_now().await;
        server.write_all(&b[3..]).await.unwrap();
        assert!(matches!(receive(&mut server).await, Request::CancelSession));
        reply(&mut server, error()).await; // Complete Error is a valid cleanup boundary.
        assert!(matches!(
            receive(&mut server).await,
            Request::CreateSession { .. }
        ));
        reply(&mut server, error()).await;
        assert!(matches!(receive(&mut server).await, Request::CancelSession));
        reply(&mut server, Response::Success).await;
    };
    let actor = async {
        let mut token = 0;
        for _ in 0..2 {
            assert!(matches!(
                attempt(
                    &mut client,
                    "sample".into(),
                    &session(),
                    &mut rx,
                    &tx,
                    &mut token,
                    Duration::from_secs(2)
                )
                .await,
                Attempt::Idle(_)
            ));
        }
    };
    tokio::join!(actor, daemon);
}
#[tokio::test]
async fn failed_cleanup_disables_retry_and_start_eof_is_indeterminate() {
    for start in [false, true] {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let (_commands, mut rx) = mpsc::channel(16);
        let (tx, _) = std_mpsc::channel();
        let daemon = async move {
            receive(&mut server).await;
            reply(&mut server, if start { Response::Success } else { error() }).await;
            let request = receive(&mut server).await;
            if start {
                assert!(matches!(request, Request::StartSession { .. }));
            } else {
                assert!(matches!(request, Request::CancelSession));
            }
            drop(server);
        };
        let mut token = 0;
        let s = session();
        let (result, _) = tokio::join!(
            attempt(
                &mut client,
                "sample".into(),
                &s,
                &mut rx,
                &tx,
                &mut token,
                Duration::from_secs(2)
            ),
            daemon
        );
        assert!(matches!(result, Attempt::Disconnected(_)));
    }
}
#[tokio::test]
async fn bounded_invalid_eof_and_timeout() {
    for bytes in [
        vec![],
        vec![0, 0, 0, 0],
        (MAX_FRAME as u32 + 1).to_ne_bytes().to_vec(),
        [2u32.to_ne_bytes().as_slice(), b"xx"].concat(),
        vec![1, 0],
    ] {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let daemon = async move {
            receive(&mut server).await;
            server.write_all(&bytes).await.unwrap();
        };
        let (result, _) = tokio::join!(wire(&mut client, Request::CancelSession), daemon);
        assert!(result.is_err());
    }
    let (mut client, server) = UnixStream::pair().unwrap();
    drop(server);
    assert!(wire(&mut client, Request::CancelSession).await.is_err());
    let (mut client, _server) = UnixStream::pair().unwrap();
    let (commands, mut rx) = mpsc::channel(16);
    let (tx, _) = std_mpsc::channel();
    commands.send(Command::Cancel).await.unwrap();
    let mut latch = Latch::default();
    assert!(
        exchange(
            &mut client,
            Request::CancelSession,
            &mut rx,
            &tx,
            &mut latch,
            Duration::from_millis(15),
            false
        )
        .await
        .is_err()
    );
    assert!(latch.cancel);
}
#[tokio::test]
async fn latched_shutdown_with_lost_start_reply_exits_unsuccessfully() {
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let (commands, mut rx) = mpsc::channel(16);
    let (tx, _) = std_mpsc::channel();
    let daemon = async {
        receive(&mut server).await;
        reply(&mut server, Response::Success).await;
        assert!(matches!(
            receive(&mut server).await,
            Request::StartSession { .. }
        ));
        commands.send(Command::Shutdown).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while commands.capacity() != 16 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        drop(server);
    };
    let mut token = 0;
    let s = session();
    let (result, _) = tokio::join!(
        attempt(
            &mut client,
            "sample".into(),
            &s,
            &mut rx,
            &tx,
            &mut token,
            Duration::from_secs(2)
        ),
        daemon
    );
    assert!(matches!(result, Attempt::Shutdown));
}

#[tokio::test]
async fn shutdown_cleans_pending_auth_and_cannot_cancel_started_session() {
    for starting in [false, true] {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let (commands, mut rx) = mpsc::channel(16);
        let (tx, events) = std_mpsc::channel();
        let daemon = async {
            receive(&mut server).await;
            reply(
                &mut server,
                if starting {
                    Response::Success
                } else {
                    prompt(AuthMessageType::Secret)
                },
            )
            .await;
            if starting {
                assert!(matches!(
                    receive(&mut server).await,
                    Request::StartSession { .. }
                ));
            } else {
                prompt_token(&events).await;
            }
            commands.send(Command::Shutdown).await.unwrap();
            if !starting {
                assert!(matches!(receive(&mut server).await, Request::CancelSession));
            }
            reply(&mut server, Response::Success).await;
        };
        let mut token = 0;
        let s = session();
        let (result, _) = tokio::join!(
            attempt(
                &mut client,
                "sample".into(),
                &s,
                &mut rx,
                &tx,
                &mut token,
                Duration::from_secs(2)
            ),
            daemon
        );
        assert!(if starting {
            matches!(result, Attempt::Launched)
        } else {
            matches!(result, Attempt::Shutdown)
        });
    }
}
