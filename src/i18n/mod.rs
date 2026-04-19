use serde_json::Value;
use std::{
    collections::HashMap,
    sync::OnceLock,
};

// ── Embedded locale files ────────────────────────────────────────────────────

const EN_US_UI:      &str = include_str!("../../i18n/en-US/ui.json");
const EN_US_LCD:     &str = include_str!("../../i18n/en-US/lcd.json");
const EN_US_SERVICE: &str = include_str!("../../i18n/en-US/service.json");
const EN_US_ERRORS:  &str = include_str!("../../i18n/en-US/errors.json");
const FR_FR_UI:      &str = include_str!("../../i18n/fr-FR/ui.json");
const FR_FR_LCD:     &str = include_str!("../../i18n/fr-FR/lcd.json");
const FR_FR_SERVICE: &str = include_str!("../../i18n/fr-FR/service.json");
const FR_FR_ERRORS:  &str = include_str!("../../i18n/fr-FR/errors.json");

// ── Global catalog ───────────────────────────────────────────────────────────

type LocaleMap = HashMap<String, String>;

static CATALOG: OnceLock<Catalog> = OnceLock::new();

struct Catalog {
    primary_locale: String,
    primary: LocaleMap,
    fallback: LocaleMap,
}

/// Initialize the global i18n catalog with the given locale.
///
/// `locale` must be `"en-US"` or `"fr-FR"`.
/// Subsequent calls are no-ops — the [`OnceLock`] is set on the first call.
pub fn init(locale: &str) {
    CATALOG.get_or_init(|| build_catalog(locale));
}

/// Translate `key` using the active locale.
///
/// Key format: `<namespace>.<path>` where `<namespace>` is the JSON file
/// stem (e.g. `"ui"`, `"errors"`, `"service"`, `"lcd"`) and `<path>` is
/// the dotted key path within that file.
///
/// Examples:
/// - `"ui.app.title"` → file `ui.json`, key `app.title`
/// - `"errors.error.invalid_ip"` → file `errors.json`, key `error.invalid_ip`
///
/// Resolution order: primary locale → `en-US` fallback → raw key string.
pub fn t(key: &str) -> String {
    let catalog = CATALOG.get_or_init(|| build_catalog("en-US"));
    catalog
        .primary
        .get(key)
        .or_else(|| catalog.fallback.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// Return the active locale tag (e.g. `"en-US"`).
pub fn locale() -> &'static str {
    CATALOG.get().map(|c| c.primary_locale.as_str()).unwrap_or("en-US")
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn build_catalog(locale: &str) -> Catalog {
    let primary = load_locale(locale);
    let fallback = if locale != "en-US" {
        load_locale("en-US")
    } else {
        HashMap::new()
    };
    Catalog {
        primary_locale: locale.to_owned(),
        primary,
        fallback,
    }
}

/// Load all embedded JSON files for `locale` into a flat key map.
///
/// Each file's logical stem becomes the namespace prefix (e.g. `"ui"`, `"errors"`).
/// Resulting keys: `<namespace>.<dotted.path.within.file>`.
/// Both flat and nested JSON objects are supported.
fn load_locale(locale: &str) -> LocaleMap {
    let mut map = LocaleMap::new();
    let files: &[(&str, &str)] = match locale {
        "en-US" => &[
            ("ui",      EN_US_UI),
            ("lcd",     EN_US_LCD),
            ("service", EN_US_SERVICE),
            ("errors",  EN_US_ERRORS),
        ],
        "fr-FR" => &[
            ("ui",      FR_FR_UI),
            ("lcd",     FR_FR_LCD),
            ("service", FR_FR_SERVICE),
            ("errors",  FR_FR_ERRORS),
        ],
        _ => return map,
    };
    for (namespace, content) in files {
        if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(content) {
            for (k, v) in &obj {
                flatten_into(v, &format!("{namespace}.{k}"), &mut map);
            }
        }
    }
    map
}

/// Recursively flatten `value` into `map` under `prefix`.
///
/// String leaves are inserted at `prefix`; object values recurse appending
/// each child key with a `.` separator.  Other JSON types are ignored.
fn flatten_into(value: &Value, prefix: &str, map: &mut LocaleMap) {
    match value {
        Value::String(s) => {
            map.insert(prefix.to_owned(), s.clone());
        }
        Value::Object(obj) => {
            for (k, v) in obj {
                flatten_into(v, &format!("{prefix}.{k}"), map);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── flatten_into ─────────────────────────────────────────────────────────

    #[test]
    fn test_flatten_into_flat_string() {
        let mut map = LocaleMap::new();
        flatten_into(&Value::String("AtlasIP".into()), "ui.app.title", &mut map);
        assert_eq!(map.get("ui.app.title").map(String::as_str), Some("AtlasIP"));
    }

    #[test]
    fn test_flatten_into_nested_object() {
        let mut map = LocaleMap::new();
        let v = serde_json::json!({"title": "AtlasIP", "version": "0.3.0"});
        flatten_into(&v, "ui.app", &mut map);
        assert_eq!(map.get("ui.app.title").map(String::as_str), Some("AtlasIP"));
        assert_eq!(map.get("ui.app.version").map(String::as_str), Some("0.3.0"));
    }

    #[test]
    fn test_flatten_into_ignores_non_string_leaves() {
        let mut map = LocaleMap::new();
        flatten_into(&Value::Bool(true), "prefix", &mut map);
        flatten_into(&serde_json::json!([1, 2]), "prefix2", &mut map);
        assert!(map.is_empty());
    }

    // ── Locale loading ────────────────────────────────────────────────────────

    #[test]
    fn test_load_locale_en_us_has_ui_keys() {
        let map = load_locale("en-US");
        assert_eq!(map.get("ui.nav.analysis").map(String::as_str), Some("Analysis"));
        assert_eq!(map.get("errors.error.invalid_ip").map(String::as_str), Some("Invalid IP address"));
        assert_eq!(map.get("service.windows.installed").map(String::as_str), Some("AtlasIP Service installed successfully."));
        // Note: service.json keys are stored WITHOUT the "service." prefix that was
        // mistakenly present before Bloc 12 — namespace adds it automatically.
    }

    #[test]
    fn test_load_locale_fr_fr_has_ui_keys() {
        let map = load_locale("fr-FR");
        assert_eq!(map.get("ui.nav.analysis").map(String::as_str), Some("Analyse"));
    }

    #[test]
    fn test_load_locale_keys_are_namespaced() {
        let map = load_locale("en-US");
        assert!(map.keys().any(|k| k.starts_with("ui.")));
        assert!(map.keys().any(|k| k.starts_with("errors.")));
        assert!(map.keys().any(|k| k.starts_with("service.")));
        // lcd.json is currently empty (no LCD source code uses i18n yet)
    }

    #[test]
    fn test_load_locale_unknown_returns_empty() {
        let map = load_locale("xx-XX");
        assert!(map.is_empty());
    }

    // ── Fallback ─────────────────────────────────────────────────────────────

    #[test]
    fn test_build_catalog_fr_fr_has_non_empty_fallback() {
        let catalog = build_catalog("fr-FR");
        // primary is fr-FR; fallback is en-US (non-empty)
        assert!(!catalog.fallback.is_empty());
        assert!(catalog.fallback.contains_key("ui.nav.analysis"));
    }

    #[test]
    fn test_build_catalog_en_us_has_empty_fallback() {
        let catalog = build_catalog("en-US");
        // No double-loading when primary is already en-US
        assert!(catalog.fallback.is_empty());
    }

    #[test]
    fn test_build_catalog_fr_fr_primary_wins_over_fallback() {
        let catalog = build_catalog("fr-FR");
        // "Analyse" (fr-FR) should be in primary, "Analysis" (en-US) in fallback
        assert_eq!(catalog.primary.get("ui.nav.analysis").map(String::as_str), Some("Analyse"));
        assert_eq!(catalog.fallback.get("ui.nav.analysis").map(String::as_str), Some("Analysis"));
    }

    // ── Nested resolution ─────────────────────────────────────────────────────

    #[test]
    fn test_load_locale_all_values_are_non_empty() {
        // No key in a valid locale should have an empty string value.
        let map = load_locale("en-US");
        for (k, v) in &map {
            assert!(!v.is_empty(), "empty value for key: {k}");
        }
    }

    #[test]
    fn test_load_locale_en_us_and_fr_fr_same_key_count() {
        // Both locales must expose the same set of keys (parity check).
        let en = load_locale("en-US");
        let fr = load_locale("fr-FR");
        assert_eq!(
            en.len(), fr.len(),
            "en-US has {} keys, fr-FR has {} — they must match",
            en.len(), fr.len()
        );
    }

    // ── Missing key ───────────────────────────────────────────────────────────

    #[test]
    fn test_t_returns_key_when_absent() {
        let key = "nonexistent.__bloc11_test__.key";
        assert_eq!(t(key), key);
    }

    #[test]
    fn test_build_catalog_returns_key_when_absent_in_both_locales() {
        let catalog = build_catalog("fr-FR");
        let key = "nonexistent.__bloc12_test__.key";
        let result = catalog.primary.get(key)
            .or_else(|| catalog.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned());
        assert_eq!(result, key);
    }
}
