use crate::{
    accounts, power,
    sessions::{self, Session},
    state,
};
use greetd_ipc::{AuthMessageType, Request, Response};
use std::{path::PathBuf, sync::mpsc as std_mpsc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::mpsc,
};

const MAX_FRAME: usize = 64 * 1024; // Incoming IPC replies, not outgoing requests.
// greetd 0.10.3's worker/conv receive internal JSON in 10240-byte datagrams.
// Public PostAuthMessageResponse JSON is 24 bytes larger than PamResponse;
// StartSession JSON is 14 bytes larger than Args (same strings/escaping).
// Bounding the public JSON therefore conservatively bounds both internal forms.
const MAX_REQUEST: usize = 10240;
#[derive(Clone)]
pub struct Config {
    pub preview: bool,
    pub socket: PathBuf,
    pub state: Option<PathBuf>,
}
pub enum Command {
    Begin { username: String, session: usize },
    Answer { token: u32, text: String },
    Cancel,
    Shutdown,
    Power(usize),
}
#[derive(Debug)]
pub enum Event {
    Sessions {
        names: Vec<String>,
        selected: usize,
    },
    Capabilities(u32),
    Accounts(Vec<accounts::Account>),
    View {
        state: &'static str,
        message: String,
        prompt: String,
        token: u32,
    },
    Exit,
}
#[derive(Default)]
struct Latch {
    cancel: bool,
    shutdown: bool,
}
impl Latch {
    fn command(&mut self, command: Option<Command>) {
        match command {
            Some(Command::Cancel) => self.cancel = true,
            Some(Command::Shutdown) | None => {
                self.cancel = true;
                self.shutdown = true;
            }
            _ => {} // No queued credentials, duplicate answers, or competing actions.
        }
    }
    fn drain(&mut self, rx: &mut mpsc::Receiver<Command>) {
        loop {
            match rx.try_recv() {
                Ok(command) => self.command(Some(command)),
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.command(None);
                    break;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
            }
        }
    }
}
fn view(
    tx: &std_mpsc::Sender<Event>,
    state: &'static str,
    message: &str,
    prompt: &str,
    token: u32,
) {
    let _ = tx.send(Event::View {
        state,
        message: message.into(),
        prompt: prompt.into(),
        token,
    });
}

fn encode_request(request: &Request) -> Result<Vec<u8>, &'static str> {
    match request {
        Request::PostAuthMessageResponse {
            response: Some(text),
        } if text.contains('\0') => {
            return Err("Answers cannot contain NUL characters. Nothing was sent.");
        }
        Request::StartSession { cmd, env }
            if cmd.is_empty()
                || cmd.iter().any(|s| s.is_empty() || s.contains('\0'))
                || env.iter().any(|s| {
                    s.contains('\0') || !s.split_once('=').is_some_and(|(key, _)| !key.is_empty())
                }) =>
        {
            return Err("Invalid session command or environment. Nothing was sent.");
        }
        _ => {}
    }
    let bytes =
        serde_json::to_vec(request).map_err(|_| "Cannot encode request. Nothing was sent.")?;
    if bytes.len() > MAX_REQUEST {
        return Err("Encoded request exceeds the 10 KiB daemon limit. Nothing was sent.");
    }
    Ok(bytes)
}

async fn wire(stream: &mut UnixStream, request: Request) -> Result<Response, &'static str> {
    // Deliberately do not use greetd_ipc's unbounded codec or log request data.
    let bytes = encode_request(&request)?;
    stream
        .write_all(&(bytes.len() as u32).to_ne_bytes())
        .await
        .map_err(|_| "Socket write failed.")?;
    stream
        .write_all(&bytes)
        .await
        .map_err(|_| "Socket write failed.")?;
    let mut header = [0; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|_| "Socket closed during reply.")?;
    let size = u32::from_ne_bytes(header) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err("Invalid reply size.");
    }
    let mut bytes = vec![0; size];
    stream
        .read_exact(&mut bytes)
        .await
        .map_err(|_| "Socket closed during reply.")?;
    serde_json::from_slice(&bytes).map_err(|_| "Invalid daemon reply.")
}

async fn exchange(
    stream: &mut UnixStream,
    request: Request,
    rx: &mut mpsc::Receiver<Command>,
    tx: &std_mpsc::Sender<Event>,
    latch: &mut Latch,
    timeout: Duration,
    starting: bool,
) -> Result<Response, &'static str> {
    // Pin ONCE. Cancellation must not restart a partial frame read/write.
    let exchange = tokio::time::timeout(timeout, wire(stream, request));
    tokio::pin!(exchange);
    loop {
        tokio::select! {
            result = &mut exchange => return result.map_err(|_| "Daemon reply timed out.")?,
            command = rx.recv(), if !rx.is_closed() => {
                latch.command(command);
                if latch.cancel && !starting { view(tx, "cancelling", "Cancelling after the pending daemon reply…", "", 0); }
            }
        }
    }
}

async fn cleanup(
    stream: &mut UnixStream,
    rx: &mut mpsc::Receiver<Command>,
    tx: &std_mpsc::Sender<Event>,
    latch: &mut Latch,
    timeout: Duration,
) -> bool {
    view(
        tx,
        "cancelling",
        "Waiting for authentication cleanup…",
        "",
        0,
    );
    // greetd 0.10.3 removes configuring before asking its worker to cancel.
    // Thus a COMPLETE Error reply is also a cleanup boundary, not a transport error.
    matches!(
        exchange(
            stream,
            Request::CancelSession,
            rx,
            tx,
            latch,
            timeout,
            false
        )
        .await,
        Ok(Response::Success | Response::Error { .. })
    )
}

enum Attempt {
    Idle(String),
    Disconnected(String),
    Launched,
    Shutdown,
}
async fn attempt(
    stream: &mut UnixStream,
    username: String,
    session: &Session,
    rx: &mut mpsc::Receiver<Command>,
    tx: &std_mpsc::Sender<Event>,
    next_token: &mut u32,
    timeout: Duration,
) -> Attempt {
    // Snapshot and validate before creating authentication state. No malformed
    // or oversized start can reach the irreversible boundary below.
    let start = Request::StartSession {
        cmd: vec![session.command.clone()],
        env: session.env.clone(),
    };
    if let Err(error) = encode_request(&start) {
        return Attempt::Idle(format!("Cannot use selected session: {error}"));
    }
    let mut latch = Latch::default();
    let mut request = Some(Request::CreateSession { username });
    let failure;
    loop {
        view(tx, "waiting", "Waiting for authentication…", "", 0);
        let response = exchange(
            stream,
            request.take().expect("next authentication request"),
            rx,
            tx,
            &mut latch,
            timeout,
            false,
        )
        .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return uncertain(
                    tx,
                    &latch,
                    format!(
                        "{error} Authentication cleanup is uncertain. Do not retry here; contact the administrator."
                    ),
                );
            }
        };
        latch.drain(rx);
        if latch.cancel {
            failure = "Authentication cancelled.".into();
            break;
        }
        match response {
            Response::Error { description, .. } => {
                failure = format!("Authentication failed: {description}");
                break;
            }
            Response::Success => {
                // Last cancellable boundary. From here StartSession is never
                // cancelled/replayed, even if the window closes or reply is lost.
                latch.drain(rx);
                if latch.cancel {
                    failure = "Authentication cancelled.".into();
                    break;
                }
                view(
                    tx,
                    "starting",
                    "Requesting session start. Cancellation is now disabled…",
                    "",
                    0,
                );
                let result = exchange(stream, start, rx, tx, &mut latch, timeout, true).await;
                match result {
                    Ok(Response::Success) => return Attempt::Launched,
                    Ok(Response::Error { description, .. }) => { failure = format!("Session start rejected: {description}"); break; }
                    _ => return uncertain(tx, &latch, "Session start outcome is unknown. It may already be scheduled. No retry or rollback was attempted; contact the administrator.".into()),
                }
            }
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => {
                *next_token = next_token.checked_add(1).expect("prompt token exhausted");
                let token = *next_token;
                let kind = match auth_message_type {
                    AuthMessageType::Visible => "visible",
                    AuthMessageType::Secret => "secret",
                    AuthMessageType::Info => "info",
                    AuthMessageType::Error => "error",
                };
                view(tx, kind, "", &auth_message, token);
                loop {
                    match rx.recv().await {
                        Some(Command::Answer {
                            token: answer_token,
                            text,
                        }) if answer_token == token => {
                            let answer = Request::PostAuthMessageResponse {
                                response: if matches!(
                                    auth_message_type,
                                    AuthMessageType::Visible | AuthMessageType::Secret
                                ) {
                                    Some(text)
                                } else {
                                    None
                                },
                            };
                            if let Err(error) = encode_request(&answer) {
                                view(tx, kind, error, &auth_message, token);
                                continue;
                            }
                            request = Some(answer);
                            break;
                        }
                        command => {
                            latch.command(command);
                            if latch.cancel {
                                break;
                            }
                        }
                    }
                }
                latch.drain(rx);
                if latch.cancel {
                    failure = "Authentication cancelled.".into();
                    break;
                }
            }
        }
    }
    if !cleanup(stream, rx, tx, &mut latch, timeout).await {
        return uncertain(tx, &latch, "Authentication cleanup could not be confirmed. Retry is disabled; contact the administrator.".into());
    }
    latch.drain(rx);
    if latch.shutdown {
        Attempt::Shutdown
    } else {
        Attempt::Idle(failure)
    }
}

fn uncertain(tx: &std_mpsc::Sender<Event>, latch: &Latch, message: String) -> Attempt {
    if latch.shutdown {
        eprintln!("{message}");
        view(tx, "disconnected", &message, "", 0);
        Attempt::Shutdown
    } else {
        Attempt::Disconnected(message)
    }
}

async fn disconnected(
    rx: &mut mpsc::Receiver<Command>,
    tx: &std_mpsc::Sender<Event>,
    message: &str,
) -> bool {
    view(tx, "disconnected", message, "", 0);
    while let Some(command) = rx.recv().await {
        if matches!(command, Command::Shutdown) {
            break;
        }
    }
    let _ = tx.send(Event::Exit);
    false
}

pub async fn run(
    config: Config,
    mut rx: mpsc::Receiver<Command>,
    tx: std_mpsc::Sender<Event>,
) -> bool {
    if config.preview {
        return preview(&mut rx, &tx).await;
    }
    let sessions = sessions::discover();
    let selected = config
        .state
        .as_ref()
        .and_then(|p| state::load(p).ok())
        .and_then(|id| sessions.iter().position(|s| s.id == id))
        .unwrap_or(0);
    let _ = tx.send(Event::Sessions {
        names: sessions.iter().map(|s| s.name.clone()).collect(),
        selected,
    });
    let timeout = Duration::from_secs(30);
    let stream =
        tokio::time::timeout(Duration::from_secs(5), UnixStream::connect(&config.socket)).await;
    let mut stream = match stream {
        Ok(Ok(s)) => s,
        _ => {
            return disconnected(
                &mut rx,
                &tx,
                "Cannot connect to greetd. No reconnect will be attempted.",
            )
            .await;
        }
    };
    // Startup queries run concurrently on the same current-thread runtime;
    // a slow/unavailable system bus never delays authentication or shutdown.
    let caps = tokio::spawn(power::capabilities());
    let account_events = tx.clone();
    tokio::spawn(async move {
        let _ = account_events.send(Event::Accounts(accounts::discover().await));
    });
    view(
        &tx,
        "idle",
        if sessions.is_empty() {
            "No trusted supported Wayland sessions are installed."
        } else {
            "Enter your username to begin."
        },
        "",
        0,
    );
    let mut caps = Some(caps);
    let mut capabilities = 0;
    let mut token = 0;
    loop {
        let command = tokio::select! {
            command = rx.recv() => command,
            result = async { caps.as_mut().unwrap().await }, if caps.is_some() => {
                capabilities = result.unwrap_or(0); caps = None;
                let _ = tx.send(Event::Capabilities(capabilities));
                continue;
            }
        };
        match command {
            Some(Command::Begin { username, session }) if accounts::valid_username(&username) => {
                let Some(session) = sessions.get(session) else {
                    continue;
                };
                match attempt(
                    &mut stream,
                    username,
                    session,
                    &mut rx,
                    &tx,
                    &mut token,
                    timeout,
                )
                .await
                {
                    Attempt::Launched => {
                        if let Some(path) = &config.state
                            && state::save(path, &session.id).is_err()
                        {
                            eprintln!("Selected session could not be saved (handoff continues).");
                        }
                        view(
                            &tx,
                            "handoff",
                            "Session scheduled. Leaving the greeter…",
                            "",
                            0,
                        );
                        let _ = tx.send(Event::Exit);
                        return true;
                    }
                    Attempt::Idle(message) => view(&tx, "idle", &message, "", 0),
                    Attempt::Disconnected(message) => {
                        return disconnected(&mut rx, &tx, &message).await;
                    }
                    Attempt::Shutdown => break,
                }
            }
            Some(Command::Power(action)) if action < 3 && capabilities & (1 << action) != 0 => {
                view(&tx, "power", "Requesting power action…", "", 0);
                let message = power::perform(action)
                    .await
                    .err()
                    .unwrap_or("Power request accepted.");
                view(&tx, "idle", message, "", 0);
            }
            Some(Command::Shutdown) | None => break,
            _ => {}
        }
    }
    let _ = tx.send(Event::Exit);
    false
}

async fn preview(rx: &mut mpsc::Receiver<Command>, tx: &std_mpsc::Sender<Event>) -> bool {
    let _ = tx.send(Event::Sessions {
        names: vec![
            "Hyprland".into(),
            "Hyprland (uwsm-managed)".into(),
            "Weston".into(),
        ],
        selected: 0,
    });
    let _ = tx.send(Event::Capabilities(7));
    let _ = tx.send(Event::Accounts(accounts::preview()));
    view(
        tx,
        "idle",
        "Preview only. Use sample text, never real credentials.",
        "",
        0,
    );
    let mut token = 0;
    while let Some(command) = rx.recv().await {
        match command {
            Command::Begin { .. } => {
                token += 1;
                view(
                    tx,
                    "secret",
                    "No authentication request will be sent.",
                    "Password",
                    token,
                );
            }
            Command::Answer { .. } => view(
                tx,
                "idle",
                "Preview complete. Nothing was sent or saved.",
                "",
                0,
            ),
            Command::Cancel => view(tx, "idle", "Preview cancelled.", "", 0),
            Command::Power(_) => view(
                tx,
                "idle",
                "Preview: power action simulated. Nothing was requested.",
                "",
                0,
            ),
            Command::Shutdown => break,
        }
    }
    let _ = tx.send(Event::Exit);
    false
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
