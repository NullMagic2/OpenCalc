//! Linux process integration. The Linux GUI itself is implemented with gtk4-rs.

use std::path::PathBuf;
use std::io::{self, Write};

pub fn invalid_input_beep() {
    // GTK4 no longer exposes the old gdk_display_beep API consistently across
    // backends.  Emit the terminal bell as the portable Linux fallback.
    let _ = io::stdout().write_all(b"\x07");
    let _ = io::stdout().flush();
}

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
