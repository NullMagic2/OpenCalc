//! Tiny human-readable `.cfg` persistence for Calculator preferences.
//!
//! Linux writes `OpenCalc.cfg` beside the executable when possible. If that
//! directory is not writable, it creates `$HOME/.OpenCalc/OpenCalc.cfg` and
//! keeps using that fallback for the rest of the process.
//!
//! The format is deliberately dependency-free and easy to edit by hand:
//!
//! mode=scientific
//! language=en
//! decimal_separator=comma
//! history_visible=true
//! graph_visible=false
//! # Windows only:
//! history_width=210

use crate::calc::Mode;
use crate::i18n::Language;
use std::fs;
#[cfg(target_os = "linux")]
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::path::Path;

const FILE_NAME: &str = "OpenCalc.cfg";
#[cfg(target_os = "linux")]
const LINUX_FALLBACK_DIR_NAME: &str = ".OpenCalc";
#[cfg(not(target_os = "linux"))]
pub const DEFAULT_HISTORY_WIDTH: i32 = 210;
#[cfg(not(target_os = "linux"))]
pub const MIN_HISTORY_WIDTH: i32 = 120;
#[cfg(not(target_os = "linux"))]
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
    pub mode: Mode,
    pub language: Language,
    pub decimal_separator: DecimalSeparator,
    pub history_visible: bool,
    #[cfg(not(target_os = "linux"))]
    pub history_width: i32,
    pub graph_visible: bool,
    storage_path: PathBuf,
}

impl Settings {
    pub fn load(system_decimal_separator: char) -> Self {
        let primary_path = storage_path();

        #[cfg(target_os = "linux")]
        {
            return Self::load_linux(
                system_decimal_separator,
                primary_path,
                linux_fallback_storage_path(),
            );
        }

        #[cfg(not(target_os = "linux"))]
        {
            let mut settings = Self::defaults(system_decimal_separator, primary_path);
            if let Ok(text) = fs::read_to_string(&settings.storage_path) {
                settings.apply_text(&text);
            }
            settings
        }
    }

    fn defaults(system_decimal_separator: char, storage_path: PathBuf) -> Self {
        Self {
            mode: Mode::Standard,
            language: Language::English,
            decimal_separator: DecimalSeparator::from_char(system_decimal_separator),
            history_visible: true,
            #[cfg(not(target_os = "linux"))]
            history_width: DEFAULT_HISTORY_WIDTH,
            graph_visible: false,
            storage_path,
        }
    }

    #[cfg(target_os = "linux")]
    fn load_linux(
        system_decimal_separator: char,
        primary_path: PathBuf,
        fallback_path: Option<PathBuf>,
    ) -> Self {
        // Select the file that can actually be updated before reading settings.
        // This keeps loading and saving on the same path instead of reading a
        // stale executable-side file and later writing different values home.
        let storage_path = select_linux_storage_path(&primary_path, fallback_path.as_deref())
            .unwrap_or_else(|_| primary_path.clone());
        let mut settings = Self::defaults(system_decimal_separator, storage_path.clone());

        if let Some(text) = read_nonempty_settings(&storage_path) {
            settings.apply_text(&text);
            return settings;
        }

        // A newly writable executable directory may contain no CFG while a
        // previous run used the home fallback. Restore those values once and
        // migrate them to the now-active location.
        let alternate_path = if storage_path == primary_path {
            fallback_path.as_deref()
        } else {
            Some(primary_path.as_path())
        };
        if let Some(text) = alternate_path.and_then(read_nonempty_settings) {
            settings.apply_text(&text);
        }

        // Create a normalized CFG for defaults or migrate the alternate file.
        let _ = settings.save();
        settings
    }

    pub fn save(&mut self) -> io::Result<()> {
        let text = self.to_text();

        #[cfg(target_os = "linux")]
        {
            let fallback_path = linux_fallback_storage_path();
            self.storage_path = write_linux_settings(
                &self.storage_path,
                &text,
                fallback_path.as_deref(),
            )?;
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            fs::write(&self.storage_path, text)
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
                "mode" => {
                    if let Some(mode) = parse_mode(value) {
                        self.mode = mode;
                    }
                }
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
                #[cfg(not(target_os = "linux"))]
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

    #[cfg(target_os = "linux")]
    fn to_text(&self) -> String {
        format!(
            "# OpenCalc preferences\nmode={}\nlanguage={}\ndecimal_separator={}\nhistory_visible={}\ngraph_visible={}\n",
            mode_as_cfg(self.mode),
            self.language.code(),
            self.decimal_separator.as_cfg(),
            self.history_visible,
            self.graph_visible
        )
    }

    #[cfg(not(target_os = "linux"))]
    fn to_text(&self) -> String {
        format!(
            "# OpenCalc preferences\nmode={}\nlanguage={}\ndecimal_separator={}\nhistory_visible={}\nhistory_width={}\ngraph_visible={}\n",
            mode_as_cfg(self.mode),
            self.language.code(),
            self.decimal_separator.as_cfg(),
            self.history_visible,
            self.history_width,
            self.graph_visible
        )
    }
}

fn mode_as_cfg(mode: Mode) -> &'static str {
    match mode {
        Mode::Standard => "standard",
        Mode::Scientific => "scientific",
    }
}

fn parse_mode(value: &str) -> Option<Mode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "standard" | "std" => Some(Mode::Standard),
        "scientific" | "sci" => Some(Mode::Scientific),
        _ => None,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "show" | "shown" => Some(true),
        "0" | "false" | "no" | "off" | "hide" | "hidden" => Some(false),
        _ => None,
    }
}

fn storage_path() -> PathBuf {
    if let Some(path) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join(FILE_NAME)))
    {
        return path;
    }

    #[cfg(target_os = "linux")]
    if let Some(path) = linux_fallback_storage_path() {
        return path;
    }

    PathBuf::from(FILE_NAME)
}

#[cfg(target_os = "linux")]
fn linux_fallback_storage_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .map(|home| home.join(LINUX_FALLBACK_DIR_NAME).join(FILE_NAME))
}


#[cfg(target_os = "linux")]
fn select_linux_storage_path(
    primary_path: &Path,
    fallback_path: Option<&Path>,
) -> io::Result<PathBuf> {
    match make_settings_file_writable(primary_path, false) {
        Ok(()) => Ok(primary_path.to_path_buf()),
        Err(primary_error) => {
            let Some(fallback_path) = fallback_path.filter(|path| *path != primary_path) else {
                return Err(primary_error);
            };
            make_settings_file_writable(fallback_path, true).map_err(|fallback_error| {
                combined_storage_error(
                    primary_path,
                    &primary_error,
                    fallback_path,
                    fallback_error,
                )
            })?;
            Ok(fallback_path.to_path_buf())
        }
    }
}

#[cfg(target_os = "linux")]
fn make_settings_file_writable(path: &Path, create_parent: bool) -> io::Result<()> {
    if create_parent {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
}

#[cfg(target_os = "linux")]
fn read_nonempty_settings(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .filter(|text| !text.trim().is_empty())
}

#[cfg(target_os = "linux")]
fn write_linux_settings(
    primary_path: &Path,
    text: &str,
    fallback_path: Option<&Path>,
) -> io::Result<PathBuf> {
    match fs::write(primary_path, text) {
        Ok(()) => Ok(primary_path.to_path_buf()),
        Err(primary_error) => {
            let Some(fallback_path) = fallback_path.filter(|path| *path != primary_path) else {
                return Err(primary_error);
            };

            if let Some(parent) = fallback_path.parent() {
                if let Err(fallback_error) = fs::create_dir_all(parent) {
                    return Err(combined_storage_error(
                        primary_path,
                        &primary_error,
                        fallback_path,
                        fallback_error,
                    ));
                }
            }

            match fs::write(fallback_path, text) {
                Ok(()) => Ok(fallback_path.to_path_buf()),
                Err(fallback_error) => Err(combined_storage_error(
                    primary_path,
                    &primary_error,
                    fallback_path,
                    fallback_error,
                )),
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn combined_storage_error(
    primary_path: &Path,
    primary_error: &io::Error,
    fallback_path: &Path,
    fallback_error: io::Error,
) -> io::Error {
    io::Error::new(
        fallback_error.kind(),
        format!(
            "could not write {} ({primary_error}); could not write fallback {} ({fallback_error})",
            primary_path.display(),
            fallback_path.display()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_preferences_and_ignores_unknown_keys() {
        let mut settings = Settings {
            mode: Mode::Standard,
            language: Language::English,
            decimal_separator: DecimalSeparator::Period,
            history_visible: true,
            #[cfg(not(target_os = "linux"))]
            history_width: DEFAULT_HISTORY_WIDTH,
            graph_visible: false,
            storage_path: PathBuf::from("unused.cfg"),
        };
        settings.apply_text(
            "# comment\nmode=scientific\nlanguage=es\ndecimal_separator=comma\nhistory_visible=false\nhistory_width=333\ngraph_visible=true\nfuture_option=ignored\n",
        );
        assert_eq!(settings.mode, Mode::Scientific);
        assert_eq!(settings.language, Language::Spanish);
        assert_eq!(settings.decimal_separator, DecimalSeparator::Comma);
        assert!(!settings.history_visible);
        #[cfg(not(target_os = "linux"))]
        assert_eq!(settings.history_width, 333);
        assert!(settings.graph_visible);
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn clamps_persisted_history_width_to_supported_range() {
        let mut settings = Settings {
            mode: Mode::Standard,
            language: Language::English,
            decimal_separator: DecimalSeparator::Period,
            history_visible: true,
            #[cfg(not(target_os = "linux"))]
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
            mode: Mode::Scientific,
            language: Language::Portuguese,
            decimal_separator: DecimalSeparator::Comma,
            history_visible: false,
            #[cfg(not(target_os = "linux"))]
            history_width: 275,
            graph_visible: true,
            storage_path: PathBuf::from("unused.cfg"),
        };
        let text = settings.to_text();
        assert!(text.contains("mode=scientific\n"));
        assert!(text.contains("language=pt\n"));
        assert!(text.contains("decimal_separator=comma\n"));
        assert!(text.contains("history_visible=false\n"));
        #[cfg(not(target_os = "linux"))]
        assert!(text.contains("history_width=275\n"));
        #[cfg(target_os = "linux")]
        assert!(!text.contains("history_width="));
        assert!(text.contains("graph_visible=true\n"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_save_prefers_the_executable_directory() {
        let root = unique_test_directory("primary");
        let primary = root.join("app").join(FILE_NAME);
        let fallback = root.join(LINUX_FALLBACK_DIR_NAME).join(FILE_NAME);
        fs::create_dir_all(primary.parent().unwrap()).unwrap();

        let written = write_linux_settings(&primary, "language=en\n", Some(&fallback)).unwrap();
        assert_eq!(written, primary);
        assert_eq!(fs::read_to_string(&written).unwrap(), "language=en\n");
        assert!(!fallback.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_save_creates_and_uses_the_home_fallback_after_a_write_failure() {
        let root = unique_test_directory("fallback");
        let primary = root.join("missing-parent").join(FILE_NAME);
        let fallback = root.join(LINUX_FALLBACK_DIR_NAME).join(FILE_NAME);

        let written = write_linux_settings(&primary, "language=pt\n", Some(&fallback)).unwrap();
        assert_eq!(written, fallback);
        assert_eq!(fs::read_to_string(&written).unwrap(), "language=pt\n");

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_load_restores_preferences_from_the_executable_directory() {
        let root = unique_test_directory("load-primary");
        let primary = root.join("app").join(FILE_NAME);
        let fallback = root.join(LINUX_FALLBACK_DIR_NAME).join(FILE_NAME);
        fs::create_dir_all(primary.parent().unwrap()).unwrap();
        fs::write(
            &primary,
            "mode=scientific\nlanguage=es\ndecimal_separator=comma\nhistory_visible=false\ngraph_visible=true\n",
        )
        .unwrap();

        let settings = Settings::load_linux('.', primary.clone(), Some(fallback));
        assert_eq!(settings.storage_path, primary);
        assert_eq!(settings.mode, Mode::Scientific);
        assert_eq!(settings.language, Language::Spanish);
        assert_eq!(settings.decimal_separator, DecimalSeparator::Comma);
        assert!(!settings.history_visible);
        assert!(settings.graph_visible);

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_load_restores_preferences_from_home_when_primary_is_not_writable() {
        let root = unique_test_directory("load-fallback");
        fs::create_dir_all(&root).unwrap();
        let blocked_parent = root.join("not-a-directory");
        fs::write(&blocked_parent, "block").unwrap();
        let primary = blocked_parent.join(FILE_NAME);
        let fallback = root.join(LINUX_FALLBACK_DIR_NAME).join(FILE_NAME);
        fs::create_dir_all(fallback.parent().unwrap()).unwrap();
        fs::write(
            &fallback,
            "mode=scientific\nlanguage=pt\ndecimal_separator=comma\nhistory_visible=false\ngraph_visible=true\n",
        )
        .unwrap();

        let settings = Settings::load_linux('.', primary, Some(fallback.clone()));
        assert_eq!(settings.storage_path, fallback);
        assert_eq!(settings.mode, Mode::Scientific);
        assert_eq!(settings.language, Language::Portuguese);
        assert_eq!(settings.decimal_separator, DecimalSeparator::Comma);
        assert!(!settings.history_visible);
        assert!(settings.graph_visible);

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_load_migrates_home_preferences_to_a_new_writable_primary_file() {
        let root = unique_test_directory("load-migrate");
        let primary = root.join("app").join(FILE_NAME);
        let fallback = root.join(LINUX_FALLBACK_DIR_NAME).join(FILE_NAME);
        fs::create_dir_all(primary.parent().unwrap()).unwrap();
        fs::create_dir_all(fallback.parent().unwrap()).unwrap();
        fs::write(
            &fallback,
            "mode=scientific\nlanguage=es\ndecimal_separator=comma\nhistory_visible=false\ngraph_visible=true\n",
        )
        .unwrap();

        let settings = Settings::load_linux('.', primary.clone(), Some(fallback));
        assert_eq!(settings.storage_path, primary);
        assert_eq!(settings.mode, Mode::Scientific);
        assert_eq!(settings.language, Language::Spanish);
        assert_eq!(settings.decimal_separator, DecimalSeparator::Comma);
        assert!(!settings.history_visible);
        assert!(settings.graph_visible);
        let migrated = fs::read_to_string(&settings.storage_path).unwrap();
        assert!(migrated.contains("mode=scientific\n"));
        assert!(migrated.contains("language=es\n"));
        assert!(migrated.contains("history_visible=false\n"));
        assert!(migrated.contains("graph_visible=true\n"));

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    fn unique_test_directory(name: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "opencalc-settings-{name}-{}-{nonce}",
            std::process::id()
        ))
    }
}
