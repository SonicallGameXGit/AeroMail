//! In-memory stand-ins for the mail servers, the keychain and the
//! provider's sign-in page.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use mail_core::account::{
    AccountSettings, AuthMethod, Protocol, Security, ServerRole, ServerSettings,
};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::mail::{FolderRole, MessageFlags};
use mail_core::oauth::Tokens;
use mail_core::transport::{
    Connector, Envelope, Flag, FolderState, IncomingSession, LogoSource, PendingSignIn,
    RemoteFolder, RemoteMessage, SecretStore, SignInFlow,
};
use tokio::sync::{Notify, oneshot};

pub const PASSWORD: &str = "secret";

#[derive(Clone)]
pub struct FakeMessage {
    pub uid: u32,
    pub flags: MessageFlags,
    pub raw: Vec<u8>,
}

pub struct FakeFolder {
    pub path: String,
    pub role: FolderRole,
    pub uid_validity: u32,
    pub next_uid: u32,
    pub messages: Vec<FakeMessage>,
}

pub struct SentMail {
    pub from: String,
    pub recipients: Vec<String>,
    pub raw: Vec<u8>,
}

/// The whole remote side: one account's mailbox plus what was sent.
#[derive(Default)]
pub struct Remote {
    pub folders: Vec<FakeFolder>,
    pub sent: Vec<SentMail>,
    pub connections: u32,
    pub body_downloads: u32,
    /// Makes the next liveness check fail, as if the server hung up.
    pub drop_connection: bool,
    pub outgoing_down: bool,
    /// The access tokens the servers accept from accounts that sign in on
    /// the provider's page.
    pub access_tokens: Vec<String>,
    /// What each connection and each sent message signed in with.
    pub secrets_used: Vec<String>,
    /// Ends the wait of a connection that listens for news; see `change`.
    pub news: Arc<Notify>,
    /// How often only the new messages were asked for.
    pub fetched_new: u32,
    /// How often the whole folder was listed.
    pub listed: u32,
}

pub type SharedRemote = Arc<Mutex<Remote>>;

pub fn raw_message(subject: &str, body: &str) -> Vec<u8> {
    format!(
        "From: Ann <ann@example.com>\r\nTo: me@example.com\r\nSubject: {subject}\r\n\
         Date: Tue, 14 Nov 2023 22:13:20 +0000\r\nMessage-ID: <{subject}@example.com>\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n"
    )
    .into_bytes()
}

impl Remote {
    pub fn standard() -> SharedRemote {
        let folder = |path: &str, role| FakeFolder {
            path: path.to_string(),
            role,
            uid_validity: 1,
            next_uid: 1,
            messages: Vec::new(),
        };

        Arc::new(Mutex::new(Remote {
            folders: vec![
                folder("INBOX", FolderRole::Inbox),
                folder("Sent", FolderRole::Sent),
                folder("Trash", FolderRole::Trash),
                folder("Archive", FolderRole::Archive),
            ],
            ..Remote::default()
        }))
    }

    pub fn folder(&mut self, path: &str) -> &mut FakeFolder {
        self.folders
            .iter_mut()
            .find(|folder| folder.path == path)
            .unwrap()
    }

    /// Delivers a message and returns its UID.
    pub fn deliver(&mut self, path: &str, subject: &str, seen: bool) -> u32 {
        let folder = self.folder(path);
        let uid = folder.next_uid;
        folder.next_uid += 1;
        folder.messages.push(FakeMessage {
            uid,
            flags: MessageFlags {
                seen,
                ..MessageFlags::default()
            },
            raw: raw_message(subject, &format!("Body of {subject}")),
        });

        uid
    }

    pub fn uids(&mut self, path: &str) -> Vec<u32> {
        self.folder(path)
            .messages
            .iter()
            .map(|message| message.uid)
            .collect()
    }
}

fn missing_folder() -> MailError {
    MailError::new(ErrorKind::Protocol, "no such folder")
}

struct FakeSession {
    remote: SharedRemote,
}

impl FakeSession {
    fn with_folder<T>(&self, path: &str, work: impl FnOnce(&mut FakeFolder) -> T) -> MailResult<T> {
        let mut remote = self.remote.lock().unwrap();
        let folder = remote
            .folders
            .iter_mut()
            .find(|folder| folder.path == path)
            .ok_or_else(missing_folder)?;

        Ok(work(folder))
    }
}

fn header_of(raw: &[u8]) -> Vec<u8> {
    let end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map_or(raw.len(), |position| position + 4);

    raw[..end].to_vec()
}

#[async_trait]
impl IncomingSession for FakeSession {
    async fn is_alive(&mut self) -> bool {
        let mut remote = self.remote.lock().unwrap();
        if remote.drop_connection {
            remote.drop_connection = false;

            return false;
        }

        true
    }

    async fn list_folders(&mut self) -> MailResult<Vec<RemoteFolder>> {
        let remote = self.remote.lock().unwrap();

        Ok(remote
            .folders
            .iter()
            .map(|folder| RemoteFolder {
                path: folder.path.clone(),
                name: folder.path.clone(),
                depth: 0,
                role: folder.role,
                selectable: true,
                unread: folder
                    .messages
                    .iter()
                    .filter(|message| !message.flags.seen)
                    .count() as u32,
                total: folder.messages.len() as u32,
            })
            .collect())
    }

    async fn folder_state(&mut self, folder: &str) -> MailResult<FolderState> {
        self.remote.lock().unwrap().listed += 1;
        self.with_folder(folder, |folder| FolderState {
            uid_validity: folder.uid_validity,
            uids: folder.messages.iter().map(|message| message.uid).collect(),
            unread: folder
                .messages
                .iter()
                .filter(|message| !message.flags.seen)
                .count() as u32,
        })
    }

    async fn fetch_headers(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<RemoteMessage>> {
        self.with_folder(folder, |folder| {
            folder
                .messages
                .iter()
                .filter(|message| uids.contains(&message.uid))
                .map(|message| RemoteMessage {
                    uid: message.uid,
                    flags: message.flags,
                    size: message.raw.len() as u32,
                    received: 1_700_000_000,
                    header: header_of(&message.raw),
                })
                .collect()
        })
    }

    async fn fetch_new(&mut self, folder: &str, after: u32) -> MailResult<Vec<RemoteMessage>> {
        self.remote.lock().unwrap().fetched_new += 1;
        let uids: Vec<u32> = self.with_folder(folder, |folder| {
            folder
                .messages
                .iter()
                .map(|message| message.uid)
                .filter(|uid| *uid > after)
                .collect()
        })?;

        self.fetch_headers(folder, &uids).await
    }

    async fn fetch_flags(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<(u32, MessageFlags)>> {
        self.with_folder(folder, |folder| {
            folder
                .messages
                .iter()
                .filter(|message| uids.contains(&message.uid))
                .map(|message| (message.uid, message.flags))
                .collect()
        })
    }

    async fn fetch_raw(&mut self, folder: &str, uid: u32) -> MailResult<Vec<u8>> {
        self.remote.lock().unwrap().body_downloads += 1;

        self.with_folder(folder, |folder| {
            folder
                .messages
                .iter()
                .find(|message| message.uid == uid)
                .map(|message| message.raw.clone())
        })?
        .ok_or_else(|| MailError::not_found("message"))
    }

    async fn set_flag(
        &mut self,
        folder: &str,
        uids: &[u32],
        flag: Flag,
        on: bool,
    ) -> MailResult<()> {
        self.with_folder(folder, |folder| {
            for message in &mut folder.messages {
                if !uids.contains(&message.uid) {
                    continue;
                }
                match flag {
                    Flag::Seen => message.flags.seen = on,
                    Flag::Flagged => message.flags.flagged = on,
                    Flag::Answered => message.flags.answered = on,
                }
            }
        })
    }

    async fn move_messages(&mut self, folder: &str, uids: &[u32], target: &str) -> MailResult<()> {
        let moved = self.with_folder(folder, |folder| {
            let (moved, kept) = std::mem::take(&mut folder.messages)
                .into_iter()
                .partition(|message| uids.contains(&message.uid));
            folder.messages = kept;

            moved
        })?;

        self.with_folder(target, |target| {
            let moved: Vec<FakeMessage> = moved;
            for mut message in moved {
                message.uid = target.next_uid;
                target.next_uid += 1;
                target.messages.push(message);
            }
        })
    }

    async fn delete_messages(&mut self, folder: &str, uids: &[u32]) -> MailResult<()> {
        self.with_folder(folder, |folder| {
            folder
                .messages
                .retain(|message| !uids.contains(&message.uid));
        })
    }

    async fn wait_for_change(&mut self, _: &str, limit: Duration) -> MailResult<()> {
        let news = Arc::clone(&self.remote.lock().unwrap().news);
        let _ = tokio::time::timeout(limit, news.notified()).await;

        Ok(())
    }

    async fn append(&mut self, folder: &str, raw: &[u8], seen: bool) -> MailResult<()> {
        self.with_folder(folder, |folder| {
            let uid = folder.next_uid;
            folder.next_uid += 1;
            folder.messages.push(FakeMessage {
                uid,
                flags: MessageFlags {
                    seen,
                    ..MessageFlags::default()
                },
                raw: raw.to_vec(),
            });
        })
    }
}

pub struct FakeConnector {
    pub remote: SharedRemote,
}

impl FakeConnector {
    fn check_secret(
        &self,
        server: &ServerSettings,
        secret: &str,
        role: ServerRole,
    ) -> MailResult<()> {
        let mut remote = self.remote.lock().unwrap();
        let accepted = match server.auth {
            AuthMethod::Password => secret == PASSWORD,
            AuthMethod::OAuth2 => remote.access_tokens.iter().any(|token| token == secret),
        };
        if !accepted {
            return Err(MailError::new(ErrorKind::Auth, "invalid credentials").on(role));
        }
        remote.secrets_used.push(secret.to_string());

        Ok(())
    }
}

#[async_trait]
impl Connector for FakeConnector {
    async fn connect_incoming(
        &self,
        server: &ServerSettings,
        secret: &str,
    ) -> MailResult<Box<dyn IncomingSession>> {
        self.check_secret(server, secret, ServerRole::Incoming)?;
        self.remote.lock().unwrap().connections += 1;

        Ok(Box::new(FakeSession {
            remote: Arc::clone(&self.remote),
        }))
    }

    async fn check_outgoing(&self, server: &ServerSettings, secret: &str) -> MailResult<()> {
        if self.remote.lock().unwrap().outgoing_down {
            return Err(
                MailError::new(ErrorKind::Connect, "connection refused").on(ServerRole::Outgoing)
            );
        }

        self.check_secret(server, secret, ServerRole::Outgoing)
    }

    async fn send(
        &self,
        server: &ServerSettings,
        secret: &str,
        envelope: Envelope<'_>,
        raw: &[u8],
    ) -> MailResult<()> {
        self.check_outgoing(server, secret).await?;
        self.remote.lock().unwrap().sent.push(SentMail {
            from: envelope.from.to_string(),
            recipients: envelope.recipients.to_vec(),
            raw: raw.to_vec(),
        });

        Ok(())
    }
}

#[derive(Default)]
pub struct FakeSecrets {
    pub values: Mutex<HashMap<String, String>>,
    pub unavailable: Mutex<bool>,
}

impl FakeSecrets {
    fn check(&self) -> MailResult<()> {
        if *self.unavailable.lock().unwrap() {
            return Err(MailError::new(ErrorKind::Keychain, "no keychain service"));
        }

        Ok(())
    }
}

impl SecretStore for FakeSecrets {
    fn get(&self, key: &str) -> MailResult<Option<String>> {
        self.check()?;

        Ok(self.values.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, secret: &str) -> MailResult<()> {
        self.check()?;
        self.values
            .lock()
            .unwrap()
            .insert(key.to_string(), secret.to_string());

        Ok(())
    }

    fn delete(&self, key: &str) -> MailResult<()> {
        self.check()?;
        self.values.lock().unwrap().remove(key);

        Ok(())
    }
}

pub const SIGN_IN_URL: &str = "https://accounts.example.com/sign-in";
pub const REFRESH_TOKEN: &str = "refresh-1";

/// The provider's sign-in page and token address. The page is "used" by the
/// test itself: `approve` and `refuse` are what the user does in the browser.
pub struct FakeSignIn {
    remote: SharedRemote,
    /// Seconds an access token is said to last.
    pub lifetime: Mutex<u64>,
    /// The user took the app's permission back at the provider.
    pub revoked: Mutex<bool>,
    /// Server and address of every sign-in that was begun.
    pub begun: Mutex<Vec<(String, String)>>,
    pub renewals: Mutex<u32>,
    issued: Mutex<u32>,
    browser: Mutex<Option<oneshot::Sender<MailResult<Tokens>>>>,
}

struct FakePending {
    answer: oneshot::Receiver<MailResult<Tokens>>,
}

#[async_trait]
impl PendingSignIn for FakePending {
    fn url(&self) -> &str {
        SIGN_IN_URL
    }

    async fn finish(self: Box<Self>) -> MailResult<Tokens> {
        match self.answer.await {
            Ok(answer) => answer,
            // The browser was closed: nothing ever comes.
            Err(_) => std::future::pending().await,
        }
    }
}

impl FakeSignIn {
    pub fn new(remote: SharedRemote) -> Self {
        Self {
            remote,
            lifetime: Mutex::new(3600),
            revoked: Mutex::new(false),
            begun: Mutex::new(Vec::new()),
            renewals: Mutex::new(0),
            issued: Mutex::new(0),
            browser: Mutex::new(None),
        }
    }

    /// A new access token; the servers accept it and no earlier one.
    fn issue(&self) -> Tokens {
        let mut issued = self.issued.lock().unwrap();
        *issued += 1;
        let access = format!("access-{issued}");
        self.remote.lock().unwrap().access_tokens = vec![access.clone()];

        Tokens {
            access,
            refresh: None,
            expires_in: *self.lifetime.lock().unwrap(),
        }
    }

    fn answer(&self, answer: MailResult<Tokens>) {
        let browser = self.browser.lock().unwrap().take().unwrap();
        let _ = browser.send(answer);
    }

    /// The user signs in and allows the app.
    pub fn approve(&self) {
        self.answer(Ok(Tokens {
            refresh: Some(REFRESH_TOKEN.to_string()),
            ..self.issue()
        }));
    }

    /// The user presses "Cancel" on the provider's page.
    pub fn refuse(&self) {
        self.answer(Err(MailError::new(
            ErrorKind::SignIn,
            "the provider answered: access_denied",
        )));
    }
}

#[async_trait]
impl SignInFlow for FakeSignIn {
    async fn begin(&self, host: &str, email: &str) -> MailResult<Box<dyn PendingSignIn>> {
        if host != "imap.example.com" {
            return Err(MailError::new(
                ErrorKind::Unsupported,
                format!("no sign-in page is known for {host}"),
            ));
        }
        self.begun
            .lock()
            .unwrap()
            .push((host.to_string(), email.to_string()));

        let (browser, answer) = oneshot::channel();
        *self.browser.lock().unwrap() = Some(browser);

        Ok(Box::new(FakePending { answer }))
    }

    async fn refresh(&self, _host: &str, refresh_token: &str) -> MailResult<Tokens> {
        if *self.revoked.lock().unwrap() || refresh_token != REFRESH_TOKEN {
            return Err(MailError::new(
                ErrorKind::SignIn,
                "invalid_grant: Token has been expired or revoked.",
            ));
        }
        *self.renewals.lock().unwrap() += 1;

        Ok(self.issue())
    }
}

pub const LOGO: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";

/// The internet as far as logos go: which domains publish one, and a record
/// of every question asked.
#[derive(Default)]
pub struct FakeLogos {
    pub published: Mutex<HashMap<String, Vec<u8>>>,
    pub asked: Mutex<Vec<String>>,
    pub offline: Mutex<bool>,
}

impl FakeLogos {
    pub fn publish(&self, domain: &str, bytes: &[u8]) {
        self.published
            .lock()
            .unwrap()
            .insert(domain.to_string(), bytes.to_vec());
    }

    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

#[async_trait]
impl LogoSource for FakeLogos {
    async fn logo(&self, domain: &str) -> MailResult<Option<Vec<u8>>> {
        self.asked.lock().unwrap().push(domain.to_string());
        if *self.offline.lock().unwrap() {
            return Err(MailError::new(ErrorKind::Connect, "no network"));
        }

        Ok(self.published.lock().unwrap().get(domain).cloned())
    }
}

pub fn settings() -> AccountSettings {
    let server = |protocol, host: &str, port| ServerSettings {
        protocol,
        host: host.to_string(),
        port,
        security: Security::Tls,
        auth: AuthMethod::Password,
        username: "me@example.com".to_string(),
    };

    AccountSettings {
        email: "me@example.com".to_string(),
        display_name: "Me Myself".to_string(),
        incoming: server(Protocol::Imap, "imap.example.com", 993),
        outgoing: server(Protocol::Smtp, "smtp.example.com", 465),
    }
}

/// `settings`, signing in on the provider's page instead of with a password.
pub fn sign_in_settings() -> AccountSettings {
    let mut settings = settings();
    settings.incoming.auth = AuthMethod::OAuth2;
    settings.outgoing.auth = AuthMethod::OAuth2;

    settings
}
