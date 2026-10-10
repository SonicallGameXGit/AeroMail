//! Signing in on the provider's own page (OAuth2) instead of typing a
//! password: which providers the app knows, and the values the sign-in
//! passes between the crates.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy)]
pub struct OAuthProvider {
    pub name: &'static str,
    /// The mail servers that accept this provider's tokens.
    pub hosts: &'static [&'static str],
    pub authorize_url: &'static str,
    pub token_url: &'static str,
    pub scope: &'static str,
    /// Asked for as well when the user picks the account on the page instead
    /// of the app naming it: the provider then says which address it was.
    pub identity_scope: &'static str,
    /// How the page asks the user (OpenID `prompt`). When the user picks the
    /// account there, `select_account` comes first.
    pub prompt: &'static str,
    /// What this provider's sign-in page needs beyond the standard parameters.
    pub extra: &'static [(&'static str, &'static str)],
    /// How the provider recognises this app. Registered by the owner and
    /// given to the build; a build made without them cannot sign in here.
    pub client_id: Option<&'static str>,
    /// Not a secret for a desktop app (it ships inside the program), but the
    /// provider still asks for it.
    pub client_secret: Option<&'static str>,
}

fn given(value: Option<&'static str>) -> Option<&'static str> {
    value.filter(|value| !value.trim().is_empty())
}

impl OAuthProvider {
    pub fn client_id(&self) -> Option<&'static str> {
        given(self.client_id)
    }

    pub fn client_secret(&self) -> Option<&'static str> {
        given(self.client_secret)
    }
}

pub const PROVIDERS: &[OAuthProvider] = &[OAuthProvider {
    name: "Google",
    // Custom domains hosted by Google use the same servers.
    hosts: &["imap.gmail.com", "pop.gmail.com", "smtp.gmail.com"],
    authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    // The only scope Google accepts for IMAP and SMTP.
    scope: "https://mail.google.com/",
    identity_scope: "openid email",
    // `consent` makes Google hand out a lasting token again when the account
    // is added a second time.
    prompt: "consent",
    // `offline` asks for a token that outlives the hour.
    extra: &[("access_type", "offline")],
    client_id: option_env!("MAILCLIENT_GOOGLE_CLIENT_ID"),
    client_secret: option_env!("MAILCLIENT_GOOGLE_CLIENT_SECRET"),
}];

pub fn provider_for(host: &str) -> Option<&'static OAuthProvider> {
    let host = host.trim();

    PROVIDERS.iter().find(|provider| {
        provider
            .hosts
            .iter()
            .any(|known| known.eq_ignore_ascii_case(host))
    })
}

/// Whether this build can sign in to the server through its provider's page.
pub fn sign_in_ready(host: &str) -> bool {
    provider_for(host).is_some_and(|provider| provider.client_id().is_some())
}

/// What a finished sign-in, or a renewal, gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    /// Opens the mail servers for a short time.
    pub access: String,
    /// Gets new access tokens without asking the user again. Providers may
    /// leave it out of a renewal: the one already held stays valid then.
    pub refresh: Option<String>,
    /// Seconds until `access` stops working.
    pub expires_in: u64,
    /// The address the user signed in as, when the provider said so and
    /// vouches for it.
    pub email: Option<String>,
}

/// A sign-in that waits for the user: the page to open in their browser, and
/// the number to finish or cancel it by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SignIn {
    pub id: u32,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use crate::address::is_valid_host;

    use super::*;

    #[test]
    fn finds_the_provider_of_a_server_whatever_the_case() {
        assert_eq!(provider_for("imap.gmail.com").unwrap().name, "Google");
        assert_eq!(provider_for(" SMTP.Gmail.com ").unwrap().name, "Google");
        assert!(provider_for("imap.example.com").is_none());
    }

    #[test]
    fn every_entry_is_well_formed() {
        for provider in PROVIDERS {
            assert!(!provider.name.is_empty());
            assert!(!provider.scope.is_empty());
            assert!(provider.authorize_url.starts_with("https://"));
            assert!(provider.token_url.starts_with("https://"));
            for host in provider.hosts {
                assert!(is_valid_host(host), "{host} is not a valid host");
            }
        }
    }

    #[test]
    fn an_empty_client_id_counts_as_none() {
        let provider = OAuthProvider {
            client_id: Some("  "),
            client_secret: Some(""),
            ..PROVIDERS[0]
        };

        assert_eq!(provider.client_id(), None);
        assert_eq!(provider.client_secret(), None);
    }
}
