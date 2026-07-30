//! Portable stubs for non-Windows, non-Linux targets.

use std::ffi::c_void;

pub fn set_companion_application_active(_companion_hwnd: *mut c_void, _active: bool) {}

pub fn install_companion_activation_guard(
    _owner_hwnd: *mut c_void,
    _companion_hwnd: *mut c_void,
) {
}

pub fn activate_statistics_companion(_companion_hwnd: *mut c_void) {}

pub fn message(title: &str, body: &str) {
    eprintln!("{title}: {body}");
}

pub fn copy_text(_text: &str) -> Result<(), String> {
    Err("Clipboard integration is currently implemented for Windows only.".into())
}

pub fn paste_text() -> Result<Option<String>, String> {
    Err("Clipboard integration is currently implemented for Windows only.".into())
}

pub fn set_calculator_icon(_hwnd: *mut c_void) {}

pub fn enable_modern_dpi_awareness() {}

pub fn scale_classic_control_metric(_hwnd: *mut c_void, logical: i32) -> i32 {
    logical
}

pub fn enable_frame_resizing(_hwnd: *mut c_void) {}

pub fn disable_frame_resizing(_hwnd: *mut c_void) {}

pub fn fit_calculator_surface(
    _frame_hwnd: *mut c_void,
    _panel_hwnd: *mut c_void,
    _logical_width: i32,
    _logical_height: i32,
) -> bool {
    false
}

pub fn center_window_on_work_area(_hwnd: *mut c_void) -> bool {
    false
}

pub fn history_text_position_from_point(
    _text_hwnd: *mut c_void,
    _x: i32,
    _y: i32,
) -> Option<usize> {
    None
}

pub fn position_statistics_companion(
    _owner_hwnd: *mut c_void,
    _stats_hwnd: *mut c_void,
) -> bool {
    false
}

pub fn client_size_pixels(_hwnd: *mut c_void) -> Option<(i32, i32)> {
    None
}

pub fn set_window_rect_pixels(
    _hwnd: *mut c_void,
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
) -> bool {
    false
}

pub fn install_classic_sunken_field_painter(_hwnd: *mut c_void) {}

pub fn install_classic_display_painter(_hwnd: *mut c_void) {}

pub fn install_classic_group_box_painter(_hwnd: *mut c_void) {}

pub fn install_classic_splitter_painter(_hwnd: *mut c_void) {}

pub fn install_classic_separator_painter(_hwnd: *mut c_void) {}

pub fn install_classic_vertical_separator_painter(_hwnd: *mut c_void) {}

pub fn make_pointer_passthrough(_hwnd: *mut c_void) {}

pub fn install_classic_button_painter(
    _hwnd: *mut c_void,
    _red: u8,
    _green: u8,
    _blue: u8,
) {
}

pub fn pulse_classic_button(_hwnd: *mut c_void) {}

pub fn has_keyboard_focus(_hwnd: *mut c_void) -> bool {
    false
}

pub fn editable_owns_clipboard(_hwnd: *mut c_void) -> bool {
    false
}

pub fn selected_text(_hwnd: *mut c_void) -> Option<String> {
    None
}

pub fn insert_text_at_selection(_hwnd: *mut c_void, _text: &str) -> bool {
    false
}

pub fn enable_clip_siblings(_hwnd: *mut c_void) {}

pub fn install_selector_notifier(
    _parent: *mut c_void,
    _children: &[*mut c_void],
    _callback: Box<dyn Fn(usize)>,
) {
}

pub fn is_button_checked(_hwnd: *mut c_void) -> bool {
    false
}

pub fn install_context_help(_hwnd: *mut c_void, _text: &str, _menu_label: &str) {}

pub fn install_context_help_dismissal(_hwnd: *mut c_void) {}

pub fn install_window_state_notifier(
    _hwnd: *mut c_void,
    _callback: Box<dyn Fn(bool)>,
) {
}

pub fn dismiss_context_tooltip() {}

pub(super) fn viewer_missing_message() -> &'static str {
    "hlp-viewer was not found. Place the native executable beside OpenCalc."
}

pub(super) fn find_viewer() -> Option<std::path::PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = super::executable_dir() {
        candidates.push(dir.join("hlp-viewer"));
    }
    if let Ok(dir) = std::env::current_dir() {
        candidates.push(dir.join("hlp-viewer"));
    }
    super::first_file(candidates)
}

pub(super) fn append_system_help_candidates(
    _candidates: &mut Vec<std::path::PathBuf>,
    _filenames: &[&str],
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_error_names_native_companion() {
        let message = viewer_missing_message();
        assert!(message.contains("hlp-viewer") && !message.contains(".exe"));
    }
}
