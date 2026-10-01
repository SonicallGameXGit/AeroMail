//! Platform-specific behaviour, one module per OS, behind common functions.

#[cfg(target_os = "linux")]
mod linux;

/// Must run first thing at startup, before any other thread exists.
pub fn prepare_environment() {
    #[cfg(target_os = "linux")]
    linux::prepare_environment();
}
