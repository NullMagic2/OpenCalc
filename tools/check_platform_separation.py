#!/usr/bin/env python3
"""Verify the direct Windows/wxDragon and Linux/GTK4 split."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"


def fail(message: str) -> None:
    raise AssertionError(message)


def require(path: Path) -> str:
    if not path.is_file():
        fail(f"required file is missing: {path}")
    return path.read_text(encoding="utf-8")


def main() -> int:
    ui_mod = require(SRC / "ui" / "mod.rs")
    linux_ui = require(SRC / "ui" / "linux.rs")
    windows_ui = require(SRC / "ui" / "windows.rs")
    linux_platform = require(SRC / "platform" / "linux.rs")
    linux_locale = require(SRC / "locale" / "linux.rs")
    cargo = require(ROOT / "Cargo.toml")

    for obsolete in (
        SRC / "ui" / "other.rs",
        SRC / "platform" / "other.rs",
        SRC / "locale" / "other.rs",
    ):
        if obsolete.exists():
            fail(f"obsolete fallback backend still exists: {obsolete}")

    for needle in ('mod linux;', 'mod windows;', 'pub use linux::run;', 'pub use windows::run;'):
        if needle not in ui_mod:
            fail(f"UI selector is missing: {needle}")
    if 'compile_error!("OpenCalc supports only Linux and Windows.")' not in ui_mod:
        fail("unsupported targets are not rejected explicitly")

    for forbidden in ("wxdragon", "unsafe", 'extern "C"', "#[link("):
        if forbidden in linux_ui:
            fail(f"native GTK4 UI contains forbidden low-level/legacy code: {forbidden}")
    for required in (
        "gtk::Application",
        "gtk::ApplicationWindow",
        "gtk::Fixed",
        "gtk::CssProvider",
        "gtk::EventControllerKey",
        "gtk::GestureClick",
        "gtk::Popover",
        "gtk::DrawingArea",
        "bind_context_help",
        "classic_menu_popover",
        "panel_separator",
        "#f0f0f0",
        "CairoBackend",
    ):
        if required not in linux_ui:
            fail(f"native GTK4 UI is missing: {required}")

    if "wxdragon::" not in windows_ui:
        fail("Windows UI no longer uses the existing wxDragon implementation")
    if "gtk::" in windows_ui:
        fail("Windows UI directly depends on gtk4-rs")
    if re.search(r"#\[cfg\([^\]]*target_os", windows_ui):
        fail("Windows UI still contains dead cross-platform cfg branches")
    for stale in ("wxGTK", "GTK3", "set_linux_surface", "install_calculator_key_handler"):
        if stale in windows_ui:
            fail(f"Windows UI still contains deleted Linux implementation code: {stale}")

    for text, label in ((linux_platform, "Linux platform"), (linux_locale, "Linux locale")):
        for forbidden in ("unsafe", 'extern "C"', "#[link(", "gtk_"):
            if forbidden in text:
                fail(f"{label} contains direct native FFI: {forbidden}")

    if "package = \"gtk4\"" not in cargo or 'features = ["v4_10"]' not in cargo:
        fail("Cargo.toml does not select gtk4-rs with the GTK 4.10 API")
    if "plotters-cairo" not in cargo:
        fail("Linux graph rendering is not using plotters-cairo")
    if 'version = "0.11.4"' not in cargo or 'gdk4-x11 = "0.11.4"' not in cargo:
        fail("Cargo.toml does not select the requested gtk4-rs/gdk4-x11 0.11.4 dependency range")
    if "gdk4-x11" not in cargo or "x11rb" not in cargo:
        fail("Linux deterministic X11 startup centering dependencies are missing")
    if 'ashpd = { version = "0.13.13"' not in cargo or '"file_chooser"' not in cargo:
        fail("Linux graph export is missing its direct XDG FileChooser portal dependency")
    if '"gtk4_x11"' not in cargo or '"gtk4"]' in cargo:
        fail("ashpd must use its X11-only GTK integration; the broad gtk4 feature pulls in unneeded Wayland symbols")
    if "gtk::FileDialog" in linux_ui or "FileChooserNative" in linux_ui or "FileChooserDialog" in linux_ui:
        fail("Linux graph export still instantiates an in-process GTK file chooser")
    for required in ("SelectedFiles::save_file", "WindowIdentifier::from_native", "PortalFileFilter"):
        if required not in linux_ui:
            fail(f"Linux graph export is missing direct portal integration: {required}")
    if "plotters-wxdragon" not in cargo or "wxdragon" not in cargo:
        fail("Windows dependencies were not retained")

    for path in ROOT.rglob("*.rs"):
        text = path.read_text(encoding="utf-8")
        for literal in re.findall(
            r'\binclude_(?:bytes|str)!\s*\(\s*["\']([^"\']+)["\']\s*\)', text
        ):
            included = (path.parent / literal).resolve()
            if not included.is_file():
                fail(f"{path} references missing include asset: {literal}")

    print("PASS: Linux selects a native safe gtk4-rs interface.")
    print("PASS: Windows retains the existing wxDragon interface.")
    print("PASS: no portable/other fallback backend remains.")
    print("PASS: Linux GUI, platform, and locale code contain no raw C FFI.")
    print("PASS: safe X11 startup centering is declared only for the Linux target.")
    print("PASS: target-specific GUI and graph dependencies are declared.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
