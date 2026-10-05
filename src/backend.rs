use crate::{
    accounts,
    controller::{Command, Event},
};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use std::{
    ffi::OsStr,
    path::PathBuf,
    pin::Pin,
    sync::{Mutex, OnceLock, mpsc},
};

pub struct Bridge {
    pub commands: tokio::sync::mpsc::Sender<Command>,
    pub events: mpsc::Receiver<Event>,
    pub preview: bool,
}
pub static BRIDGE: OnceLock<Mutex<Option<Bridge>>> = OnceLock::new();

/// The user config base directory, mirroring the XDG spec: XDG_CONFIG_HOME
/// when set and non-empty, else $HOME/.config. Shared by both path helpers.
fn user_config_base(xdg_config_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    xdg_config_home
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })
}

/// Ordered theme.json candidates: the user's config directory first, then the
/// system path. Pure helper taking environment values as parameters so tests
/// never mutate the environment. `skip_system` drops the /etc candidate (test
/// harness isolation; see skip_system_config). QML parses this JSON array and
/// loads the first readable file; presentation only, never an authentication
/// input.
fn theme_paths_json(
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
    skip_system: bool,
) -> String {
    let mut paths = Vec::new();
    if let Some(base) = user_config_base(xdg_config_home, home) {
        paths.push(base.join("waylight").join("theme.json"));
    }
    if !skip_system {
        paths.push(PathBuf::from("/etc/waylight/theme.json"));
    }
    let encoded: Vec<String> = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    serde_json::to_string(&encoded).expect("path list serialization cannot fail")
}

/// Ordered waylight.json candidates, mirroring theme_paths_json. Pure helper
/// taking environment values as parameters so tests never mutate the
/// environment; `skip_system` drops the /etc candidate. main.rs resolves the
/// UI language from the first readable candidate; presentation only, never an
/// authentication input.
pub fn config_language_json(
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
    skip_system: bool,
) -> String {
    let mut paths = Vec::new();
    if let Some(base) = user_config_base(xdg_config_home, home) {
        paths.push(base.join("waylight").join("waylight.json"));
    }
    if !skip_system {
        paths.push(PathBuf::from("/etc/waylight/waylight.json"));
    }
    let encoded: Vec<String> = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    serde_json::to_string(&encoded).expect("path list serialization cannot fail")
}

/// Test-only isolation switch (WAYLIGHT_SKIP_SYSTEM_CONFIG=1): drops the
/// /etc/waylight candidates from both search lists so harnesses never couple
/// to the host's system configuration. The packaged greeter never sets it.
pub fn skip_system_config() -> bool {
    std::env::var_os("WAYLIGHT_SKIP_SYSTEM_CONFIG").as_deref() == Some(OsStr::new("1"))
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }
    // Small Rust-test construction seam; not exposed to QML.
    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[cxx_name = "make_unique"]
        #[allow(dead_code)]
        fn backend_make_unique() -> UniquePtr<Backend>;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, state)]
        #[qproperty(QString, message)]
        #[qproperty(QString, prompt)]
        #[qproperty(QString, accounts)]
        #[qproperty(QStringList, sessions)]
        #[qproperty(i32, selected)]
        #[qproperty(u32, capabilities)]
        #[qproperty(bool, preview)]
        #[qproperty(bool, closing)]
        #[qproperty(QString, theme_paths, cxx_name = "themePaths")]
        #[qproperty(QString, config_paths, cxx_name = "configPaths")]
        #[qproperty(QString, ui_language, cxx_name = "uiLanguage")]
        type Backend = super::BackendRust;
        #[qinvokable]
        fn poll(self: Pin<&mut Backend>);
        #[qinvokable]
        fn begin(self: Pin<&mut Backend>, username: QString, session: i32);
        #[qinvokable]
        fn reset_identity(self: Pin<&mut Backend>) -> bool;
        #[qinvokable]
        fn choose_user(self: Pin<&mut Backend>, username: QString, session: i32);
        #[qinvokable]
        fn choose_manual(self: Pin<&mut Backend>);
        #[qinvokable]
        fn answer(self: Pin<&mut Backend>, text: QString);
        #[qinvokable]
        fn cancel(self: Pin<&mut Backend>);
        #[qinvokable]
        fn shutdown(self: Pin<&mut Backend>);
        #[qinvokable]
        fn power(self: Pin<&mut Backend>, action: i32);
        #[qsignal]
        fn finished(self: Pin<&mut Backend>);
        #[qsignal]
        #[cxx_name = "identityChosen"]
        fn identity_chosen(self: Pin<&mut Backend>, username: QString);
    }
}

// Only intentions, never PAM answers. The session index is snapshotted against
// the controller's immutable discovered session list.
#[derive(Debug)]
enum Intent {
    User { username: String, session: i32 },
    Manual,
    Power(i32),
}
impl Intent {
    fn message(&self) -> String {
        let action = match self {
            Self::User { username, .. } => format!("Sign in as {username}"),
            Self::Manual => "Enter a username manually".into(),
            Self::Power(action) => ["Sleep", "Restart", "Shut down"][*action as usize].into(),
        };
        format!("{action} requested; waiting for authentication cleanup…")
    }
}

pub struct BackendRust {
    state: QString,
    message: QString,
    prompt: QString,
    accounts: QString,
    sessions: QStringList,
    selected: i32,
    capabilities: u32,
    preview: bool,
    bridge: Bridge,
    token: u32,
    cancelling: bool,
    closing: bool,
    theme_paths: QString,
    config_paths: QString,
    ui_language: QString,
    pending: Option<Intent>,
}
impl Default for BackendRust {
    fn default() -> Self {
        let bridge = BRIDGE
            .get()
            .expect("bridge initialized before QML")
            .lock()
            .unwrap()
            .take()
            .expect("only one backend");
        Self {
            state: "loading".into(),
            message: "Connecting…".into(),
            prompt: QString::default(),
            accounts: "[]".into(),
            sessions: QStringList::default(),
            selected: 0,
            capabilities: 0,
            preview: bridge.preview,
            bridge,
            token: 0,
            cancelling: false,
            closing: false,
            theme_paths: QString::from(theme_paths_json(
                std::env::var_os("XDG_CONFIG_HOME").as_deref(),
                std::env::var_os("HOME").as_deref(),
                skip_system_config(),
            )),
            config_paths: QString::from(config_language_json(
                std::env::var_os("XDG_CONFIG_HOME").as_deref(),
                std::env::var_os("HOME").as_deref(),
                skip_system_config(),
            )),
            // Resolved by install_language before the engine loaded.
            ui_language: QString::from(crate::i18n::installed_language()),
            pending: None,
        }
    }
}
impl ffi::Backend {
    fn send(mut self: Pin<&mut Self>, command: Command) -> bool {
        if self.rust().bridge.commands.try_send(command).is_err() {
            self.as_mut().rust_mut().pending = None;
            self.as_mut().set_state("disconnected".into());
            self.as_mut().set_message(
                "Worker unavailable. Close the greeter; no retry was attempted.".into(),
            );
            return false;
        }
        true
    }
    pub fn poll(mut self: Pin<&mut Self>) {
        loop {
            let event = self.rust().bridge.events.try_recv();
            match event {
                Ok(Event::Sessions { names, selected }) => {
                    let mut list = QStringList::default();
                    for name in names {
                        list.append(QString::from(name));
                    }
                    self.as_mut().set_sessions(list);
                    self.as_mut().set_selected(selected as i32);
                }
                Ok(Event::Capabilities(caps)) => self.as_mut().set_capabilities(caps),
                Ok(Event::Accounts(accounts)) => {
                    let entries: Vec<_> = accounts.into_iter().map(|account| {
                        serde_json::json!({"username": account.username, "name": account.name, "picture": account.picture})
                    }).collect();
                    self.as_mut()
                        .set_accounts(serde_json::Value::Array(entries).to_string().into());
                }
                Ok(Event::View {
                    state,
                    message,
                    prompt,
                    token,
                }) => {
                    if matches!(state, "starting" | "handoff" | "disconnected") {
                        self.as_mut().rust_mut().pending = None;
                    }
                    if self.rust().cancelling
                        && !matches!(
                            state,
                            "idle" | "disconnected" | "starting" | "handoff" | "cancelling"
                        )
                    {
                        continue;
                    }
                    if state == "idle" && self.rust().closing {
                        continue;
                    }
                    if matches!(state, "idle" | "disconnected") {
                        self.as_mut().rust_mut().cancelling = false;
                    }
                    if state == "idle"
                        && let Some(intent) = self.as_mut().rust_mut().pending.take()
                    {
                        // No observable interactive idle between cleanup and dispatch.
                        if self.as_mut().execute(intent) {
                            continue;
                        }
                    }
                    self.as_mut().rust_mut().token = token;
                    self.as_mut().set_prompt(prompt.into());
                    let message = self
                        .rust()
                        .pending
                        .as_ref()
                        .map(Intent::message)
                        .unwrap_or(message);
                    self.as_mut().set_message(message.into());
                    self.as_mut().set_state(state.into());
                }
                Ok(Event::Exit) | Err(mpsc::TryRecvError::Disconnected) => {
                    self.as_mut().rust_mut().pending = None;
                    self.as_mut().set_closing(true);
                    self.as_mut().finished();
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
    }
    // Idle is reached only after confirmed authentication cleanup. Identity edits
    // never change controller state or bypass that boundary.
    pub fn reset_identity(mut self: Pin<&mut Self>) -> bool {
        if self.rust().closing || self.state().to_string() != "idle" {
            return false;
        }
        self.as_mut().rust_mut().token = 0;
        self.as_mut().set_prompt(QString::default());
        self.as_mut().set_message(QString::default());
        true
    }
    pub fn begin(mut self: Pin<&mut Self>, username: QString, session: i32) {
        let username = username.to_string();
        if self.rust().closing
            || self.state().to_string() != "idle"
            || !accounts::valid_username(&username)
            || session < 0
            || session as isize >= self.sessions().len()
        {
            return;
        }
        if self.as_mut().send(Command::Begin {
            username,
            session: session as usize,
        }) {
            self.as_mut().set_state("waiting".into());
        }
    }
    pub fn answer(mut self: Pin<&mut Self>, text: QString) {
        if self.rust().closing
            || !matches!(
                self.state().to_string().as_str(),
                "secret" | "visible" | "info" | "error"
            )
        {
            return;
        }
        let token = self.rust().token;
        if self.as_mut().send(Command::Answer {
            token,
            text: text.to_string(),
        }) {
            self.as_mut().set_state("waiting".into());
        }
    }
    pub fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().pending = None;
        if self.state().to_string() == "cancelling" {
            self.as_mut()
                .set_message("Cancellation requested; waiting for authentication cleanup…".into());
        }
        self.cancel_internal();
    }
    fn cancel_internal(mut self: Pin<&mut Self>) {
        if self.rust().cancelling
            || !matches!(
                self.state().to_string().as_str(),
                "waiting" | "secret" | "visible" | "info" | "error"
            )
        {
            return;
        }
        if self.as_mut().send(Command::Cancel) {
            self.as_mut().rust_mut().cancelling = true;
            self.as_mut().set_state("cancelling".into());
            self.as_mut()
                .set_message("Cancellation requested; waiting for the daemon…".into());
        }
    }
    pub fn shutdown(mut self: Pin<&mut Self>) {
        if self.rust().closing {
            return;
        }
        self.as_mut().rust_mut().pending = None;
        self.as_mut().set_closing(true);
        self.as_mut().rust_mut().cancelling = true;
        if !self.as_mut().send(Command::Shutdown) {
            self.as_mut().finished();
            return;
        }
        if !matches!(self.state().to_string().as_str(), "starting" | "handoff") {
            self.as_mut().set_state("cancelling".into());
            self.as_mut()
                .set_message("Closing after authentication cleanup…".into());
        }
    }
    fn valid_intent(&self, intent: &Intent) -> bool {
        !self.rust().closing
            && match intent {
                Intent::User { username, session } => {
                    accounts::valid_username(username)
                        && *session >= 0
                        && (*session as isize) < self.sessions().len()
                }
                Intent::Manual => true,
                Intent::Power(action) => {
                    (0..3).contains(action) && self.capabilities() & (1 << action) != 0
                }
            }
    }
    fn request(mut self: Pin<&mut Self>, intent: Intent) {
        if !self.valid_intent(&intent) {
            return;
        }
        match self.state().to_string().as_str() {
            "idle" => {
                self.as_mut().execute(intent);
            }
            "waiting" | "secret" | "visible" | "info" | "error" | "cancelling" => {
                self.as_mut().rust_mut().pending = Some(intent);
                self.as_mut().cancel_internal();
                if let Some(intent) = &self.rust().pending {
                    let message = intent.message();
                    self.as_mut().set_message(message.into());
                }
            }
            _ => {}
        }
    }
    // Called only in idle or directly on a confirmed controller idle event.
    fn execute(mut self: Pin<&mut Self>, intent: Intent) -> bool {
        if !self.valid_intent(&intent) {
            return false;
        }
        self.as_mut().rust_mut().token = 0;
        self.as_mut().set_prompt(QString::default());
        self.as_mut().set_message(QString::default());
        match intent {
            Intent::User { username, session } => {
                if self.as_mut().send(Command::Begin {
                    username: username.clone(),
                    session: session as usize,
                }) {
                    self.as_mut().set_state("waiting".into());
                    self.as_mut().identity_chosen(username.into());
                }
            }
            Intent::Manual => {
                self.as_mut().set_state("idle".into());
                self.as_mut().identity_chosen(QString::default());
            }
            Intent::Power(action) => {
                if self.as_mut().send(Command::Power(action as usize)) {
                    self.as_mut().set_state("power".into());
                }
            }
        }
        true
    }
    pub fn choose_user(self: Pin<&mut Self>, username: QString, session: i32) {
        self.request(Intent::User {
            username: username.to_string(),
            session,
        });
    }
    pub fn choose_manual(self: Pin<&mut Self>) {
        self.request(Intent::Manual);
    }
    pub fn power(self: Pin<&mut Self>, action: i32) {
        self.request(Intent::Power(action));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_paths_search_order_is_user_then_system() {
        let system = "/etc/waylight/theme.json";
        assert_eq!(
            theme_paths_json(
                Some(OsStr::new("/custom/cfg")),
                Some(OsStr::new("/home/u")),
                false
            ),
            format!("[\"/custom/cfg/waylight/theme.json\",\"{system}\"]")
        );
        // An empty XDG_CONFIG_HOME falls back to $HOME/.config.
        assert_eq!(
            theme_paths_json(Some(OsStr::new("")), Some(OsStr::new("/home/u")), false),
            format!("[\"/home/u/.config/waylight/theme.json\",\"{system}\"]")
        );
        assert_eq!(
            theme_paths_json(None, Some(OsStr::new("/home/u")), false),
            format!("[\"/home/u/.config/waylight/theme.json\",\"{system}\"]")
        );
        // Without any usable user base only the system path remains.
        assert_eq!(
            theme_paths_json(None, None, false),
            format!("[\"{system}\"]")
        );
        assert_eq!(
            theme_paths_json(Some(OsStr::new("")), Some(OsStr::new("")), false),
            format!("[\"{system}\"]")
        );
        // JSON escaping survives unusual but legal directory names.
        assert_eq!(
            theme_paths_json(Some(OsStr::new("/a\"b\\c")), None, false),
            "[\"/a\\\"b\\\\c/waylight/theme.json\",\"/etc/waylight/theme.json\"]"
        );
    }

    #[test]
    fn config_paths_search_order_is_user_then_system() {
        let system = "/etc/waylight/waylight.json";
        assert_eq!(
            config_language_json(
                Some(OsStr::new("/custom/cfg")),
                Some(OsStr::new("/home/u")),
                false
            ),
            format!("[\"/custom/cfg/waylight/waylight.json\",\"{system}\"]")
        );
        // An empty XDG_CONFIG_HOME falls back to $HOME/.config.
        assert_eq!(
            config_language_json(Some(OsStr::new("")), Some(OsStr::new("/home/u")), false),
            format!("[\"/home/u/.config/waylight/waylight.json\",\"{system}\"]")
        );
        assert_eq!(
            config_language_json(None, Some(OsStr::new("/home/u")), false),
            format!("[\"/home/u/.config/waylight/waylight.json\",\"{system}\"]")
        );
        // Without any usable user base only the system path remains.
        assert_eq!(
            config_language_json(None, None, false),
            format!("[\"{system}\"]")
        );
        assert_eq!(
            config_language_json(Some(OsStr::new("")), Some(OsStr::new("")), false),
            format!("[\"{system}\"]")
        );
    }

    #[test]
    fn skip_system_flag_drops_the_etc_candidates() {
        // Harness isolation: with the flag set only the user candidate stays,
        // in both lists, and an absent user base yields an empty list.
        assert_eq!(
            theme_paths_json(Some(OsStr::new("/cfg")), Some(OsStr::new("/home/u")), true),
            "[\"/cfg/waylight/theme.json\"]"
        );
        assert_eq!(
            config_language_json(Some(OsStr::new("/cfg")), Some(OsStr::new("/home/u")), true),
            "[\"/cfg/waylight/waylight.json\"]"
        );
        assert_eq!(theme_paths_json(None, None, true), "[]");
        assert_eq!(config_language_json(None, None, true), "[]");
    }
}
