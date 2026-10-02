use std::sync::Arc;

use mail_core::account::ServerRole;
use mail_core::draft::ReplyMode;
use mail_core::error::ErrorKind;
use mail_core::mail::{Account, Folder, FolderRole, MessageContent, OutgoingMessage};
use mail_core::settings::AppSettings;
use mail_core::transport::Flag;
use mail_store::Store;

use crate::testing::{
    FakeConnector, FakeLogos, FakeSecrets, LOGO, PASSWORD, Remote, SharedRemote, settings,
};
use crate::{Engine, LOGO_TTL, PAGE, unix_now};

struct World {
    engine: Engine,
    remote: SharedRemote,
    secrets: Arc<FakeSecrets>,
    logos: Arc<FakeLogos>,
}

fn world() -> World {
    let remote = Remote::standard();
    let secrets = Arc::new(FakeSecrets::default());
    let connector = Arc::new(FakeConnector {
        remote: Arc::clone(&remote),
    });
    let logos = Arc::new(FakeLogos::default());
    let engine = Engine::new(
        Store::open_in_memory().unwrap(),
        connector,
        secrets.clone(),
        logos.clone(),
    );

    World {
        engine,
        remote,
        secrets,
        logos,
    }
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

    assert_eq!(error.kind, ErrorKind::Invalid);
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
        ..on
    };
    world.engine.set_settings(off).unwrap();
    assert_eq!(world.engine.settings(), Ok(off));
    assert_eq!(world.engine.sender_picture("a@revolut.com").await, Ok(None));
    assert!(world.logos.asked().is_empty());

    world.engine.set_settings(on).unwrap();
    let picture = world.engine.sender_picture("a@revolut.com").await;
    assert_eq!(picture.unwrap().as_deref(), Some(PICTURE));
}
