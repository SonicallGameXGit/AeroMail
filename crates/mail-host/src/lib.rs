//! The commands a shell can ask of the mail engine, as JSON in and JSON out.
//! Nothing here knows how the request arrived: the desktop shell sends lines
//! over stdin (`main.rs`), a web server would call `dispatch` from a handler.

#[cfg(test)]
mod tests;

use mail_core::account::{AccountSettings, Protocol, Security};
use mail_core::draft::ReplyMode;
use mail_core::error::{ErrorKind, MailError};
use mail_core::mail::{MailEvent, OutgoingMessage};
use mail_core::settings::AppSettings;
use mail_core::transport::Flag;
use mail_engine::Engine;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Request {
    id: u64,
    command: String,
    #[serde(default)]
    params: Value,
}

#[derive(Deserialize)]
struct Email {
    email: String,
}

#[derive(Deserialize)]
struct Settings {
    settings: AccountSettings,
}

#[derive(Deserialize)]
struct Port {
    protocol: Protocol,
    security: Security,
}

#[derive(Deserialize)]
struct NewAccount {
    settings: AccountSettings,
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignInId {
    sign_in_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountId {
    account_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FolderId {
    folder_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sync {
    folder_id: i64,
    grow: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Open {
    message_id: i64,
    allow_remote: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Seen {
    message_ids: Vec<i64>,
    seen: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Flagged {
    message_ids: Vec<i64>,
    flagged: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageIds {
    message_ids: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Move {
    message_ids: Vec<i64>,
    folder_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reply {
    message_id: i64,
    mode: ReplyMode,
}

#[derive(Deserialize)]
struct Send {
    message: OutgoingMessage,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Save {
    message_id: i64,
    index: u32,
    path: String,
}

#[derive(Deserialize)]
struct NewSettings {
    settings: AppSettings,
}

fn encode<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or_else(
        |error| json!({ "kind": "Protocol", "server": null, "detail": error.to_string() }),
    )
}

fn args<T: DeserializeOwned>(params: Value) -> Result<T, Value> {
    serde_json::from_value(params)
        .map_err(|error| encode(&MailError::new(ErrorKind::Invalid, error.to_string())))
}

fn reply<T: Serialize, E: Serialize>(result: Result<T, E>) -> Result<Value, Value> {
    match result {
        Ok(value) => Ok(encode(&value)),
        Err(error) => Err(encode(&error)),
    }
}

/// Runs one command. Both sides are JSON: the value on success, and on
/// failure a `MailError` (or an `AddressError` for `detect_account_settings`).
pub async fn dispatch(engine: &Engine, command: &str, params: Value) -> Result<Value, Value> {
    match command {
        "detect_account_settings" => {
            let Email { email } = args(params)?;
            reply(mail_autoconfig::detect(&email))
        }
        "validate_account_settings" => {
            let Settings { settings } = args(params)?;
            Ok(encode(&mail_core::account::validate(&settings)))
        }
        "default_port" => {
            let Port { protocol, security } = args(params)?;
            Ok(encode(&mail_core::account::default_port(
                protocol, security,
            )))
        }
        "list_accounts" => reply(engine.list_accounts()),
        "add_account" => {
            let NewAccount { settings, password } = args(params)?;
            reply(engine.add_account(settings, &password).await)
        }
        // Adding an account on the provider's own page: the shell opens the
        // address `begin_sign_in` gives in the browser, `finish_sign_in`
        // answers once the user is done there.
        "begin_sign_in" => {
            let Settings { settings } = args(params)?;
            reply(engine.begin_sign_in(settings).await)
        }
        "finish_sign_in" => {
            let SignInId { sign_in_id } = args(params)?;
            reply(engine.finish_sign_in(sign_in_id).await)
        }
        "cancel_sign_in" => {
            let SignInId { sign_in_id } = args(params)?;
            reply(engine.cancel_sign_in(sign_in_id))
        }
        "remove_account" => {
            let AccountId { account_id } = args(params)?;
            reply(engine.remove_account(account_id))
        }
        "list_folders" => {
            let AccountId { account_id } = args(params)?;
            reply(engine.list_folders(account_id))
        }
        "refresh_folders" => {
            let AccountId { account_id } = args(params)?;
            reply(engine.refresh_folders(account_id).await)
        }
        "list_messages" => {
            let FolderId { folder_id } = args(params)?;
            reply(engine.list_messages(folder_id))
        }
        "sync_folder" => {
            let Sync { folder_id, grow } = args(params)?;
            reply(engine.sync_folder(folder_id, grow).await)
        }
        "open_message" => {
            let Open {
                message_id,
                allow_remote,
            } = args(params)?;
            reply(engine.open_message(message_id, allow_remote).await)
        }
        "set_seen" => {
            let Seen { message_ids, seen } = args(params)?;
            reply(engine.set_flag(&message_ids, Flag::Seen, seen).await)
        }
        "set_flagged" => {
            let Flagged {
                message_ids,
                flagged,
            } = args(params)?;
            reply(engine.set_flag(&message_ids, Flag::Flagged, flagged).await)
        }
        "delete_messages" => {
            let MessageIds { message_ids } = args(params)?;
            reply(engine.delete_messages(&message_ids).await)
        }
        "move_messages" => {
            let Move {
                message_ids,
                folder_id,
            } = args(params)?;
            reply(engine.move_messages(&message_ids, folder_id).await)
        }
        "reply_draft" => {
            let Reply { message_id, mode } = args(params)?;
            reply(engine.reply_draft(message_id, mode).await)
        }
        "send_message" => {
            let Send { message } = args(params)?;
            reply(engine.send_message(&message).await)
        }
        // Writes an attachment to a path the user picked in the save dialog.
        "save_attachment" => {
            let Save {
                message_id,
                index,
                path,
            } = args(params)?;
            let saved = match engine.attachment(message_id, index).await {
                Ok((_, bytes)) => std::fs::write(&path, bytes).map_err(|error| {
                    MailError::new(ErrorKind::Storage, format!("{path}: {error}"))
                }),
                Err(error) => Err(error),
            };
            reply(saved)
        }
        "sender_picture" => {
            let Email { email } = args(params)?;
            reply(engine.sender_picture(&email).await)
        }
        "settings" => reply(engine.settings()),
        "default_settings" => Ok(encode(&AppSettings::default())),
        "set_settings" => {
            let NewSettings { settings } = args(params)?;
            reply(engine.set_settings(settings))
        }
        _ => Err(encode(&MailError::new(
            ErrorKind::Invalid,
            format!("unknown command: {command}"),
        ))),
    }
}

/// An announcement of the engine as one line: `{ "event": ... }`. It has no
/// `id`, which is how a shell tells it from the answer to a request.
pub fn event_line(event: &MailEvent) -> String {
    json!({ "event": encode(event) }).to_string()
}

/// Answers one request line with one response line: `{ id, ok, value }` or
/// `{ id, ok, error }`. A line that is not a request has no id to answer to
/// and gives `None`.
pub async fn handle_line(engine: &Engine, line: &str) -> Option<String> {
    let request: Request = serde_json::from_str(line).ok()?;
    let response = match dispatch(engine, &request.command, request.params).await {
        Ok(value) => json!({ "id": request.id, "ok": true, "value": value }),
        Err(error) => json!({ "id": request.id, "ok": false, "error": error }),
    };

    Some(response.to_string())
}
