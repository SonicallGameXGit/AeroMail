use std::sync::Arc;
use std::time::Duration;

use mail_core::error::{ErrorKind, MailError, MailResult};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

pub(crate) const TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) async fn tcp(host: &str, port: u16) -> MailResult<TcpStream> {
    let connecting = TcpStream::connect((host, port));
    match tokio::time::timeout(TIMEOUT, connecting).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(error)) => Err(MailError::new(
            ErrorKind::Connect,
            format!("{host}:{port}: {error}"),
        )),
        Err(_) => Err(MailError::new(
            ErrorKind::Connect,
            format!("{host}:{port}: no answer"),
        )),
    }
}

fn tls_error(detail: impl ToString) -> MailError {
    MailError::new(ErrorKind::Tls, detail.to_string())
}

/// Upgrades a connection to TLS, verifying the certificate against the
/// bundled Mozilla root certificates.
pub(crate) async fn wrap(stream: TcpStream, host: &str) -> MailResult<TlsStream<TcpStream>> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(tls_error)?
        .with_root_certificates(roots)
        .with_no_client_auth();

    let name = ServerName::try_from(host.to_string()).map_err(tls_error)?;
    let connecting = TlsConnector::from(Arc::new(config)).connect(name, stream);
    match tokio::time::timeout(TIMEOUT, connecting).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(error)) => Err(tls_error(error)),
        Err(_) => Err(tls_error("the TLS handshake got no answer")),
    }
}
