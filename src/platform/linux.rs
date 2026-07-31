//! Linux process integration. The Linux GUI itself is implemented with gtk4-rs.

use std::path::PathBuf;

pub fn message(title: &str, body: &str) {
    if title.trim().is_empty() {
        eprintln!("{body}");
    } else {
        eprintln!("{title}: {body}");
    }
}

pub(super) fn find_viewer() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("hlp-viewer")))
        .filter(|path| path.is_file())
}

pub(super) const fn viewer_missing_message() -> &'static str {
    "The Linux hlp-viewer executable was not found beside OpenCalc."
}
