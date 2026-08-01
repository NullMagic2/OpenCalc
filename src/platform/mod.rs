//! Small operating-system services that are not owned by either GUI toolkit.

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
compile_error!("OpenCalc supports only Linux and Windows.");

pub use imp::message;

#[cfg(target_os = "windows")]
pub use imp::{
    activate_statistics_companion, apply_classic_theme, center_window_on_work_area,
    client_size_pixels, copy_text, dismiss_context_tooltip,
    enable_clip_siblings, enable_modern_dpi_awareness, fit_calculator_surface,
    has_keyboard_focus, history_text_position_from_point,
    install_classic_button_painter, install_classic_display_painter,
    install_classic_group_box_painter, install_classic_separator_painter,
    install_classic_splitter_painter, install_classic_sunken_field_painter,
    install_classic_vertical_separator_painter, install_companion_activation_guard,
    install_context_help, install_context_help_dismissal, install_selector_notifier,
    install_window_state_notifier, install_select_all_shortcut, is_button_checked, paste_text,
    selected_text, position_statistics_companion, pulse_classic_button,
    scale_classic_control_metric,
    set_calculator_icon, set_companion_application_active,
    set_window_rect_pixels,
};

pub fn launch_help(language: Language) -> Result<(), String> {
    let viewer = imp::find_viewer().ok_or_else(|| imp::viewer_missing_message().to_owned())?;
    let help = find_calc_help(language).ok_or_else(|| {
        "The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc.".to_string()
    })?;

    Command::new(&viewer)
        .arg(&help)
        .spawn()
        .map_err(|_| NOT_ENOUGH_MEMORY_FOR_DATA.to_string())?;
    Ok(())
}

fn help_filenames(language: Language) -> &'static [&'static str] {
    match language {
        Language::English => &["CALC_EN.HLP", "calc_en.hlp"],
        Language::Portuguese => &["CALC_PT-BR.HLP", "calc_pt-br.hlp"],
        Language::Spanish => &["CALC_ES.HLP", "calc_es.hlp"],
    }
}

fn find_calc_help(language: Language) -> Option<PathBuf> {
    let filenames = help_filenames(language);
    let mut candidates = Vec::new();

    if let Some(dir) = executable_dir() {
        let help = dir.join("Help");
        candidates.extend(filenames.iter().map(|name| help.join(name)));
    }
    #[cfg(target_os = "windows")]
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
}
