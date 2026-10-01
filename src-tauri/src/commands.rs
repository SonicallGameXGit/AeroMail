//! Commands exposed to the UI. These only forward to the core crates.

use std::sync::Arc;

use mail_autoconfig::Detection;
use mail_core::account::{AccountSettings, Protocol, Security, SettingsIssue};
use mail_core::address::AddressError;
use mail_core::draft::{Draft, ReplyMode};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::mail::{Account, Folder, MessageContent, MessageSummary, OutgoingMessage};
use mail_core::transport::Flag;
use mail_engine::Engine;
use tauri::State;

type EngineState<'a> = State<'a, Arc<Engine>>;

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

#[tauri::command]
pub fn list_accounts(engine: EngineState<'_>) -> MailResult<Vec<Account>> {
    engine.list_accounts()
}

#[tauri::command]
pub async fn add_account(
    engine: EngineState<'_>,
    settings: AccountSettings,
    password: String,
) -> MailResult<Account> {
    engine.add_account(settings, &password).await
}

#[tauri::command]
pub fn remove_account(engine: EngineState<'_>, account_id: i64) -> MailResult<()> {
    engine.remove_account(account_id)
}

#[tauri::command]
pub fn list_folders(engine: EngineState<'_>, account_id: i64) -> MailResult<Vec<Folder>> {
    engine.list_folders(account_id)
}

#[tauri::command]
pub async fn refresh_folders(engine: EngineState<'_>, account_id: i64) -> MailResult<Vec<Folder>> {
    engine.refresh_folders(account_id).await
}

#[tauri::command]
pub fn list_messages(engine: EngineState<'_>, folder_id: i64) -> MailResult<Vec<MessageSummary>> {
    engine.list_messages(folder_id)
}

#[tauri::command]
pub async fn sync_folder(
    engine: EngineState<'_>,
    folder_id: i64,
    grow: bool,
) -> MailResult<Vec<MessageSummary>> {
    engine.sync_folder(folder_id, grow).await
}

#[tauri::command]
pub async fn open_message(
    engine: EngineState<'_>,
    message_id: i64,
    allow_remote: bool,
) -> MailResult<MessageContent> {
    engine.open_message(message_id, allow_remote).await
}

#[tauri::command]
pub async fn set_seen(
    engine: EngineState<'_>,
    message_ids: Vec<i64>,
    seen: bool,
) -> MailResult<()> {
    engine.set_flag(&message_ids, Flag::Seen, seen).await
}

#[tauri::command]
pub async fn set_flagged(
    engine: EngineState<'_>,
    message_ids: Vec<i64>,
    flagged: bool,
) -> MailResult<()> {
    engine.set_flag(&message_ids, Flag::Flagged, flagged).await
}

#[tauri::command]
pub async fn delete_messages(engine: EngineState<'_>, message_ids: Vec<i64>) -> MailResult<()> {
    engine.delete_messages(&message_ids).await
}

#[tauri::command]
pub async fn move_messages(
    engine: EngineState<'_>,
    message_ids: Vec<i64>,
    folder_id: i64,
) -> MailResult<()> {
    engine.move_messages(&message_ids, folder_id).await
}

#[tauri::command]
pub async fn reply_draft(
    engine: EngineState<'_>,
    message_id: i64,
    mode: ReplyMode,
) -> MailResult<Draft> {
    engine.reply_draft(message_id, mode).await
}

#[tauri::command]
pub async fn send_message(engine: EngineState<'_>, message: OutgoingMessage) -> MailResult<()> {
    engine.send_message(&message).await
}

/// Writes an attachment to a path the user picked in the save dialog.
#[tauri::command]
pub async fn save_attachment(
    engine: EngineState<'_>,
    message_id: i64,
    index: u32,
    path: String,
) -> MailResult<()> {
    let (_, bytes) = engine.attachment(message_id, index).await?;

    std::fs::write(&path, bytes)
        .map_err(|error| MailError::new(ErrorKind::Storage, format!("{path}: {error}")))
}
