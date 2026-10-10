//! Signing in on the provider's own page (OAuth2 for an installed app): the
//! user's browser opens the page, the provider sends the browser back to a
//! local address this file listens on, and the code it brings is traded for
//! tokens. Which providers exist is decided by `mail_core::oauth`.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::oauth::{OAuthProvider, Tokens, provider_for};
use mail_core::transport::{PendingSignIn, SignInFlow};
use reqwest::Url;
use ring::digest::{SHA256, digest};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::tls;

/// How long the user has to finish on the provider's page.
const WAIT: Duration = Duration::from_secs(5 * 60);

/// Browsers open connections they never use; such a one is given up quickly.
const REQUEST_WAIT: Duration = Duration::from_secs(2);

const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// Signs in on the real internet.
#[derive(Default)]
pub struct NetSignIn;

fn sign_in(detail: impl Into<String>) -> MailError {
    MailError::new(ErrorKind::SignIn, detail)
}

fn unreachable(detail: impl ToString) -> MailError {
    MailError::new(ErrorKind::Connect, detail.to_string())
}

/// The provider behind a mail server, when this build can sign in there.
fn provider_of(host: &str) -> MailResult<OAuthProvider> {
    let Some(provider) = provider_for(host) else {
        return Err(MailError::new(
            ErrorKind::Unsupported,
            format!("no sign-in page is known for {host}"),
        ));
    };
    if provider.client_id().is_none() {
        return Err(MailError::new(
            ErrorKind::Unsupported,
            format!(
                "this build was made without a client id for {}",
                provider.name
            ),
        ));
    }

    Ok(*provider)
}

/// 32 random bytes as text that is safe inside an address.
fn random_text() -> MailResult<String> {
    let mut bytes = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| MailError::new(ErrorKind::Protocol, "the system gave no random numbers"))?;

    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

struct Pending {
    provider: OAuthProvider,
    listener: TcpListener,
    /// Where the provider sends the browser back to; asked for again when
    /// the code is traded in.
    redirect: String,
    /// Proves the answer belongs to this sign-in and not to another page.
    state: String,
    /// Proves the code is traded in by the one who asked for it (PKCE).
    verifier: String,
    url: String,
}

async fn begin(provider: OAuthProvider, email: &str) -> MailResult<Pending> {
    let client_id = provider.client_id().unwrap_or_default();

    // Only this machine can reach it, and the system picks a free port.
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(unreachable)?;
    let port = listener.local_addr().map_err(unreachable)?.port();
    let redirect = format!("http://127.0.0.1:{port}");

    let state = random_text()?;
    let verifier = random_text()?;
    let challenge = URL_SAFE_NO_PAD.encode(digest(&SHA256, verifier.as_bytes()));

    let mut parameters = vec![
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect.as_str()),
        ("scope", provider.scope),
        ("state", state.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("login_hint", email),
    ];
    parameters.extend_from_slice(provider.extra);
    let url = Url::parse_with_params(provider.authorize_url, parameters)
        .map_err(|error| MailError::new(ErrorKind::Protocol, error.to_string()))?
        .to_string();

    Ok(Pending {
        provider,
        listener,
        redirect,
        state,
        verifier,
        url,
    })
}

#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Code(String),
    /// The provider's own word for why there is no code.
    Refused(String),
    /// Not the provider's answer to this sign-in: an icon request, another
    /// page guessing the port.
    Unrelated,
}

/// What a request to the local address says, from its path and query.
fn read_answer(target: &str, state: &str) -> Answer {
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Answer::Unrelated;
    };
    let value = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };

    if value("state").as_deref() != Some(state) {
        return Answer::Unrelated;
    }
    if let Some(error) = value("error") {
        return Answer::Refused(error);
    }

    match value("code") {
        Some(code) if !code.is_empty() => Answer::Code(code),
        _ => Answer::Unrelated,
    }
}

/// The path and query of a `GET` request, read up to the end of its head.
async fn request_target(stream: &mut TcpStream) -> Option<String> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 || head.len() + read > MAX_REQUEST_BYTES {
            return None;
        }
        head.extend_from_slice(&chunk[..read]);
    }

    let head = String::from_utf8_lossy(&head);
    let mut words = head.lines().next()?.split(' ');
    if words.next()? != "GET" {
        return None;
    }

    words.next().map(str::to_string)
}

/// The page left in the browser. Plain on purpose: it is the one screen of
/// the app that is not drawn by the app.
/// Shown in the user's browser after the provider answers. It is drawn like the app's own
/// window: the design tokens are the app's `tokens.css` itself (so both themes and the
/// reduced-transparency rules come along), and the rest mirrors `Backdrop.svelte` and
/// `GlassWindow.svelte`. Change those and this page's `answer.html` together.
const ANSWER_PAGE: &str = include_str!("answer.html");
const DESIGN_TOKENS: &str = include_str!("../../../src/lib/design/tokens.css");

async fn respond(stream: &mut TcpStream, status: &str, title: &str, text: &str) {
    let body = ANSWER_PAGE
        .replace("/*tokens*/", DESIGN_TOKENS)
        .replace("{title}", title)
        .replace("{text}", text);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    // The browser may be gone already; the sign-in does not depend on it.
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

impl Pending {
    /// Waits for the browser to come back with the provider's answer.
    async fn code(&self) -> MailResult<String> {
        loop {
            let (mut stream, _) = self.listener.accept().await.map_err(unreachable)?;
            let target = tokio::time::timeout(REQUEST_WAIT, request_target(&mut stream)).await;
            let Ok(Some(target)) = target else {
                continue;
            };

            match read_answer(&target, &self.state) {
                Answer::Code(code) => {
                    respond(
                        &mut stream,
                        "200 OK",
                        "Signed in",
                        "You can close this tab and go back to MailClient.",
                    )
                    .await;

                    return Ok(code);
                }
                Answer::Refused(reason) => {
                    respond(
                        &mut stream,
                        "200 OK",
                        "Sign-in not completed",
                        "Nothing was changed. You can close this tab and try again in MailClient.",
                    )
                    .await;

                    return Err(sign_in(format!("the provider answered: {reason}")));
                }
                Answer::Unrelated => {
                    respond(
                        &mut stream,
                        "404 Not Found",
                        "Nothing here",
                        "This address only finishes a sign-in to MailClient.",
                    )
                    .await;
                }
            }
        }
    }
}

#[async_trait]
impl PendingSignIn for Pending {
    fn url(&self) -> &str {
        &self.url
    }

    async fn finish(self: Box<Self>) -> MailResult<Tokens> {
        let code = match tokio::time::timeout(WAIT, self.code()).await {
            Ok(code) => code?,
            Err(_) => return Err(sign_in("nobody signed in within five minutes")),
        };

        request_tokens(
            &self.provider,
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &self.redirect),
                ("code_verifier", &self.verifier),
            ],
        )
        .await
    }
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

/// One question to the provider's token address. Errors carry the
/// provider's own words, never a token.
async fn request_tokens(provider: &OAuthProvider, grant: &[(&str, &str)]) -> MailResult<Tokens> {
    let mut form = grant.to_vec();
    form.push(("client_id", provider.client_id().unwrap_or_default()));
    if let Some(secret) = provider.client_secret() {
        form.push(("client_secret", secret));
    }

    let client = reqwest::Client::builder()
        .tls_backend_preconfigured(tls::client_config()?)
        // The tests talk to a local stand-in without TLS.
        .https_only(!cfg!(test))
        .redirect(reqwest::redirect::Policy::none())
        .timeout(tls::TIMEOUT)
        .build()
        .map_err(unreachable)?;
    let response = client
        .post(provider.token_url)
        .form(&form)
        .send()
        .await
        .map_err(|error| unreachable(error.without_url()))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| unreachable(error.without_url()))?;

    let Ok(answer) = serde_json::from_slice::<TokenAnswer>(&body) else {
        return Err(MailError::new(
            ErrorKind::Protocol,
            format!("{} answered {status} without tokens", provider.name),
        ));
    };
    if let Some(error) = answer.error {
        let detail = match answer.error_description {
            Some(description) => format!("{error}: {description}"),
            None => error.clone(),
        };
        // The permission is gone (revoked, expired, already used): only the
        // user can give a new one. Anything else is the app's own fault.
        let kind = match error.as_str() {
            "invalid_grant" | "access_denied" => ErrorKind::SignIn,
            _ => ErrorKind::Protocol,
        };

        return Err(MailError::new(kind, detail));
    }
    let Some(access) = answer.access_token else {
        return Err(MailError::new(
            ErrorKind::Protocol,
            format!(
                "{} answered {status} without an access token",
                provider.name
            ),
        ));
    };

    Ok(Tokens {
        access,
        refresh: answer.refresh_token,
        // Unknown lifetime: treated as used up, so it is renewed every time.
        expires_in: answer.expires_in.unwrap_or(0),
    })
}

#[async_trait]
impl SignInFlow for NetSignIn {
    async fn begin(&self, host: &str, email: &str) -> MailResult<Box<dyn PendingSignIn>> {
        let pending = begin(provider_of(host)?, email).await?;

        Ok(Box::new(pending))
    }

    async fn refresh(&self, host: &str, refresh_token: &str) -> MailResult<Tokens> {
        request_tokens(
            &provider_of(host)?,
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ],
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncBufReadExt, BufReader};

    use super::*;

    /// A token address on a local socket. It hands out one fixed pair of
    /// tokens, refuses the refresh token "revoked", and remembers what it
    /// was asked.
    struct FakeTokens {
        url: &'static str,
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl FakeTokens {
        async fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let asked = Arc::new(Mutex::new(Vec::new()));

            let shared = Arc::clone(&asked);
            tokio::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    serve_tokens(stream, Arc::clone(&shared)).await;
                }
            });

            Self {
                url: Box::leak(format!("http://127.0.0.1:{port}/token").into_boxed_str()),
                asked,
            }
        }

        fn provider(&self) -> OAuthProvider {
            OAuthProvider {
                name: "Test",
                hosts: &[],
                authorize_url: "https://accounts.example.com/auth",
                token_url: self.url,
                scope: "mail",
                extra: &[("access_type", "offline")],
                client_id: Some("app-id"),
                client_secret: Some("app-secret"),
            }
        }

        /// The form of the last question, as name and value pairs.
        fn last_form(&self) -> Vec<(String, String)> {
            let asked = self.asked.lock().unwrap();
            let body = asked.last().unwrap();

            Url::parse(&format!("http://form/?{body}"))
                .unwrap()
                .query_pairs()
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect()
        }
    }

    async fn serve_tokens(stream: TcpStream, asked: Arc<Mutex<Vec<String>>>) {
        let mut lines = BufReader::new(stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            lines.read_line(&mut line).await.unwrap();
            let line = line.trim_end().to_lowercase();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        lines.read_exact(&mut body).await.unwrap();
        let body = String::from_utf8(body).unwrap();

        let (status, answer) = if body.contains("refresh_token=revoked") {
            (
                "400 Bad Request",
                r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#,
            )
        } else {
            (
                "200 OK",
                r#"{"access_token":"access-1","refresh_token":"refresh-1","expires_in":3599}"#,
            )
        };
        asked.lock().unwrap().push(body);

        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{answer}",
            answer.len()
        );
        lines
            .get_mut()
            .write_all(response.as_bytes())
            .await
            .unwrap();
    }

    fn parameter(url: &str, name: &str) -> String {
        Url::parse(url)
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
            .unwrap()
    }

    /// What the browser does when the provider sends it back.
    async fn visit(redirect: &str, target: &str) -> String {
        let address = redirect.trim_start_matches("http://");
        let mut stream = TcpStream::connect(address).await.unwrap();
        let request = format!("GET {target} HTTP/1.1\r\nHost: {address}\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        response
    }

    #[tokio::test]
    async fn a_sign_in_trades_the_browsers_answer_for_tokens() {
        let tokens = FakeTokens::start().await;
        let pending = begin(tokens.provider(), "me@example.com").await.unwrap();

        let url = pending.url().to_string();
        assert!(url.starts_with("https://accounts.example.com/auth?"));
        assert_eq!(parameter(&url, "response_type"), "code");
        assert_eq!(parameter(&url, "client_id"), "app-id");
        assert_eq!(parameter(&url, "scope"), "mail");
        assert_eq!(parameter(&url, "login_hint"), "me@example.com");
        assert_eq!(parameter(&url, "code_challenge_method"), "S256");
        assert_eq!(parameter(&url, "access_type"), "offline");
        let redirect = parameter(&url, "redirect_uri");
        assert!(redirect.starts_with("http://127.0.0.1:"));
        let state = parameter(&url, "state");

        let finishing = tokio::spawn(Box::new(pending).finish());

        // Neither an icon request nor a page that does not know the state
        // ends the wait.
        let icon = visit(&redirect, "/favicon.ico").await;
        assert!(icon.starts_with("HTTP/1.1 404"));
        let stranger = visit(&redirect, "/?state=guessed&code=stolen").await;
        assert!(stranger.starts_with("HTTP/1.1 404"));

        let page = visit(&redirect, &format!("/?state={state}&code=the-code")).await;
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(page.contains("Signed in"));

        let received = finishing.await.unwrap().unwrap();
        assert_eq!(
            received,
            Tokens {
                access: "access-1".to_string(),
                refresh: Some("refresh-1".to_string()),
                expires_in: 3599,
            }
        );

        let form = tokens.last_form();
        let sent = |name: &str| {
            form.iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
                .unwrap()
        };
        assert_eq!(sent("grant_type"), "authorization_code");
        assert_eq!(sent("code"), "the-code");
        assert_eq!(sent("redirect_uri"), redirect);
        assert_eq!(sent("client_id"), "app-id");
        assert_eq!(sent("client_secret"), "app-secret");
        // The verifier is the one the page was given the hash of.
        let hashed = URL_SAFE_NO_PAD.encode(digest(&SHA256, sent("code_verifier").as_bytes()));
        assert_eq!(hashed, parameter(&url, "code_challenge"));
    }

    #[tokio::test]
    async fn a_refusal_on_the_page_ends_the_sign_in() {
        let tokens = FakeTokens::start().await;
        let pending = begin(tokens.provider(), "me@example.com").await.unwrap();
        let redirect = parameter(pending.url(), "redirect_uri");
        let state = parameter(pending.url(), "state");

        let finishing = tokio::spawn(Box::new(pending).finish());
        let page = visit(&redirect, &format!("/?state={state}&error=access_denied")).await;
        assert!(page.contains("Sign-in not completed"));

        let error = finishing.await.unwrap().unwrap_err();
        assert_eq!(error.kind, ErrorKind::SignIn);
        assert!(error.detail.contains("access_denied"));
        assert!(tokens.asked.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn every_sign_in_has_its_own_state_and_verifier() {
        let tokens = FakeTokens::start().await;
        let first = begin(tokens.provider(), "me@example.com").await.unwrap();
        let second = begin(tokens.provider(), "me@example.com").await.unwrap();

        assert_ne!(first.state, second.state);
        assert_ne!(first.verifier, second.verifier);
        assert_ne!(first.state, first.verifier);
    }

    #[tokio::test]
    async fn a_refresh_token_gives_a_new_access_token() {
        let tokens = FakeTokens::start().await;

        let renewed = request_tokens(
            &tokens.provider(),
            &[("grant_type", "refresh_token"), ("refresh_token", "kept")],
        )
        .await
        .unwrap();

        assert_eq!(renewed.access, "access-1");
        let form = tokens.last_form();
        assert!(form.contains(&("grant_type".to_string(), "refresh_token".to_string())));
        assert!(form.contains(&("refresh_token".to_string(), "kept".to_string())));
    }

    #[tokio::test]
    async fn a_revoked_refresh_token_asks_for_a_new_sign_in() {
        let tokens = FakeTokens::start().await;

        let error = request_tokens(
            &tokens.provider(),
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", "revoked"),
            ],
        )
        .await
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::SignIn);
        assert!(error.detail.contains("expired or revoked"));
    }

    #[tokio::test]
    async fn a_server_without_a_known_sign_in_page_is_unsupported() {
        let error = NetSignIn
            .begin("imap.example.com", "me@example.com")
            .await
            .err()
            .unwrap();

        assert_eq!(error.kind, ErrorKind::Unsupported);
    }

    #[test]
    fn only_the_answer_to_this_sign_in_is_read() {
        let read = |target: &str| read_answer(target, "s1");

        assert_eq!(read("/?state=s1&code=c1"), Answer::Code("c1".to_string()));
        assert_eq!(
            read("/?code=c%2F1&state=s1"),
            Answer::Code("c/1".to_string())
        );
        assert_eq!(
            read("/?state=s1&error=access_denied"),
            Answer::Refused("access_denied".to_string())
        );
        assert_eq!(read("/?state=other&code=c1"), Answer::Unrelated);
        assert_eq!(read("/?code=c1"), Answer::Unrelated);
        assert_eq!(read("/?state=s1"), Answer::Unrelated);
        assert_eq!(read("/?state=s1&code="), Answer::Unrelated);
        assert_eq!(read("/favicon.ico"), Answer::Unrelated);
    }
}
