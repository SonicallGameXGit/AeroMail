use std::sync::Arc;

use async_trait::async_trait;
use mail_core::account::{ServerRole, ServerSettings};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::oauth::Tokens;
use mail_core::transport::{
    Connector, Envelope, IncomingSession, LogoSource, PendingSignIn, SecretStore, SignInFlow,
};
use mail_engine::Engine;
use mail_store::Store;
use serde_json::{Value, json};

use mail_core::mail::MailEvent;

use super::{dispatch, event_line, handle_line};

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

/// Every domain publishes the same tiny drawing.
struct OneLogo;

#[async_trait]
impl LogoSource for OneLogo {
    async fn logo(&self, _: &str) -> MailResult<Option<Vec<u8>>> {
        Ok(Some(b"<svg></svg>".to_vec()))
    }
}

/// A provider whose page is never answered.
struct SilentPage;

struct NeverDone;

#[async_trait]
impl PendingSignIn for NeverDone {
    fn url(&self) -> &str {
        "https://accounts.example.com/sign-in"
    }

    async fn finish(self: Box<Self>) -> MailResult<Tokens> {
        std::future::pending().await
    }
}

#[async_trait]
impl SignInFlow for SilentPage {
    async fn begin(&self, _: &str, _: Option<&str>) -> MailResult<Box<dyn PendingSignIn>> {
        Ok(Box::new(NeverDone))
    }

    async fn refresh(&self, _: &str, _: &str) -> MailResult<Tokens> {
        Err(MailError::new(ErrorKind::SignIn, "invalid_grant"))
    }
}

fn engine() -> Engine {
    Engine::new(
        Store::open_in_memory().unwrap(),
        Arc::new(Offline),
        Arc::new(NoSecrets),
        Arc::new(OneLogo),
        Arc::new(SilentPage),
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
async fn a_typed_address_is_recognized_under_the_names_the_ui_uses() {
    let known = dispatch(
        &engine(),
        "recognize_address",
        json!({ "email": "john@gmail" }),
    )
    .await;
    assert_eq!(
        known,
        Ok(json!({ "type": "known", "brand": "Gmail", "provider": "Gmail" }))
    );

    let mistyped = dispatch(
        &engine(),
        "recognize_address",
        json!({ "email": "john@gmail.ru" }),
    )
    .await;
    assert_eq!(
        mistyped,
        Ok(json!({ "type": "mistyped", "provider": "Gmail", "suggestion": "gmail.com" }))
    );
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
async fn a_sign_in_is_begun_finished_and_cancelled_under_the_names_the_ui_uses() {
    let engine = engine();
    let detection = dispatch(
        &engine,
        "detect_account_settings",
        json!({ "email": "someone@example.org" }),
    )
    .await
    .unwrap();
    let mut settings = detection["settings"].clone();
    settings["incoming"]["auth"] = json!("OAuth2");
    settings["outgoing"]["auth"] = json!("OAuth2");

    let started = dispatch(&engine, "begin_sign_in", json!({ "settings": settings }))
        .await
        .unwrap();
    assert_eq!(
        started,
        json!({ "id": 1, "url": "https://accounts.example.com/sign-in" })
    );

    let (finished, cancelled) = tokio::join!(
        dispatch(&engine, "finish_sign_in", json!({ "signInId": 1 })),
        dispatch(&engine, "cancel_sign_in", json!({ "signInId": 1 })),
    );
    assert_eq!(cancelled, Ok(Value::Null));
    assert_eq!(finished.unwrap_err()["kind"], "SignIn");
}

#[tokio::test]
async fn picking_an_account_on_the_providers_page_goes_by_the_names_the_ui_uses() {
    // Which providers are offered depends on the client ids the build was given.
    let choices = dispatch(&engine(), "sign_in_choices", Value::Null).await;
    assert!(choices.is_ok_and(|choices| choices.is_array()));

    let unknown = dispatch(
        &engine(),
        "begin_chosen_sign_in",
        json!({ "brand": "Nowhere" }),
    )
    .await
    .unwrap_err();
    assert_eq!(unknown["kind"], "Invalid");
}

#[tokio::test]
async fn address_presets_list_the_well_known_providers() {
    let presets = dispatch(&engine(), "address_presets", Value::Null)
        .await
        .unwrap();
    assert_eq!(
        presets[0],
        json!({ "provider": "Gmail", "domain": "gmail.com", "brand": "Gmail" }),
    );
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

#[tokio::test]
async fn settings_and_sender_pictures_go_under_the_names_the_ui_uses() {
    let engine = engine();
    let ask = || {
        dispatch(
            &engine,
            "sender_picture",
            json!({ "email": "a@example.com" }),
        )
    };

    assert_eq!(
        ask().await,
        Ok(json!("data:image/svg+xml;base64,PHN2Zz48L3N2Zz4="))
    );
    let notifications = json!({
        "enabled": true,
        "sound": true,
        "whileFocused": false,
        "duration": "System",
        "seconds": 10,
        "title": "{sender}",
        "body": "{subject}\n{preview}",
        "previewLength": 90,
        "icon": "Sender",
        "customIcon": null,
    });
    assert_eq!(
        dispatch(&engine, "settings", Value::Null).await,
        Ok(
            json!({ "senderPictures": true, "remoteImages": false, "switchToNewAccount": true, "theme": "System", "notifications": notifications, "conversations": { "enabled": true, "sameSubject": true, "otherFolders": true } })
        )
    );

    let quiet = json!({
        "enabled": false,
        "sound": false,
        "whileFocused": true,
        "duration": "UntilDismissed",
        "seconds": 30,
        "title": "{subject}",
        "body": "",
        "previewLength": 40,
        "icon": "None",
        "customIcon": null,
    });
    let grouping = json!({ "enabled": false, "sameSubject": false, "otherFolders": true });
    let changed = json!({ "senderPictures": false, "remoteImages": true, "switchToNewAccount": false, "theme": "Slate", "notifications": quiet, "conversations": grouping });
    let off = dispatch(&engine, "set_settings", json!({ "settings": changed })).await;
    assert_eq!(off, Ok(Value::Null));
    assert_eq!(
        dispatch(&engine, "settings", Value::Null).await,
        Ok(changed)
    );
    assert_eq!(ask().await, Ok(Value::Null));
}

#[tokio::test]
async fn conversations_are_asked_for_under_the_names_the_ui_uses() {
    let engine = engine();
    let kind = |answer: Result<Value, Value>| answer.unwrap_err()["kind"].clone();

    let listed = dispatch(&engine, "list_conversations", json!({ "folderId": 4 })).await;
    assert_eq!(kind(listed), "NotFound");
    let opened = dispatch(&engine, "conversation", json!({ "messageId": 9 })).await;
    assert_eq!(kind(opened), "NotFound");
}

#[test]
fn announcements_are_lines_without_an_id() {
    let line = event_line(&MailEvent::FolderChanged {
        account_id: 1,
        folder_id: 2,
    });

    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap(),
        json!({ "event": { "type": "folderChanged", "accountId": 1, "folderId": 2 } })
    );
}
