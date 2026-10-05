//! Structural validation of the configuration files the daemon writes.
//!
//! Only structure is checked here: the text must parse as a JSON object, stay
//! within the 1,000,000-byte cap (the greeter's own Theme.qml loadFile cap),
//! and use only known top-level sections (`colors`,
//! `background`, `font`, `layout` for theme.json; `language` for
//! waylight.json). Token-level validation deliberately stays fail-open inside
//! the greeter's `Theme.qml` — this daemon never duplicates that validator, so
//! the two can never disagree about a value. Unknown keys inside known
//! sections are accepted and round-trip untouched.

use crate::i18n::LANGUAGES;
use serde_json::Value;

// Matches Theme.qml's loadFile cap, which ignores any file over 1,000,000
// responseText characters. The daemon caps the same size in bytes so a file
// it accepts can never be wholly ignored by the greeter: UTF-8 characters
// are at least one byte, so ≤ 1,000,000 bytes is always ≤ 1,000,000 chars.
pub const MAX_JSON_BYTES: usize = 1_000_000;

const THEME_SECTIONS: [&str; 4] = ["colors", "background", "font", "layout"];
const CONFIG_SECTIONS: [&str; 1] = ["language"];

fn object(json: &str, what: &str) -> Result<(), String> {
    if json.len() > MAX_JSON_BYTES {
        return Err(format!("{what} is over the {MAX_JSON_BYTES}-byte limit"));
    }
    let value: Value =
        serde_json::from_str(json).map_err(|error| format!("{what} is not valid JSON: {error}"))?;
    if !value.is_object() {
        return Err(format!("{what} must be a JSON object"));
    }
    Ok(())
}

fn sections(value: &str, known: &[&str], what: &str) -> Result<(), String> {
    let value: Value = serde_json::from_str(value).expect("already parsed by object()");
    let map = value.as_object().expect("already checked as object");
    for key in map.keys() {
        if !known.contains(&key.as_str()) {
            let list = known.join(", ");
            return Err(format!(
                "{what} has unknown section '{key}'; expected one of {list}"
            ));
        }
    }
    Ok(())
}

/// Structural validation for theme.json. Every section must be a JSON
/// object; unknown keys inside the known sections are the greeter's
/// fail-open business, not ours.
pub fn theme(json: &str) -> Result<(), String> {
    object(json, "theme")?;
    sections(json, &THEME_SECTIONS, "theme")?;
    let value: Value = serde_json::from_str(json).expect("already parsed by object()");
    let map = value.as_object().expect("already checked as object");
    for key in THEME_SECTIONS {
        if let Some(section) = map.get(key)
            && !section.is_object()
        {
            return Err(format!("theme section '{key}' must be a JSON object"));
        }
    }
    Ok(())
}

/// Structural contract for hand-written waylight.json. The daemon itself
/// always writes the canonical `{"language": "<code>"}` form (see
/// `ConfigService::store_language`), so this check is not on the daemon's
/// own write path: it exists as the documented structural contract a
/// hand-edited file must satisfy for the greeter to pick it up (JSON object,
/// only the `language` key, string value; the supported codes are enforced
/// by `language` when a value is submitted through SetLanguage). The
/// greeter's own reader stays fail-open and never calls this.
pub fn config(json: &str) -> Result<(), String> {
    object(json, "config")?;
    sections(json, &CONFIG_SECTIONS, "config")?;
    let value: Value = serde_json::from_str(json).expect("already parsed by object()");
    if let Some(language) = value.get("language")
        && !language.is_string()
    {
        return Err(String::from("config key 'language' must be a string"));
    }
    Ok(())
}

/// The `language` value must be one of the codes the greeter supports;
/// anything else would silently fail open to English and only confuse the
/// user, so the daemon rejects it up front.
pub fn language(code: &str) -> Result<(), String> {
    if LANGUAGES.contains(&code) {
        Ok(())
    } else {
        Err(format!(
            "unsupported language '{code}'; supported: {}",
            LANGUAGES.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_accepts_known_sections_and_unknown_keys() {
        assert_eq!(theme("{}"), Ok(()));
        assert_eq!(theme(r#"{"colors": {}}"#), Ok(()));
        assert_eq!(
            theme(
                r##"{"colors": {"accent": "#ff0000", "madeUp": 1}, "layout": {"tileWidth": 84}}"##
            ),
            Ok(())
        );
        // Sections must be objects.
        assert_eq!(
            theme(r##"{"colors": {"accent": "#ff0000"}, "font": "sans-serif"}"##),
            Err(String::from("theme section 'font' must be a JSON object"))
        );
        assert_eq!(
            theme(r#"{"colors": []}"#),
            Err(String::from("theme section 'colors' must be a JSON object"))
        );
    }

    #[test]
    fn theme_rejects_unknown_sections_and_bad_structure() {
        assert!(
            theme(r#"{"preset": "midnight"}"#)
                .unwrap_err()
                .contains("unknown section 'preset'")
        );
        assert!(
            theme(r#"{"language": "ar"}"#)
                .unwrap_err()
                .contains("unknown section 'language'")
        );
        assert!(theme("[]").unwrap_err().contains("must be a JSON object"));
        assert!(theme("not json").unwrap_err().contains("not valid JSON"));
        assert!(theme("").unwrap_err().contains("not valid JSON"));
    }

    #[test]
    fn theme_enforces_the_size_cap() {
        let oversized = format!(
            "{{\"colors\": {{\"a\": \"{}\"}}}}",
            "x".repeat(MAX_JSON_BYTES)
        );
        assert!(
            theme(&oversized)
                .unwrap_err()
                .contains("1000000-byte limit")
        );
        // Exact boundary: a file of exactly MAX_JSON_BYTES bytes passes. It
        // can never be wholly ignored by the greeter, whose cap only drops
        // files over 1,000,000 characters (characters are at least one byte).
        // The JSON wrapper around the payload is 21 bytes.
        let exact = format!(
            "{{\"colors\": {{\"a\": \"{}\"}}}}",
            "x".repeat(MAX_JSON_BYTES - 21)
        );
        assert_eq!(exact.len(), MAX_JSON_BYTES);
        assert_eq!(theme(&exact), Ok(()));
        // One byte more is rejected.
        let over = format!(
            "{{\"colors\": {{\"a\": \"{}\"}}}}",
            "x".repeat(MAX_JSON_BYTES - 20)
        );
        assert_eq!(over.len(), MAX_JSON_BYTES + 1);
        assert!(theme(&over).is_err());
    }

    #[test]
    fn config_accepts_only_the_language_section() {
        assert_eq!(config("{}"), Ok(()));
        assert_eq!(config(r#"{"language": "ar"}"#), Ok(()));
        assert!(
            config(r#"{"colors": {}}"#)
                .unwrap_err()
                .contains("unknown section")
        );
        assert!(
            config(r#"{"language": 5}"#)
                .unwrap_err()
                .contains("must be a string")
        );
        assert!(
            config("null")
                .unwrap_err()
                .contains("must be a JSON object")
        );
    }

    #[test]
    fn language_must_be_a_supported_code() {
        for code in [
            "en", "zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg",
        ] {
            assert_eq!(language(code), Ok(()));
        }
        assert!(language("de").unwrap_err().contains("unsupported language"));
        assert!(language("").unwrap_err().contains("unsupported language"));
        assert!(language("AR").unwrap_err().contains("unsupported language"));
    }
}
