use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Loaded translation table for one language.
#[derive(Debug, Clone)]
pub struct Translations {
    lang: String,
    map: HashMap<String, String>,
}

impl Translations {
    /// Load translations from `locales/{lang}.json`.
    pub fn load(lang: &str) -> Result<Self> {
        // TODO: resolve path to `locales/{lang}.json` (relative to the binary
        // or a well-known directory), read the file, parse JSON, and flatten
        // keys into `map`.
        todo!("Translations::load")
    }

    /// Return the translated string for `key`, falling back to the key itself
    /// if no translation exists.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.map.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    pub fn lang(&self) -> &str {
        &self.lang
    }
}
