//! Shared, configurable keyboard commands for both native interfaces.
use crate::calc::{Base, BinaryOp};
use crate::expr::AngleMode;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Digit(char), Dot, Back, CE, C, Sign, Eq, Percent, Bin(BinaryOp),
    KeyboardStar, Unary(&'static str), Exp, MemC, MemR, MemS, MemAdd, Pi,
    Open, Close, StatsOpen, StatsDat, StatsAvg, StatsSum, StatsDev,
    ToggleFE, Copy, Paste, About, Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Button(Action), Percent, Square, Inv, Hyp, Undo, Redo,
    Base(Base), Angle(AngleMode), Word, F6,
}

pub struct ShortcutDef {
    pub id: &'static str,
    pub label: &'static str,
    pub defaults: &'static str,
    pub command: Command,
    pub scientific: bool,
}

macro_rules! shortcuts {
    ($(($id:literal, $label:literal, $keys:literal, $command:expr, $scientific:expr)),* $(,)?) => {
        pub const DEFINITIONS: &[ShortcutDef] = &[
            $(ShortcutDef { id: $id, label: $label, defaults: $keys, command: $command, scientific: $scientific }),*
        ];
    };
}

use Action as A;
use Command as C;
shortcuts! {
    ("clear", "Clear calculation (C)", "C, Escape", C::Button(A::C), false),
    ("clear_entry", "Clear entry (CE)", "Delete, NumpadDelete", C::Button(A::CE), false),
    ("back", "Backspace", "Backspace, Left, NumpadLeft", C::Button(A::Back), false),
    ("equals", "Equals", "Enter, NumpadEnter, =", C::Button(A::Eq), false),
    ("sign", "Change sign", "F9", C::Button(A::Sign), false),
    ("decimal", "Decimal point", "Period, Comma, NumpadDecimal, NumpadSeparator", C::Button(A::Dot), false),
    ("add", "Add", "Plus, NumpadAdd", C::Button(A::Bin(BinaryOp::Add)), false),
    ("subtract", "Subtract", "-, −, NumpadSubtract", C::Button(A::Bin(BinaryOp::Sub)), false),
    ("multiply", "Multiply / type ** for power", "*", C::Button(A::KeyboardStar), false),
    ("numpad_multiply", "Multiply (numpad)", "NumpadMultiply, ×", C::Button(A::Bin(BinaryOp::Mul)), false),
    ("divide", "Divide", "/, ÷, NumpadDivide", C::Button(A::Bin(BinaryOp::Div)), false),
    ("percent", "Percent / scientific Mod", "%", C::Percent, false),
    ("square", "Square root / scientific square", "@", C::Square, false),
    ("reciprocal", "Reciprocal (1/x)", "R, Shift+R", C::Button(A::Unary("recip")), false),
    ("copy", "Copy", "Ctrl+C, Ctrl+Insert, Ctrl+NumpadInsert", C::Button(A::Copy), false),
    ("paste", "Paste", "Ctrl+V, Shift+Insert, Shift+NumpadInsert", C::Button(A::Paste), false),
    ("undo", "Undo", "Ctrl+Z", C::Undo, false),
    ("redo", "Redo", "Ctrl+Y", C::Redo, false),
    ("help", "Help", "F1", C::Button(A::Help), false),
    ("memory_clear", "Memory clear", "Ctrl+L", C::Button(A::MemC), false),
    ("memory_recall", "Memory recall", "Ctrl+R", C::Button(A::MemR), false),
    ("memory_store", "Memory store", "Ctrl+M", C::Button(A::MemS), false),
    ("memory_add", "Memory add", "Ctrl+P", C::Button(A::MemAdd), false),
    ("digit0", "Digit 0", "0, Numpad0", C::Button(A::Digit('0')), false),
    ("digit1", "Digit 1", "1, Numpad1", C::Button(A::Digit('1')), false),
    ("digit2", "Digit 2", "2, Numpad2", C::Button(A::Digit('2')), false),
    ("digit3", "Digit 3", "3, Numpad3", C::Button(A::Digit('3')), false),
    ("digit4", "Digit 4", "4, Numpad4", C::Button(A::Digit('4')), false),
    ("digit5", "Digit 5", "5, Numpad5", C::Button(A::Digit('5')), false),
    ("digit6", "Digit 6", "6, Numpad6", C::Button(A::Digit('6')), false),
    ("digit7", "Digit 7", "7, Numpad7", C::Button(A::Digit('7')), false),
    ("digit8", "Digit 8", "8, Numpad8", C::Button(A::Digit('8')), false),
    ("digit9", "Digit 9", "9, Numpad9", C::Button(A::Digit('9')), false),
    ("hex_a", "Hex digit A", "A, Shift+A", C::Button(A::Digit('A')), true),
    ("hex_b", "Hex digit B", "B, Shift+B", C::Button(A::Digit('B')), true),
    ("hex_c", "Hex digit C", "Shift+C", C::Button(A::Digit('C')), true),
    ("hex_d", "Hex digit D", "D, Shift+D", C::Button(A::Digit('D')), true),
    ("hex_e", "Hex digit E", "E, Shift+E", C::Button(A::Digit('E')), true),
    ("hex_f", "Hex digit F", "F, Shift+F", C::Button(A::Digit('F')), true),
    ("open", "Open parenthesis", "(", C::Button(A::Open), false),
    ("close", "Close parenthesis", ")", C::Button(A::Close), false),
    ("power", "Power (x^y)", "Y, Shift+Y", C::Button(A::Bin(BinaryOp::Pow)), true),
    ("sin", "Sine", "S, Shift+S", C::Button(A::Unary("sin")), true),
    ("cos", "Cosine", "O, Shift+O", C::Button(A::Unary("cos")), true),
    ("tan", "Tangent", "T, Shift+T", C::Button(A::Unary("tan")), true),
    ("ln", "Natural logarithm", "N, Shift+N", C::Button(A::Unary("ln")), true),
    ("log", "Logarithm", "L, Shift+L", C::Button(A::Unary("log")), true),
    ("dms", "Degrees / minutes / seconds", "M, Shift+M", C::Button(A::Unary("dms")), true),
    ("exp", "Exponent entry", "X, Shift+X", C::Button(A::Exp), true),
    ("pi", "Pi", "P, Shift+P", C::Button(A::Pi), true),
    ("inv", "Toggle inverse", "I, Shift+I", C::Inv, true),
    ("hyp", "Toggle hyperbolic", "H, Shift+H", C::Hyp, true),
    ("fe", "Toggle F-E notation", "V, Shift+V", C::Button(A::ToggleFE), true),
    ("factorial", "Factorial", "!", C::Button(A::Unary("factorial")), true),
    ("cube", "Cube", "#", C::Button(A::Unary("cube")), true),
    ("and", "Bitwise AND", "&", C::Button(A::Bin(BinaryOp::And)), true),
    ("or", "Bitwise OR", "|", C::Button(A::Bin(BinaryOp::Or)), true),
    ("xor", "Bitwise XOR", "^", C::Button(A::Bin(BinaryOp::Xor)), true),
    ("lsh", "Left shift", "<", C::Button(A::Bin(BinaryOp::Lsh)), true),
    ("not", "Bitwise NOT", "~", C::Button(A::Unary("not")), true),
    ("int", "Integer part", "Semicolon", C::Button(A::Unary("int")), true),
    ("stats", "Open statistics", "Ctrl+S", C::Button(A::StatsOpen), true),
    ("stats_data", "Statistics: add data", "Insert, NumpadInsert", C::Button(A::StatsDat), true),
    ("stats_average", "Statistics: average", "Ctrl+A", C::Button(A::StatsAvg), true),
    ("stats_sum", "Statistics: sum", "Ctrl+T", C::Button(A::StatsSum), true),
    ("stats_deviation", "Statistics: standard deviation", "Ctrl+D", C::Button(A::StatsDev), true),
    ("degrees", "Degrees / Dword", "F2", C::Angle(AngleMode::Degrees), false),
    ("word", "Radians / Word", "F3", C::Word, false),
    ("grads", "Grads / Byte", "F4", C::Angle(AngleMode::Grads), false),
    ("hex", "Hexadecimal base", "F5", C::Base(Base::Hex), false),
    ("radians_decimal", "Decimal base", "F6", C::F6, false),
    ("oct", "Octal base", "F7", C::Base(Base::Oct), false),
    ("bin", "Binary base", "F8", C::Base(Base::Bin), false),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyChord {
    pub key: String,
    pub control: bool,
    pub shift: bool,
}

impl KeyChord {
    pub fn new(name: &str, control: bool, shift: bool) -> Result<Self, String> {
        let lower = name.to_ascii_lowercase().replace("kp_", "numpad");
        let key = match lower.as_str() {
            "return" => "Enter".into(), "esc" => "Escape".into(),
            "space" | " " => "Space".into(), "plus" => "+".into(),
            "period" | "decimal" => ".".into(), "comma" => ",".into(),
            "semicolon" => ";".into(),
            _ if name.chars().count() == 1 && !name.chars().next().unwrap().is_control() => name.to_ascii_uppercase(),
            _ => {
                const NAMED: &[&str] = &[
                    "Enter", "Escape", "Backspace", "Delete", "Insert", "Left", "Right", "Up", "Down", "Home", "End", "Space",
                    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
                    "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8", "Numpad9",
                    "NumpadAdd", "NumpadSubtract", "NumpadMultiply", "NumpadDivide", "NumpadDecimal", "NumpadSeparator", "NumpadEnter", "NumpadInsert", "NumpadDelete", "NumpadLeft",
                ];
                NAMED.iter().find(|key| key.to_ascii_lowercase() == lower)
                    .ok_or_else(|| format!("Unknown key: {name}"))?.to_string()
            }
        };
        // A printable symbol already includes its keyboard-layout Shift translation.
        let shift = shift && !(key.chars().count() == 1 && !key.chars().next().unwrap().is_alphanumeric());
        Ok(Self { key, control, shift })
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rest = text.trim();
        let (mut control, mut shift) = (false, false);
        while let Some((modifier, tail)) = rest.split_once('+') {
            match modifier.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" if !control => control = true,
                "shift" if !shift => shift = true,
                _ => return Err(format!("Use Ctrl and/or Shift, followed by a key (write Plus for +): {text}")),
            }
            rest = tail.trim();
        }
        Self::new(rest, control, shift)
    }

    pub fn label(&self) -> String {
        let name = match self.key.as_str() { "+" => "Plus", "," => "Comma", ";" => "Semicolon", "." => "Period", other => other };
        format!("{}{}{name}", if self.control { "Ctrl+" } else { "" }, if self.shift { "Shift+" } else { "" })
    }

    pub fn native_display_key(&self) -> bool {
        matches!(self.key.as_str(), "Left" | "Right" | "Up" | "Down" | "Home" | "End")
            || (self.control && matches!(self.key.as_str(), "A" | "C" | "Insert" | "NumpadInsert"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcuts { bindings: Vec<Vec<KeyChord>> }

impl Default for Shortcuts {
    fn default() -> Self {
        Self::from_texts(&DEFINITIONS.iter().map(|d| d.defaults.to_owned()).collect::<Vec<_>>())
            .expect("valid default shortcuts")
    }
}

impl Shortcuts {
    pub fn texts(&self) -> Vec<String> {
        self.bindings.iter().map(|keys| keys.iter().map(KeyChord::label).collect::<Vec<_>>().join(", ")).collect()
    }

    pub fn from_texts(texts: &[String]) -> Result<Self, String> {
        if texts.len() != DEFINITIONS.len() { return Err("Incomplete shortcut settings".into()); }
        let mut bindings: Vec<Vec<KeyChord>> = Vec::new();
        for (index, text) in texts.iter().enumerate() {
            let mut keys = Vec::new();
            for value in text.split(',').filter(|value| !value.trim().is_empty()) {
                let key = KeyChord::parse(value).map_err(|error| format!("{}: {error}", DEFINITIONS[index].label))?;
                if let Some(other) = bindings.iter().position(|keys| keys.contains(&key)) {
                    return Err(format!("{} is assigned to both {} and {}.", key.label(), DEFINITIONS[other].label, DEFINITIONS[index].label));
                }
                if !keys.contains(&key) { keys.push(key); }
            }
            bindings.push(keys);
        }
        Ok(Self { bindings })
    }

    pub fn resolve(&self, key: &KeyChord, scientific: bool) -> Option<Command> {
        self.bindings.iter().zip(DEFINITIONS).find_map(|(keys, def)|
            ((!def.scientific || scientific) && keys.contains(key)).then_some(def.command))
    }

    pub fn to_config(&self) -> String {
        DEFINITIONS.iter().zip(self.texts()).map(|(def, keys)| format!("shortcut.{}={}\n", def.id, keys)).collect()
    }

    pub fn to_preset_config(&self) -> String {
        let mut text = String::from(
            "# OpenCalc keyboard shortcut preset\n\
# Edit with any text editor. Leave a value blank to disable that action.\n",
        );
        text.push_str(&self.to_config());
        text
    }

    pub fn from_config(text: &str) -> Result<Self, String> {
        let mut values = Self::default().texts();
        for line in text.lines() {
            let Some((key, value)) = line.trim().split_once('=') else { continue; };
            let Some(id) = key.trim().strip_prefix("shortcut.") else { continue; };
            if let Some(index) = DEFINITIONS.iter().position(|def| def.id == id) { values[index] = value.trim().to_owned(); }
        }
        Self::from_texts(&values)
    }
}

pub fn find_shortcut(query: &str, texts: &[String]) -> Option<usize> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return None;
    }
    DEFINITIONS.iter().enumerate().find_map(|(index, def)| {
        let keys = texts.get(index).map(String::as_str).unwrap_or(def.defaults);
        (def.id.to_ascii_lowercase().contains(&query)
            || def.label.to_ascii_lowercase().contains(&query)
            || keys.to_ascii_lowercase().contains(&query))
            .then_some(index)
    })
}

pub const PRESET_DIR_NAME: &str = "shortcuts";
pub const DEFAULT_PRESET_FILE: &str = "default.cfg";

/// Human-editable shortcut presets live beside the executable in
/// `shortcuts/*.cfg`.
pub fn preset_directory() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join(PRESET_DIR_NAME)))
        .unwrap_or_else(|| PathBuf::from(PRESET_DIR_NAME))
}

pub fn preset_names() -> io::Result<Vec<String>> {
    preset_names_in(&preset_directory())
}

fn preset_names_in(directory: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(names),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !extension.eq_ignore_ascii_case("cfg") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            if !stem.trim().is_empty() {
                names.push(stem.to_string());
            }
        }
    }
    names.sort_by_key(|name| {
        let lower = name.to_ascii_lowercase();
        (lower != "default", lower)
    });
    names.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    Ok(names)
}

pub fn load_preset(name: &str) -> Result<Shortcuts, String> {
    load_preset_from(&preset_directory(), name)
}

fn load_preset_from(directory: &Path, name: &str) -> Result<Shortcuts, String> {
    let path = preset_path(directory, name)?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read shortcut preset {}: {error}", path.display()))?;
    Shortcuts::from_config(&text)
        .map_err(|error| format!("Invalid shortcut preset {}: {error}", path.display()))
}

pub fn save_preset(name: &str, shortcuts: &Shortcuts) -> Result<PathBuf, String> {
    save_preset_to(&preset_directory(), name, shortcuts)
}

fn save_preset_to(directory: &Path, name: &str, shortcuts: &Shortcuts) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("Unable to create shortcut preset folder {}: {error}", directory.display()))?;
    let path = preset_path(directory, name)?;
    fs::write(&path, shortcuts.to_preset_config())
        .map_err(|error| format!("Unable to save shortcut preset {}: {error}", path.display()))?;
    Ok(path)
}

pub fn delete_preset(name: &str) -> Result<(), String> {
    let path = preset_path(&preset_directory(), name)?;
    fs::remove_file(&path)
        .map_err(|error| format!("Unable to delete shortcut preset {}: {error}", path.display()))
}

fn preset_path(directory: &Path, name: &str) -> Result<PathBuf, String> {
    let mut clean = name.trim();
    if clean.to_ascii_lowercase().ends_with(".cfg") {
        clean = &clean[..clean.len() - 4];
    }
    if clean.is_empty() {
        return Err("Enter a shortcut preset name.".into());
    }
    if clean == "."
        || clean == ".."
        || clean.chars().any(|ch| matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        return Err("Preset names may not contain path or filename-reserved characters.".into());
    }
    Ok(directory.join(format!("{clean}.cfg")))
}

#[cfg(target_os = "windows")]
pub fn windows_key(raw: i32, unicode: Option<char>, control: bool, shift: bool) -> Option<KeyChord> {
    let name = match raw {
        8 => "Backspace".into(), 13 => "Enter".into(), 27 => "Escape".into(), 127 => "Delete".into(),
        312 => "End".into(), 313 => "Home".into(), 314 => "Left".into(), 315 => "Up".into(), 316 => "Right".into(), 317 => "Down".into(),
        322 => "Insert".into(), 324..=333 => format!("Numpad{}", raw - 324),
        334 | 387 => "NumpadMultiply".into(), 335 | 388 => "NumpadAdd".into(),
        336 | 389 => "NumpadSeparator".into(), 337 | 390 => "NumpadSubtract".into(),
        338 | 391 => "NumpadDecimal".into(), 339 | 392 => "NumpadDivide".into(),
        340..=351 => format!("F{}", raw - 339), 370 | 386 => "NumpadEnter".into(),
        376 => "NumpadLeft".into(), 384 => "NumpadInsert".into(), 385 => "NumpadDelete".into(),
        _ if raw >= 300 => return None,
        _ if control && (65..=90).contains(&raw) => char::from_u32(raw as u32)?.to_string(),
        _ => unicode.filter(|ch| !ch.is_control()).or_else(|| char::from_u32(raw as u32))?.to_string(),
    };
    KeyChord::new(&name, control, shift).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(text: &str) -> KeyChord { KeyChord::parse(text).unwrap() }
    #[test]
    fn defaults_clear_with_c_and_keep_hex_c() {
        let shortcuts = Shortcuts::default();
        for scientific in [false, true] {
            assert_eq!(shortcuts.resolve(&key("c"), scientific), Some(C::Button(A::C)));
            assert_eq!(shortcuts.resolve(&key("Escape"), scientific), Some(C::Button(A::C)));
        }
        assert_eq!(shortcuts.resolve(&key("Shift+C"), true), Some(C::Button(A::Digit('C'))));
        assert_eq!(shortcuts.resolve(&key("Shift+C"), false), None);
        assert_eq!(shortcuts.resolve(&key("Ctrl+C"), true), Some(C::Button(A::Copy)));
    }
    #[test]
    fn remapping_replaces_old_keys_and_survives_config_round_trip() {
        let mut texts = Shortcuts::default().texts();
        texts[0] = "Q, Ctrl+Shift+K".into();
        let custom = Shortcuts::from_texts(&texts).unwrap();
        assert_eq!(custom.resolve(&key("C"), false), None);
        assert_eq!(custom.resolve(&key("Q"), false), Some(C::Button(A::C)));
        assert_eq!(Shortcuts::from_config(&custom.to_config()).unwrap(), custom);
        texts[0].clear();
        assert_eq!(Shortcuts::from_texts(&texts).unwrap().resolve(&key("Escape"), false), None);
    }
    #[test]
    fn invalid_or_conflicting_bindings_are_rejected() {
        let mut texts = Shortcuts::default().texts();
        for invalid in ["Ctrl+Plus+X", "Alt+F4", "Ctrl+", "Banana", "Ctrl+Ctrl+Q", "Enter"] {
            texts[0] = invalid.into();
            assert!(Shortcuts::from_texts(&texts).is_err(), "{invalid}");
        }
    }
    #[test]
    fn shifted_symbols_and_native_edit_keys_are_preserved() {
        assert_eq!(KeyChord::new("+", false, true).unwrap(), key("Plus"));
        assert_eq!(key("control+shift+q"), key("Ctrl+Shift+Q"));
        for text in ["Ctrl+A", "Ctrl+C", "Ctrl+Insert", "Shift+Left", "Home"] {
            assert!(key(text).native_display_key());
        }
        assert!(!key("C").native_display_key());
        assert!(!key("Ctrl+V").native_display_key());
    }
    #[test]
    fn defaults_cover_digits_operators_memory_and_scientific_keys() {
        let shortcuts = Shortcuts::default();
        for n in 0..=9 {
            let expected = Some(C::Button(A::Digit(char::from(b'0' + n))));
            assert_eq!(shortcuts.resolve(&key(&n.to_string()), false), expected);
            assert_eq!(shortcuts.resolve(&key(&format!("Numpad{n}")), false), expected);
        }
        assert_eq!(shortcuts.resolve(&key("*"), false), Some(C::Button(A::KeyboardStar)));
        assert_eq!(shortcuts.resolve(&key("Ctrl+M"), false), Some(C::Button(A::MemS)));
        assert_eq!(shortcuts.resolve(&key("S"), false), None);
        assert_eq!(shortcuts.resolve(&key("S"), true), Some(C::Button(A::Unary("sin"))));
        assert_eq!(shortcuts.resolve(&key("F2"), true), Some(C::Angle(AngleMode::Degrees)));
        assert_eq!(shortcuts.resolve(&key("F3"), true), Some(C::Word));
        assert_eq!(shortcuts.resolve(&key("F4"), true), Some(C::Angle(AngleMode::Grads)));
        assert_eq!(shortcuts.resolve(&key("F5"), true), Some(C::Base(Base::Hex)));
        assert_eq!(shortcuts.resolve(&key("F6"), true), Some(C::F6));
        assert_eq!(shortcuts.resolve(&key("F7"), true), Some(C::Base(Base::Oct)));
        assert_eq!(shortcuts.resolve(&key("F8"), true), Some(C::Base(Base::Bin)));
        // The original accelerator table also dispatches these keys in Standard
        // mode; the UI then routes all seven selector commands through Decimal.
        assert_eq!(shortcuts.resolve(&key("F2"), false), Some(C::Angle(AngleMode::Degrees)));
        assert_eq!(shortcuts.resolve(&key("F3"), false), Some(C::Word));
        assert_eq!(shortcuts.resolve(&key("F4"), false), Some(C::Angle(AngleMode::Grads)));
        assert_eq!(shortcuts.resolve(&key("F5"), false), Some(C::Base(Base::Hex)));
        assert_eq!(shortcuts.resolve(&key("F6"), false), Some(C::F6));
        assert_eq!(shortcuts.resolve(&key("F7"), false), Some(C::Base(Base::Oct)));
        assert_eq!(shortcuts.resolve(&key("F8"), false), Some(C::Base(Base::Bin)));
    }
    #[test]
    fn old_settings_and_unknown_future_commands_use_defaults() {
        assert_eq!(Shortcuts::from_config("mode=scientific\nshortcut.future=Q").unwrap(), Shortcuts::default());
    }
    #[test]
    fn preset_files_are_plain_text_and_restore_defaults_ignore_an_edited_default_file() {
        assert_eq!(
            Shortcuts::from_config(include_str!("../shortcuts/default.cfg")).unwrap(),
            Shortcuts::default(),
            "the shipped default.cfg must mirror the compiled pristine defaults"
        );
        let root = std::env::temp_dir().join(format!("opencalc-shortcuts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let defaults = Shortcuts::default();
        save_preset_to(&root, "default", &defaults).unwrap();
        assert_eq!(load_preset_from(&root, "default").unwrap(), defaults);

        let mut altered_texts = defaults.texts();
        altered_texts[0] = "Q".into();
        let altered = Shortcuts::from_texts(&altered_texts).unwrap();
        save_preset_to(&root, "default.cfg", &altered).unwrap();
        assert_eq!(load_preset_from(&root, "default").unwrap(), altered);
        assert_eq!(Shortcuts::default(), defaults, "Restore Defaults must not trust an edited default.cfg");

        save_preset_to(&root, "Minimal keys", &defaults).unwrap();
        assert_eq!(preset_names_in(&root).unwrap(), vec!["default".to_string(), "Minimal keys".to_string()]);
        assert!(preset_path(&root, "../escape").is_err());
        let _ = fs::remove_dir_all(&root);
    }
    #[test]
    fn shortcut_search_matches_action_id_label_and_current_keys() {
        let texts = Shortcuts::default().texts();
        assert_eq!(find_shortcut("memory store", &texts), DEFINITIONS.iter().position(|def| def.id == "memory_store"));
        assert_eq!(find_shortcut("ctrl+m", &texts), DEFINITIONS.iter().position(|def| def.id == "memory_store"));
        assert_eq!(find_shortcut("radians_decimal", &texts), DEFINITIONS.iter().position(|def| def.id == "radians_decimal"));
        assert_eq!(find_shortcut("not a shortcut", &texts), None);
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_events_normalize_control_characters_and_numpad() {
        assert_eq!(windows_key(306, None, false, true), None);
        assert_eq!(windows_key(308, None, true, false), None);
        assert_eq!(windows_key(67, Some('\u{3}'), true, false), Some(key("Ctrl+C")));
        assert_eq!(windows_key(99, Some('c'), false, false), Some(key("C")));
        assert_eq!(windows_key(67, Some('C'), false, true), Some(key("Shift+C")));
        assert_eq!(windows_key(388, None, false, false), Some(key("NumpadAdd")));
        assert_eq!(windows_key(43, Some('+'), false, true), Some(key("Plus")));
    }
}
