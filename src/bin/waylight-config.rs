//! `waylight-config` — the Waylight greeter configuration GUI. A normal user
//! desktop application: it edits theme and language drafts locally, shows a
//! live preview of the greeter and applies changes through the privileged
//! `waylight-configd` D-Bus service (polkit prompts at the daemon). The UI
//! language follows waylight.json exactly like the greeter, resolved before
//! the QML engine loads.

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use waylight_greeter::config_backend::Reply;
use waylight_greeter::{backend, config_backend, i18n};
use zbus::Connection;

async fn connect_once(connection: &mut Option<Connection>) -> Result<&Connection, String> {
    if connection.is_none() {
        let system = Connection::system()
            .await
            .map_err(|error| format!("system bus unavailable: {error}"))?;
        *connection = Some(system);
    }
    Ok(connection
        .as_ref()
        .expect("connection established or previously cached"))
}

async fn run_request(
    connection: &mut Option<Connection>,
    request: config_backend::Request,
) -> Result<config_backend::Reply, String> {
    use config_backend::{Config1Proxy, Reply, Request as RequestKind};
    let system = connect_once(connection).await?;
    let proxy = Config1Proxy::new(system)
        .await
        .map_err(|error| format!("waylight-configd is unavailable: {error}"))?;
    match request {
        RequestKind::Reload => {
            let (theme, config, theme_path, config_path) = proxy
                .get_all()
                .await
                .map_err(|error| format!("GetAll failed: {error}"))?;
            Ok(Reply::Loaded {
                theme,
                config,
                theme_path,
                config_path,
            })
        }
        RequestKind::SetTheme(json) => {
            proxy
                .set_theme(&json)
                .await
                .map_err(|error| format!("SetTheme failed: {error}"))?;
            Ok(Reply::Saved)
        }
        RequestKind::SetLanguage(code) => {
            proxy
                .set_language(&code)
                .await
                .map_err(|error| format!("SetLanguage failed: {error}"))?;
            Ok(Reply::Saved)
        }
    }
}

async fn serve(mut worker: config_backend::Worker) {
    let mut connection: Option<Connection> = None;
    while let Some(request) = worker.requests.recv().await {
        let reply = run_request(&mut connection, request)
            .await
            .unwrap_or_else(Reply::Failed);
        let _ = worker.replies.send(reply);
    }
}

fn main() -> std::process::ExitCode {
    // No CLI surface: the utility is purely graphical.
    if std::env::args_os().nth(1).is_some() {
        eprintln!("waylight-config takes no arguments");
        return 2.into();
    }
    // Same presentation-only environment as the greeter: no QML disk cache,
    // and XHR reads for the optional local theme/background files.
    unsafe {
        std::env::set_var("QML_DISABLE_DISK_CACHE", "1");
        std::env::set_var("QML_XHR_ALLOW_FILE_READ", "1");
    }
    let (bridge, worker) = config_backend::channels(16);
    config_backend::BRIDGE
        .set(Mutex::new(Some(bridge)))
        .ok()
        .expect("single initialization");
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Tokio runtime");
        runtime.block_on(serve(worker));
    });
    let mut app = QGuiApplication::new();
    // The GUI's own UI language is the greeter's resolved language; the
    // translator and layout direction must exist before qsTr evaluation.
    let config_paths = backend::config_language_json(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    );
    i18n::install_language(&config_paths);
    let mut engine = QQmlApplicationEngine::new();
    let failed = Arc::new(AtomicBool::new(false));
    let flag = failed.clone();
    let loaded = engine.pin_mut().on_object_created(move |_, object, _| {
        if object.is_null() {
            flag.store(true, Ordering::Relaxed);
        }
    });
    engine
        .pin_mut()
        .load(&QUrl::from("qrc:/qt/qml/Waylight/ConfigApp.qml"));
    if !failed.load(Ordering::Relaxed) {
        app.pin_mut().exec();
    }
    drop(loaded);
    // The worker thread ends when the process exits; it holds no system state.
    if failed.load(Ordering::Relaxed) {
        1.into()
    } else {
        0.into()
    }
}
