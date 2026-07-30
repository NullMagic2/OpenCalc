//! Operating-system integration facade.
//!
//! The shared application calls one stable API.  Each target compiles exactly
//! one implementation module, so Win32, GTK3, and portable fallback code do not
//! share a source file or leak target-specific conditionals into the UI.

use crate::errors::NOT_ENOUGH_MEMORY_FOR_DATA;
use crate::i18n::Language;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod imp;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[path = "other.rs"]
mod imp;

pub use imp::{
    activate_statistics_companion, center_window_on_work_area, client_size_pixels, copy_text,
    disable_frame_resizing, enable_frame_resizing, dismiss_context_tooltip, enable_clip_siblings,
    editable_owns_clipboard, enable_modern_dpi_awareness, fit_calculator_surface,
    has_keyboard_focus, insert_text_at_selection, selected_text,
    history_text_position_from_point, install_classic_button_painter,
    install_classic_display_painter, install_classic_group_box_painter,
    install_classic_separator_painter, install_classic_splitter_painter,
    install_classic_sunken_field_painter,
    install_classic_vertical_separator_painter, install_companion_activation_guard,
    install_context_help, install_context_help_dismissal, install_selector_notifier,
    install_window_state_notifier, is_button_checked, message, paste_text,
    position_statistics_companion, pulse_classic_button, scale_classic_control_metric,
    set_calculator_icon, set_companion_application_active, set_window_rect_pixels,
};

pub fn launch_help(language: Language) -> Result<(), String> {
    let viewer = imp::find_viewer().ok_or_else(|| imp::viewer_missing_message().to_owned())?;
    let help = find_calc_help(language).ok_or_else(|| {
        "The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc.".to_string()
    })?;

    Command::new(&viewer)
        .arg(&help)
        .spawn()
        // CALC.EXE routes a failed WinHelpA call to resource ID 74. The Rust
        // port uses an external HLP viewer, but once both files are resolved a
        // spawn failure is the closest equivalent to that original path.
        .map_err(|_| NOT_ENOUGH_MEMORY_FOR_DATA.to_string())?;
    Ok(())
}

fn help_filenames(language: Language) -> &'static [&'static str] {
    match language {
        Language::English => &["CALC_EN.HLP", "calc_en.hlp"],
        Language::Portuguese => &[
            "CALC_PT-BR.HLP",
            "calc_pt-br.hlp",
            "CALC_EN.HLP",
            "calc_en.hlp",
        ],
        Language::Spanish => &[
            "CALC_ES.HLP",
            "calc_es.hlp",
            "CALC_EN.HLP",
            "calc_en.hlp",
        ],
    }
}

fn find_calc_help(language: Language) -> Option<PathBuf> {
    let filenames = help_filenames(language);
    let mut candidates = Vec::new();

    if let Some(dir) = executable_dir() {
        let help = dir.join("Help");
        candidates.extend(filenames.iter().map(|name| help.join(name)));
    }
    if let Ok(dir) = std::env::current_dir() {
        let help = dir.join("Help");
        candidates.extend(filenames.iter().map(|name| help.join(name)));
    }

    imp::append_system_help_candidates(&mut candidates, filenames);
    first_file(candidates)
}

fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn first_file(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_file_ignores_missing_paths() {
        let missing = std::env::temp_dir().join("definitely-not-a-real-calc-help-file.hlp");
        assert_eq!(first_file(vec![missing]), None);
    }

    #[test]
    fn help_filenames_prefer_the_selected_language() {
        assert_eq!(help_filenames(Language::English)[0], "CALC_EN.HLP");
        assert_eq!(help_filenames(Language::Portuguese)[0], "CALC_PT-BR.HLP");
        assert_eq!(help_filenames(Language::Spanish)[0], "CALC_ES.HLP");
    }

    #[test]
    fn localized_help_can_fall_back_to_english() {
        assert!(help_filenames(Language::Portuguese).contains(&"CALC_EN.HLP"));
        assert!(help_filenames(Language::Spanish).contains(&"CALC_EN.HLP"));
    }
}
