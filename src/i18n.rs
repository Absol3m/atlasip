use serde_json::Value;
use std::{
    collections::HashMap,
    fmt,
    fs,
    path::{Path, PathBuf},
};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum I18nError {
    IoError(String),
    ParseError(String),
}

impl fmt::Display for I18nError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I18nError::IoError(s) => write!(f, "I18n I/O error: {s}"),
            I18nError::ParseError(s) => write!(f, "I18n parse error: {s}"),
        }
    }
}

impl std::error::Error for I18nError {}

// ---------------------------------------------------------------------------
// I18n struct
// ---------------------------------------------------------------------------

/// Dual-language translation table with automatic fallback.
///
/// Resolution order for [`I18n::t`]:
/// 1. `primary_map`
/// 2. `fallback_map`
/// 3. The key itself (standard i18n passthrough)
pub struct I18n {
    primary: String,
    fallback: String,
    primary_map: HashMap<String, String>,
    fallback_map: HashMap<String, String>,
}

impl I18n {
    // ── Constructors ────────────────────────────────────────────────────────

    /// Load both `primary` and `fallback` locale files from `./locales/`.
    pub fn new(primary: &str, fallback: &str) -> Result<Self, I18nError> {
        let primary_map = Self::load_locale(primary)?;
        let fallback_map = Self::load_locale(fallback)?;
        Ok(Self {
            primary: primary.to_owned(),
            fallback: fallback.to_owned(),
            primary_map,
            fallback_map,
        })
    }

    // ── Translation ─────────────────────────────────────────────────────────

    /// Return the translation for `key`.
    ///
    /// Falls back to the fallback locale, then to the key itself.
    pub fn t(&self, key: &str) -> String {
        if let Some(v) = self.primary_map.get(key) {
            return v.clone();
        }
        if let Some(v) = self.fallback_map.get(key) {
            return v.clone();
        }
        key.to_owned()
    }

    // ── Locale loading ──────────────────────────────────────────────────────

    /// Load the translation map for `lang` from `./locales/{lang}.json`.
    pub fn load_locale(lang: &str) -> Result<HashMap<String, String>, I18nError> {
        let path = PathBuf::from("./locales").join(format!("{lang}.json"));
        load_locale_from_path(&path)
    }

    // ── Accessors ───────────────────────────────────────────────────────────

    pub fn primary_lang(&self) -> &str {
        &self.primary
    }

    pub fn fallback_lang(&self) -> &str {
        &self.fallback
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read and deserialise a flat JSON object at `path` into a string→string map.
///
/// Only top-level string values are included; nested objects and arrays are
/// ignored to keep the format simple and forward-compatible.
fn load_locale_from_path(path: &Path) -> Result<HashMap<String, String>, I18nError> {
    let contents =
        fs::read_to_string(path).map_err(|e| I18nError::IoError(e.to_string()))?;

    let value: Value =
        serde_json::from_str(&contents).map_err(|e| I18nError::ParseError(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| I18nError::ParseError("locale file must be a JSON object".to_owned()))?;

    let map = obj
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
        .collect();

    Ok(map)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    /// Write `contents` to `<temp_dir>/<subdir>/<filename>`, creating the
    /// directory if needed. Returns the file path.
    fn write_temp_locale(subdir: &str, filename: &str, contents: &str) -> PathBuf {
        let dir = env::temp_dir().join(subdir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(filename);
        fs::write(&path, contents).unwrap();
        path
    }

    // ── load_locale_from_path ───────────────────────────────────────────────

    #[test]
    fn test_load_valid_locale() {
        let json = r#"{"hello": "Hello", "bye": "Goodbye"}"#;
        let path = write_temp_locale("atlasip_i18n_valid", "en.json", json);

        let map = load_locale_from_path(&path).expect("should load");
        assert_eq!(map.get("hello").map(String::as_str), Some("Hello"));
        assert_eq!(map.get("bye").map(String::as_str), Some("Goodbye"));
    }

    #[test]
    fn test_missing_locale_file() {
        let path = env::temp_dir().join("atlasip_i18n_missing").join("nope.json");
        // Ensure it really does not exist.
        let _ = fs::remove_file(&path);

        let result = load_locale_from_path(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), I18nError::IoError(_)));
    }

    #[test]
    fn test_parse_error() {
        let path = write_temp_locale("atlasip_i18n_bad", "bad.json", "not json {{");

        let result = load_locale_from_path(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), I18nError::ParseError(_)));
    }

    #[test]
    fn test_parse_error_non_object() {
        // Valid JSON but not an object.
        let path = write_temp_locale("atlasip_i18n_array", "array.json", r#"["a","b"]"#);

        let result = load_locale_from_path(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), I18nError::ParseError(_)));
    }

    // ── I18n::t ─────────────────────────────────────────────────────────────

    /// Build an I18n instance directly from two in-memory maps (no file I/O).
    fn make_i18n(
        primary_entries: &[(&str, &str)],
        fallback_entries: &[(&str, &str)],
    ) -> I18n {
        I18n {
            primary: "pr".to_owned(),
            fallback: "fb".to_owned(),
            primary_map: primary_entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            fallback_map: fallback_entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_translation_primary() {
        let i18n = make_i18n(
            &[("greeting", "Bonjour"), ("bye", "Au revoir")],
            &[("greeting", "Hello"), ("bye", "Goodbye")],
        );
        // Primary takes precedence.
        assert_eq!(i18n.t("greeting"), "Bonjour");
        assert_eq!(i18n.t("bye"), "Au revoir");
    }

    #[test]
    fn test_translation_fallback() {
        let i18n = make_i18n(
            &[("only_primary", "Primaire")],
            &[("only_fallback", "Fallback value"), ("only_primary", "Primary EN")],
        );
        // Key missing in primary → use fallback.
        assert_eq!(i18n.t("only_fallback"), "Fallback value");
        // Key present in primary → primary wins even though fallback also has it.
        assert_eq!(i18n.t("only_primary"), "Primaire");
    }

    #[test]
    fn test_translation_missing_key() {
        let i18n = make_i18n(&[], &[]);
        // Neither map has the key → return the key itself.
        assert_eq!(i18n.t("unknown.key"), "unknown.key");
        assert_eq!(i18n.t(""), "");
    }

    // ── I18n::new (file-based) ───────────────────────────────────────────────

    #[test]
    fn test_i18n_new_from_files() {
        let dir = env::temp_dir().join("atlasip_i18n_new");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("fr.json"),
            r#"{"app.title": "AtlasIP FR", "only_fr": "seulement FR"}"#,
        )
        .unwrap();
        fs::write(
            dir.join("en.json"),
            r#"{"app.title": "AtlasIP EN", "only_en": "only EN"}"#,
        )
        .unwrap();

        // Load manually via helper (I18n::new uses ./locales which may not
        // exist in the test runner's CWD; test the loading logic directly).
        let primary_map = load_locale_from_path(&dir.join("fr.json")).unwrap();
        let fallback_map = load_locale_from_path(&dir.join("en.json")).unwrap();

        let i18n = I18n {
            primary: "fr".to_owned(),
            fallback: "en".to_owned(),
            primary_map,
            fallback_map,
        };

        assert_eq!(i18n.t("app.title"), "AtlasIP FR"); // primary wins
        assert_eq!(i18n.t("only_en"), "only EN");      // fallback used
        assert_eq!(i18n.t("only_fr"), "seulement FR"); // primary only
        assert_eq!(i18n.t("missing"), "missing");       // passthrough
        assert_eq!(i18n.primary_lang(), "fr");
        assert_eq!(i18n.fallback_lang(), "en");
    }

    #[test]
    fn test_non_string_values_ignored() {
        // Numbers, arrays, and nested objects in the JSON are silently dropped.
        let json = r#"{"key_str": "value", "key_num": 42, "key_arr": [1,2], "key_obj": {}}"#;
        let path = write_temp_locale("atlasip_i18n_mixed", "mixed.json", json);

        let map = load_locale_from_path(&path).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key_str").map(String::as_str), Some("value"));
        assert!(!map.contains_key("key_num"));
        assert!(!map.contains_key("key_arr"));
        assert!(!map.contains_key("key_obj"));
    }
}
