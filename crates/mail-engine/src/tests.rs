use std::sync::Arc;

use mail_core::account::{AccountSettings, ServerRole};
use mail_core::category::{Category, CategoryRule};
use mail_core::draft::ReplyMode;
use mail_core::error::ErrorKind;
use mail_core::mail::{
    Account, Folder, FolderRole, MailEvent, MessageContent, MessageSummary, OutgoingMessage,
};
use mail_core::settings::AppSettings;
use mail_core::transport::Flag;
use mail_store::Store;

use crate::testing::{
    FakeConnector, FakeLogos, FakeSecrets, FakeSignIn, LOGO, PASSWORD, REFRESH_TOKEN, Remote,
    SIGN_IN_URL, SharedRemote, settings, sign_in_settings,
};
use crate::{Engine, LOGO_TTL, PAGE, unix_now};

struct World {
    engine: Engine,
    remote: SharedRemote,
    secrets: Arc<FakeSecrets>,
    logos: Arc<FakeLogos>,
    sign_in: Arc<FakeSignIn>,
}

fn world() -> World {
    let remote = Remote::standard();
    let secrets = Arc::new(FakeSecrets::default());
    let connector = Arc::new(FakeConnector {
        remote: Arc::clone(&remote),
    });
    let logos = Arc::new(FakeLogos::default());
    let sign_in = Arc::new(FakeSignIn::new(Arc::clone(&remote)));
    let engine = Engine::new(
        Store::open_in_memory().unwrap(),
        connector,
        secrets.clone(),
        logos.clone(),
        sign_in.clone(),
    );

    World {
        engine,
        remote,
        secrets,
        logos,
        sign_in,
    }
}

/// An account added by signing in on the provider's page, the user approving.
async fn with_signed_in_account() -> (World, Account) {
    let world = world();
    let started = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();
    world.sign_in.approve();
    let account = world.engine.finish_sign_in(started.id).await.unwrap();

    (world, account)
}

fn kept_secret(world: &World, account: &Account) -> Option<String> {
    world
        .secrets
        .values
        .lock()
        .unwrap()
        .get(&format!("account-{}", account.id))
        .cloned()
}

async fn with_account() -> (World, Account) {
    let world = world();
    let account = world
        .engine
        .add_account(settings(), PASSWORD)
        .await
        .unwrap();

    (world, account)
}

fn folder(world: &World, account: &Account, role: FolderRole) -> Folder {
    world
        .engine
        .list_folders(account.id)
        .unwrap()
        .into_iter()
        .find(|folder| folder.role == role)
        .unwrap()
}

fn outgoing(account: &Account) -> OutgoingMessage {
    OutgoingMessage {
        account_id: account.id,
        to: vec!["ann@example.com".to_string()],
        cc: Vec::new(),
        bcc: Vec::new(),
        subject: "Hello".to_string(),
        body: "How are you?".to_string(),
        in_reply_to: None,
        references: Vec::new(),
    }
}

// Accounts

#[tokio::test]
async fn adding_an_account_stores_it_with_its_folders_and_password() {
    let (world, account) = with_account().await;

    assert_eq!(world.engine.list_accounts().unwrap(), vec![account.clone()]);
    let roles: Vec<FolderRole> = world
        .engine
        .list_folders(account.id)
        .unwrap()
        .iter()
        .map(|folder| folder.role)
        .collect();
    assert_eq!(
        roles,
        vec![
            FolderRole::Inbox,
            FolderRole::Sent,
            FolderRole::Archive,
            FolderRole::Trash
        ],
    );
    assert_eq!(
        world
            .secrets
            .values
            .lock()
            .unwrap()
            .get("account-1")
            .map(String::as_str),
        Some(PASSWORD),
    );
}

#[tokio::test]
async fn a_wrong_password_adds_nothing() {
    let world = world();

    let error = world
        .engine
        .add_account(settings(), "wrong")
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Auth);
    assert_eq!(error.server, Some(ServerRole::Incoming));
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
    assert!(world.secrets.values.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_broken_outgoing_server_adds_nothing_and_says_which_side_failed() {
    let world = world();
    world.remote.lock().unwrap().outgoing_down = true;

    let error = world
        .engine
        .add_account(settings(), PASSWORD)
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Connect);
    assert_eq!(error.server, Some(ServerRole::Outgoing));
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
}

#[tokio::test]
async fn invalid_settings_are_rejected_before_connecting() {
    let world = world();
    let mut bad = settings();
    bad.incoming.host = String::new();

    let error = world.engine.add_account(bad, PASSWORD).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);
    assert_eq!(error.server, Some(ServerRole::Incoming));

    let error = world.engine.add_account(settings(), "").await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);

    assert_eq!(world.remote.lock().unwrap().connections, 0);
}

#[tokio::test]
async fn the_same_account_cannot_be_added_twice() {
    let (world, _) = with_account().await;

    let error = world
        .engine
        .add_account(settings(), PASSWORD)
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Duplicate);
    assert_eq!(world.engine.list_accounts().unwrap().len(), 1);
}

#[tokio::test]
async fn an_unavailable_keychain_adds_nothing() {
    let world = world();
    *world.secrets.unavailable.lock().unwrap() = true;

    let error = world
        .engine
        .add_account(settings(), PASSWORD)
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Keychain);
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
}

#[tokio::test]
async fn removing_an_account_forgets_everything_local_but_leaves_the_server_alone() {
    let (world, account) = with_account().await;
    world.remote.lock().unwrap().deliver("INBOX", "Keep", false);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    world.engine.remove_account(account.id).unwrap();

    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
    assert!(world.secrets.values.lock().unwrap().is_empty());
    assert_eq!(world.remote.lock().unwrap().uids("INBOX").len(), 1);
}

// Signing in on the provider's page

#[tokio::test]
async fn signing_in_adds_the_account_and_keeps_only_the_lasting_token() {
    let world = world();

    let started = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();
    assert_eq!(started.url, SIGN_IN_URL);
    // Nothing exists until the user is done in the browser.
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());

    world.sign_in.approve();
    let account = world.engine.finish_sign_in(started.id).await.unwrap();

    assert_eq!(world.engine.list_accounts().unwrap(), vec![account.clone()]);
    assert_eq!(
        *world.sign_in.begun.lock().unwrap(),
        vec![(
            "imap.example.com".to_string(),
            Some("me@example.com".to_string())
        )],
    );
    assert_eq!(
        kept_secret(&world, &account).as_deref(),
        Some(REFRESH_TOKEN)
    );
    // Both servers were opened with the access token, never the lasting one.
    assert_eq!(
        world.remote.lock().unwrap().secrets_used,
        vec!["access-1".to_string(), "access-1".to_string()],
    );
    assert_eq!(folder(&world, &account, FolderRole::Inbox).path, "INBOX");
}

#[tokio::test]
async fn a_sign_in_the_user_refuses_adds_nothing() {
    let world = world();
    let started = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();

    world.sign_in.refuse();
    let error = world.engine.finish_sign_in(started.id).await.unwrap_err();

    assert_eq!(error.kind, ErrorKind::SignIn);
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
    assert!(world.secrets.values.lock().unwrap().is_empty());
    assert_eq!(world.remote.lock().unwrap().connections, 0);
}

#[tokio::test]
async fn a_cancelled_sign_in_stops_waiting_and_adds_nothing() {
    let world = world();
    let started = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();

    // The user never comes back from the browser.
    let (finished, cancelled) = tokio::join!(world.engine.finish_sign_in(started.id), async {
        world.engine.cancel_sign_in(started.id)
    });

    cancelled.unwrap();
    assert_eq!(finished.unwrap_err().kind, ErrorKind::SignIn);
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
    // Cancelling again, or something that never existed, is not an error.
    world.engine.cancel_sign_in(started.id).unwrap();
    world.engine.cancel_sign_in(99).unwrap();
}

#[tokio::test]
async fn starting_another_sign_in_gives_the_earlier_one_up() {
    let world = world();
    let first = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();
    let second = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();
    assert_ne!(first.id, second.id);

    let error = world.engine.finish_sign_in(first.id).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::SignIn);

    world.sign_in.approve();
    world.engine.finish_sign_in(second.id).await.unwrap();
    assert_eq!(world.engine.list_accounts().unwrap().len(), 1);
}

#[tokio::test]
async fn the_two_ways_to_sign_in_do_not_mix() {
    let world = world();

    // Settings for the provider's page cannot be added with a password...
    let error = world
        .engine
        .add_account(sign_in_settings(), PASSWORD)
        .await
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);

    // ...password settings have no page to sign in on...
    let error = world.engine.begin_sign_in(settings()).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);

    // ...and the two servers of one account sign in the same way.
    let mut mixed = settings();
    mixed.incoming.auth = mail_core::account::AuthMethod::OAuth2;
    let error = world.engine.begin_sign_in(mixed).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);
    assert_eq!(error.server, Some(ServerRole::Outgoing));

    assert!(world.sign_in.begun.lock().unwrap().is_empty());
    assert_eq!(world.remote.lock().unwrap().connections, 0);
}

#[tokio::test]
async fn a_server_without_a_sign_in_page_says_so_before_any_browser_opens() {
    let world = world();
    let mut elsewhere = sign_in_settings();
    elsewhere.incoming.host = "imap.elsewhere.example".to_string();

    let error = world.engine.begin_sign_in(elsewhere).await.unwrap_err();

    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[tokio::test]
async fn an_account_that_is_already_added_does_not_open_the_browser_again() {
    let (world, _) = with_signed_in_account().await;

    let error = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Duplicate);
    assert_eq!(world.sign_in.begun.lock().unwrap().len(), 1);
}

/// Settings for the provider's servers, before anyone has picked an account.
fn no_address(mut settings: AccountSettings) -> AccountSettings {
    settings.email = String::new();
    settings.incoming.username = String::new();
    settings.outgoing.username = String::new();

    settings
}

#[tokio::test]
async fn an_account_picked_on_the_providers_page_is_added_under_its_address() {
    let world = world();

    let started = world
        .engine
        .begin_chosen_sign_in(no_address(sign_in_settings()))
        .await
        .unwrap();
    assert_eq!(started.url, SIGN_IN_URL);
    assert_eq!(
        *world.sign_in.begun.lock().unwrap(),
        vec![("imap.example.com".to_string(), None)],
    );

    world.sign_in.approve_as(Some("Picked@Example.com"));
    let account = world.engine.finish_sign_in(started.id).await.unwrap();

    assert_eq!(account.settings.email, "Picked@Example.com");
    assert_eq!(account.settings.incoming.username, "Picked@Example.com");
    assert_eq!(account.settings.outgoing.username, "Picked@Example.com");
    assert_eq!(world.engine.list_accounts().unwrap(), vec![account.clone()]);
    assert_eq!(
        kept_secret(&world, &account).as_deref(),
        Some(REFRESH_TOKEN)
    );
}

#[tokio::test]
async fn picking_an_account_that_is_already_added_adds_nothing() {
    let (world, existing) = with_signed_in_account().await;

    let started = world
        .engine
        .begin_chosen_sign_in(no_address(sign_in_settings()))
        .await
        .unwrap();
    world.sign_in.approve_as(Some("me@example.com"));
    let error = world.engine.finish_sign_in(started.id).await.unwrap_err();

    assert_eq!(error.kind, ErrorKind::Duplicate);
    assert_eq!(world.engine.list_accounts().unwrap(), vec![existing]);
}

#[tokio::test]
async fn a_provider_that_does_not_say_which_account_was_picked_adds_nothing() {
    let world = world();

    let started = world
        .engine
        .begin_chosen_sign_in(no_address(sign_in_settings()))
        .await
        .unwrap();
    world.sign_in.approve_as(None);
    let error = world.engine.finish_sign_in(started.id).await.unwrap_err();

    assert_eq!(error.kind, ErrorKind::SignIn);
    assert_eq!(world.engine.list_accounts().unwrap(), Vec::new());
    assert!(world.secrets.values.lock().unwrap().is_empty());
}

#[tokio::test]
async fn picking_an_account_needs_settings_for_the_providers_page() {
    let world = world();

    let error = world
        .engine
        .begin_chosen_sign_in(no_address(settings()))
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Invalid);
    assert!(world.sign_in.begun.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_access_token_is_reused_while_it_lasts() {
    let (world, account) = with_signed_in_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);

    world.remote.lock().unwrap().drop_connection = true;
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap();

    assert_eq!(*world.sign_in.renewals.lock().unwrap(), 0);
    let remote = world.remote.lock().unwrap();
    assert_eq!(remote.connections, 2);
    assert!(remote.secrets_used.iter().all(|used| used == "access-1"));
    assert_eq!(remote.sent.len(), 1);
}

#[tokio::test]
async fn an_access_token_that_ran_out_is_renewed_without_asking_the_user() {
    let world = world();
    // Every token is used up the moment it is handed out.
    *world.sign_in.lifetime.lock().unwrap() = 0;
    let started = world
        .engine
        .begin_sign_in(sign_in_settings())
        .await
        .unwrap();
    world.sign_in.approve();
    let account = world.engine.finish_sign_in(started.id).await.unwrap();
    let inbox = folder(&world, &account, FolderRole::Inbox);

    world.remote.lock().unwrap().drop_connection = true;
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap();

    assert_eq!(*world.sign_in.renewals.lock().unwrap(), 2);
    assert_eq!(
        world
            .remote
            .lock()
            .unwrap()
            .secrets_used
            .last()
            .map(String::as_str),
        Some("access-3"),
    );
    // What renews the tokens stays the same in the keychain.
    assert_eq!(
        kept_secret(&world, &account).as_deref(),
        Some(REFRESH_TOKEN)
    );
}

#[tokio::test]
async fn a_permission_taken_back_at_the_provider_asks_for_a_new_sign_in() {
    let (world, account) = with_signed_in_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);

    // The servers stop accepting the token the app still believes in.
    *world.sign_in.revoked.lock().unwrap() = true;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.access_tokens.clear();
        remote.drop_connection = true;
    }

    let refused = world.engine.sync_folder(inbox.id, false).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Auth);
    // The refused token is not tried again: the renewal tells what is wrong.
    world.remote.lock().unwrap().drop_connection = true;
    let renewal = world.engine.sync_folder(inbox.id, false).await.unwrap_err();
    assert_eq!(renewal.kind, ErrorKind::SignIn);
    let sending = world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap_err();
    assert_eq!(sending.kind, ErrorKind::SignIn);
}

#[tokio::test]
async fn a_token_the_server_refuses_early_is_renewed_on_the_next_try() {
    let (world, account) = with_signed_in_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    {
        let mut remote = world.remote.lock().unwrap();
        remote.access_tokens.clear();
        remote.drop_connection = true;
    }

    world.engine.sync_folder(inbox.id, false).await.unwrap_err();
    world.remote.lock().unwrap().drop_connection = true;
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    assert_eq!(*world.sign_in.renewals.lock().unwrap(), 1);
}

#[tokio::test]
async fn removing_a_signed_in_account_forgets_its_tokens() {
    let (world, account) = with_signed_in_account().await;

    world.engine.remove_account(account.id).unwrap();

    assert!(world.secrets.values.lock().unwrap().is_empty());
    assert!(world.engine.access_tokens().unwrap().is_empty());
}

// Reading

#[tokio::test]
async fn sync_shows_new_mail_newest_first_with_decoded_headers() {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.deliver("INBOX", "First", true);
        remote.deliver("INBOX", "Second", false);
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);

    let messages = world.engine.sync_folder(inbox.id, false).await.unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].subject, "Second");
    assert!(!messages[0].flags.seen);
    assert_eq!(messages[1].subject, "First");
    assert!(messages[1].flags.seen);
    assert_eq!(messages[0].from[0].name.as_deref(), Some("Ann"));
    assert_eq!(messages[0].date, 1_700_000_000);

    let inbox = folder(&world, &account, FolderRole::Inbox);
    assert_eq!((inbox.unread, inbox.total), (1, 2));
    assert_eq!(world.engine.list_messages(inbox.id).unwrap(), messages);
}

#[tokio::test]
async fn sync_follows_changes_made_elsewhere() {
    let (world, account) = with_account().await;
    let (first, second) = {
        let mut remote = world.remote.lock().unwrap();
        (
            remote.deliver("INBOX", "First", false),
            remote.deliver("INBOX", "Second", false),
        )
    };
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let before = world.engine.sync_folder(inbox.id, false).await.unwrap();

    {
        let mut remote = world.remote.lock().unwrap();
        let folder = remote.folder("INBOX");
        folder.messages.retain(|message| message.uid != first);
        folder
            .messages
            .iter_mut()
            .find(|message| message.uid == second)
            .unwrap()
            .flags
            .seen = true;
        remote.deliver("INBOX", "Third", false);
    }
    let after = world.engine.sync_folder(inbox.id, false).await.unwrap();

    let subjects: Vec<&str> = after
        .iter()
        .map(|message| message.subject.as_str())
        .collect();
    assert_eq!(subjects, vec!["Third", "Second"]);
    assert!(after[1].flags.seen);
    // A message that stays keeps its local id.
    assert_eq!(after[1].id, before[0].id);
}

#[tokio::test]
async fn only_the_newest_page_is_loaded_until_more_is_asked_for() {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        for number in 0..(PAGE + 20) {
            remote.deliver("INBOX", &format!("Message-{number}"), true);
        }
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);

    let first = world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert_eq!(first.len(), PAGE);
    assert_eq!(first[0].subject, format!("Message-{}", PAGE + 19));

    let again = world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert_eq!(again.len(), PAGE);

    let more = world.engine.sync_folder(inbox.id, true).await.unwrap();
    assert_eq!(more.len(), PAGE + 20);
    assert_eq!(more[PAGE + 19].subject, "Message-0");
    assert_eq!(
        folder(&world, &account, FolderRole::Inbox).total as usize,
        PAGE + 20
    );
}

#[tokio::test]
async fn a_renumbered_folder_is_reloaded_from_scratch() {
    let (world, account) = with_account().await;
    world.remote.lock().unwrap().deliver("INBOX", "Old", false);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    {
        let mut remote = world.remote.lock().unwrap();
        let folder = remote.folder("INBOX");
        folder.uid_validity = 2;
        folder.messages.clear();
        folder.next_uid = 1;
        remote.deliver("INBOX", "New", false);
    }
    let messages = world.engine.sync_folder(inbox.id, false).await.unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].subject, "New");
}

#[tokio::test]
async fn opening_a_message_downloads_it_once_and_does_not_mark_it_read() {
    let (world, account) = with_account().await;
    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Hello", false);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;

    let content = world.engine.open_message(id, false).await.unwrap();
    assert_eq!(
        content.text.as_deref().map(str::trim_end),
        Some("Body of Hello")
    );
    assert_eq!(content.html, None);
    assert_eq!(content.message_id.as_deref(), Some("Hello@example.com"));
    assert!(!content.summary.flags.seen);

    world.engine.open_message(id, true).await.unwrap();
    assert_eq!(world.remote.lock().unwrap().body_downloads, 1);
    assert!(
        !world.remote.lock().unwrap().folder("INBOX").messages[0]
            .flags
            .seen
    );
}

#[tokio::test]
async fn remote_images_are_blocked_until_asked_for_or_allowed_in_the_settings() {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.deliver("INBOX", "News", false);
        remote.folder("INBOX").messages[0].raw = b"From: Ann <ann@example.com>\r\n\
            Subject: News\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
            <p>Hi</p><img src=\"https://tracker.example/pixel.png\">\r\n"
            .to_vec();
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;
    let shows_image = |content: &MessageContent| {
        content
            .html
            .as_deref()
            .is_some_and(|html| html.contains("tracker.example"))
    };

    let blocked = world.engine.open_message(id, false).await.unwrap();
    assert!(blocked.remote_content_blocked);
    assert!(!shows_image(&blocked));

    let asked = world.engine.open_message(id, true).await.unwrap();
    assert!(!asked.remote_content_blocked);
    assert!(shows_image(&asked));

    let always = AppSettings {
        remote_images: true,
        ..AppSettings::default()
    };
    world.engine.set_settings(always).unwrap();
    let allowed = world.engine.open_message(id, false).await.unwrap();
    assert!(!allowed.remote_content_blocked);
    assert!(shows_image(&allowed));

    world.engine.set_settings(AppSettings::default()).unwrap();
    let again = world.engine.open_message(id, false).await.unwrap();
    assert!(again.remote_content_blocked);
}

#[tokio::test]
async fn a_dropped_connection_is_reopened_and_one_connection_is_reused_otherwise() {
    let (world, account) = with_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);

    world.engine.sync_folder(inbox.id, false).await.unwrap();
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert_eq!(world.remote.lock().unwrap().connections, 1);

    world.remote.lock().unwrap().drop_connection = true;
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert_eq!(world.remote.lock().unwrap().connections, 2);
}

// Changing

#[tokio::test]
async fn read_and_flag_changes_reach_the_server_and_the_counts() {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.deliver("INBOX", "One", false);
        remote.deliver("INBOX", "Two", false);
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let ids: Vec<i64> = world
        .engine
        .sync_folder(inbox.id, false)
        .await
        .unwrap()
        .iter()
        .map(|message| message.id)
        .collect();

    world.engine.set_flag(&ids, Flag::Seen, true).await.unwrap();
    assert_eq!(folder(&world, &account, FolderRole::Inbox).unread, 0);
    assert!(
        world
            .remote
            .lock()
            .unwrap()
            .folder("INBOX")
            .messages
            .iter()
            .all(|message| message.flags.seen)
    );

    // Marking read twice must not push the count below zero.
    world.engine.set_flag(&ids, Flag::Seen, true).await.unwrap();
    assert_eq!(folder(&world, &account, FolderRole::Inbox).unread, 0);

    world
        .engine
        .set_flag(&ids[..1], Flag::Seen, false)
        .await
        .unwrap();
    world
        .engine
        .set_flag(&ids[..1], Flag::Flagged, true)
        .await
        .unwrap();
    assert_eq!(folder(&world, &account, FolderRole::Inbox).unread, 1);
    let cached = world.engine.list_messages(inbox.id).unwrap();
    assert!(!cached[0].flags.seen);
    assert!(cached[0].flags.flagged);
    assert!(cached[1].flags.seen);
}

#[tokio::test]
async fn deleting_moves_to_trash_and_deleting_from_trash_is_final() {
    let (world, account) = with_account().await;
    world.remote.lock().unwrap().deliver("INBOX", "Bye", false);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let trash = folder(&world, &account, FolderRole::Trash);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;

    world.engine.delete_messages(&[id]).await.unwrap();

    assert_eq!(world.engine.list_messages(inbox.id).unwrap(), Vec::new());
    let inbox_now = folder(&world, &account, FolderRole::Inbox);
    assert_eq!((inbox_now.unread, inbox_now.total), (0, 0));
    assert_eq!(
        world.remote.lock().unwrap().uids("INBOX"),
        Vec::<u32>::new()
    );
    assert_eq!(world.remote.lock().unwrap().uids("Trash").len(), 1);

    let trashed = world.engine.sync_folder(trash.id, false).await.unwrap();
    assert_eq!(trashed[0].subject, "Bye");
    world
        .engine
        .delete_messages(&[trashed[0].id])
        .await
        .unwrap();

    assert_eq!(
        world.remote.lock().unwrap().uids("Trash"),
        Vec::<u32>::new()
    );
    assert_eq!(world.engine.list_messages(trash.id).unwrap(), Vec::new());
}

#[tokio::test]
async fn moving_takes_the_message_to_the_target_folder() {
    let (world, account) = with_account().await;
    world.remote.lock().unwrap().deliver("INBOX", "Keep", true);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let archive = folder(&world, &account, FolderRole::Archive);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;

    world.engine.move_messages(&[id], archive.id).await.unwrap();

    assert_eq!(world.engine.list_messages(inbox.id).unwrap(), Vec::new());
    let archived = world.engine.sync_folder(archive.id, false).await.unwrap();
    assert_eq!(archived[0].subject, "Keep");
    assert_eq!(
        world.engine.open_message(id, false).await.unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[tokio::test]
async fn a_flag_change_names_only_the_messages_it_changed() {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.deliver("INBOX", "Read", true);
        remote.deliver("INBOX", "Unread", false);
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let ids: Vec<i64> = world
        .engine
        .sync_folder(inbox.id, false)
        .await
        .unwrap()
        .iter()
        .map(|message| message.id)
        .collect();

    // Newest first: "Unread" is the first.
    let changed = world.engine.set_flag(&ids, Flag::Seen, true).await.unwrap();
    assert_eq!(changed, vec![ids[0]]);

    let changed = world
        .engine
        .set_flag(&ids, Flag::Flagged, false)
        .await
        .unwrap();
    assert_eq!(changed, Vec::<i64>::new());
}

/// Delivers `subjects` to the inbox, unread, and returns the account's world
/// with the inbox synced, so arrivals after it are news.
async fn with_inbox(subjects: &[&str]) -> (World, Account, Folder, Vec<MessageSummary>) {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        for subject in subjects {
            remote.deliver("INBOX", subject, false);
        }
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let messages = world.engine.sync_folder(inbox.id, false).await.unwrap();

    (world, account, inbox, messages)
}

#[tokio::test]
async fn a_deleted_message_is_put_back_quietly_under_a_new_id() {
    let (world, account, inbox, messages) = with_inbox(&["Keep", "Oops"]).await;
    let oops = messages
        .iter()
        .find(|message| message.subject == "Oops")
        .unwrap();
    let trash = folder(&world, &account, FolderRole::Trash);
    let mut events = world.engine.subscribe();

    let moves = world.engine.delete_messages(&[oops.id]).await.unwrap();
    assert_eq!(moves[0].from_folder_id, inbox.id);
    assert_eq!(moves[0].to_folder_id, Some(trash.id));
    assert_eq!(
        moves[0].messages[0].message_id.as_deref(),
        Some("Oops@example.com")
    );

    let restored = world.engine.put_back(&moves).await.unwrap();

    assert_eq!(
        world.remote.lock().unwrap().uids("Trash"),
        Vec::<u32>::new()
    );
    let cached = world.engine.list_messages(inbox.id).unwrap();
    assert_eq!(subjects(&cached), vec!["Oops", "Keep"]);
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].was, oops.id);
    assert_eq!(restored[0].now, cached[0].id);
    assert_ne!(restored[0].now, oops.id);
    let now = folder(&world, &account, FolderRole::Inbox);
    assert_eq!((now.unread, now.total), (2, 2));
    // Unread mail coming back to the inbox is not new mail.
    assert!(
        !heard(&mut events)
            .iter()
            .any(|event| matches!(event, MailEvent::NewMail { .. }))
    );

    // A later sync finds nothing to catch up on.
    let synced = world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert_eq!(subjects(&synced), vec!["Oops", "Keep"]);
}

#[tokio::test]
async fn mail_arriving_while_something_is_put_back_is_still_news() {
    let (world, _account, _inbox, messages) = with_inbox(&["Keep", "Oops"]).await;
    let moves = world
        .engine
        .delete_messages(&[messages[0].id])
        .await
        .unwrap();
    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Fresh", false);
    let mut events = world.engine.subscribe();

    world.engine.put_back(&moves).await.unwrap();

    let news: Vec<String> = heard(&mut events)
        .into_iter()
        .filter_map(|event| match event {
            MailEvent::NewMail { messages, .. } => Some(messages),
            _ => None,
        })
        .flatten()
        .map(|item| item.subject)
        .collect();
    assert_eq!(news, vec!["Fresh".to_string()]);
}

#[tokio::test]
async fn a_moved_message_is_put_back_and_one_deleted_for_good_is_not() {
    let (world, account, inbox, messages) = with_inbox(&["Filed"]).await;
    let archive = folder(&world, &account, FolderRole::Archive);

    let moves = world
        .engine
        .move_messages(&[messages[0].id], archive.id)
        .await
        .unwrap();
    assert!(!moves[0].kept);
    world.engine.put_back(&moves).await.unwrap();

    assert_eq!(
        world.remote.lock().unwrap().uids("Archive"),
        Vec::<u32>::new()
    );
    assert_eq!(
        subjects(&world.engine.list_messages(inbox.id).unwrap()),
        vec!["Filed"]
    );

    // From the trash, deleting is final: there is nothing to put back.
    let trash = folder(&world, &account, FolderRole::Trash);
    world.remote.lock().unwrap().deliver("Trash", "Gone", true);
    let trashed = world.engine.sync_folder(trash.id, false).await.unwrap();
    let moves = world
        .engine
        .delete_messages(&[trashed[0].id])
        .await
        .unwrap();
    assert_eq!(moves[0].to_folder_id, None);
    assert_eq!(world.engine.put_back(&moves).await.unwrap(), Vec::new());
    assert_eq!(
        world.remote.lock().unwrap().uids("Trash"),
        Vec::<u32>::new()
    );
}

#[tokio::test]
async fn nothing_leaves_all_mail_so_moving_from_it_copies_and_undoing_removes_the_copy() {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.folders.push(crate::testing::FakeFolder {
            path: "All".to_string(),
            role: FolderRole::All,
            uid_validity: 1,
            next_uid: 1,
            messages: Vec::new(),
        });
        remote.deliver("All", "Everywhere", true);
    }
    world.engine.refresh_folders(account.id).await.unwrap();
    let all = folder(&world, &account, FolderRole::All);
    let archive = folder(&world, &account, FolderRole::Archive);
    let id = world.engine.sync_folder(all.id, false).await.unwrap()[0].id;

    let moves = world.engine.move_messages(&[id], archive.id).await.unwrap();

    assert!(moves[0].kept);
    assert_eq!(world.remote.lock().unwrap().uids("All").len(), 1);
    assert_eq!(world.remote.lock().unwrap().uids("Archive").len(), 1);
    assert_eq!(world.engine.list_messages(all.id).unwrap()[0].id, id);

    world.engine.put_back(&moves).await.unwrap();
    assert_eq!(world.remote.lock().unwrap().uids("All").len(), 1);
    assert_eq!(
        world.remote.lock().unwrap().uids("Archive"),
        Vec::<u32>::new()
    );
}

#[tokio::test]
async fn archiving_into_all_mail_is_undone_by_copying_back() {
    let (world, account, inbox, messages) = with_inbox(&["Later"]).await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.folders.push(crate::testing::FakeFolder {
            path: "All".to_string(),
            role: FolderRole::All,
            uid_validity: 1,
            next_uid: 1,
            messages: Vec::new(),
        });
    }
    world.engine.refresh_folders(account.id).await.unwrap();
    let all = folder(&world, &account, FolderRole::All);

    let moves = world
        .engine
        .move_messages(&[messages[0].id], all.id)
        .await
        .unwrap();
    assert!(!moves[0].kept);
    world.engine.put_back(&moves).await.unwrap();

    // The message stays in All mail, as every message there does.
    assert_eq!(world.remote.lock().unwrap().uids("All").len(), 1);
    assert_eq!(
        subjects(&world.engine.list_messages(inbox.id).unwrap()),
        vec!["Later"]
    );
}

// Conversations

/// A message with the headers that decide its conversation, written on
/// `day` of October 2026. It answers the last of `parents`, and references
/// them all, as mail apps do.
fn mail(from: &str, id: &str, parents: &[&str], subject: &str, day: u32) -> Vec<u8> {
    let answers = match parents.last() {
        None => String::new(),
        Some(last) => {
            let all: Vec<String> = parents.iter().map(|parent| format!("<{parent}>")).collect();
            format!("In-Reply-To: <{last}>\r\nReferences: {}\r\n", all.join(" "))
        }
    };

    format!(
        "From: {from}\r\nTo: someone@example.com\r\nSubject: {subject}\r\n\
         Date: {day} Oct 2026 10:00:00 +0000\r\nMessage-ID: <{id}>\r\n{answers}\r\n\
         Text of {id}\r\n"
    )
    .into_bytes()
}

const ANN: &str = "Ann <ann@example.com>";
const ME: &str = "Me <me@example.com>";

/// An inbox with a conversation of three (one of them the user's reply in
/// Sent) and a message of its own; Trash holds a reply that was thrown away.
async fn with_conversation() -> (World, Account) {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.deliver_raw("INBOX", mail(ANN, "a@x", &[], "Lunch", 1), true);
        remote.deliver_raw("Sent", mail(ME, "b@x", &["a@x"], "Re: Lunch", 2), true);
        remote.deliver_raw("Trash", mail(ANN, "t@x", &["a@x"], "Re: Lunch", 3), false);
        remote.deliver_raw(
            "INBOX",
            mail("bob@example.com", "n@x", &[], "News", 4),
            true,
        );
        remote.deliver_raw(
            "INBOX",
            mail(ANN, "c@x", &["a@x", "b@x"], "Re: Lunch", 5),
            false,
        );
    }
    for role in [FolderRole::Inbox, FolderRole::Sent, FolderRole::Trash] {
        let folder = folder(&world, &account, role);
        world.engine.sync_folder(folder.id, false).await.unwrap();
    }

    (world, account)
}

fn subjects(messages: &[MessageSummary]) -> Vec<&str> {
    messages
        .iter()
        .map(|message| message.subject.as_str())
        .collect()
}

#[tokio::test]
async fn replies_are_one_conversation_with_the_users_own_reply_from_sent() {
    let (world, account) = with_conversation().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let messages = world.engine.list_messages(inbox.id).unwrap();

    let rows = world.engine.list_conversations(inbox.id).unwrap();

    assert_eq!(rows.len(), 2);
    let lunch = &rows[0];
    assert_eq!(lunch.latest.subject, "Re: Lunch");
    assert_eq!(lunch.message_ids, vec![messages[0].id, messages[2].id]);
    assert_eq!((lunch.count, lunch.unread, lunch.flagged), (3, 1, false));
    let senders: Vec<&str> = lunch
        .senders
        .iter()
        .map(|sender| sender.email.as_str())
        .collect();
    assert_eq!(senders, vec!["ann@example.com", "me@example.com"]);
    assert_eq!(rows[1].latest.subject, "News");
    assert_eq!(rows[1].count, 1);

    // Oldest first, whichever of its messages is asked for.
    let opened = world.engine.conversation(messages[2].id).unwrap();
    assert_eq!(subjects(&opened), vec!["Lunch", "Re: Lunch", "Re: Lunch"]);
    assert_eq!(opened[1].from[0].email, "me@example.com");
    assert_eq!(world.engine.conversation(messages[0].id).unwrap(), opened);
    assert_eq!(
        subjects(&world.engine.conversation(messages[1].id).unwrap()),
        vec!["News"]
    );
}

#[tokio::test]
async fn a_conversation_listed_in_trash_shows_what_is_kept_elsewhere() {
    let (world, account) = with_conversation().await;
    let trash = folder(&world, &account, FolderRole::Trash);

    let rows = world.engine.list_conversations(trash.id).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].message_ids.len(), 1);
    assert_eq!(rows[0].count, 4);
}

#[tokio::test]
async fn a_message_kept_in_two_folders_is_shown_once() {
    let (world, account) = with_conversation().await;
    world
        .remote
        .lock()
        .unwrap()
        .deliver_raw("Archive", mail(ANN, "a@x", &[], "Lunch", 1), true);
    let archive = folder(&world, &account, FolderRole::Archive);
    world.engine.sync_folder(archive.id, false).await.unwrap();
    let inbox = folder(&world, &account, FolderRole::Inbox);

    let rows = world.engine.list_conversations(inbox.id).unwrap();
    assert_eq!(rows[0].count, 3);

    // Listed in the archive, its own copy is the one shown.
    let row = &world.engine.list_conversations(archive.id).unwrap()[0];
    let opened = world.engine.conversation(row.latest.id).unwrap();
    assert_eq!(opened.len(), 3);
    assert_eq!(opened[0].folder_id, archive.id);
}

#[tokio::test]
async fn the_settings_decide_what_is_grouped() {
    let (world, account) = with_conversation().await;
    {
        let mut remote = world.remote.lock().unwrap();
        remote.deliver_raw(
            "INBOX",
            mail("bank@example.com", "1@x", &[], "Your code", 6),
            true,
        );
        remote.deliver_raw(
            "INBOX",
            mail("bank@example.com", "2@x", &[], "Your code", 7),
            true,
        );
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    let counts = |world: &World| -> Vec<u32> {
        world
            .engine
            .list_conversations(inbox.id)
            .unwrap()
            .iter()
            .map(|row| row.count)
            .collect()
    };
    assert_eq!(counts(&world), vec![2, 3, 1]);

    let mut settings = AppSettings::default();
    settings.conversations.same_subject = false;
    world.engine.set_settings(settings.clone()).unwrap();
    assert_eq!(counts(&world), vec![1, 1, 3, 1]);

    settings.conversations.other_folders = false;
    world.engine.set_settings(settings.clone()).unwrap();
    assert_eq!(counts(&world), vec![1, 1, 2, 1]);
    let latest = world.engine.list_conversations(inbox.id).unwrap()[2]
        .latest
        .id;
    assert_eq!(world.engine.conversation(latest).unwrap().len(), 2);

    settings.conversations.enabled = false;
    world.engine.set_settings(settings).unwrap();
    assert_eq!(counts(&world), vec![1, 1, 1, 1, 1]);
    assert_eq!(world.engine.conversation(latest).unwrap().len(), 1);
}

#[tokio::test]
async fn a_conversation_shows_each_message_without_what_it_quotes() {
    let (world, account) = with_account().await;
    let raw = format!(
        "{}Sure.\r\n\r\nAnn wrote:\r\n> Lunch?\r\n",
        String::from_utf8(mail(ME, "b@x", &["a@x"], "Re: Lunch", 2))
            .unwrap()
            .replace("Text of b@x\r\n", "")
    );
    world
        .remote
        .lock()
        .unwrap()
        .deliver_raw("INBOX", raw.into_bytes(), true);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;

    let content = world.engine.open_message(id, false).await.unwrap();

    assert_eq!(content.text_without_quote.as_deref(), Some("Sure."));
    assert!(content.text.unwrap().contains("> Lunch?"));
}

#[tokio::test]
async fn the_users_replies_are_fetched_from_sent_when_conversations_show_them() {
    let (world, account) = with_account().await;
    let sent = folder(&world, &account, FolderRole::Sent);

    world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap();
    assert_eq!(
        subjects(&world.engine.list_messages(sent.id).unwrap()),
        vec!["Hello"]
    );

    let mut settings = AppSettings::default();
    settings.conversations.other_folders = false;
    world.engine.set_settings(settings).unwrap();
    world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap();
    assert_eq!(world.engine.list_messages(sent.id).unwrap().len(), 1);
}

// Writing

#[tokio::test]
async fn sending_delivers_to_everyone_hides_bcc_and_keeps_a_copy_in_sent() {
    let (world, account) = with_account().await;
    let mut message = outgoing(&account);
    message.to = vec![" ann@example.com ".to_string(), String::new()];
    message.cc = vec!["bob@example.com".to_string()];
    message.bcc = vec!["hidden@example.com".to_string()];

    world.engine.send_message(&message).await.unwrap();

    let mut remote = world.remote.lock().unwrap();
    assert_eq!(remote.sent.len(), 1);
    assert_eq!(remote.sent[0].from, "me@example.com");
    assert_eq!(
        remote.sent[0].recipients,
        vec!["ann@example.com", "bob@example.com", "hidden@example.com"],
    );
    let raw = String::from_utf8(remote.sent[0].raw.clone()).unwrap();
    assert!(raw.contains("Subject: Hello"));
    assert!(raw.contains("Me Myself"));
    assert!(raw.contains("How are you?"));
    assert!(!raw.contains("hidden@example.com"));

    let copy = remote.folder("Sent").messages.clone();
    assert_eq!(copy.len(), 1);
    assert!(copy[0].flags.seen);
    assert_eq!(copy[0].raw, remote.sent[0].raw);
}

#[tokio::test]
async fn sending_needs_valid_recipients() {
    let (world, account) = with_account().await;

    let mut nobody = outgoing(&account);
    nobody.to = Vec::new();
    assert_eq!(
        world.engine.send_message(&nobody).await.unwrap_err().kind,
        ErrorKind::Invalid
    );

    let mut broken = outgoing(&account);
    broken.to = vec!["not an address".to_string()];
    let error = world.engine.send_message(&broken).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);
    assert!(error.detail.contains("not an address"));

    assert!(world.remote.lock().unwrap().sent.is_empty());
}

#[tokio::test]
async fn a_failed_send_is_reported_and_leaves_no_copy() {
    let (world, account) = with_account().await;
    world.remote.lock().unwrap().outgoing_down = true;

    let error = world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Connect);
    assert_eq!(error.server, Some(ServerRole::Outgoing));
    assert!(
        world
            .remote
            .lock()
            .unwrap()
            .folder("Sent")
            .messages
            .is_empty()
    );
}

#[tokio::test]
async fn message_ids_are_unique() {
    let (world, account) = with_account().await;

    world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap();
    world
        .engine
        .send_message(&outgoing(&account))
        .await
        .unwrap();

    let remote = world.remote.lock().unwrap();
    let id = |index: usize| {
        let raw = String::from_utf8(remote.sent[index].raw.clone()).unwrap();
        raw.lines()
            .find(|line| line.starts_with("Message-ID:"))
            .unwrap()
            .to_string()
    };
    assert!(id(0).ends_with("@example.com>"));
    assert_ne!(id(0), id(1));
}

#[tokio::test]
async fn a_reply_draft_addresses_the_sender_and_quotes_the_message() {
    let (world, account) = with_account().await;
    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Question", false);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;

    let draft = world
        .engine
        .reply_draft(id, ReplyMode::ReplyAll)
        .await
        .unwrap();

    assert_eq!(draft.to, vec!["ann@example.com"]);
    // The account's own address is not copied.
    assert_eq!(draft.cc, Vec::<String>::new());
    assert_eq!(draft.subject, "Re: Question");
    assert_eq!(draft.in_reply_to.as_deref(), Some("Question@example.com"));
    assert!(draft.body.contains("> Body of Question"));
}

#[tokio::test]
async fn attachments_come_from_the_cached_message() {
    let (world, account) = with_account().await;
    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Plain", false);
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let id = world.engine.sync_folder(inbox.id, false).await.unwrap()[0].id;

    let error = world.engine.attachment(id, 0).await.unwrap_err();

    assert_eq!(error.kind, ErrorKind::NotFound);
}

// Sender pictures

const PICTURE: &str =
    "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjwvc3ZnPg==";

#[tokio::test]
async fn a_sender_whose_domain_publishes_a_logo_gets_it_as_a_picture() {
    let world = world();
    world.logos.publish("revolut.com", LOGO);

    let picture = world.engine.sender_picture("No-Reply@Revolut.com").await;

    assert_eq!(picture.unwrap().as_deref(), Some(PICTURE));
    assert_eq!(world.logos.asked(), ["revolut.com"]);
}

#[tokio::test]
async fn a_sender_without_a_logo_or_without_a_domain_has_no_picture() {
    let world = world();

    assert_eq!(
        world.engine.sender_picture("jan@example.com").await,
        Ok(None)
    );
    assert_eq!(
        world.engine.sender_picture("not an address").await,
        Ok(None)
    );
    assert_eq!(world.engine.sender_picture("me@localhost").await, Ok(None));
    // Only a real domain is ever asked about.
    assert_eq!(world.logos.asked(), ["example.com"]);
}

#[tokio::test]
async fn a_domain_is_asked_once_and_the_answer_is_remembered() {
    let world = world();
    world.logos.publish("revolut.com", LOGO);

    for email in [
        "a@revolut.com",
        "b@revolut.com",
        "c@example.com",
        "d@example.com",
    ] {
        world.engine.sender_picture(email).await.unwrap();
    }
    // Both "has one" and "has none" are answers worth keeping.
    assert_eq!(world.logos.asked(), ["revolut.com", "example.com"]);

    let (first, second) = tokio::join!(
        world.engine.sender_picture("a@fresh.example"),
        world.engine.sender_picture("b@fresh.example"),
    );
    assert_eq!((first, second), (Ok(None), Ok(None)));
    assert_eq!(world.logos.asked().len(), 3);
}

#[tokio::test]
async fn an_old_answer_is_checked_again() {
    let world = world();
    let (now, _) = unix_now();
    world
        .engine
        .db(|store| store.put_logo("revolut.com", None, now - LOGO_TTL - 1))
        .unwrap();
    world.logos.publish("revolut.com", LOGO);

    let picture = world.engine.sender_picture("a@revolut.com").await;

    assert_eq!(picture.unwrap().as_deref(), Some(PICTURE));
}

#[tokio::test]
async fn without_a_network_there_is_no_picture_no_error_and_a_later_retry() {
    let world = world();
    world.logos.publish("revolut.com", LOGO);
    *world.logos.offline.lock().unwrap() = true;

    assert_eq!(world.engine.sender_picture("a@revolut.com").await, Ok(None));

    *world.logos.offline.lock().unwrap() = false;
    let picture = world.engine.sender_picture("a@revolut.com").await;
    assert_eq!(picture.unwrap().as_deref(), Some(PICTURE));
}

#[tokio::test]
async fn an_old_picture_still_serves_while_offline() {
    let world = world();
    let (now, _) = unix_now();
    world
        .engine
        .db(|store| store.put_logo("revolut.com", Some(LOGO), now - LOGO_TTL - 1))
        .unwrap();
    *world.logos.offline.lock().unwrap() = true;

    let picture = world.engine.sender_picture("a@revolut.com").await;

    assert_eq!(picture.unwrap().as_deref(), Some(PICTURE));
}

#[tokio::test]
async fn something_that_is_not_a_drawing_is_not_shown() {
    let world = world();
    world.logos.publish("revolut.com", b"<html>surprise</html>");

    assert_eq!(world.engine.sender_picture("a@revolut.com").await, Ok(None));
}

#[tokio::test]
async fn turned_off_nothing_is_asked_and_it_can_be_turned_on_again() {
    let world = world();
    world.logos.publish("revolut.com", LOGO);
    let on = world.engine.settings().unwrap();
    assert!(on.sender_pictures);

    let off = AppSettings {
        sender_pictures: false,
        ..on.clone()
    };
    world.engine.set_settings(off.clone()).unwrap();
    assert_eq!(world.engine.settings(), Ok(off));
    assert_eq!(world.engine.sender_picture("a@revolut.com").await, Ok(None));
    assert!(world.logos.asked().is_empty());

    world.engine.set_settings(on).unwrap();
    let picture = world.engine.sender_picture("a@revolut.com").await;
    assert_eq!(picture.unwrap().as_deref(), Some(PICTURE));
}

// News

fn heard(events: &mut tokio::sync::broadcast::Receiver<MailEvent>) -> Vec<MailEvent> {
    let mut heard = Vec::new();
    while let Ok(event) = events.try_recv() {
        heard.push(event);
    }

    heard
}

fn arrivals(events: &[MailEvent]) -> Vec<Vec<String>> {
    events
        .iter()
        .filter_map(|event| match event {
            MailEvent::NewMail { messages, .. } => Some(
                messages
                    .iter()
                    .map(|message| message.subject.clone())
                    .collect(),
            ),
            MailEvent::FolderChanged { .. } => None,
        })
        .collect()
}

#[tokio::test]
async fn unread_mail_that_arrives_in_the_inbox_is_announced_once() {
    let (world, account) = with_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.remote.lock().unwrap().deliver("INBOX", "Old", false);
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    let mut events = world.engine.subscribe();

    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Fresh", false);
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    let first = heard(&mut events);
    assert_eq!(arrivals(&first), vec![vec!["Fresh".to_string()]]);
    assert!(first.contains(&MailEvent::FolderChanged {
        account_id: account.id,
        folder_id: inbox.id,
    }));
    let Some(MailEvent::NewMail {
        account: address,
        messages,
        ..
    }) = first
        .iter()
        .find(|event| matches!(event, MailEvent::NewMail { .. }))
    else {
        panic!("no announcement");
    };
    assert_eq!(address, "me@example.com");
    assert_eq!(messages[0].from.as_ref().unwrap().email, "ann@example.com");

    // Nothing changed: nothing is said.
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert!(heard(&mut events).is_empty());
}

#[tokio::test]
async fn what_was_already_there_or_already_read_is_not_news() {
    let (world, account) = with_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let mut events = world.engine.subscribe();

    // The first sync of a folder is catching up, not arrivals.
    for index in 0..3 {
        world
            .remote
            .lock()
            .unwrap()
            .deliver("INBOX", &format!("Before {index}"), false);
    }
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    assert!(arrivals(&heard(&mut events)).is_empty());

    // Read on another device already, and loading older mail: not news.
    world.remote.lock().unwrap().deliver("INBOX", "Read", true);
    world.engine.sync_folder(inbox.id, true).await.unwrap();
    assert!(arrivals(&heard(&mut events)).is_empty());
}

#[tokio::test]
async fn only_the_inbox_is_announced() {
    let (world, account) = with_account().await;
    let archive = folder(&world, &account, FolderRole::Archive);
    world
        .remote
        .lock()
        .unwrap()
        .deliver("Archive", "Old", false);
    world.engine.sync_folder(archive.id, false).await.unwrap();
    let mut events = world.engine.subscribe();

    world
        .remote
        .lock()
        .unwrap()
        .deliver("Archive", "Filed", false);
    world.engine.sync_folder(archive.id, false).await.unwrap();

    let heard = heard(&mut events);
    assert!(arrivals(&heard).is_empty());
    // The list still has to show it.
    assert_eq!(heard.len(), 1);
}

#[tokio::test]
async fn with_notifications_off_the_lists_still_hear_of_new_mail() {
    let (world, account) = with_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.remote.lock().unwrap().deliver("INBOX", "Old", false);
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    let mut off = AppSettings::default();
    off.notifications.enabled = false;
    world.engine.set_settings(off).unwrap();
    let mut events = world.engine.subscribe();

    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Fresh", false);
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    let heard = heard(&mut events);
    assert!(arrivals(&heard).is_empty());
    assert_eq!(heard.len(), 1);
}

#[tokio::test]
async fn the_watcher_fetches_new_mail_as_soon_as_the_server_has_news() {
    let World { engine, remote, .. } = world();
    let engine = Arc::new(engine);
    let mut events = engine.subscribe();
    // Started before any account exists: it must notice the new one.
    tokio::spawn(Arc::clone(&engine).watch());

    remote.lock().unwrap().deliver("INBOX", "Old", false);
    let account = engine.add_account(settings(), PASSWORD).await.unwrap();
    let inbox = engine
        .list_folders(account.id)
        .unwrap()
        .into_iter()
        .find(|folder| folder.role == FolderRole::Inbox)
        .unwrap();
    engine.sync_folder(inbox.id, false).await.unwrap();

    let news = {
        let mut remote = remote.lock().unwrap();
        remote.deliver("INBOX", "Fresh", false);

        Arc::clone(&remote.news)
    };
    news.notify_one();

    let announced = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Ok(MailEvent::NewMail { messages, .. }) = events.recv().await {
                return messages;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(announced[0].subject, "Fresh");
    assert_eq!(announced[0].preview.as_deref(), Some("Body of Fresh"));
    // Taken in by asking for what is new only, not by listing the inbox.
    assert!(remote.lock().unwrap().fetched_new >= 1);

    // The sync that follows tells nothing twice.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(arrivals(&heard(&mut events)).is_empty());

    // The message came down for its preview and is not downloaded again.
    let downloads = remote.lock().unwrap().body_downloads;
    engine.open_message(announced[0].id, false).await.unwrap();
    assert_eq!(remote.lock().unwrap().body_downloads, downloads);
}

#[tokio::test]
async fn the_message_is_not_downloaded_when_the_notification_does_not_show_it() {
    let (world, account) = with_account().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.remote.lock().unwrap().deliver("INBOX", "Old", false);
    world.engine.sync_folder(inbox.id, false).await.unwrap();
    let mut settings = AppSettings::default();
    settings.notifications.body = "{subject}".to_string();
    world.engine.set_settings(settings).unwrap();
    let mut events = world.engine.subscribe();

    world
        .remote
        .lock()
        .unwrap()
        .deliver("INBOX", "Fresh", false);
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    let Some(MailEvent::NewMail { messages, .. }) = heard(&mut events)
        .into_iter()
        .find(|event| matches!(event, MailEvent::NewMail { .. }))
    else {
        panic!("no announcement");
    };
    assert_eq!(messages[0].preview, None);
    assert_eq!(world.remote.lock().unwrap().body_downloads, 0);
}

#[tokio::test]
async fn notification_settings_are_kept_and_nonsense_is_refused() {
    let world = world();
    let mut settings = AppSettings::default();
    settings.notifications.title = "{subject}".to_string();
    settings.notifications.custom_icon = Some("data:image/png;base64,AAAA".to_string());
    world.engine.set_settings(settings.clone()).unwrap();
    assert_eq!(world.engine.settings(), Ok(settings.clone()));

    // A picture taken away again is gone, not kept from before.
    settings.notifications.custom_icon = None;
    world.engine.set_settings(settings.clone()).unwrap();
    assert_eq!(world.engine.settings(), Ok(settings.clone()));

    let mut wrong = settings.clone();
    wrong.notifications.preview_length = 0;
    let error = world.engine.set_settings(wrong).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);
    assert_eq!(world.engine.settings(), Ok(settings));
}

// Categories

/// A message from `from` about `subject`, with `headers` (whole lines, each
/// ending in CRLF) added to its header.
fn mail_with(from: &str, subject: &str, headers: &str) -> Vec<u8> {
    format!(
        "From: {from}\r\nTo: me@example.com\r\nSubject: {subject}\r\n{headers}\
         Date: 1 Oct 2026 10:00:00 +0000\r\nMessage-ID: <{subject}@x>\r\n\r\nText\r\n"
    )
    .into_bytes()
}

/// An inbox with one message for each category.
async fn with_every_kind_of_mail() -> (World, Account) {
    let (world, account) = with_account().await;
    {
        let mut remote = world.remote.lock().unwrap();
        let mut deliver = |raw| remote.deliver_raw("INBOX", raw, true);
        deliver(mail_with(ANN, "Lunch", ""));
        deliver(mail_with(
            "Facebook <notification@facebookmail.com>",
            "Ann tagged you",
            "",
        ));
        deliver(mail_with(
            "Shop <hello@shop.example>",
            "Autumn sale",
            "List-Unsubscribe: <https://shop.example/u>\r\n",
        ));
        deliver(mail_with("Shop <hello@shop.example>", "Your receipt", ""));
        deliver(mail_with(
            "Bob <bob@example.com>",
            "Borrowing",
            "List-Id: <rust.lists.example>\r\nList-Post: <mailto:rust@lists.example>\r\n",
        ));
    }
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.engine.sync_folder(inbox.id, false).await.unwrap();

    (world, account)
}

/// Each row of the folder's list: its subject and its tab.
fn tabs(world: &World, folder: &Folder) -> Vec<(String, Option<Category>)> {
    world
        .engine
        .list_conversations(folder.id)
        .unwrap()
        .into_iter()
        .map(|row| (row.latest.subject, row.category))
        .collect()
}

fn tab(subject: &str, category: Option<Category>) -> (String, Option<Category>) {
    (subject.to_string(), category)
}

#[tokio::test]
async fn the_inbox_is_sorted_into_the_categories_that_are_turned_on() {
    let (world, account) = with_every_kind_of_mail().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);

    // Updates and Forums are off at first: their mail stays in Primary.
    assert_eq!(
        tabs(&world, &inbox),
        vec![
            tab("Borrowing", Some(Category::Primary)),
            tab("Your receipt", Some(Category::Primary)),
            tab("Autumn sale", Some(Category::Promotions)),
            tab("Ann tagged you", Some(Category::Social)),
            tab("Lunch", Some(Category::Primary)),
        ]
    );

    let mut settings = AppSettings::default();
    settings.categories.updates = true;
    settings.categories.forums = true;
    world.engine.set_settings(settings.clone()).unwrap();
    assert_eq!(
        tabs(&world, &inbox)
            .into_iter()
            .map(|(_, category)| category)
            .collect::<Vec<_>>(),
        vec![
            Some(Category::Forums),
            Some(Category::Updates),
            Some(Category::Promotions),
            Some(Category::Social),
            Some(Category::Primary),
        ]
    );

    settings.categories.enabled = false;
    world.engine.set_settings(settings).unwrap();
    assert!(
        tabs(&world, &inbox)
            .iter()
            .all(|(_, category)| category.is_none())
    );
}

#[tokio::test]
async fn only_the_inbox_is_sorted() {
    let (world, account) = with_account().await;
    world.remote.lock().unwrap().deliver_raw(
        "Archive",
        mail_with(
            "Facebook <notification@facebookmail.com>",
            "Ann tagged you",
            "",
        ),
        true,
    );
    let archive = folder(&world, &account, FolderRole::Archive);
    world.engine.sync_folder(archive.id, false).await.unwrap();

    assert_eq!(tabs(&world, &archive), vec![tab("Ann tagged you", None)]);
}

#[tokio::test]
async fn a_sender_the_user_put_in_a_category_stays_there() {
    let (world, account) = with_every_kind_of_mail().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    let category_of = |subject: &str| {
        tabs(&world, &inbox)
            .into_iter()
            .find(|(listed, _)| listed == subject)
            .and_then(|(_, category)| category)
    };

    world
        .engine
        .set_category_rule(" Hello@Shop.example ", Some(Category::Social))
        .unwrap();
    assert_eq!(category_of("Autumn sale"), Some(Category::Social));
    assert_eq!(category_of("Your receipt"), Some(Category::Social));
    assert_eq!(
        world.engine.category_rules().unwrap(),
        vec![CategoryRule {
            sender: "hello@shop.example".to_string(),
            category: Category::Social,
        }]
    );

    // A rule for a category that is turned off still lands in Primary.
    world
        .engine
        .set_category_rule("hello@shop.example", Some(Category::Forums))
        .unwrap();
    assert_eq!(category_of("Autumn sale"), Some(Category::Primary));

    world
        .engine
        .set_category_rule("hello@shop.example", None)
        .unwrap();
    assert_eq!(category_of("Autumn sale"), Some(Category::Promotions));
    assert_eq!(world.engine.category_rules().unwrap(), Vec::new());

    let error = world
        .engine
        .set_category_rule("  ", Some(Category::Social))
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid);
}

#[tokio::test]
async fn mail_from_someone_the_user_wrote_to_stays_in_primary() {
    let (world, account) = with_every_kind_of_mail().await;
    let inbox = folder(&world, &account, FolderRole::Inbox);
    world.remote.lock().unwrap().deliver_raw(
        "Sent",
        format!(
            "From: {ME}\r\nTo: Shop <hello@shop.example>\r\nSubject: Refund\r\n\
             Message-ID: <refund@x>\r\n\r\nHello\r\n"
        )
        .into_bytes(),
        true,
    );
    let sent = folder(&world, &account, FolderRole::Sent);
    world.engine.sync_folder(sent.id, false).await.unwrap();

    let autumn = tabs(&world, &inbox)
        .into_iter()
        .find(|(subject, _)| subject == "Autumn sale");
    assert_eq!(autumn, Some(tab("Autumn sale", Some(Category::Primary))));

    // What the user decided still comes first.
    world
        .engine
        .set_category_rule("hello@shop.example", Some(Category::Promotions))
        .unwrap();
    let autumn = tabs(&world, &inbox)
        .into_iter()
        .find(|(subject, _)| subject == "Autumn sale");
    assert_eq!(autumn, Some(tab("Autumn sale", Some(Category::Promotions))));
}
