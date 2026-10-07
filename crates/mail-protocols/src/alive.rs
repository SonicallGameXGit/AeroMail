//! Spares the liveness check of a connection that has just answered.
//!
//! The engine asks `is_alive` before every operation. On a real server that
//! is a round-trip of its own (well over 100 ms on Gmail), paid before each
//! click. A connection that answered a moment ago is taken as alive instead;
//! one that failed is checked again at once, so a dead one is replaced on
//! the next operation.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use mail_core::error::MailResult;
use mail_core::mail::MessageFlags;
use mail_core::transport::{Flag, FolderState, IncomingSession, RemoteFolder, RemoteMessage};

/// How long after an answer the connection is not asked again. Servers and
/// routers drop idle connections after minutes, not seconds.
pub(crate) const TRUST: Duration = Duration::from_secs(30);

pub(crate) struct Trusted<S> {
    session: S,
    trust: Duration,
    /// When the server last answered; `None` after a failure.
    answered: Option<Instant>,
}

impl<S: IncomingSession> Trusted<S> {
    /// Wraps a session that has just signed in.
    pub(crate) fn new(session: S, trust: Duration) -> Self {
        Self {
            session,
            trust,
            answered: Some(Instant::now()),
        }
    }

    fn noted<T>(&mut self, result: MailResult<T>) -> MailResult<T> {
        self.answered = result.is_ok().then(Instant::now);

        result
    }
}

#[async_trait]
impl<S: IncomingSession> IncomingSession for Trusted<S> {
    async fn is_alive(&mut self) -> bool {
        if self
            .answered
            .is_some_and(|answered| answered.elapsed() < self.trust)
        {
            return true;
        }

        let alive = self.session.is_alive().await;
        self.answered = alive.then(Instant::now);

        alive
    }

    async fn list_folders(&mut self) -> MailResult<Vec<RemoteFolder>> {
        let result = self.session.list_folders().await;
        self.noted(result)
    }

    async fn folder_state(&mut self, folder: &str) -> MailResult<FolderState> {
        let result = self.session.folder_state(folder).await;
        self.noted(result)
    }

    async fn fetch_headers(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<RemoteMessage>> {
        let result = self.session.fetch_headers(folder, uids).await;
        self.noted(result)
    }

    async fn fetch_flags(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<(u32, MessageFlags)>> {
        let result = self.session.fetch_flags(folder, uids).await;
        self.noted(result)
    }

    async fn fetch_raw(&mut self, folder: &str, uid: u32) -> MailResult<Vec<u8>> {
        let result = self.session.fetch_raw(folder, uid).await;
        self.noted(result)
    }

    async fn set_flag(
        &mut self,
        folder: &str,
        uids: &[u32],
        flag: Flag,
        on: bool,
    ) -> MailResult<()> {
        let result = self.session.set_flag(folder, uids, flag, on).await;
        self.noted(result)
    }

    async fn move_messages(&mut self, folder: &str, uids: &[u32], target: &str) -> MailResult<()> {
        let result = self.session.move_messages(folder, uids, target).await;
        self.noted(result)
    }

    async fn delete_messages(&mut self, folder: &str, uids: &[u32]) -> MailResult<()> {
        let result = self.session.delete_messages(folder, uids).await;
        self.noted(result)
    }

    async fn append(&mut self, folder: &str, raw: &[u8], seen: bool) -> MailResult<()> {
        let result = self.session.append(folder, raw, seen).await;
        self.noted(result)
    }
}

#[cfg(test)]
mod tests {
    use mail_core::account::{AuthMethod, Protocol, Security, ServerSettings};

    use super::*;
    use crate::fake_server::FakeImap;
    use crate::imap;

    async fn session(server: &FakeImap, trust: Duration) -> Trusted<imap::ImapSession> {
        let settings = ServerSettings {
            protocol: Protocol::Imap,
            host: "127.0.0.1".to_string(),
            port: server.port,
            security: Security::None,
            auth: AuthMethod::Password,
            username: "me@example.com".to_string(),
        };

        Trusted::new(imap::connect(&settings, "secret").await.unwrap(), trust)
    }

    fn checks(server: &FakeImap) -> usize {
        server
            .commands()
            .iter()
            .filter(|sent| *sent == "NOOP")
            .count()
    }

    #[tokio::test]
    async fn a_connection_that_just_answered_is_not_asked_again() {
        let server = FakeImap::start().await;
        let mut session = session(&server, TRUST).await;

        assert!(session.is_alive().await);
        session.fetch_flags("INBOX", &[5]).await.unwrap();
        assert!(session.is_alive().await);

        assert_eq!(checks(&server), 0);
    }

    #[tokio::test]
    async fn a_connection_that_was_quiet_for_too_long_is_asked() {
        let server = FakeImap::start().await;
        let mut session = session(&server, Duration::ZERO).await;

        assert!(session.is_alive().await);
        assert!(session.is_alive().await);

        assert_eq!(checks(&server), 2);
    }

    #[tokio::test]
    async fn a_connection_that_failed_is_asked_at_once() {
        let server = FakeImap::start().await;
        let mut session = session(&server, TRUST).await;

        assert!(session.fetch_flags("Missing", &[5]).await.is_err());
        assert!(session.is_alive().await);
        assert_eq!(checks(&server), 1);

        // That answer counts again.
        assert!(session.is_alive().await);
        assert_eq!(checks(&server), 1);
    }
}
