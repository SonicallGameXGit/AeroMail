//! Commands exposed to the UI. These only forward to the core crates.

use mail_autoconfig::Detection;
use mail_core::account::{AccountSettings, Protocol, Security, SettingsIssue};
use mail_core::address::AddressError;

#[tauri::command]
pub fn detect_account_settings(email: String) -> Result<Detection, AddressError> {
    mail_autoconfig::detect(&email)
}

#[tauri::command]
pub fn validate_account_settings(settings: AccountSettings) -> Vec<SettingsIssue> {
    mail_core::account::validate(&settings)
}

#[tauri::command]
pub fn default_port(protocol: Protocol, security: Security) -> u16 {
    mail_core::account::default_port(protocol, security)
}
