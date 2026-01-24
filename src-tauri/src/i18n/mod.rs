//! Internationalization module
//!
//! Provides translations for French (fr) and English (en) languages.
//! Supports automatic language detection based on system locale.

mod en;
mod fr;

use std::collections::HashMap;

/// Internationalization manager
pub struct I18n {
    current_lang: String,
    translations: HashMap<String, String>,
}

impl I18n {
    /// Create a new I18n instance with the specified language
    pub fn new(lang: &str) -> Self {
        let mut i18n = Self {
            current_lang: String::new(),
            translations: HashMap::new(),
        };
        i18n.set_language(lang);
        i18n
    }

    /// Set the current language
    pub fn set_language(&mut self, lang: &str) {
        let lang = if lang == "auto" {
            self.detect_system_language()
        } else {
            lang.to_string()
        };

        self.current_lang = lang.clone();
        self.translations = match lang.as_str() {
            "fr" => fr::get_translations(),
            "en" | _ => en::get_translations(),
        };

        log::info!("Language set to: {}", self.current_lang);
    }

    /// Get a translated string by key
    pub fn get(&self, key: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Get all translations
    pub fn get_all(&self) -> HashMap<String, String> {
        self.translations.clone()
    }

    /// Get the current language code
    pub fn current_language(&self) -> &str {
        &self.current_lang
    }

    /// Get available languages
    pub fn available_languages() -> Vec<(&'static str, &'static str)> {
        vec![("en", "English"), ("fr", "Fran\u{00E7}ais")]
    }

    /// Detect system language
    fn detect_system_language(&self) -> String {
        // Try to detect from environment variables
        let lang_env = std::env::var("LANG")
            .or_else(|_| std::env::var("LC_ALL"))
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .unwrap_or_else(|_| "en".to_string());

        // Extract language code (e.g., "fr_FR.UTF-8" -> "fr")
        let lang_code = lang_env
            .split('_')
            .next()
            .unwrap_or("en")
            .split('.')
            .next()
            .unwrap_or("en");

        // Only return supported languages
        match lang_code {
            "fr" => "fr".to_string(),
            _ => "en".to_string(),
        }
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::new("auto")
    }
}
