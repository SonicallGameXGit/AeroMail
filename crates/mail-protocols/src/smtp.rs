use lettre::address::Envelope as SmtpEnvelope;
use lettre::transport::smtp::Error as SmtpError;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use mail_core::account::{AuthMethod, Security, ServerSettings};
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::transport::Envelope;

use crate::tls::TIMEOUT;

fn smtp_error(error: SmtpError, auth: AuthMethod) -> MailError {
    // A refused token is answered with a question (334) before the 535, and
    // lettre gives up at the question with an error of its own. Nothing else
    // of that kind can happen once the envelope is built.
    let rejected_token = auth == AuthMethod::OAuth2 && error.is_client();
    let rejected_login = rejected_token
        || error
            .status()
            .is_some_and(|code| matches!(code.to_string().as_str(), "535" | "534" | "530"));

    let kind = if rejected_login {
        ErrorKind::Auth
    } else if error.is_tls() {
        ErrorKind::Tls
    } else if error.is_response() || error.is_permanent() || error.is_transient() {
        ErrorKind::Protocol
    } else {
        ErrorKind::Connect
    };

    // lettre keeps the server's own words in the source of the error.
    let mut detail = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(inner) = source {
        detail = format!("{detail}: {inner}");
        source = inner.source();
    }

    MailError::new(kind, detail)
}

fn transport(
    server: &ServerSettings,
    secret: &str,
) -> MailResult<AsyncSmtpTransport<Tokio1Executor>> {
    let host = server.host.trim().to_string();
    let parameters = || {
        TlsParameters::new(host.clone())
            .map_err(|error| MailError::new(ErrorKind::Tls, error.to_string()))
    };
    let tls = match server.security {
        Security::Tls => Tls::Wrapper(parameters()?),
        Security::StartTls => Tls::Required(parameters()?),
        Security::None => Tls::None,
    };

    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host.clone())
        .port(server.port)
        .tls(tls)
        .credentials(Credentials::new(
            server.username.clone(),
            secret.to_string(),
        ))
        .timeout(Some(TIMEOUT));
    if server.auth == AuthMethod::OAuth2 {
        builder = builder.authentication(vec![Mechanism::Xoauth2]);
    }

    Ok(builder.build())
}

pub(crate) async fn check(server: &ServerSettings, secret: &str) -> MailResult<()> {
    let reachable = transport(server, secret)?
        .test_connection()
        .await
        .map_err(|error| smtp_error(error, server.auth))?;
    if !reachable {
        return Err(MailError::new(
            ErrorKind::Connect,
            "the server did not answer",
        ));
    }

    Ok(())
}

fn address(text: &str) -> MailResult<Address> {
    text.trim()
        .parse()
        .map_err(|_| MailError::new(ErrorKind::Invalid, format!("{text} is not a valid address")))
}

pub(crate) async fn send(
    server: &ServerSettings,
    secret: &str,
    envelope: Envelope<'_>,
    raw: &[u8],
) -> MailResult<()> {
    let recipients = envelope
        .recipients
        .iter()
        .map(|recipient| address(recipient))
        .collect::<MailResult<Vec<_>>>()?;
    let envelope = SmtpEnvelope::new(Some(address(envelope.from)?), recipients)
        .map_err(|error| MailError::new(ErrorKind::Invalid, error.to_string()))?;

    transport(server, secret)?
        .send_raw(&envelope, raw)
        .await
        .map_err(|error| smtp_error(error, server.auth))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use mail_core::account::Protocol;

    use super::*;
    use crate::fake_server::FakeSmtp;

    fn settings(port: u16) -> ServerSettings {
        ServerSettings {
            protocol: Protocol::Smtp,
            host: "127.0.0.1".to_string(),
            port,
            security: Security::None,
            auth: AuthMethod::Password,
            username: "me@example.com".to_string(),
        }
    }

    #[tokio::test]
    async fn check_signs_in() {
        let server = FakeSmtp::start().await;

        check(&settings(server.port), "secret").await.unwrap();
    }

    #[tokio::test]
    async fn wrong_password_is_an_auth_error() {
        let server = FakeSmtp::start().await;

        let error = check(&settings(server.port), "wrong").await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Auth);
    }

    fn token_settings(port: u16) -> ServerSettings {
        ServerSettings {
            auth: AuthMethod::OAuth2,
            ..settings(port)
        }
    }

    #[tokio::test]
    async fn check_signs_in_with_an_access_token() {
        let server = FakeSmtp::start().await;

        check(&token_settings(server.port), "token").await.unwrap();
    }

    #[tokio::test]
    async fn a_refused_access_token_is_an_auth_error() {
        let server = FakeSmtp::start().await;

        let error = check(&token_settings(server.port), "expired")
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Auth, "{error}");
    }

    #[tokio::test]
    async fn unreachable_server_is_a_connect_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let error = check(&settings(port), "secret").await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Connect);
    }

    #[tokio::test]
    async fn sends_to_every_recipient_of_the_envelope() {
        let server = FakeSmtp::start().await;
        let recipients = [
            "ann@example.com".to_string(),
            "hidden@example.com".to_string(),
        ];
        let envelope = Envelope {
            from: "me@example.com",
            recipients: &recipients,
        };

        send(
            &settings(server.port),
            "secret",
            envelope,
            b"Subject: Hi\r\n\r\nHello\r\n",
        )
        .await
        .unwrap();

        let received = server.received();
        assert_eq!(received.from, "<me@example.com>");
        assert_eq!(
            received.recipients,
            vec!["<ann@example.com>", "<hidden@example.com>"]
        );
        assert!(received.data.contains("Subject: Hi"));
        assert!(received.data.contains("Hello"));
    }

    #[tokio::test]
    async fn rejects_an_invalid_recipient_before_connecting() {
        let recipients = ["not an address".to_string()];
        let envelope = Envelope {
            from: "me@example.com",
            recipients: &recipients,
        };

        let error = send(&settings(1), "secret", envelope, b"x")
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Invalid);
    }
}
