#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod calc;
mod calculation_log;
mod expr;
mod errors;
mod i18n;
mod history;
mod graph;
mod locale;
mod platform;
mod settings;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod tooltip;
mod ui;

fn main() {
    // CALC.EXE reserves a 0x400-byte resource-string buffer during startup and
    // displays resource ID 78 ("Not Enough Memory") if that allocation fails.
    // Use a fallible allocation here so the clean-room implementation retains
    // the same recoverable startup failure instead of relying only on Rust's
    // process-level OOM handling. Keep the allocation alive for the UI lifetime.
    let mut resource_reserve = Vec::<u8>::new();
    if resource_reserve.try_reserve_exact(0x400).is_err() {
        platform::message("", errors::STARTUP_NOT_ENOUGH_MEMORY);
        return;
    }

    if let Err(error) = ui::run() {
        platform::message("Calculator", &error);
    }

    drop(resource_reserve);
}
