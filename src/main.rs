mod accounts;
mod backend;
mod controller;
mod i18n;
mod power;
mod sessions;
mod state;

use controller::{Command, Config};
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use std::{
    io::Read,
    path::{Path, PathBuf},
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

// The greeter's UI language (waylight.json) is presentation-only: missing,
// malformed or unknown content never prevents startup and falls back to
// English. ar and ur additionally mirror the layout.
const LANGUAGES: [&str; 11] = [
    "en", "zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg",
];
const RTL_LANGUAGES: [&str; 2] = ["ar", "ur"];
// {"language": "ar"} is a few bytes; the cap keeps a runaway file from being
// read while allowing indented, commented-by-hand edits.
const LANGUAGE_FILE_LIMIT: u64 = 64 * 1024;

/// A candidate file wins only when it parses as a JSON object whose
/// "language" is one of the supported codes.
fn validated_language(contents: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(contents).ok()?;
    let requested = value.as_object()?.get("language")?.as_str()?;
    LANGUAGES
        .iter()
        .find(|language| **language == requested)
        .copied()
}

/// Decodes the candidate list produced by config_language_json. A malformed
/// list yields no candidates (English) instead of failing startup.
fn language_candidates(paths_json: &str) -> Vec<PathBuf> {
    serde_json::from_str::<Vec<String>>(paths_json)
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

/// Reads at most `limit` bytes; unreadable, oversized or non-UTF-8 files are
/// skipped by the caller.
fn read_limited(path: &Path, limit: u64) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes).ok()?;
    String::from_utf8(bytes).ok()
}

/// The first readable candidate with a supported language wins; anything else
/// is English.
fn resolve_language(candidates: &[PathBuf]) -> &'static str {
    for path in candidates {
        if let Some(contents) = read_limited(path, LANGUAGE_FILE_LIMIT)
            && let Some(language) = validated_language(&contents)
        {
            return language;
        }
    }
    "en"
}

/// Installs the translator and layout direction before the QML engine loads.
/// A missing catalog keeps English with a warning; nothing here is fatal.
fn install_language(paths_json: &str) -> &'static str {
    let language = resolve_language(&language_candidates(paths_json));
    if language != "en" {
        let path = format!(":/qt/qml/Waylight/i18n/waylight_{language}.qm");
        if !i18n::ffi::waylight_install_translator(&QString::from(path)) {
            eprintln!(
                "waylight: translation for '{language}' is missing; falling back to English."
            );
        }
    }
    i18n::ffi::waylight_set_layout_direction(RTL_LANGUAGES.contains(&language));
    language
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
    // The translator and the layout direction must exist before the first
    // qsTr evaluation, so the UI language is resolved ahead of the engine.
    let config_paths = backend::config_language_json(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    );
    install_language(&config_paths);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waylight_json_language_validation() {
        assert_eq!(validated_language(r#"{"language": "ar"}"#), Some("ar"));
        assert_eq!(validated_language(r#"{"language": "bg"}"#), Some("bg"));
        assert_eq!(validated_language(r#"{"language": "pt"}"#), Some("pt"));
        // Extra keys are ignored.
        assert_eq!(
            validated_language(r#"{"language": "ru", "note": "x"}"#),
            Some("ru")
        );
        // Unsupported, wrong-typed, missing and malformed content fails open.
        assert_eq!(validated_language(r#"{"language": "de"}"#), None);
        assert_eq!(validated_language(r#"{"language": 5}"#), None);
        assert_eq!(validated_language(r#"{}"#), None);
        assert_eq!(validated_language(r#"[{"language": "ar"}]"#), None);
        assert_eq!(validated_language("not json"), None);
        assert_eq!(validated_language(""), None);
    }

    #[test]
    fn language_candidates_decode_the_shared_path_order() {
        assert_eq!(
            language_candidates(
                "[\"/custom/cfg/waylight/waylight.json\",\"/etc/waylight/waylight.json\"]"
            ),
            vec![
                PathBuf::from("/custom/cfg/waylight/waylight.json"),
                PathBuf::from("/etc/waylight/waylight.json")
            ]
        );
        assert_eq!(language_candidates("not json"), Vec::<PathBuf>::new());
        assert_eq!(language_candidates("[]"), Vec::<PathBuf>::new());
    }

    #[test]
    fn resolve_language_walks_candidates_and_fails_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let user = dir.path().join("user.json");
        let system = dir.path().join("system.json");
        // No candidate file at all.
        assert_eq!(resolve_language(&[user.clone(), system.clone()]), "en");
        // Unreadable/invalid candidates are skipped in order.
        std::fs::write(&user, b"{broken").expect("write");
        std::fs::write(&system, br#"{"language": "es"}"#).expect("write");
        assert_eq!(resolve_language(&[user.clone(), system.clone()]), "es");
        // The first valid file wins over later ones.
        std::fs::write(&user, br#"{"language": "ar"}"#).expect("write");
        assert_eq!(resolve_language(&[user.clone(), system.clone()]), "ar");
        // An unsupported language falls through; with no valid candidate left
        // the greeter stays English.
        std::fs::write(&user, br#"{"language": "de"}"#).expect("write");
        std::fs::write(&system, b"also broken").expect("write");
        assert_eq!(resolve_language(&[user.clone(), system.clone()]), "en");
        // Files beyond the cap are ignored, never read into memory.
        std::fs::write(&user, [b' '; (LANGUAGE_FILE_LIMIT + 1) as usize]).expect("write");
        std::fs::write(&system, br#"{"language": "fr"}"#).expect("write");
        assert_eq!(resolve_language(&[user, system]), "fr");
    }
}
