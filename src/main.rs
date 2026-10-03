mod accounts;
mod backend;
mod controller;
mod power;
mod sessions;
mod state;

use controller::{Command, Config};
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

fn config() -> Result<Option<Config>, String> {
    let mut args = std::env::args_os().skip(1);
    let mut preview = false;
    let mut state = None;
    let mut help = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--preview") if !preview => preview = true,
            Some("--state-dir") if state.is_none() => {
                let path = PathBuf::from(
                    args.next()
                        .ok_or("--state-dir requires a private absolute directory")?,
                );
                if !path.is_absolute() {
                    return Err("--state-dir must be absolute".into());
                }
                state = Some(path);
            }
            Some("--help") if !help => help = true,
            _ => return Err("Unknown or duplicate argument. Use --help.".into()),
        }
    }
    if help {
        println!(
            "waylight-greeter [--state-dir /private/greeter/directory]\nwaylight-greeter --preview\nReal mode requires GREETD_SOCK. No state is stored unless --state-dir is supplied."
        );
        return Ok(None);
    }
    if preview && state.is_some() {
        return Err("--preview cannot use --state-dir".into());
    }
    let socket = if preview {
        PathBuf::new()
    } else {
        let path = PathBuf::from(
            std::env::var_os("GREETD_SOCK")
                .ok_or("GREETD_SOCK is required; use --preview for isolated demonstration.")?,
        );
        if !path.is_absolute() {
            return Err("GREETD_SOCK must be an absolute socket path".into());
        }
        path
    };
    Ok(Some(Config {
        preview,
        socket,
        state,
    }))
}
fn main() -> std::process::ExitCode {
    let config = match config() {
        Ok(Some(c)) => c,
        Ok(None) => return 0.into(),
        Err(e) => {
            eprintln!("{e}");
            return 2.into();
        }
    };
    // Suppress Qt's disk cache in both modes (especially isolated preview).
    // QML reads the optional theme.json through XHR; Qt 6 disables local file
    // reads by default and this opt-in is presentation-only. Both are set
    // before spawning threads; no other environment mutation is performed.
    unsafe {
        std::env::set_var("QML_DISABLE_DISK_CACHE", "1");
        std::env::set_var("QML_XHR_ALLOW_FILE_READ", "1");
    }
    let preview = config.preview;
    let (commands, rx) = tokio::sync::mpsc::channel(16);
    let (tx, events) = std::sync::mpsc::channel();
    backend::BRIDGE
        .set(Mutex::new(Some(backend::Bridge {
            commands: commands.clone(),
            events,
            preview,
        })))
        .ok()
        .expect("single initialization");
    let signals = commands.clone();
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Tokio runtime");
        runtime.block_on(async move {
            use tokio::signal::unix::{SignalKind, signal};
            let (Ok(mut term), Ok(mut interrupt)) = (
                signal(SignalKind::terminate()),
                signal(SignalKind::interrupt()),
            ) else {
                eprintln!("Cannot install graceful-shutdown signal handlers.");
                let _ = tx.send(controller::Event::Exit);
                return false;
            };
            tokio::spawn(async move {
                tokio::select! { _ = term.recv() => {}, _ = interrupt.recv() => {} }
                let _ = signals.send(Command::Shutdown).await;
            });
            controller::run(config, rx, tx).await
        })
    });
    let mut app = QGuiApplication::new();
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
        .load(&QUrl::from("qrc:/qt/qml/Waylight/App.qml"));
    if !failed.load(Ordering::Relaxed) {
        app.pin_mut().exec();
    }
    drop(loaded);
    // Event loop has stopped. Request cleanup and join before destroying Qt or
    // exiting; a disconnected/failed worker never authorizes successful handoff.
    let _ = commands.blocking_send(Command::Shutdown);
    let launched = worker.join().unwrap_or(false);
    if launched && !failed.load(Ordering::Relaxed) {
        0.into()
    } else {
        1.into()
    }
}
