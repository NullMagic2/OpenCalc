#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::run;
#[cfg(target_os = "windows")]
pub use windows::run;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("OpenCalc supports only Linux and Windows.");
