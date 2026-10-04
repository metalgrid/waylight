// Qt Linguist plumbing for the greeter's UI language. The waylight.json
// resolution itself lives in main.rs; this bridge only talks to Qt so a
// missing catalog is reported to the caller instead of being fatal.

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
