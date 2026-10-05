// UI-language plumbing shared by the greeter and the config GUI.
//
// `waylight.json` resolution (LANGUAGES, validated_language, resolve_language,
// install_language) reads the first readable candidate file and installs the
// matching translator and layout direction before any QML engine loads.
// Everything is fail-open: missing, malformed or unknown content never
// prevents startup and falls back to English. ar and ur additionally mirror
// the layout. The cxx bridge below only talks to Qt so a missing catalog is
// reported to the caller instead of being fatal.

use cxx_qt_lib::QString;
use std::{io::Read, path::Path, path::PathBuf, sync::OnceLock};

pub const LANGUAGES: [&str; 11] = [
    "en", "zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg",
];
pub const RTL_LANGUAGES: [&str; 2] = ["ar", "ur"];
// {"language": "ar"} is a few bytes; the cap keeps a runaway file from being
// read while allowing indented, commented-by-hand edits.
pub const LANGUAGE_FILE_LIMIT: u64 = 64 * 1024;

/// A candidate file wins only when it parses as a JSON object whose
/// "language" is one of the supported codes.
pub fn validated_language(contents: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(contents).ok()?;
    let requested = value.as_object()?.get("language")?.as_str()?;
    LANGUAGES
        .iter()
        .find(|language| **language == requested)
        .copied()
}

/// Decodes the candidate list produced by backend::config_language_json. A
/// malformed list yields no candidates (English) instead of failing startup.
pub fn language_candidates(paths_json: &str) -> Vec<PathBuf> {
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
pub fn resolve_language(candidates: &[PathBuf]) -> &'static str {
    for path in candidates {
        if let Some(contents) = read_limited(path, LANGUAGE_FILE_LIMIT)
            && let Some(language) = validated_language(&contents)
        {
            return language;
        }
    }
    "en"
}

static RESOLVED: OnceLock<&'static str> = OnceLock::new();

/// The language `install_language` resolved for this process; "en" before it
/// runs. QML reads this through the Backend's `uiLanguage` property so the
/// clock and other locale-sensitive formatting follow the chosen language.
pub fn installed_language() -> &'static str {
    RESOLVED.get().copied().unwrap_or("en")
}

/// Installs the translator and layout direction before the QML engine loads.
/// A missing catalog keeps English with a warning; nothing here is fatal.
pub fn install_language(paths_json: &str) -> &'static str {
    let language = resolve_language(&language_candidates(paths_json));
    let _ = RESOLVED.set(language);
    if language != "en" {
        let path = format!(":/qt/qml/Waylight/i18n/waylight_{language}.qm");
        if !ffi::waylight_install_translator(&QString::from(path)) {
            eprintln!(
                "waylight: translation for '{language}' is missing; falling back to English."
            );
        }
    }
    ffi::waylight_set_layout_direction(RTL_LANGUAGES.contains(&language));
    language
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("waylight-greeter/waylight_i18n.h");
        /// Installs a compiled catalog for the rest of the process. False when
        /// the resource is missing or unreadable (caller keeps English).
        fn waylight_install_translator(path: &QString) -> bool;
        /// Deterministic layout direction; ar/ur mirror before the engine loads.
        fn waylight_set_layout_direction(right_to_left: bool);
        /// Loads one catalog and translates a single string directly, without
        /// an application. Empty result: missing catalog or untranslated
        /// source. Unit tests only.
        #[allow(dead_code)]
        fn waylight_translate_catalog(
            path: &QString,
            context: &QString,
            source: &QString,
        ) -> QString;
    }
}

#[cfg(test)]
mod tests {
    use super::ffi;
    use cxx_qt_lib::QString;

    // The QML translation context is the file name without extension.
    const CONTEXT: &str = "Main";
    // en is the source language and intentionally ships no catalog.
    const LANGUAGES: [&str; 10] = ["zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg"];

    fn translate(language: &str, source: &str) -> String {
        ffi::waylight_translate_catalog(
            &QString::from(format!(":/qt/qml/Waylight/i18n/waylight_{language}.qm")),
            &QString::from(CONTEXT),
            &QString::from(source),
        )
        .to_string()
    }

    #[test]
    fn every_language_translates_core_strings() {
        for language in LANGUAGES {
            for source in ["Continue", "Cancel", "Sign in", "Enter answer", "Username"] {
                let translated = translate(language, source);
                assert!(
                    !translated.is_empty(),
                    "{language}: {source:?} missing from catalog"
                );
                assert_ne!(
                    translated, source,
                    "{language}: {source:?} was not translated"
                );
            }
        }
    }

    #[test]
    fn spanish_and_arabic_translations_match_the_reviewed_table() {
        assert_eq!(translate("es", "Continue"), "Continuar");
        assert_eq!(translate("es", "Enter answer"), "Introducir respuesta");
        assert_eq!(
            translate("es", "More… · Enter a username manually"),
            "Más… · Introducir un usuario manualmente"
        );
        assert_eq!(translate("ar", "Continue"), "متابعة");
        assert_eq!(translate("ar", "Cancel"), "إلغاء");
        assert_eq!(
            translate("ar", "Shut down this computer?"),
            "إيقاف تشغيل هذا الحاسوب؟"
        );
    }

    #[test]
    fn french_titles_keep_the_nbsp_before_the_question_mark() {
        // French typography: U+00A0 before "?", so the mark can never wrap
        // onto its own line in the power dialog.
        assert_eq!(
            translate("fr", "Restart this computer?"),
            "Redémarrer cet ordinateur\u{00a0}?"
        );
        assert_eq!(
            translate("fr", "Shut down this computer?"),
            "Éteindre cet ordinateur\u{00a0}?"
        );
    }

    #[test]
    fn bulgarian_continue_uses_the_standard_verb() {
        assert_eq!(translate("bg", "Continue"), "Продължи");
    }

    #[test]
    fn power_labels_translate_while_keys_stay_untranslated() {
        // objectName keys must never translate; only the display labels do.
        for (key, es, ar) in [
            ("Sleep", "Suspender", "السكون"),
            ("Restart", "Reiniciar", "إعادة التشغيل"),
            ("Shut Down", "Apagar", "إيقاف التشغيل"),
        ] {
            assert_eq!(translate("es", key), es);
            assert_eq!(translate("ar", key), ar);
        }
    }

    #[test]
    fn missing_catalogs_return_empty() {
        // No waylight_en.qm: English is the source language.
        assert!(translate("en", "Continue").is_empty());
        // Unsupported languages have no catalog either.
        assert!(translate("de", "Continue").is_empty());
    }
}

#[cfg(test)]
mod language_tests {
    use super::{LANGUAGE_FILE_LIMIT, language_candidates, resolve_language, validated_language};
    use std::path::PathBuf;

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
