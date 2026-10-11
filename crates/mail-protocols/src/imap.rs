use std::fmt::Debug;
use std::time::Duration;

use async_imap::error::Error as ImapError;
use async_imap::types::{
    Capability, Fetch, Flag as ImapFlag, Name, NameAttribute, UnsolicitedResponse,
};
use async_imap::{Authenticator, Client, Session};
use async_trait::async_trait;
use futures::TryStreamExt;
use mail_core::account::{AuthMethod, Security, ServerSettings};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::mail::{FolderRole, MessageFlags};
use mail_core::transport::{Flag, FolderState, IncomingSession, RemoteFolder, RemoteMessage};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::tls;

trait Stream: AsyncRead + AsyncWrite + Unpin + Send + Debug {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + Debug> Stream for T {}
type BoxStream = Box<dyn Stream>;

/// Messages asked for in one command; keeps command lines short.
const CHUNK: usize = 100;

pub(crate) struct ImapSession {
    /// Taken out while the connection idles, and gone for good when that
    /// failed: a connection in an unknown state is never reused.
    session: Option<Session<BoxStream>>,
    selected: Option<String>,
    can_move: bool,
    can_uid_expunge: bool,
    can_idle: bool,
}

/// The header fields of a message the list shows, groups by and sorts into
/// inbox tabs by (`mail_render::parse_header`). Only these: whole header
/// blocks are several times larger (signatures, routing) and were most of a
/// sync.
pub(crate) const LIST_FIELDS: &str = "SUBJECT FROM TO CC DATE MESSAGE-ID IN-REPLY-TO REFERENCES \
     LIST-ID LIST-POST LIST-UNSUBSCRIBE PRECEDENCE AUTO-SUBMITTED \
     X-CAMPAIGN X-CAMPAIGNID X-CAMPAIGN-ID X-MC-USER X-MAILIN-CAMPAIGN X-SFMC-STACK";

/// Servers without IDLE are asked again this often.
const POLL: Duration = Duration::from_secs(60);

fn imap_error(error: ImapError) -> MailError {
    let kind = match &error {
        ImapError::Io(_) | ImapError::ConnectionLost => ErrorKind::Connect,
        _ => ErrorKind::Protocol,
    };

    MailError::new(kind, error.to_string())
}

/// A server that does not answer in time is treated as gone.
async fn within<T>(
    limit: Duration,
    work: impl Future<Output = Result<T, ImapError>>,
) -> MailResult<T> {
    match tokio::time::timeout(limit, work).await {
        Ok(result) => result.map_err(imap_error),
        Err(_) => Err(MailError::new(
            ErrorKind::Connect,
            "the server stopped answering",
        )),
    }
}

fn uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn flags_of(fetch: &Fetch) -> MessageFlags {
    let mut flags = MessageFlags::default();
    for flag in fetch.flags() {
        match flag {
            ImapFlag::Seen => flags.seen = true,
            ImapFlag::Flagged => flags.flagged = true,
            ImapFlag::Answered => flags.answered = true,
            _ => {}
        }
    }

    flags
}

async fn greeting<T>(client: &mut Client<T>) -> MailResult<()>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Debug,
{
    let response = tokio::time::timeout(tls::TIMEOUT, client.read_response()).await;
    match response {
        Ok(Ok(Some(_))) => Ok(()),
        Ok(Ok(None)) => Err(MailError::new(
            ErrorKind::Connect,
            "the server closed the connection",
        )),
        Ok(Err(error)) => Err(MailError::new(ErrorKind::Connect, error.to_string())),
        Err(_) => Err(MailError::new(
            ErrorKind::Connect,
            "the server did not greet",
        )),
    }
}

/// The XOAUTH2 sign-in: the address and an access token in one answer.
struct XOAuth2 {
    answer: Option<String>,
}

impl Authenticator for XOAuth2 {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> String {
        // A second question is the server's report of a refused token; it
        // waits for an empty line before saying no.
        self.answer.take().unwrap_or_default()
    }
}

pub(crate) async fn connect(server: &ServerSettings, secret: &str) -> MailResult<ImapSession> {
    let host = server.host.trim();
    let tcp = tls::tcp(host, server.port).await?;

    let client: Client<BoxStream> = match server.security {
        Security::Tls => {
            let stream: BoxStream = Box::new(tls::wrap(tcp, host).await?);
            let mut client = Client::new(stream);
            greeting(&mut client).await?;
            client
        }
        Security::StartTls => {
            let mut plain = Client::new(tcp);
            greeting(&mut plain).await?;
            plain
                .run_command_and_check_ok("STARTTLS", None)
                .await
                .map_err(|error| MailError::new(ErrorKind::Tls, error.to_string()))?;
            let stream: BoxStream = Box::new(tls::wrap(plain.into_inner(), host).await?);
            Client::new(stream)
        }
        Security::None => {
            let stream: BoxStream = Box::new(tcp);
            let mut client = Client::new(stream);
            greeting(&mut client).await?;
            client
        }
    };

    let signed_in = match server.auth {
        AuthMethod::Password => client.login(&server.username, secret).await,
        AuthMethod::OAuth2 => {
            let answer = format!("user={}\x01auth=Bearer {secret}\x01\x01", server.username);
            client
                .authenticate(
                    "XOAUTH2",
                    XOAuth2 {
                        answer: Some(answer),
                    },
                )
                .await
        }
    };
    let mut session = signed_in.map_err(|(error, _)| match error {
        ImapError::No(detail) | ImapError::Bad(detail) => MailError::new(ErrorKind::Auth, detail),
        other => imap_error(other),
    })?;

    let capabilities = session.capabilities().await.map_err(imap_error)?;
    let has = |name: &str| capabilities.has(&Capability::Atom(name.to_string()));
    let can_move = has("MOVE");
    let can_uid_expunge = has("UIDPLUS");
    let can_idle = has("IDLE");

    Ok(ImapSession {
        session: Some(session),
        selected: None,
        can_move,
        can_uid_expunge,
        can_idle,
    })
}

fn describe(name: &Name) -> RemoteFolder {
    let path = name.name().to_string();
    let segments: Vec<&str> = match name.delimiter() {
        Some(delimiter) if !delimiter.is_empty() => path.split(delimiter).collect(),
        _ => vec![path.as_str()],
    };
    let last = segments.last().copied().unwrap_or(path.as_str());
    let readable = utf7_imap::decode_utf7_imap(last.to_string());

    let mut selectable = true;
    let mut announced = None;
    for attribute in name.attributes() {
        match attribute {
            NameAttribute::NoSelect => selectable = false,
            NameAttribute::All => announced = Some(FolderRole::All),
            NameAttribute::Archive => announced = Some(FolderRole::Archive),
            NameAttribute::Drafts => announced = Some(FolderRole::Drafts),
            NameAttribute::Flagged => announced = Some(FolderRole::Flagged),
            NameAttribute::Junk => announced = Some(FolderRole::Junk),
            NameAttribute::Sent => announced = Some(FolderRole::Sent),
            NameAttribute::Trash => announced = Some(FolderRole::Trash),
            _ => {}
        }
    }

    let role = if path.eq_ignore_ascii_case("INBOX") {
        FolderRole::Inbox
    } else {
        match announced {
            Some(role) => role,
            // "Inbox" somewhere deeper is an ordinary folder.
            None => match FolderRole::from_name(&readable) {
                FolderRole::Inbox => FolderRole::Other,
                role => role,
            },
        }
    };

    let depth = u32::try_from(segments.len().saturating_sub(1)).unwrap_or(0);

    RemoteFolder {
        path,
        name: readable,
        depth,
        role,
        selectable,
        unread: 0,
        total: 0,
    }
}

impl ImapSession {
    fn live(&mut self) -> MailResult<&mut Session<BoxStream>> {
        self.session
            .as_mut()
            .ok_or_else(|| MailError::new(ErrorKind::Connect, "the connection was lost"))
    }

    async fn select(&mut self, folder: &str) -> MailResult<()> {
        if self.selected.as_deref() == Some(folder) {
            return Ok(());
        }

        self.selected = None;
        self.live()?.select(folder).await.map_err(imap_error)?;
        self.selected = Some(folder.to_string());

        Ok(())
    }

    async fn store(&mut self, uids: &[u32], change: &str) -> MailResult<()> {
        for chunk in uids.chunks(CHUNK) {
            let updates = self
                .live()?
                .uid_store(uid_set(chunk), change)
                .await
                .map_err(imap_error)?;
            updates.try_collect::<Vec<_>>().await.map_err(imap_error)?;
        }

        Ok(())
    }

    /// What the message list shows of the messages in `set`, in one command.
    async fn fetch_summaries(&mut self, set: &str) -> MailResult<Vec<RemoteMessage>> {
        let query = format!(
            "(UID FLAGS RFC822.SIZE INTERNALDATE BODY.PEEK[HEADER.FIELDS ({LIST_FIELDS})])"
        );
        let stream = self
            .live()?
            .uid_fetch(set, query)
            .await
            .map_err(imap_error)?;
        let fetched: Vec<Fetch> = stream.try_collect().await.map_err(imap_error)?;

        Ok(fetched
            .iter()
            .filter_map(|fetch| {
                Some(RemoteMessage {
                    uid: fetch.uid?,
                    flags: flags_of(fetch),
                    size: fetch.size.unwrap_or(0),
                    received: fetch
                        .internal_date()
                        .map(|date| date.timestamp())
                        .unwrap_or(0),
                    header: fetch.header().unwrap_or_default().to_vec(),
                })
            })
            .collect())
    }

    async fn fetch(&mut self, uids: &[u32], query: &str) -> MailResult<Vec<Fetch>> {
        let mut fetched = Vec::new();
        for chunk in uids.chunks(CHUNK) {
            let stream = self
                .live()?
                .uid_fetch(uid_set(chunk), query)
                .await
                .map_err(imap_error)?;
            let mut part: Vec<Fetch> = stream.try_collect().await.map_err(imap_error)?;
            fetched.append(&mut part);
        }

        Ok(fetched)
    }
}

#[async_trait]
impl IncomingSession for ImapSession {
    async fn is_alive(&mut self) -> bool {
        match self.live() {
            Ok(session) => session.noop().await.is_ok(),
            Err(_) => false,
        }
    }

    async fn list_folders(&mut self) -> MailResult<Vec<RemoteFolder>> {
        let names: Vec<Name> = {
            let stream = self
                .live()?
                .list(Some(""), Some("*"))
                .await
                .map_err(imap_error)?;
            stream.try_collect().await.map_err(imap_error)?
        };

        let mut folders: Vec<RemoteFolder> = names.iter().map(describe).collect();
        for folder in &mut folders {
            if !folder.selectable {
                continue;
            }
            // Counts are a nicety; a folder that refuses STATUS still shows up.
            if let Ok(status) = self.live()?.status(&folder.path, "(MESSAGES UNSEEN)").await {
                folder.total = status.exists;
                folder.unread = status.unseen.unwrap_or(0);
            }
        }

        Ok(folders)
    }

    async fn folder_state(&mut self, folder: &str) -> MailResult<FolderState> {
        // Always a fresh SELECT: its answer carries the current UID validity.
        self.selected = None;
        let mailbox = self.live()?.select(folder).await.map_err(imap_error)?;
        self.selected = Some(folder.to_string());

        let mut uids: Vec<u32> = self
            .live()?
            .uid_search("ALL")
            .await
            .map_err(imap_error)?
            .into_iter()
            .collect();
        uids.sort_unstable();
        let unread = self
            .live()?
            .uid_search("UNSEEN")
            .await
            .map_err(imap_error)?
            .len();

        Ok(FolderState {
            uid_validity: mailbox.uid_validity.unwrap_or(0),
            uids,
            unread: u32::try_from(unread).unwrap_or(u32::MAX),
        })
    }

    async fn fetch_headers(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<RemoteMessage>> {
        self.select(folder).await?;
        let mut fetched = Vec::new();
        for chunk in uids.chunks(CHUNK) {
            fetched.extend(self.fetch_summaries(&uid_set(chunk)).await?);
        }

        Ok(fetched)
    }

    async fn fetch_new(&mut self, folder: &str, after: u32) -> MailResult<Vec<RemoteMessage>> {
        self.select(folder).await?;
        let first = after.saturating_add(1);
        let fetched = self.fetch_summaries(&format!("{first}:*")).await?;

        // `n:*` always matches the last message, even one below `n`.
        Ok(fetched
            .into_iter()
            .filter(|message| message.uid > after)
            .collect())
    }

    async fn fetch_flags(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<(u32, MessageFlags)>> {
        self.select(folder).await?;
        let fetched = self.fetch(uids, "(UID FLAGS)").await?;

        Ok(fetched
            .iter()
            .filter_map(|fetch| Some((fetch.uid?, flags_of(fetch))))
            .collect())
    }

    async fn fetch_raw(&mut self, folder: &str, uid: u32) -> MailResult<Vec<u8>> {
        self.select(folder).await?;
        let fetched = self.fetch(&[uid], "(UID BODY.PEEK[])").await?;

        fetched
            .iter()
            .find_map(|fetch| fetch.body().map(<[u8]>::to_vec))
            .ok_or_else(|| MailError::not_found("message"))
    }

    async fn set_flag(
        &mut self,
        folder: &str,
        uids: &[u32],
        flag: Flag,
        on: bool,
    ) -> MailResult<()> {
        self.select(folder).await?;
        let name = match flag {
            Flag::Seen => "\\Seen",
            Flag::Flagged => "\\Flagged",
            Flag::Answered => "\\Answered",
        };
        let sign = if on { '+' } else { '-' };

        self.store(uids, &format!("{sign}FLAGS.SILENT ({name})"))
            .await
    }

    async fn move_messages(&mut self, folder: &str, uids: &[u32], target: &str) -> MailResult<()> {
        self.select(folder).await?;

        if self.can_move {
            for chunk in uids.chunks(CHUNK) {
                self.live()?
                    .uid_mv(uid_set(chunk), target)
                    .await
                    .map_err(imap_error)?;
            }

            return Ok(());
        }

        for chunk in uids.chunks(CHUNK) {
            self.live()?
                .uid_copy(uid_set(chunk), target)
                .await
                .map_err(imap_error)?;
        }

        self.delete_messages(folder, uids).await
    }

    async fn delete_messages(&mut self, folder: &str, uids: &[u32]) -> MailResult<()> {
        self.select(folder).await?;
        self.store(uids, "+FLAGS.SILENT (\\Deleted)").await?;

        if self.can_uid_expunge {
            for chunk in uids.chunks(CHUNK) {
                let expunged = self
                    .live()?
                    .uid_expunge(uid_set(chunk))
                    .await
                    .map_err(imap_error)?;
                expunged.try_collect::<Vec<_>>().await.map_err(imap_error)?;
            }
        } else {
            let expunged = self.live()?.expunge().await.map_err(imap_error)?;
            expunged.try_collect::<Vec<_>>().await.map_err(imap_error)?;
        }

        Ok(())
    }

    async fn append(&mut self, folder: &str, raw: &[u8], seen: bool) -> MailResult<()> {
        let flags = if seen { Some("(\\Seen)") } else { None };

        self.live()?
            .append(folder, flags, None, raw)
            .await
            .map_err(imap_error)
    }

    async fn wait_for_change(&mut self, folder: &str, limit: Duration) -> MailResult<()> {
        if !self.can_idle {
            tokio::time::sleep(limit.min(POLL)).await;

            return Ok(());
        }

        self.select(folder).await?;
        // Mail that arrived since the last wait is told in the answer to the
        // next command; idling first would only hear of it much later. The
        // check costs one round trip, before the wait, not after the news.
        self.live()?.noop().await.map_err(imap_error)?;
        let mut changed = false;
        while let Ok(news) = self.live()?.unsolicited_responses.try_recv() {
            changed |= matches!(
                news,
                UnsolicitedResponse::Exists(_)
                    | UnsolicitedResponse::Recent(_)
                    | UnsolicitedResponse::Expunge(_)
            );
        }
        if changed {
            return Ok(());
        }

        let Some(session) = self.session.take() else {
            return Err(MailError::new(
                ErrorKind::Connect,
                "the connection was lost",
            ));
        };

        let mut idle = session.idle();
        within(tls::TIMEOUT, idle.init()).await?;
        // The stop source must outlive the wait: dropping it ends the wait.
        let waited = {
            let (wait, _stop) = idle.wait_with_timeout(limit);
            wait.await
        };
        waited.map_err(imap_error)?;
        // A connection that went away while idling would never answer.
        self.session = Some(within(tls::TIMEOUT, idle.done()).await?);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mail_core::account::Protocol;

    use super::*;
    use crate::fake_server::FakeImap;

    fn settings(port: u16) -> ServerSettings {
        ServerSettings {
            protocol: Protocol::Imap,
            host: "127.0.0.1".to_string(),
            port,
            security: Security::None,
            auth: AuthMethod::Password,
            username: "me@example.com".to_string(),
        }
    }

    #[tokio::test]
    async fn wrong_password_is_an_auth_error() {
        let server = FakeImap::start().await;

        let error = connect(&settings(server.port), "wrong")
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind, ErrorKind::Auth);
    }

    fn token_settings(port: u16) -> ServerSettings {
        ServerSettings {
            auth: AuthMethod::OAuth2,
            ..settings(port)
        }
    }

    #[tokio::test]
    async fn signs_in_with_an_access_token() {
        let server = FakeImap::start().await;
        let mut session = connect(&token_settings(server.port), "token")
            .await
            .unwrap();

        assert!(session.is_alive().await);
        let commands = server.commands();
        assert!(commands.iter().any(|sent| sent == "AUTHENTICATE XOAUTH2"));
        assert!(!commands.iter().any(|sent| sent.starts_with("LOGIN")));
    }

    #[tokio::test]
    async fn a_refused_access_token_is_an_auth_error() {
        let server = FakeImap::start().await;

        let error = connect(&token_settings(server.port), "expired")
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind, ErrorKind::Auth);
    }

    #[tokio::test]
    async fn unreachable_server_is_a_connect_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let error = connect(&settings(port), "secret").await.err().unwrap();
        assert_eq!(error.kind, ErrorKind::Connect);
    }

    #[tokio::test]
    async fn lists_folders_with_roles_names_and_counts() {
        let server = FakeImap::start().await;
        let mut session = connect(&settings(server.port), "secret").await.unwrap();

        let folders = session.list_folders().await.unwrap();
        let find = |path: &str| folders.iter().find(|folder| folder.path == path).unwrap();

        let inbox = find("INBOX");
        assert_eq!(inbox.role, FolderRole::Inbox);
        assert_eq!((inbox.total, inbox.unread), (2, 1));

        let sent = find("[Gmail]/Sent Mail");
        assert_eq!(sent.role, FolderRole::Sent);
        assert_eq!(sent.name, "Sent Mail");
        assert_eq!(sent.depth, 1);

        assert!(!find("[Gmail]").selectable);
        assert_eq!(find("Spam").role, FolderRole::Junk);
        assert_eq!(find("&BB8EQAQ4BDIENQRC-").name, "Привет");
    }

    #[tokio::test]
    async fn reads_folder_state_headers_and_bodies() {
        let server = FakeImap::start().await;
        let mut session = connect(&settings(server.port), "secret").await.unwrap();

        let state = session.folder_state("INBOX").await.unwrap();
        assert_eq!(state.uid_validity, 42);
        assert_eq!(state.uids, vec![5, 9]);
        assert_eq!(state.unread, 1);

        let mut headers = session.fetch_headers("INBOX", &[5, 9]).await.unwrap();
        headers.sort_by_key(|message| message.uid);
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].uid, 5);
        assert!(headers[0].flags.seen);
        assert_eq!(headers[0].size, 120);
        assert_eq!(headers[0].received, 1_700_000_000);
        assert!(headers[0].header.starts_with(b"Subject: First"));
        assert!(!headers[1].flags.seen);
        assert!(headers[1].flags.flagged);

        let flags = session.fetch_flags("INBOX", &[5]).await.unwrap();
        assert_eq!(flags.len(), 1);
        assert!(flags[0].1.seen);

        let raw = session.fetch_raw("INBOX", 5).await.unwrap();
        assert_eq!(raw, b"Subject: First\r\n\r\nBody of the first\r\n");
    }

    #[tokio::test]
    async fn idle_returns_when_the_server_announces_a_message() {
        let server = FakeImap::start().await;
        let mut session = connect(&settings(server.port), "secret").await.unwrap();

        // A long limit: only the server's announcement can end this in time.
        tokio::time::timeout(
            Duration::from_secs(10),
            session.wait_for_change("INBOX", Duration::from_secs(600)),
        )
        .await
        .unwrap()
        .unwrap();

        let commands = server.commands();
        assert!(commands.iter().any(|sent| sent == "IDLE"));
        assert!(commands.iter().any(|sent| sent == "DONE"));
        // The connection is usable again afterwards.
        assert!(session.is_alive().await);
        session.fetch_flags("INBOX", &[5]).await.unwrap();
    }

    #[tokio::test]
    async fn mail_told_in_another_answer_ends_the_wait_without_idling() {
        let server = FakeImap::start().await;
        let mut session = connect(&settings(server.port), "secret").await.unwrap();
        session.select("INBOX").await.unwrap();
        server.deliver();

        tokio::time::timeout(
            Duration::from_secs(10),
            session.wait_for_change("INBOX", Duration::from_secs(600)),
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!server.commands().iter().any(|sent| sent == "IDLE"));
    }

    #[tokio::test]
    async fn asks_only_for_messages_above_the_newest_known() {
        let server = FakeImap::start().await;
        let mut session = connect(&settings(server.port), "secret").await.unwrap();

        let new = session.fetch_new("INBOX", 5).await.unwrap();

        assert_eq!(
            new.iter().map(|message| message.uid).collect::<Vec<_>>(),
            vec![9]
        );
        assert!(new[0].header.starts_with(b"Subject: Second"));
        let commands = server.commands();
        assert!(
            commands
                .iter()
                .any(|sent| sent.starts_with("UID FETCH 6:* (UID FLAGS")),
            "{commands:#?}"
        );
    }

    #[tokio::test]
    async fn sends_the_right_commands_for_changes() {
        let server = FakeImap::start().await;
        let mut session = connect(&settings(server.port), "secret").await.unwrap();

        session
            .set_flag("INBOX", &[5, 9], Flag::Seen, true)
            .await
            .unwrap();
        session
            .set_flag("INBOX", &[9], Flag::Flagged, false)
            .await
            .unwrap();
        session.move_messages("INBOX", &[5], "Trash").await.unwrap();
        session.delete_messages("INBOX", &[9]).await.unwrap();
        session
            .append("Sent", b"Subject: Hi\r\n\r\nHello\r\n", true)
            .await
            .unwrap();
        assert!(session.is_alive().await);

        let commands = server.commands();
        let expected = [
            "UID STORE 5,9 +FLAGS.SILENT (\\Seen)",
            "UID STORE 9 -FLAGS.SILENT (\\Flagged)",
            "UID MOVE 5 \"Trash\"",
            "UID STORE 9 +FLAGS.SILENT (\\Deleted)",
            "UID EXPUNGE 9",
            "APPEND \"Sent\" (\\Seen) {22}",
            "NOOP",
        ];
        for command in expected {
            assert!(
                commands.iter().any(|sent| sent == command),
                "missing {command:?} in {commands:#?}"
            );
        }
        assert_eq!(server.appended(), b"Subject: Hi\r\n\r\nHello\r\n");
    }
}
