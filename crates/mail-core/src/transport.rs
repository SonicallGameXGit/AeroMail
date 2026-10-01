//! Traits the engine uses to reach mail servers and the OS keychain. Real
//! implementations live in other crates; tests use in-memory fakes.

use async_trait::async_trait;

use crate::account::ServerSettings;
use crate::error::MailResult;
use crate::mail::{FolderRole, MessageFlags};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFolder {
    pub path: String,
    pub name: String,
    pub depth: u32,
    pub role: FolderRole,
    pub selectable: bool,
    pub unread: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderState {
    /// Changes when the server renumbers the folder: cached UIDs are then void.
    pub uid_validity: u32,
    /// Every message in the folder, ascending.
    pub uids: Vec<u32>,
    pub unread: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteMessage {
    pub uid: u32,
    pub flags: MessageFlags,
    pub size: u32,
    /// Unix time in seconds when the server received the message.
    pub received: i64,
    /// The raw header block.
    pub header: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Seen,
    Flagged,
    Answered,
}

/// An authenticated connection to an incoming mail server.
#[async_trait]
pub trait IncomingSession: Send {
    /// Cheap round-trip that tells whether the connection is still usable.
    async fn is_alive(&mut self) -> bool;
    async fn list_folders(&mut self) -> MailResult<Vec<RemoteFolder>>;
    async fn folder_state(&mut self, folder: &str) -> MailResult<FolderState>;
    async fn fetch_headers(&mut self, folder: &str, uids: &[u32])
    -> MailResult<Vec<RemoteMessage>>;
    async fn fetch_flags(
        &mut self,
        folder: &str,
        uids: &[u32],
    ) -> MailResult<Vec<(u32, MessageFlags)>>;
    /// The complete raw message. Must not mark it as read.
    async fn fetch_raw(&mut self, folder: &str, uid: u32) -> MailResult<Vec<u8>>;
    async fn set_flag(
        &mut self,
        folder: &str,
        uids: &[u32],
        flag: Flag,
        on: bool,
    ) -> MailResult<()>;
    async fn move_messages(&mut self, folder: &str, uids: &[u32], target: &str) -> MailResult<()>;
    /// Removes the messages for good.
    async fn delete_messages(&mut self, folder: &str, uids: &[u32]) -> MailResult<()>;
    async fn append(&mut self, folder: &str, raw: &[u8], seen: bool) -> MailResult<()>;
}

pub struct Envelope<'a> {
    pub from: &'a str,
    pub recipients: &'a [String],
}

#[async_trait]
pub trait Connector: Send + Sync {
    async fn connect_incoming(
        &self,
        server: &ServerSettings,
        password: &str,
    ) -> MailResult<Box<dyn IncomingSession>>;

    /// Connects and signs in to the outgoing server without sending anything.
    async fn check_outgoing(&self, server: &ServerSettings, password: &str) -> MailResult<()>;

    async fn send(
        &self,
        server: &ServerSettings,
        password: &str,
        envelope: Envelope<'_>,
        raw: &[u8],
    ) -> MailResult<()>;
}

/// Where passwords live. Implemented on top of the OS keychain.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> MailResult<Option<String>>;
    fn set(&self, key: &str, secret: &str) -> MailResult<()>;
    fn delete(&self, key: &str) -> MailResult<()>;
}
