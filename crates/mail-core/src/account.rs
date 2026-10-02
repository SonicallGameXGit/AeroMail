use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::address::is_valid_host;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Protocol {
    Imap,
    Pop3,
    Smtp,
}

impl Protocol {
    pub fn is_incoming(self) -> bool {
        matches!(self, Protocol::Imap | Protocol::Pop3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Security {
    /// TLS from the first byte (IMAPS, POP3S, SMTPS).
    Tls,
    /// Plain connection upgraded with STARTTLS / STLS before authenticating.
    StartTls,
    /// No encryption. Only ever used on an explicit user decision.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AuthMethod {
    Password,
    OAuth2,
}

/// The standard port for a protocol / security pair. Used to pre-fill the
/// manual form; the user can always type something else.
pub fn default_port(protocol: Protocol, security: Security) -> u16 {
    match (protocol, security) {
        (Protocol::Imap, Security::Tls) => 993,
        (Protocol::Imap, _) => 143,
        (Protocol::Pop3, Security::Tls) => 995,
        (Protocol::Pop3, _) => 110,
        (Protocol::Smtp, Security::Tls) => 465,
        (Protocol::Smtp, Security::StartTls) => 587,
        (Protocol::Smtp, Security::None) => 25,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerSettings {
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub auth: AuthMethod,
    pub username: String,
}

/// Everything needed to connect one account, without secrets. Passwords and
/// tokens live in the OS keychain and never travel inside this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountSettings {
    pub email: String,
    pub display_name: String,
    pub incoming: ServerSettings,
    pub outgoing: ServerSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ServerRole {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum IssueKind {
    EmptyHost,
    InvalidHost,
    InvalidPort,
    EmptyUsername,
    /// Incoming server set to SMTP, or outgoing server set to IMAP / POP3.
    WrongProtocol,
    /// The two servers sign in differently, but an account has one secret.
    MixedAuth,
    /// Not an error: the connection is unencrypted and the user must confirm it.
    Unencrypted,
}

impl IssueKind {
    pub fn is_blocking(self) -> bool {
        !matches!(self, IssueKind::Unencrypted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsIssue {
    pub server: ServerRole,
    pub kind: IssueKind,
    pub blocking: bool,
}

/// Checks settings for problems that can be found without touching the network.
pub fn validate(settings: &AccountSettings) -> Vec<SettingsIssue> {
    let servers = [
        (ServerRole::Incoming, &settings.incoming),
        (ServerRole::Outgoing, &settings.outgoing),
    ];

    let mut issues = Vec::new();
    for (role, server) in servers {
        for kind in server_issues(role, server) {
            issues.push(SettingsIssue {
                server: role,
                kind,
                blocking: kind.is_blocking(),
            });
        }
    }

    if settings.incoming.auth != settings.outgoing.auth {
        let kind = IssueKind::MixedAuth;
        issues.push(SettingsIssue {
            server: ServerRole::Outgoing,
            kind,
            blocking: kind.is_blocking(),
        });
    }

    issues
}

fn server_issues(role: ServerRole, server: &ServerSettings) -> Vec<IssueKind> {
    let mut kinds = Vec::new();

    let host = server.host.trim();
    if host.is_empty() {
        kinds.push(IssueKind::EmptyHost);
    } else if !is_valid_host(host) {
        kinds.push(IssueKind::InvalidHost);
    }

    if server.port == 0 {
        kinds.push(IssueKind::InvalidPort);
    }

    if server.username.trim().is_empty() {
        kinds.push(IssueKind::EmptyUsername);
    }

    let expects_incoming = role == ServerRole::Incoming;
    if server.protocol.is_incoming() != expects_incoming {
        kinds.push(IssueKind::WrongProtocol);
    }

    if server.security == Security::None {
        kinds.push(IssueKind::Unencrypted);
    }

    kinds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(protocol: Protocol, host: &str, port: u16, security: Security) -> ServerSettings {
        ServerSettings {
            protocol,
            host: host.to_string(),
            port,
            security,
            auth: AuthMethod::Password,
            username: "john@example.com".to_string(),
        }
    }

    fn valid_settings() -> AccountSettings {
        AccountSettings {
            email: "john@example.com".to_string(),
            display_name: "John".to_string(),
            incoming: server(Protocol::Imap, "imap.example.com", 993, Security::Tls),
            outgoing: server(Protocol::Smtp, "smtp.example.com", 465, Security::Tls),
        }
    }

    fn kinds(settings: &AccountSettings, role: ServerRole) -> Vec<IssueKind> {
        validate(settings)
            .into_iter()
            .filter(|issue| issue.server == role)
            .map(|issue| issue.kind)
            .collect()
    }

    #[test]
    fn default_ports() {
        assert_eq!(default_port(Protocol::Imap, Security::Tls), 993);
        assert_eq!(default_port(Protocol::Imap, Security::StartTls), 143);
        assert_eq!(default_port(Protocol::Pop3, Security::Tls), 995);
        assert_eq!(default_port(Protocol::Pop3, Security::None), 110);
        assert_eq!(default_port(Protocol::Smtp, Security::Tls), 465);
        assert_eq!(default_port(Protocol::Smtp, Security::StartTls), 587);
        assert_eq!(default_port(Protocol::Smtp, Security::None), 25);
    }

    #[test]
    fn valid_settings_have_no_issues() {
        assert_eq!(validate(&valid_settings()), Vec::new());
    }

    #[test]
    fn reports_host_port_and_username() {
        let mut settings = valid_settings();
        settings.incoming.host = "  ".to_string();
        settings.incoming.port = 0;
        settings.incoming.username = String::new();
        settings.outgoing.host = "smtp.example.com:465".to_string();

        assert_eq!(
            kinds(&settings, ServerRole::Incoming),
            vec![
                IssueKind::EmptyHost,
                IssueKind::InvalidPort,
                IssueKind::EmptyUsername,
            ],
        );
        assert_eq!(
            kinds(&settings, ServerRole::Outgoing),
            vec![IssueKind::InvalidHost],
        );
    }

    #[test]
    fn reports_protocol_on_the_wrong_side() {
        let mut settings = valid_settings();
        settings.incoming.protocol = Protocol::Smtp;
        settings.outgoing.protocol = Protocol::Pop3;

        assert_eq!(
            kinds(&settings, ServerRole::Incoming),
            vec![IssueKind::WrongProtocol],
        );
        assert_eq!(
            kinds(&settings, ServerRole::Outgoing),
            vec![IssueKind::WrongProtocol],
        );
    }

    #[test]
    fn reports_servers_that_sign_in_differently() {
        let mut settings = valid_settings();
        settings.incoming.auth = AuthMethod::OAuth2;

        assert_eq!(kinds(&settings, ServerRole::Incoming), Vec::new());
        assert_eq!(
            kinds(&settings, ServerRole::Outgoing),
            vec![IssueKind::MixedAuth],
        );

        settings.outgoing.auth = AuthMethod::OAuth2;
        assert_eq!(validate(&settings), Vec::new());
    }

    #[test]
    fn unencrypted_is_a_warning_not_a_blocker() {
        let mut settings = valid_settings();
        settings.incoming.security = Security::None;

        let issues = validate(&settings);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, IssueKind::Unencrypted);
        assert!(!issues[0].blocking);
    }

    #[test]
    fn serialises_in_the_shape_the_ui_expects() {
        let json = serde_json::to_value(valid_settings()).unwrap();
        assert_eq!(json["displayName"], "John");
        assert_eq!(json["incoming"]["protocol"], "Imap");
        assert_eq!(json["outgoing"]["security"], "Tls");
    }
}
