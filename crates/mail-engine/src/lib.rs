//! The operations the UI calls. The engine owns the local store and talks to
//! mail servers and the keychain only through the core traits, so everything
//! here runs in tests against in-memory fakes.

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use mail_core::account::{AccountSettings, AuthMethod, ServerRole, validate};
use mail_core::address::EmailAddress;
use mail_core::draft::{Draft, ReplyMode, reply_draft};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::logo::{is_logo, sender_domain};
use mail_core::mail::{
    Account, Attachment, Contact, Folder, FolderRole, MailEvent, MessageContent, MessageFlags,
    MessageSummary, NewMailItem, OutgoingMessage,
};
use mail_core::oauth::{SignIn, Tokens};
use mail_core::settings::AppSettings;
use mail_core::transport::{
    Connector, Envelope, Flag, FolderState, IncomingSession, LogoSource, PendingSignIn,
    RemoteFolder, RemoteMessage, SecretStore, SignInFlow,
};
use mail_render::{ComposeInput, compose, parse_header, render};
use mail_store::{NewMessage, Store};
use tokio::sync::{Mutex as AsyncMutex, Notify, OwnedMutexGuard, broadcast};
use tokio::task::JoinHandle;

/// How many messages a folder shows at first, and how many "load more" adds.
pub const PAGE: usize = 50;

/// How long an answer about a domain's logo is trusted before asking again.
pub const LOGO_TTL: i64 = 7 * 24 * 60 * 60;

/// An access token is renewed this many seconds before the provider's own
/// deadline, so one that is about to run out is never sent.
const TOKEN_MARGIN: i64 = 60;

/// How many new messages a notification names, and so needs the text of.
const PREVIEWS: usize = 3;
/// A message larger than this is not downloaded only for its preview.
const PREVIEW_SIZE: u32 = 512 * 1024;

/// How long one wait for the server's news lasts. Shorter than the 30
/// minutes servers allow, because routers drop quiet connections sooner.
const WATCH_LIMIT: Duration = Duration::from_secs(5 * 60);

/// Pauses after a failed check (offline, server down): doubling from the
/// first to the last, then staying there.
const RETRY_FIRST: Duration = Duration::from_secs(5);
const RETRY_LAST: Duration = Duration::from_secs(5 * 60);

type Session = Box<dyn IncomingSession>;
type Slot = Arc<AsyncMutex<Session>>;

/// Placeholder kept in a slot until the first real connection is made.
struct Disconnected;

fn disconnected<T>() -> MailResult<T> {
    Err(MailError::new(ErrorKind::Connect, "not connected").on(ServerRole::Incoming))
}

#[async_trait]
impl IncomingSession for Disconnected {
    async fn is_alive(&mut self) -> bool {
        false
    }

    async fn list_folders(&mut self) -> MailResult<Vec<RemoteFolder>> {
        disconnected()
    }

    async fn folder_state(&mut self, _: &str) -> MailResult<FolderState> {
        disconnected()
    }

    async fn fetch_headers(&mut self, _: &str, _: &[u32]) -> MailResult<Vec<RemoteMessage>> {
        disconnected()
    }

    async fn fetch_new(&mut self, _: &str, _: u32) -> MailResult<Vec<RemoteMessage>> {
        disconnected()
    }

    async fn fetch_flags(&mut self, _: &str, _: &[u32]) -> MailResult<Vec<(u32, MessageFlags)>> {
        disconnected()
    }

    async fn fetch_raw(&mut self, _: &str, _: u32) -> MailResult<Vec<u8>> {
        disconnected()
    }

    async fn set_flag(&mut self, _: &str, _: &[u32], _: Flag, _: bool) -> MailResult<()> {
        disconnected()
    }

    async fn move_messages(&mut self, _: &str, _: &[u32], _: &str) -> MailResult<()> {
        disconnected()
    }

    async fn delete_messages(&mut self, _: &str, _: &[u32]) -> MailResult<()> {
        disconnected()
    }

    async fn append(&mut self, _: &str, _: &[u8], _: bool) -> MailResult<()> {
        disconnected()
    }

    async fn wait_for_change(&mut self, _: &str, _: Duration) -> MailResult<()> {
        disconnected()
    }
}

/// A sign-in the user is finishing in their browser.
struct WaitingSignIn {
    id: u32,
    settings: AccountSettings,
    /// The user picks the account on the provider's page: `settings` have
    /// no address until the provider says which it was.
    chosen: bool,
    /// Taken by the one call that waits for the browser.
    flow: Option<Box<dyn PendingSignIn>>,
    cancelled: Arc<Notify>,
}

/// An access token and the unix time it is used until.
struct Access {
    token: String,
    until: i64,
}

pub struct Engine {
    store: Mutex<Store>,
    connector: Arc<dyn Connector>,
    secrets: Arc<dyn SecretStore>,
    sign_in: Arc<dyn SignInFlow>,
    /// One sign-in at a time: starting another gives the earlier one up.
    waiting_sign_in: Mutex<Option<WaitingSignIn>>,
    sign_in_counter: AtomicU32,
    /// Access tokens of the accounts that sign in on their provider's page.
    /// Only in memory: the keychain holds what renews them.
    access: Mutex<HashMap<i64, Access>>,
    /// One live incoming connection per account, reused between operations.
    sessions: Mutex<HashMap<i64, Slot>>,
    message_counter: AtomicU64,
    logos: Arc<dyn LogoSource>,
    /// One lookup per domain at a time: a list full of one sender asks once.
    logo_lookups: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
    events: broadcast::Sender<MailEvent>,
    folder_locks: Mutex<HashMap<i64, Arc<AsyncMutex<()>>>>,
    /// Wakes `watch` when an account was added or removed.
    accounts_changed: Notify,
}

/// A drawing as an address an `<img>` can show without any request.
fn picture(svg: Vec<u8>) -> String {
    format!("data:image/svg+xml;base64,{}", STANDARD.encode(svg))
}

fn secret_key(account_id: i64) -> String {
    format!("account-{account_id}")
}

fn unix_now() -> (i64, u32) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    (
        i64::try_from(now.as_secs()).unwrap_or(0),
        now.subsec_nanos(),
    )
}

fn invalid(detail: impl Into<String>) -> MailError {
    MailError::new(ErrorKind::Invalid, detail)
}

fn no_sign_in(detail: impl Into<String>) -> MailError {
    MailError::new(ErrorKind::SignIn, detail)
}

/// Whether the account signs in on its provider's page. Both servers do the
/// same: `validate` refuses settings where they differ.
fn uses_sign_in(settings: &AccountSettings) -> bool {
    settings.incoming.auth == AuthMethod::OAuth2
}

/// The settings, signing in as `email` on both servers.
fn with_address(mut settings: AccountSettings, email: &str) -> AccountSettings {
    settings.email = email.to_string();
    settings.incoming.username = email.to_string();
    settings.outgoing.username = email.to_string();

    settings
}

fn trimmed(mut settings: AccountSettings) -> AccountSettings {
    settings.email = settings.email.trim().to_string();
    settings.display_name = settings.display_name.trim().to_string();
    for server in [&mut settings.incoming, &mut settings.outgoing] {
        server.host = server.host.trim().to_string();
        server.username = server.username.trim().to_string();
    }

    settings
}

fn to_new_message(remote: &RemoteMessage) -> NewMessage {
    let header = parse_header(&remote.header);

    NewMessage {
        uid: remote.uid,
        subject: header.subject,
        from: header.from,
        to: header.to,
        cc: header.cc,
        // The sender's clock can be anything; fall back to the arrival time.
        date: header.date.unwrap_or(remote.received),
        flags: remote.flags,
        size: remote.size,
    }
}

impl Engine {
    pub fn new(
        store: Store,
        connector: Arc<dyn Connector>,
        secrets: Arc<dyn SecretStore>,
        logos: Arc<dyn LogoSource>,
        sign_in: Arc<dyn SignInFlow>,
    ) -> Self {
        Self {
            store: Mutex::new(store),
            connector,
            secrets,
            sign_in,
            waiting_sign_in: Mutex::new(None),
            sign_in_counter: AtomicU32::new(0),
            access: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            message_counter: AtomicU64::new(0),
            logos,
            logo_lookups: Mutex::new(HashMap::new()),
            events: broadcast::channel(64).0,
            folder_locks: Mutex::new(HashMap::new()),
            accounts_changed: Notify::new(),
        }
    }

    /// Everything the engine finds out by itself from now on. A receiver
    /// that falls behind loses the oldest events, which is harmless: they
    /// only say "read the cache again".
    pub fn subscribe(&self) -> broadcast::Receiver<MailEvent> {
        self.events.subscribe()
    }

    fn announce(&self, event: MailEvent) {
        // Nobody listening is fine.
        let _ = self.events.send(event);
    }

    /// Runs a piece of work on the store. Never held across an await.
    fn db<T>(&self, work: impl FnOnce(&mut Store) -> MailResult<T>) -> MailResult<T> {
        let mut store = self.store.lock().map_err(|_| {
            MailError::new(ErrorKind::Storage, "the store is unusable after a crash")
        })?;

        work(&mut store)
    }

    fn slot(&self, account_id: i64) -> MailResult<Slot> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| MailError::new(ErrorKind::Connect, "the connection table is unusable"))?;
        let slot = sessions
            .entry(account_id)
            .or_insert_with(|| Arc::new(AsyncMutex::new(Box::new(Disconnected) as Session)));

        Ok(Arc::clone(slot))
    }

    /// What the keychain holds for the account: its password, or the token
    /// that renews its sign-in.
    fn kept_secret(&self, account_id: i64) -> MailResult<String> {
        self.secrets.get(&secret_key(account_id))?.ok_or_else(|| {
            MailError::new(
                ErrorKind::Keychain,
                "the password or sign-in of this account is missing from the keychain",
            )
        })
    }

    fn access_tokens(&self) -> MailResult<std::sync::MutexGuard<'_, HashMap<i64, Access>>> {
        self.access
            .lock()
            .map_err(|_| MailError::new(ErrorKind::Connect, "the token table is unusable"))
    }

    fn remember_access(&self, account_id: i64, tokens: &Tokens) -> MailResult<()> {
        let lifetime = i64::try_from(tokens.expires_in).unwrap_or(0);
        let access = Access {
            token: tokens.access.clone(),
            until: unix_now().0 + lifetime - TOKEN_MARGIN,
        };
        self.access_tokens()?.insert(account_id, access);

        Ok(())
    }

    /// What the account's servers are opened with right now: the password,
    /// or an access token, renewed when the last one ran out.
    async fn secret(&self, account: &Account) -> MailResult<String> {
        if !uses_sign_in(&account.settings) {
            return self.kept_secret(account.id);
        }

        let (now, _) = unix_now();
        if let Some(access) = self.access_tokens()?.get(&account.id)
            && now < access.until
        {
            return Ok(access.token.clone());
        }

        let refresh = self.kept_secret(account.id)?;
        let tokens = self
            .sign_in
            .refresh(&account.settings.incoming.host, &refresh)
            .await?;
        // Some providers replace the long-lived token on every renewal.
        if let Some(replaced) = &tokens.refresh
            && *replaced != refresh
        {
            self.secrets.set(&secret_key(account.id), replaced)?;
        }
        self.remember_access(account.id, &tokens)?;

        Ok(tokens.access)
    }

    /// A server refused the secret. A token that should still be good is
    /// then not trusted again: the next attempt renews it, which also finds
    /// out whether the user took the permission back.
    fn refused(&self, account_id: i64, error: MailError) -> MailError {
        if error.kind == ErrorKind::Auth
            && let Ok(mut access) = self.access_tokens()
        {
            access.remove(&account_id);
        }

        error
    }

    /// The account's incoming connection, reconnected when it went stale.
    async fn session(&self, account: &Account) -> MailResult<OwnedMutexGuard<Session>> {
        let mut session = self.slot(account.id)?.lock_owned().await;
        if !session.is_alive().await {
            let secret = self.secret(account).await?;
            *session = self
                .connector
                .connect_incoming(&account.settings.incoming, &secret)
                .await
                .map_err(|error| self.refused(account.id, error))?;
        }

        Ok(session)
    }

    // Accounts

    pub fn list_accounts(&self) -> MailResult<Vec<Account>> {
        self.db(|store| store.accounts())
    }

    /// The settings of an account to add, tidied, or why they cannot be used.
    fn new_settings(&self, settings: AccountSettings) -> MailResult<AccountSettings> {
        let settings = trimmed(settings);

        EmailAddress::parse(&settings.email).map_err(|error| invalid(error.to_string()))?;
        if let Some(issue) = validate(&settings).into_iter().find(|issue| issue.blocking) {
            return Err(invalid(format!("{:?}", issue.kind)).on(issue.server));
        }

        Ok(settings)
    }

    fn refuse_duplicate(&self, settings: &AccountSettings) -> MailResult<()> {
        let duplicate = self.list_accounts()?.into_iter().any(|account| {
            account.settings.email.eq_ignore_ascii_case(&settings.email)
                && account
                    .settings
                    .incoming
                    .host
                    .eq_ignore_ascii_case(&settings.incoming.host)
        });
        if duplicate {
            return Err(MailError::new(
                ErrorKind::Duplicate,
                "this account is already added",
            ));
        }

        Ok(())
    }

    /// Signs in to both servers with `secret`; the account is only stored
    /// when both work. `kept` is what goes to the keychain for later.
    async fn connect_and_store(
        &self,
        settings: AccountSettings,
        secret: &str,
        kept: &str,
    ) -> MailResult<Account> {
        let mut session = self
            .connector
            .connect_incoming(&settings.incoming, secret)
            .await?;
        self.connector
            .check_outgoing(&settings.outgoing, secret)
            .await?;
        let folders = session.list_folders().await?;

        let account = self.db(|store| store.insert_account(&settings))?;
        if let Err(error) = self.secrets.set(&secret_key(account.id), kept) {
            // Without it the account could never reconnect.
            self.db(|store| store.delete_account(account.id))?;

            return Err(error);
        }
        self.db(|store| store.replace_folders(account.id, &folders))?;
        *self.slot(account.id)?.lock().await = session;
        self.accounts_changed.notify_one();

        Ok(account)
    }

    /// Adds an account that signs in with a password.
    pub async fn add_account(
        &self,
        settings: AccountSettings,
        password: &str,
    ) -> MailResult<Account> {
        let settings = self.new_settings(settings)?;
        if uses_sign_in(&settings) {
            return Err(invalid(
                "these settings sign in on the provider's page, not with a password",
            ));
        }
        if password.is_empty() {
            return Err(invalid("the password is empty"));
        }
        self.refuse_duplicate(&settings)?;

        self.connect_and_store(settings, password, password).await
    }

    fn waiting(&self) -> MailResult<std::sync::MutexGuard<'_, Option<WaitingSignIn>>> {
        self.waiting_sign_in
            .lock()
            .map_err(|_| MailError::new(ErrorKind::Connect, "the sign-in is unusable"))
    }

    /// First half of adding an account that signs in on its provider's page:
    /// checks the settings and gives the page to open. Nothing is stored yet.
    pub async fn begin_sign_in(&self, settings: AccountSettings) -> MailResult<SignIn> {
        let settings = self.new_settings(settings)?;
        if !uses_sign_in(&settings) {
            return Err(invalid("these settings sign in with a password"));
        }
        self.refuse_duplicate(&settings)?;

        let flow = self
            .sign_in
            .begin(&settings.incoming.host, Some(&settings.email))
            .await?;

        self.wait_for(settings, flow, false)
    }

    /// `begin_sign_in` for an account the user picks on the provider's page:
    /// the address in `settings` (and the usernames) are filled in from the
    /// provider's answer. Everything else is checked once it is known.
    pub async fn begin_chosen_sign_in(&self, settings: AccountSettings) -> MailResult<SignIn> {
        let settings = trimmed(settings);
        if !uses_sign_in(&settings) {
            return Err(invalid("these settings sign in with a password"));
        }

        let flow = self.sign_in.begin(&settings.incoming.host, None).await?;

        self.wait_for(settings, flow, true)
    }

    /// Keeps the sign-in until the browser comes back; one at a time.
    fn wait_for(
        &self,
        settings: AccountSettings,
        flow: Box<dyn PendingSignIn>,
        chosen: bool,
    ) -> MailResult<SignIn> {
        let url = flow.url().to_string();
        let id = self.sign_in_counter.fetch_add(1, Ordering::Relaxed) + 1;

        let earlier = self.waiting()?.replace(WaitingSignIn {
            id,
            settings,
            chosen,
            flow: Some(flow),
            cancelled: Arc::new(Notify::new()),
        });
        if let Some(earlier) = earlier {
            earlier.cancelled.notify_one();
        }

        Ok(SignIn { id, url })
    }

    /// Second half: waits until the user is done in the browser, then signs
    /// in to both servers and stores the account, like `add_account`.
    pub async fn finish_sign_in(&self, sign_in_id: u32) -> MailResult<Account> {
        let (flow, cancelled, settings, chosen) = {
            let mut waiting = self.waiting()?;
            let Some(entry) = waiting.as_mut().filter(|entry| entry.id == sign_in_id) else {
                return Err(no_sign_in("this sign-in was given up"));
            };
            let Some(flow) = entry.flow.take() else {
                return Err(no_sign_in("this sign-in is already being finished"));
            };

            (
                flow,
                Arc::clone(&entry.cancelled),
                entry.settings.clone(),
                entry.chosen,
            )
        };

        let tokens = tokio::select! {
            tokens = flow.finish() => tokens,
            () = cancelled.notified() => Err(no_sign_in("the sign-in was cancelled")),
        };
        self.cancel_sign_in(sign_in_id)?;
        let tokens = tokens?;
        let Some(refresh) = &tokens.refresh else {
            return Err(no_sign_in(
                "the provider gave no permission that lasts beyond this hour",
            ));
        };
        let settings = if chosen {
            let Some(email) = &tokens.email else {
                return Err(no_sign_in(
                    "the provider did not say which account was picked",
                ));
            };
            let settings = self.new_settings(with_address(settings, email))?;
            self.refuse_duplicate(&settings)?;

            settings
        } else {
            settings
        };

        let account = self
            .connect_and_store(settings, &tokens.access, refresh)
            .await?;
        self.remember_access(account.id, &tokens)?;

        Ok(account)
    }

    /// Gives a sign-in up: a call waiting for it ends with a `SignIn` error.
    /// One that is unknown or already over is not an error.
    pub fn cancel_sign_in(&self, sign_in_id: u32) -> MailResult<()> {
        let mut waiting = self.waiting()?;
        if waiting.as_ref().is_some_and(|entry| entry.id == sign_in_id)
            && let Some(entry) = waiting.take()
        {
            entry.cancelled.notify_one();
        }

        Ok(())
    }

    /// Forgets the account locally. Nothing is deleted on the server.
    pub fn remove_account(&self, account_id: i64) -> MailResult<()> {
        self.db(|store| store.delete_account(account_id))?;
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&account_id);
        }
        if let Ok(mut access) = self.access.lock() {
            access.remove(&account_id);
        }
        self.accounts_changed.notify_one();

        self.secrets.delete(&secret_key(account_id))
    }

    // Folders

    pub fn list_folders(&self, account_id: i64) -> MailResult<Vec<Folder>> {
        self.db(|store| store.folders(account_id))
    }

    pub async fn refresh_folders(&self, account_id: i64) -> MailResult<Vec<Folder>> {
        let account = self.db(|store| store.account(account_id))?;
        let remote = self.session(&account).await?.list_folders().await?;

        self.db(|store| {
            store.replace_folders(account_id, &remote)?;
            store.folders(account_id)
        })
    }

    // Messages

    pub fn list_messages(&self, folder_id: i64) -> MailResult<Vec<MessageSummary>> {
        self.db(|store| store.messages(folder_id))
    }

    /// One writer of a folder's cached messages at a time, so two of them
    /// never both take a message for news.
    fn folder_lock(&self, folder_id: i64) -> MailResult<Arc<AsyncMutex<()>>> {
        let mut locks = self
            .folder_locks
            .lock()
            .map_err(|_| MailError::new(ErrorKind::Storage, "the folder table is unusable"))?;

        Ok(Arc::clone(locks.entry(folder_id).or_default()))
    }

    /// Brings the cached window of a folder in line with the server: new
    /// messages arrive, deleted ones disappear, flags are updated. With
    /// `grow` the window also extends one page further into the past.
    pub async fn sync_folder(&self, folder_id: i64, grow: bool) -> MailResult<Vec<MessageSummary>> {
        let _writing = self.folder_lock(folder_id)?.lock_owned().await;
        let folder = self.db(|store| store.folder(folder_id))?;
        let account = self.db(|store| store.account(folder.account_id))?;
        let mut session = self.session(&account).await?;

        let state = session.folder_state(&folder.path).await?;
        self.db(|store| store.set_uid_validity(folder_id, state.uid_validity))?;
        let cached = self.db(|store| store.uids(folder_id))?;

        let window = if grow {
            cached.len() + PAGE
        } else {
            cached.len().max(PAGE)
        };
        let wanted = &state.uids[state.uids.len().saturating_sub(window)..];

        let gone: Vec<u32> = cached
            .iter()
            .copied()
            .filter(|uid| wanted.binary_search(uid).is_err())
            .collect();
        let new: Vec<u32> = wanted
            .iter()
            .copied()
            .filter(|uid| cached.binary_search(uid).is_err())
            .collect();
        let known: Vec<u32> = wanted
            .iter()
            .copied()
            .filter(|uid| cached.binary_search(uid).is_ok())
            .collect();

        let headers = if new.is_empty() {
            Vec::new()
        } else {
            session.fetch_headers(&folder.path, &new).await?
        };
        let flags = if known.is_empty() {
            Vec::new()
        } else {
            session.fetch_flags(&folder.path, &known).await?
        };

        // Only messages newer than everything cached are "arrived": a window
        // that grows into the past, or a first sync, is not news.
        let newest = cached.last().copied();
        let arrived: Vec<u32> = new
            .iter()
            .copied()
            .filter(|uid| newest.is_some_and(|newest| *uid > newest))
            .collect();

        let total = u32::try_from(state.uids.len()).unwrap_or(u32::MAX);
        let messages = self.db(|store| {
            store.delete_uids(folder_id, &gone)?;
            for remote in &headers {
                store.upsert_message(folder_id, &to_new_message(remote))?;
            }
            for (uid, flags) in &flags {
                store.set_flags(folder_id, *uid, *flags)?;
            }
            store.set_folder_counts(folder_id, state.unread, total)?;

            store.messages(folder_id)
        })?;
        let news = self
            .news(&account, &folder, &arrived, &messages, &mut **session)
            .await?;
        drop(session);

        if !new.is_empty() || !gone.is_empty() || state.unread != folder.unread {
            self.announce(MailEvent::FolderChanged {
                account_id: account.id,
                folder_id,
            });
        }
        if let Some(news) = news {
            self.announce(news);
        }

        Ok(messages)
    }

    /// What `NewMail` says about messages that just arrived in `folder`: the
    /// unread ones, with the start of their text when the notification
    /// shows it. `None` when there is nothing to tell.
    async fn news(
        &self,
        account: &Account,
        folder: &Folder,
        arrived: &[u32],
        messages: &[MessageSummary],
        session: &mut dyn IncomingSession,
    ) -> MailResult<Option<MailEvent>> {
        if folder.role != FolderRole::Inbox || arrived.is_empty() {
            return Ok(None);
        }
        let settings = self.settings()?.notifications;
        if !settings.enabled {
            return Ok(None);
        }

        let mut items = Vec::new();
        let fresh = messages
            .iter()
            .filter(|message| arrived.contains(&message.uid) && !message.flags.seen);
        for message in fresh {
            let preview = if settings.shows_preview()
                && items.len() < PREVIEWS
                && message.size <= PREVIEW_SIZE
            {
                self.preview(message, &folder.path, session).await
            } else {
                None
            };
            items.push(NewMailItem {
                id: message.id,
                subject: message.subject.clone(),
                from: message.from.first().cloned(),
                preview,
            });
        }
        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(MailEvent::NewMail {
            account_id: account.id,
            folder_id: folder.id,
            account: account.settings.email.clone(),
            messages: items,
        }))
    }

    /// The start of a message's text. The message is downloaded for it and
    /// kept, so opening it afterwards is instant. A preview is an extra: one
    /// that cannot be had is left out, never an error.
    async fn preview(
        &self,
        message: &MessageSummary,
        path: &str,
        session: &mut dyn IncomingSession,
    ) -> Option<String> {
        let raw = match self.db(|store| store.raw(message.id)) {
            Ok(Some(raw)) => raw,
            _ => {
                let raw = session.fetch_raw(path, message.uid).await.ok()?;
                let _ = self.db(|store| store.set_raw(message.id, &raw));
                raw
            }
        };

        mail_render::preview(&raw)
    }

    /// Takes in what arrived in the inbox after its newest cached message,
    /// on the watcher's own connection. One command, so a notification does
    /// not wait for a whole sync; the sync after it does the rest.
    async fn take_arrivals(
        &self,
        account: &Account,
        inbox: &Folder,
        waiting: &mut dyn IncomingSession,
    ) -> MailResult<()> {
        let _writing = self.folder_lock(inbox.id)?.lock_owned().await;
        let cached = self.db(|store| store.uids(inbox.id))?;
        // A folder never synced is caught up by a sync, not announced.
        let Some(newest) = cached.last().copied() else {
            return Ok(());
        };

        let arrived = waiting.fetch_new(&inbox.path, newest).await?;
        if arrived.is_empty() {
            return Ok(());
        }

        let count = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
        let unread = count(arrived.iter().filter(|remote| !remote.flags.seen).count());
        let messages = self.db(|store| {
            for remote in &arrived {
                store.upsert_message(inbox.id, &to_new_message(remote))?;
            }
            let folder = store.folder(inbox.id)?;
            store.set_folder_counts(
                inbox.id,
                folder.unread.saturating_add(unread),
                folder.total.saturating_add(count(arrived.len())),
            )?;

            store.messages(inbox.id)
        })?;
        let uids: Vec<u32> = arrived.iter().map(|remote| remote.uid).collect();
        let news = self.news(account, inbox, &uids, &messages, waiting).await?;

        self.announce(MailEvent::FolderChanged {
            account_id: account.id,
            folder_id: inbox.id,
        });
        if let Some(news) = news {
            self.announce(news);
        }

        Ok(())
    }

    // Watching

    /// Keeps every account's inbox up to date for as long as it runs: new
    /// mail is fetched as soon as the server says so (or within a minute
    /// for servers that cannot), and `subscribe` hears about it. Follows
    /// accounts being added and removed. Meant to be spawned once.
    pub async fn watch(self: Arc<Self>) {
        let mut running: HashMap<i64, JoinHandle<()>> = HashMap::new();
        loop {
            let ids: Vec<i64> = self
                .list_accounts()
                .map(|accounts| accounts.iter().map(|account| account.id).collect())
                .unwrap_or_default();

            running.retain(|id, task| {
                let kept = ids.contains(id);
                if !kept {
                    task.abort();
                }

                kept
            });
            for id in ids {
                running
                    .entry(id)
                    .or_insert_with(|| tokio::spawn(Arc::clone(&self).watch_account(id)));
            }

            self.accounts_changed.notified().await;
        }
    }

    async fn watch_account(self: Arc<Self>, account_id: i64) {
        let mut failures: u32 = 0;
        loop {
            if self.watch_inbox(account_id, &mut failures).await.is_err() {
                let pause = RETRY_FIRST
                    .saturating_mul(2_u32.saturating_pow(failures))
                    .min(RETRY_LAST);
                failures = failures.saturating_add(1);
                tokio::time::sleep(pause).await;
            }
        }
    }

    /// Syncs the inbox, then waits for news on a connection of its own, over
    /// and over, until something fails. A first sync that works counts as
    /// working again (`failures`).
    async fn watch_inbox(&self, account_id: i64, failures: &mut u32) -> MailResult<()> {
        let account = self.db(|store| store.account(account_id))?;
        let inbox = self
            .list_folders(account_id)?
            .into_iter()
            .find(|folder| folder.role == FolderRole::Inbox && folder.selectable)
            .ok_or_else(|| MailError::not_found("inbox"))?;

        // Not the shared connection: waiting would keep it from every click.
        let secret = self.secret(&account).await?;
        let mut waiting = self
            .connector
            .connect_incoming(&account.settings.incoming, &secret)
            .await
            .map_err(|error| self.refused(account_id, error))?;

        self.sync_folder(inbox.id, false).await?;
        *failures = 0;
        loop {
            waiting.wait_for_change(&inbox.path, WATCH_LIMIT).await?;
            self.take_arrivals(&account, &inbox, waiting.as_mut())
                .await?;
            // What else changed (read elsewhere, deleted, moved), the usual way.
            self.sync_folder(inbox.id, false).await?;
        }
    }

    async fn raw(&self, summary: &MessageSummary) -> MailResult<Vec<u8>> {
        if let Some(raw) = self.db(|store| store.raw(summary.id))? {
            return Ok(raw);
        }

        let folder = self.db(|store| store.folder(summary.folder_id))?;
        let account = self.db(|store| store.account(folder.account_id))?;
        let raw = self
            .session(&account)
            .await?
            .fetch_raw(&folder.path, summary.uid)
            .await?;
        self.db(|store| store.set_raw(summary.id, &raw))?;

        Ok(raw)
    }

    /// The full message, downloaded on first use and read from the cache
    /// afterwards. Does not mark it as read.
    ///
    /// Remote images are kept when asked for this one message
    /// (`allow_remote`) or when the user's settings allow them everywhere.
    pub async fn open_message(
        &self,
        message_id: i64,
        allow_remote: bool,
    ) -> MailResult<MessageContent> {
        let summary = self.db(|store| store.message(message_id))?;
        let raw = self.raw(&summary).await?;
        let allow_remote = allow_remote || self.settings()?.remote_images;
        let rendered = render(&raw, allow_remote)?;

        Ok(MessageContent {
            summary,
            text: rendered.text,
            html: rendered.html,
            attachments: rendered.attachments,
            remote_content_blocked: rendered.remote_content_blocked,
            message_id: rendered.message_id,
            references: rendered.references,
            reply_to: rendered.reply_to,
        })
    }

    pub async fn attachment(
        &self,
        message_id: i64,
        index: u32,
    ) -> MailResult<(Attachment, Vec<u8>)> {
        let summary = self.db(|store| store.message(message_id))?;
        let raw = self.raw(&summary).await?;

        mail_render::attachment(&raw, index)
    }

    /// Messages grouped by the folder they are in.
    fn by_folder(&self, message_ids: &[i64]) -> MailResult<BTreeMap<i64, Vec<MessageSummary>>> {
        let mut groups: BTreeMap<i64, Vec<MessageSummary>> = BTreeMap::new();
        for id in message_ids {
            let summary = self.db(|store| store.message(*id))?;
            groups.entry(summary.folder_id).or_default().push(summary);
        }

        Ok(groups)
    }

    fn adjust_counts(&self, folder_id: i64, unread: i64, total: i64) -> MailResult<()> {
        let shifted =
            |value: u32, by: i64| u32::try_from((i64::from(value) + by).max(0)).unwrap_or(u32::MAX);

        self.db(|store| {
            let folder = store.folder(folder_id)?;
            store.set_folder_counts(
                folder_id,
                shifted(folder.unread, unread),
                shifted(folder.total, total),
            )
        })
    }

    pub async fn set_flag(&self, message_ids: &[i64], flag: Flag, on: bool) -> MailResult<()> {
        for (folder_id, messages) in self.by_folder(message_ids)? {
            let folder = self.db(|store| store.folder(folder_id))?;
            let account = self.db(|store| store.account(folder.account_id))?;
            let uids: Vec<u32> = messages.iter().map(|message| message.uid).collect();

            self.session(&account)
                .await?
                .set_flag(&folder.path, &uids, flag, on)
                .await?;

            let mut unread_change = 0;
            for message in &messages {
                let mut flags = message.flags;
                match flag {
                    Flag::Seen => {
                        if flags.seen != on {
                            unread_change += if on { -1 } else { 1 };
                        }
                        flags.seen = on;
                    }
                    Flag::Flagged => flags.flagged = on,
                    Flag::Answered => flags.answered = on,
                }
                self.db(|store| store.set_flags(folder_id, message.uid, flags))?;
            }
            self.adjust_counts(folder_id, unread_change, 0)?;
        }

        Ok(())
    }

    fn forget(&self, folder_id: i64, messages: &[MessageSummary]) -> MailResult<()> {
        let uids: Vec<u32> = messages.iter().map(|message| message.uid).collect();
        let unread = messages
            .iter()
            .filter(|message| !message.flags.seen)
            .count();
        let count = |value: usize| i64::try_from(value).unwrap_or(i64::MAX);

        self.db(|store| store.delete_uids(folder_id, &uids))?;
        self.adjust_counts(folder_id, -count(unread), -count(messages.len()))
    }

    pub async fn move_messages(
        &self,
        message_ids: &[i64],
        target_folder_id: i64,
    ) -> MailResult<()> {
        let target = self.db(|store| store.folder(target_folder_id))?;

        for (folder_id, messages) in self.by_folder(message_ids)? {
            if folder_id == target_folder_id {
                continue;
            }
            let folder = self.db(|store| store.folder(folder_id))?;
            if folder.account_id != target.account_id {
                return Err(invalid("messages cannot be moved to another account"));
            }
            let account = self.db(|store| store.account(folder.account_id))?;
            let uids: Vec<u32> = messages.iter().map(|message| message.uid).collect();

            self.session(&account)
                .await?
                .move_messages(&folder.path, &uids, &target.path)
                .await?;
            self.forget(folder_id, &messages)?;
        }

        Ok(())
    }

    /// Moves messages to the trash; messages already in the trash, or on an
    /// account without a trash folder, are removed for good.
    pub async fn delete_messages(&self, message_ids: &[i64]) -> MailResult<()> {
        for (folder_id, messages) in self.by_folder(message_ids)? {
            let folder = self.db(|store| store.folder(folder_id))?;
            let account = self.db(|store| store.account(folder.account_id))?;
            let trash = self
                .list_folders(account.id)?
                .into_iter()
                .find(|candidate| candidate.role == FolderRole::Trash && candidate.selectable);
            let uids: Vec<u32> = messages.iter().map(|message| message.uid).collect();

            let mut session = self.session(&account).await?;
            match trash {
                Some(trash) if trash.id != folder_id => {
                    session
                        .move_messages(&folder.path, &uids, &trash.path)
                        .await?;
                }
                _ => session.delete_messages(&folder.path, &uids).await?,
            }
            drop(session);

            self.forget(folder_id, &messages)?;
        }

        Ok(())
    }

    // Writing

    pub async fn reply_draft(&self, message_id: i64, mode: ReplyMode) -> MailResult<Draft> {
        let content = self.open_message(message_id, false).await?;
        let folder = self.db(|store| store.folder(content.summary.folder_id))?;
        let account = self.db(|store| store.account(folder.account_id))?;

        Ok(reply_draft(&content, &account.settings.email, mode))
    }

    fn new_message_id(&self, domain: &str) -> String {
        let (seconds, nanos) = unix_now();
        let counter = self.message_counter.fetch_add(1, Ordering::Relaxed);

        format!("{seconds:x}.{nanos:x}.{counter:x}@{domain}")
    }

    pub async fn send_message(&self, message: &OutgoingMessage) -> MailResult<()> {
        let account = self.db(|store| store.account(message.account_id))?;
        let settings = &account.settings;

        let clean = |addresses: &[String]| -> MailResult<Vec<String>> {
            addresses
                .iter()
                .map(|address| address.trim())
                .filter(|address| !address.is_empty())
                .map(|address| {
                    EmailAddress::parse(address)
                        .map(|parsed| parsed.full())
                        .map_err(|_| invalid(format!("{address} is not a valid address")))
                })
                .collect()
        };
        let to = clean(&message.to)?;
        let cc = clean(&message.cc)?;
        let bcc = clean(&message.bcc)?;

        let mut recipients = to.clone();
        recipients.extend(cc.iter().cloned());
        recipients.extend(bcc.iter().cloned());
        if recipients.is_empty() {
            return Err(invalid("the message has no recipients"));
        }

        let own =
            EmailAddress::parse(&settings.email).map_err(|error| invalid(error.to_string()))?;
        let from = Contact {
            name: Some(settings.display_name.clone()).filter(|name| !name.is_empty()),
            email: own.full(),
        };
        let raw = compose(&ComposeInput {
            from: &from,
            to: &to,
            cc: &cc,
            subject: message.subject.trim(),
            body: &message.body,
            message_id: &self.new_message_id(own.domain()),
            in_reply_to: message.in_reply_to.as_deref(),
            references: &message.references,
            date: unix_now().0,
        })?;

        let secret = self.secret(&account).await?;
        let envelope = Envelope {
            from: &from.email,
            recipients: &recipients,
        };
        self.connector
            .send(&settings.outgoing, &secret, envelope, &raw)
            .await
            .map_err(|error| self.refused(account.id, error))?;

        // The message is out. Keeping a copy in Sent is best effort: a
        // failure here must not make the send look failed.
        let sent = self
            .list_folders(account.id)?
            .into_iter()
            .find(|folder| folder.role == FolderRole::Sent && folder.selectable);
        if let Some(sent) = sent
            && let Ok(mut session) = self.session(&account).await
        {
            let _ = session.append(&sent.path, &raw, true).await;
        }

        Ok(())
    }

    // Settings

    pub fn settings(&self) -> MailResult<AppSettings> {
        let stored = self.db(|store| store.settings())?;

        Ok(AppSettings::from_stored(|name| stored.get(name).cloned()))
    }

    pub fn set_settings(&self, settings: AppSettings) -> MailResult<()> {
        settings.check()?;

        self.db(|store| {
            for (name, value) in settings.to_stored() {
                match value {
                    Some(value) => store.set_setting(name, &value)?,
                    None => store.remove_setting(name)?,
                }
            }

            Ok(())
        })
    }

    // Sender pictures

    fn logo_lookup(&self, domain: &str) -> MailResult<Arc<AsyncMutex<()>>> {
        let mut lookups = self
            .logo_lookups
            .lock()
            .map_err(|_| MailError::new(ErrorKind::Connect, "the lookup table is unusable"))?;

        Ok(Arc::clone(lookups.entry(domain.to_string()).or_default()))
    }

    /// The picture the sender's domain publishes for its mail, ready for an
    /// `<img>`. `None` when it has none, when pictures are turned off, or
    /// when it cannot be found out right now: a missing picture is never an
    /// error worth showing.
    ///
    /// Turned off means no request leaves the machine for this at all.
    pub async fn sender_picture(&self, email: &str) -> MailResult<Option<String>> {
        if !self.settings()?.sender_pictures {
            return Ok(None);
        }
        let Some(domain) = sender_domain(email) else {
            return Ok(None);
        };

        let lookup = self.logo_lookup(&domain)?;
        let _one_at_a_time = lookup.lock().await;

        let (now, _) = unix_now();
        let cached = self.db(|store| store.logo(&domain))?;
        if let Some(cached) = &cached
            && now - cached.checked < LOGO_TTL
        {
            return Ok(cached.svg.clone().map(picture));
        }

        match self.logos.logo(&domain).await {
            Ok(found) => {
                let logo = found.filter(|bytes| is_logo(bytes));
                self.db(|store| store.put_logo(&domain, logo.as_deref(), now))?;

                Ok(logo.map(picture))
            }
            // Offline or a broken resolver: the old answer still serves, and
            // nothing is stored, so the domain is asked again next time.
            Err(_) => Ok(cached.and_then(|cached| cached.svg).map(picture)),
        }
    }
}
