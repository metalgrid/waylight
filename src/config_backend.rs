//! Client bridge between the config GUI's QML and the `waylight-configd`
//! daemon. The QML side owns a `ConfigBackend` QObject; a worker thread in
//! the binary owns the zbus system-bus connection and answers requests sent
//! through the `BRIDGE` channel pair, mirroring the greeter's Backend
//! pattern. All daemon interaction is asynchronous and non-blocking for the
//! UI thread.

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::sync::{
    Mutex, OnceLock,
    mpsc::{self, Receiver, Sender, TryRecvError},
};

/// One request from the UI thread to the daemon worker.
#[derive(Debug)]
pub enum Request {
    Reload,
    SetTheme(String),
    SetLanguage(String),
}

/// One answer from the daemon worker.
#[derive(Debug)]
pub enum Reply {
    Loaded {
        theme: String,
        config: String,
        theme_path: String,
        config_path: String,
    },
    Saved,
    Failed(String),
}

pub struct Bridge {
    pub requests: tokio::sync::mpsc::Sender<Request>,
    pub replies: Receiver<Reply>,
}

pub static BRIDGE: OnceLock<Mutex<Option<Bridge>>> = OnceLock::new();

/// Handed to the binary's worker thread.
pub struct Worker {
    pub requests: tokio::sync::mpsc::Receiver<Request>,
    pub replies: Sender<Reply>,
}

pub fn channels(buffer: usize) -> (Bridge, Worker) {
    let (requests_tx, requests_rx) = tokio::sync::mpsc::channel(buffer);
    let (replies_tx, replies_rx) = mpsc::channel();
    (
        Bridge {
            requests: requests_tx,
            replies: replies_rx,
        },
        Worker {
            requests: requests_rx,
            replies: replies_tx,
        },
    )
}

// The D-Bus client proxy for the daemon, used by the binary's worker.
#[zbus::proxy(
    interface = "dev.waylight.Config1",
    default_service = "dev.waylight.Config",
    default_path = "/dev/waylight/Config"
)]
pub trait Config1 {
    fn get_all(&self) -> zbus::Result<(String, String, String, String)>;
    fn set_theme(&self, json: &str) -> zbus::Result<()>;
    fn set_language(&self, code: &str) -> zbus::Result<()>;
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, theme)]
        #[qproperty(QString, config)]
        #[qproperty(QString, theme_path, cxx_name = "themePath")]
        #[qproperty(QString, config_path, cxx_name = "configPath")]
        #[qproperty(QString, status)]
        #[qproperty(QString, message)]
        type ConfigBackend = super::ConfigBackendRust;
        #[qinvokable]
        fn poll(self: Pin<&mut ConfigBackend>);
        #[qinvokable]
        fn reload(self: Pin<&mut ConfigBackend>);
        #[qinvokable]
        fn apply_theme(self: Pin<&mut ConfigBackend>, json: QString);
        #[qinvokable]
        fn apply_language(self: Pin<&mut ConfigBackend>, code: QString);
        #[qsignal]
        fn applied(self: Pin<&mut ConfigBackend>);
    }
}

pub struct ConfigBackendRust {
    theme: QString,
    config: QString,
    theme_path: QString,
    config_path: QString,
    status: QString,
    message: QString,
    bridge: Bridge,
}

impl Default for ConfigBackendRust {
    fn default() -> Self {
        let bridge = BRIDGE
            .get()
            .expect("bridge initialized before QML")
            .lock()
            .unwrap()
            .take()
            .expect("only one config backend");
        Self {
            theme: QString::default(),
            config: QString::default(),
            theme_path: QString::default(),
            config_path: QString::default(),
            status: "loading".into(),
            message: "Connecting to the waylight-configd service…".into(),
            bridge,
        }
    }
}

impl ffi::ConfigBackend {
    fn send(mut self: std::pin::Pin<&mut Self>, request: Request) {
        if self.rust().bridge.requests.try_send(request).is_err() {
            self.as_mut().set_status("error".into());
            self.as_mut()
                .set_message("The daemon worker is unavailable; restart the application.".into());
        } else {
            self.as_mut().set_status("busy".into());
        }
    }

    pub fn reload(self: std::pin::Pin<&mut Self>) {
        self.send(Request::Reload);
    }

    pub fn apply_theme(self: std::pin::Pin<&mut Self>, json: QString) {
        self.send(Request::SetTheme(json.to_string()));
    }

    pub fn apply_language(self: std::pin::Pin<&mut Self>, code: QString) {
        self.send(Request::SetLanguage(code.to_string()));
    }

    pub fn poll(mut self: std::pin::Pin<&mut Self>) {
        loop {
            match self.rust().bridge.replies.try_recv() {
                Ok(Reply::Loaded {
                    theme,
                    config,
                    theme_path,
                    config_path,
                }) => {
                    self.as_mut().set_theme(theme.into());
                    self.as_mut().set_config(config.into());
                    self.as_mut().set_theme_path(theme_path.into());
                    self.as_mut().set_config_path(config_path.into());
                    self.as_mut().set_status("ready".into());
                    self.as_mut().set_message("Loaded from the daemon.".into());
                }
                Ok(Reply::Saved) => {
                    self.as_mut().set_status("ready".into());
                    self.as_mut().set_message(
                        "Saved. Restart the greeter (or reboot to the login screen) to apply."
                            .into(),
                    );
                    self.as_mut().applied();
                }
                Ok(Reply::Failed(message)) => {
                    self.as_mut().set_status("error".into());
                    self.as_mut().set_message(message.into());
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
    }
}
