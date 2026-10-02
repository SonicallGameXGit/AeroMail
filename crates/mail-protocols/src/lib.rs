//! Real network implementations of the core transport traits: IMAP for
//! incoming mail, SMTP for outgoing, DNS and HTTPS for sender pictures, the
//! provider's sign-in page for OAuth2.

mod imap;
mod logos;
mod oauth;
mod smtp;
mod tls;

#[cfg(test)]
mod fake_server;

use async_trait::async_trait;
use mail_core::account::{Protocol, ServerRole, ServerSettings};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::transport::{Connector, Envelope, IncomingSession};

pub use logos::NetLogos;
pub use oauth::NetSignIn;

/// Connects to real servers over the network.
#[derive(Default)]
pub struct NetConnector;

#[async_trait]
impl Connector for NetConnector {
    async fn connect_incoming(
        &self,
        server: &ServerSettings,
        secret: &str,
    ) -> MailResult<Box<dyn IncomingSession>> {
        match server.protocol {
            Protocol::Imap => {
                let session = imap::connect(server, secret)
                    .await
                    .map_err(|error| error.on(ServerRole::Incoming))?;

                Ok(Box::new(session))
            }
            Protocol::Pop3 => Err(MailError::new(
                ErrorKind::Unsupported,
                "POP3 accounts are not available yet",
            )
            .on(ServerRole::Incoming)),
            Protocol::Smtp => Err(MailError::new(
                ErrorKind::Invalid,
                "SMTP cannot be used for incoming mail",
            )
            .on(ServerRole::Incoming)),
        }
    }

    async fn check_outgoing(&self, server: &ServerSettings, secret: &str) -> MailResult<()> {
        smtp::check(server, secret)
            .await
            .map_err(|error| error.on(ServerRole::Outgoing))
    }

    async fn send(
        &self,
        server: &ServerSettings,
        secret: &str,
        envelope: Envelope<'_>,
        raw: &[u8],
    ) -> MailResult<()> {
        smtp::send(server, secret, envelope, raw)
            .await
            .map_err(|error| error.on(ServerRole::Outgoing))
    }
}

/// Checks against real servers: they prove the TLS and STARTTLS paths that
/// the local fakes cannot. Run with `cargo test -p mail-protocols -- --ignored`.
#[cfg(test)]
mod live_tests {
    use mail_core::account::{AuthMethod, Security};

    use super::*;

    fn server(protocol: Protocol, host: &str, port: u16, security: Security) -> ServerSettings {
        ServerSettings {
            protocol,
            host: host.to_string(),
            port,
            security,
            auth: AuthMethod::Password,
            username: "mailclient-test@invalid.invalid".to_string(),
        }
    }

    async fn incoming_error(host: &str, port: u16, security: Security) -> MailError {
        NetConnector
            .connect_incoming(
                &server(Protocol::Imap, host, port, security),
                "not-a-real-password",
            )
            .await
            .err()
            .unwrap()
    }

    async fn outgoing_error(host: &str, port: u16, security: Security) -> MailError {
        NetConnector
            .check_outgoing(
                &server(Protocol::Smtp, host, port, security),
                "not-a-real-password",
            )
            .await
            .unwrap_err()
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn imap_over_tls_reaches_the_login_step() {
        let error = incoming_error("imap.gmail.com", 993, Security::Tls).await;
        assert_eq!(error.kind, ErrorKind::Auth, "{error}");
        assert_eq!(error.server, Some(ServerRole::Incoming));
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn a_made_up_access_token_is_refused_by_both_servers() {
        let token = |protocol, host: &str, port| ServerSettings {
            auth: AuthMethod::OAuth2,
            ..server(protocol, host, port, Security::Tls)
        };

        let incoming = NetConnector
            .connect_incoming(&token(Protocol::Imap, "imap.gmail.com", 993), "made-up")
            .await
            .err()
            .unwrap();
        assert_eq!(incoming.kind, ErrorKind::Auth, "{incoming}");

        let outgoing = NetConnector
            .check_outgoing(&token(Protocol::Smtp, "smtp.gmail.com", 465), "made-up")
            .await
            .unwrap_err();
        assert_eq!(outgoing.kind, ErrorKind::Auth, "{outgoing}");
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn imap_with_starttls_reaches_the_login_step() {
        let error = incoming_error("imap.gmx.com", 143, Security::StartTls).await;
        assert_eq!(error.kind, ErrorKind::Auth, "{error}");
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn smtp_over_tls_reaches_the_login_step() {
        let error = outgoing_error("smtp.gmail.com", 465, Security::Tls).await;
        assert_eq!(error.kind, ErrorKind::Auth, "{error}");
        assert_eq!(error.server, Some(ServerRole::Outgoing));
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn smtp_with_starttls_reaches_the_login_step() {
        let error = outgoing_error("smtp.gmail.com", 587, Security::StartTls).await;
        assert_eq!(error.kind, ErrorKind::Auth, "{error}");
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn a_wrong_certificate_name_is_a_tls_error() {
        // This host presents a certificate issued for another name.
        let error = incoming_error("wrong.host.badssl.com", 443, Security::Tls).await;
        assert_eq!(error.kind, ErrorKind::Tls, "{error}");
    }
}
