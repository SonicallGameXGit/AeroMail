use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

use crate::account::ServerRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ErrorKind {
    /// The server could not be reached (DNS, refused, timeout).
    Connect,
    /// The encrypted connection could not be established.
    Tls,
    /// The server rejected the username or password.
    Auth,
    /// The settings ask for something the app cannot do yet.
    Unsupported,
    /// The server answered in an unexpected way.
    Protocol,
    /// The settings are not valid.
    Invalid,
    /// The OS keychain is unavailable or refused the request.
    Keychain,
    /// The local database failed.
    Storage,
    /// The account, folder or message no longer exists.
    NotFound,
}

/// The one error type that crosses crate boundaries and reaches the UI.
///
/// `detail` is technical text for the "details" line; the UI words the main
/// message itself from `kind` and `server`.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, TS)]
#[error("{kind:?}: {detail}")]
#[ts(export)]
pub struct MailError {
    pub kind: ErrorKind,
    /// Which side failed, when the error came from a server.
    pub server: Option<ServerRole>,
    pub detail: String,
}

impl MailError {
    pub fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            server: None,
            detail: detail.into(),
        }
    }

    pub fn on(mut self, server: ServerRole) -> Self {
        self.server = Some(server);
        self
    }

    pub fn not_found(what: &str) -> Self {
        Self::new(ErrorKind::NotFound, format!("{what} not found"))
    }
}

pub type MailResult<T> = Result<T, MailError>;
