//! Tiny human-readable `.cfg` persistence for Calculator preferences.
//!
//! The format is deliberately dependency-free and easy to edit by hand:
//!
//! language=en
//! decimal_separator=comma
//! history_visible=true
//! history_width=210
//! graph_visible=false

use crate::i18n::Language;
use std::fs;
use std::io;
use std::path::PathBuf;

const FILE_NAME: &str = "OpenCalc.cfg";
const LEGACY_FILE_NAME: &str = "Calculator95-Rust.cfg";

pub const DEFAULT_HISTORY_WIDTH: i32 = 210;
pub const MIN_HISTORY_WIDTH: i32 = 120;
pub const MAX_HISTORY_WIDTH: i32 = 420;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalSeparator {
    Period,
    Comma,
}

impl DecimalSeparator {
    pub const fn as_char(self) -> char {
        match self {
            Self::Period => '.',
            Self::Comma => ',',
        }
    }

    pub const fn as_cfg(self) -> &'static str {
        match self {
            Self::Period => "period",
            Self::Comma => "comma",
        }
    }

    pub fn from_char(value: char) -> Self {
        if value == ',' {
            Self::Comma
        } else {
            Self::Period
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "." | "period" | "point" | "dot" => Some(Self::Period),
            "," | "comma" => Some(Self::Comma),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Settings {
    pub language: Language,
    pub decimal_separator: DecimalSeparator,
    pub history_visible: bool,
    pub history_width: i32,
    pub graph_visible: bool,
    storage_path: PathBuf,
}

impl Settings {
    pub fn load(system_decimal_separator: char) -> Self {
        // OpenCalc writes the renamed configuration file, but reads the former
        // Calculator95-Rust.cfg once as a migration source when OpenCalc.cfg is
        // not present. The next successful save therefore completes migration
        // without discarding an existing language/history preference.
        let storage_path = preferred_storage_path();
        let load_path = settings_load_candidates()
            .into_iter()
            .find(|path| path.is_file());

        let mut settings = Self {
            language: Language::English,
            decimal_separator: DecimalSeparator::from_char(system_decimal_separator),
            history_visible: true,
            history_width: DEFAULT_HISTORY_WIDTH,
            graph_visible: false,
            storage_path,
        };

        if let Some(path) = load_path {
            if let Ok(text) = fs::read_to_string(path) {
                settings.apply_text(&text);
            }
        }
        settings
    }

    pub fn save(&mut self) -> io::Result<()> {
        let text = self.to_text();
        match fs::write(&self.storage_path, &text) {
            Ok(()) => Ok(()),
            Err(primary_error) => {
                // A portable ZIP may be placed in a non-writable directory.
                // Fall back to the current directory before reporting failure.
                let fallback = std::env::current_dir()
                    .map(|dir| dir.join(FILE_NAME))
                    .unwrap_or_else(|_| PathBuf::from(FILE_NAME));
                if fallback == self.storage_path {
                    return Err(primary_error);
                }
                fs::write(&fallback, text)?;
                self.storage_path = fallback;
                Ok(())
            }
        }
    }

    fn apply_text(&mut self, text: &str) {
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim().to_ascii_lowercase().as_str() {
                "language" => {
                    if let Some(language) = Language::parse(value) {
                        self.language = language;
                    }
                }
                "decimal_separator" => {
                    if let Some(separator) = DecimalSeparator::parse(value) {
                        self.decimal_separator = separator;
                    }
                }
                "history_visible" => {
                    if let Some(visible) = parse_bool(value) {
                        self.history_visible = visible;
                    }
                }
                "history_width" => {
                    if let Ok(width) = value.trim().parse::<i32>() {
                        self.history_width = width.clamp(MIN_HISTORY_WIDTH, MAX_HISTORY_WIDTH);
                    }
                }
                "graph_visible" => {
                    if let Some(visible) = parse_bool(value) {
                        self.graph_visible = visible;
                    }
                }
                _ => {}
            }
        }
    }

    fn to_text(&self) -> String {
        format!(
            "# OpenCalc preferences\nlanguage={}\ndecimal_separator={}\nhistory_visible={}\nhistory_width={}\ngraph_visible={}\n",
            self.language.code(),
            self.decimal_separator.as_cfg(),
            self.history_visible,
            self.history_width,
            self.graph_visible
        )
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "show" | "shown" => Some(true),
        "0" | "false" | "no" | "off" | "hide" | "hidden" => Some(false),
        _ => None,
    }
}

fn preferred_storage_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join(FILE_NAME);
        }
    }
    std::env::current_dir()
        .map(|dir| dir.join(FILE_NAME))
        .unwrap_or_else(|_| PathBuf::from(FILE_NAME))
}

fn settings_load_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(FILE_NAME));
            candidates.push(dir.join(LEGACY_FILE_NAME));
        }
    }
    if let Ok(dir) = std::env::current_dir() {
        for name in [FILE_NAME, LEGACY_FILE_NAME] {
            let path = dir.join(name);
            if !candidates.contains(&path) {
                candidates.push(path);
            }
        }
    }
    if candidates.is_empty() {
        candidates.push(PathBuf::from(FILE_NAME));
        candidates.push(PathBuf::from(LEGACY_FILE_NAME));
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_preferences_and_ignores_unknown_keys() {
        let mut settings = Settings {
            language: Language::English,
            decimal_separator: DecimalSeparator::Period,
            history_visible: true,
            history_width: DEFAULT_HISTORY_WIDTH,
            graph_visible: false,
            storage_path: PathBuf::from("unused.cfg"),
        };
        settings.apply_text(
            "# comment\nlanguage=es\ndecimal_separator=comma\nhistory_visible=false\nhistory_width=333\ngraph_visible=true\nfuture_option=ignored\n",
        );
        assert_eq!(settings.language, Language::Spanish);
        assert_eq!(settings.decimal_separator, DecimalSeparator::Comma);
        assert!(!settings.history_visible);
        assert_eq!(settings.history_width, 333);
        assert!(settings.graph_visible);
    }

    #[test]
    fn clamps_persisted_history_width_to_supported_range() {
        let mut settings = Settings {
            language: Language::English,
            decimal_separator: DecimalSeparator::Period,
            history_visible: true,
            history_width: DEFAULT_HISTORY_WIDTH,
            graph_visible: false,
            storage_path: PathBuf::from("unused.cfg"),
        };
        settings.apply_text("history_width=9999\n");
        assert_eq!(settings.history_width, MAX_HISTORY_WIDTH);
        settings.apply_text("history_width=1\n");
        assert_eq!(settings.history_width, MIN_HISTORY_WIDTH);
    }

    #[test]
    fn serializes_a_stable_human_readable_cfg() {
        let settings = Settings {
            language: Language::Portuguese,
            decimal_separator: DecimalSeparator::Comma,
            history_visible: false,
            history_width: 275,
            graph_visible: true,
            storage_path: PathBuf::from("unused.cfg"),
        };
        let text = settings.to_text();
        assert!(text.contains("language=pt\n"));
        assert!(text.contains("decimal_separator=comma\n"));
        assert!(text.contains("history_visible=false\n"));
        assert!(text.contains("history_width=275\n"));
        assert!(text.contains("graph_visible=true\n"));
    }

    #[test]
    fn renamed_configuration_uses_opencalc_and_retains_legacy_migration_name() {
        assert_eq!(FILE_NAME, "OpenCalc.cfg");
        assert_eq!(LEGACY_FILE_NAME, "Calculator95-Rust.cfg");
        let candidates = settings_load_candidates();
        let names: Vec<_> = candidates
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect();
        assert!(names.contains(&FILE_NAME));
        assert!(names.contains(&LEGACY_FILE_NAME));
    }
}
