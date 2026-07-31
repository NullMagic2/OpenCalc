//! Plain-text help catalog used by the Windows and Linux “What’s This?” context-help popups.
//!
//! The shipped `calc.tooltip` contains language-qualified sections such as
//! `[en.back]`, `[pt.back]`, and `[es.back]`.  Adding another translation only
//! requires another set of sections plus a Language entry; UI code continues to
//! refer to the stable semantic key (`back`, `sqrt`, `memory_indicator`, ...).

use crate::i18n::Language;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct TooltipCatalog {
    entries: BTreeMap<String, String>,
}

impl TooltipCatalog {
    /// Loads `calc.tooltip` beside the executable.
    /// A missing or malformed catalog is non-fatal; controls simply omit help text.
    pub fn load_default() -> Self {
        let Some(path) = find_tooltip_file() else {
            return Self::default();
        };
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(_) => Self::default(),
        }
    }

    pub fn get(&self, language: Language, key: &str) -> Option<&str> {
        let qualified = format!("{}.{}", language.code(), key);
        self.entries.get(&qualified).map(String::as_str)
    }

    fn parse(text: &str) -> Self {
        let mut entries = BTreeMap::new();
        let mut current_key: Option<String> = None;
        let mut current_lines: Vec<String> = Vec::new();

        let flush = |entries: &mut BTreeMap<String, String>,
                     key: &mut Option<String>,
                     lines: &mut Vec<String>| {
            let Some(name) = key.take() else {
                lines.clear();
                return;
            };
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            let first_nonblank = lines
                .iter()
                .position(|line| !line.trim().is_empty())
                .unwrap_or(lines.len());
            if first_nonblank != 0 {
                lines.drain(..first_nonblank);
            }
            let body = lines.join("\n");
            lines.clear();
            if !name.is_empty() && !body.trim().is_empty() {
                entries.insert(name, body);
            }
        };

        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
                flush(&mut entries, &mut current_key, &mut current_lines);
                current_key = Some(trimmed[1..trimmed.len() - 1].trim().to_owned());
                continue;
            }
            if current_key.is_none() {
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            current_lines.push(line.to_owned());
        }
        flush(&mut entries, &mut current_key, &mut current_lines);
        Self { entries }
    }
}

fn find_tooltip_file() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    [dir.join("calc.tooltip"), dir.join("CALC.TOOLTIP")]
        .into_iter()
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::TooltipCatalog;
    use crate::i18n::Language;

    #[test]
    fn parses_language_qualified_multiline_sections() {
        let catalog = TooltipCatalog::parse(
            "# provenance\n[en.back]\nDeletes the last digit.\nKeyboard = BACKSPACE\n\n[pt.back]\nExclui o último dígito.\n",
        );
        assert_eq!(
            catalog.get(Language::English, "back"),
            Some("Deletes the last digit.\nKeyboard = BACKSPACE")
        );
        assert_eq!(
            catalog.get(Language::Portuguese, "back"),
            Some("Exclui o último dígito.")
        );
    }


}
