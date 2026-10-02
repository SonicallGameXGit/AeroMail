use std::sync::Arc;

use async_trait::async_trait;
use mail_core::account::{ServerRole, ServerSettings};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::transport::{Connector, Envelope, IncomingSession, SecretStore};
use mail_engine::Engine;
use mail_store::Store;
use serde_json::{Value, json};

use super::{dispatch, handle_line};

/// No server is reachable: every connection attempt fails.
struct Offline;

fn offline(role: ServerRole) -> MailError {
    MailError::new(ErrorKind::Connect, "offline").on(role)
}

#[async_trait]
impl Connector for Offline {
    async fn connect_incoming(
        &self,
        _: &ServerSettings,
        _: &str,
    ) -> MailResult<Box<dyn IncomingSession>> {
        Err(offline(ServerRole::Incoming))
    }

    async fn check_outgoing(&self, _: &ServerSettings, _: &str) -> MailResult<()> {
        Err(offline(ServerRole::Outgoing))
    }

    async fn send(&self, _: &ServerSettings, _: &str, _: Envelope<'_>, _: &[u8]) -> MailResult<()> {
        Err(offline(ServerRole::Outgoing))
    }
}

struct NoSecrets;

impl SecretStore for NoSecrets {
    fn get(&self, _: &str) -> MailResult<Option<String>> {
        Ok(None)
    }

    fn set(&self, _: &str, _: &str) -> MailResult<()> {
        Ok(())
    }

    fn delete(&self, _: &str) -> MailResult<()> {
        Ok(())
    }
}

fn engine() -> Engine {
    Engine::new(
        Store::open_in_memory().unwrap(),
        Arc::new(Offline),
        Arc::new(NoSecrets),
    )
}

#[tokio::test]
async fn a_command_without_parameters_answers() {
    let accounts = dispatch(&engine(), "list_accounts", Value::Null).await;
    assert_eq!(accounts, Ok(json!([])));
}

#[tokio::test]
async fn parameters_arrive_under_the_names_the_ui_uses() {
    let port = dispatch(
        &engine(),
        "default_port",
        json!({ "protocol": "Imap", "security": "Tls" }),
    )
    .await;
    assert_eq!(port, Ok(json!(993)));

    // camelCase, as the UI sends it.
    let folders = dispatch(&engine(), "list_folders", json!({ "accountId": 1 })).await;
    assert_eq!(folders, Ok(json!([])));
}

#[tokio::test]
async fn a_bad_address_fails_with_the_address_error() {
    let detection = dispatch(
        &engine(),
        "detect_account_settings",
        json!({ "email": "nobody" }),
    )
    .await;
    assert_eq!(detection, Err(json!("MissingAt")));
}

#[tokio::test]
async fn an_engine_failure_keeps_its_kind_and_server() {
    let detection = dispatch(
        &engine(),
        "detect_account_settings",
        json!({ "email": "someone@example.org" }),
    )
    .await
    .unwrap();

    let added = dispatch(
        &engine(),
        "add_account",
        json!({ "settings": detection["settings"], "password": "secret" }),
    )
    .await
    .unwrap_err();
    assert_eq!(added["kind"], "Connect");
    assert_eq!(added["server"], "Incoming");
}

#[tokio::test]
async fn unknown_commands_and_wrong_parameters_are_invalid() {
    let unknown = dispatch(&engine(), "format_disk", Value::Null)
        .await
        .unwrap_err();
    assert_eq!(unknown["kind"], "Invalid");

    let wrong = dispatch(&engine(), "list_folders", json!({ "account": "one" }))
        .await
        .unwrap_err();
    assert_eq!(wrong["kind"], "Invalid");
}

#[tokio::test]
async fn a_line_is_answered_under_its_own_id() {
    let engine = engine();

    let done = handle_line(&engine, r#"{"id":7,"command":"list_accounts"}"#)
        .await
        .unwrap();
    let done: Value = serde_json::from_str(&done).unwrap();
    assert_eq!(done, json!({ "id": 7, "ok": true, "value": [] }));

    let failed = handle_line(&engine, r#"{"id":8,"command":"nope","params":{}}"#)
        .await
        .unwrap();
    let failed: Value = serde_json::from_str(&failed).unwrap();
    assert_eq!(failed["id"], 8);
    assert_eq!(failed["ok"], false);
    assert_eq!(failed["error"]["kind"], "Invalid");
}

#[tokio::test]
async fn a_line_that_is_not_a_request_gets_no_answer() {
    assert_eq!(handle_line(&engine(), "hello").await, None);
}
