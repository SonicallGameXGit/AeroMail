//! Detects mail server settings from an email address.
//!
//! Detection only ever produces a *suggestion*: the result is a complete,
//! editable `AccountSettings` that the manual setup form can show as-is. An
//! unknown domain is not an error - it yields a best-effort guess, marked as
//! such, so the user lands in the manual form with something to start from.

mod providers;
mod typed;

use mail_core::account::{AccountSettings, AuthMethod, Protocol, Security, ServerSettings};
use mail_core::address::{AddressError, EmailAddress};
use serde::Serialize;
use ts_rs::TS;

use providers::{Endpoint, Provider};
pub use typed::{Brand, TypedDomain, recognize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum DetectionSource {
    /// Found in the built-in provider table: the settings are known to be right.
    BuiltIn,
    /// Unknown domain: conventional host names were guessed and must be reviewed.
    Guess,
}

/// What the user has to type into the password field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum PasswordKind {
    /// The normal account password.
    Account,
    /// A separate app password generated in the provider's security settings.
    AppPassword,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Detection {
    pub source: DetectionSource,
    /// Provider display name, when known.
    pub provider: Option<String>,
    /// Applies when the user signs in with a password instead of OAuth2.
    pub password_kind: PasswordKind,
    /// Whether this build can sign in on the provider's own page (OAuth2)
    /// for the suggested servers.
    pub sign_in_available: bool,
    /// Suggested settings, with IMAP as the incoming protocol.
    pub settings: AccountSettings,
    /// POP3 alternative for the incoming server, when the provider offers one.
    pub pop3: Option<ServerSettings>,
}

/// Suggests settings for `email` without touching the network.
pub fn detect(email: &str) -> Result<Detection, AddressError> {
    detect_with(email, mail_core::oauth::sign_in_ready)
}

/// `detect`, told which servers this build can sign in to through their
/// provider's page.
fn detect_with(
    email: &str,
    sign_in_ready: impl Fn(&str) -> bool,
) -> Result<Detection, AddressError> {
    let address = EmailAddress::parse(email)?;

    let detection = match providers::find(address.domain()) {
        Some(provider) => {
            from_provider(&address.full(), provider, sign_in_ready(provider.imap.host))
        }
        None => guess(&address),
    };

    Ok(detection)
}

fn from_provider(email: &str, provider: &Provider, sign_in_ready: bool) -> Detection {
    // The provider's page is the one-click way. Where it cannot be used and
    // a password works too, the password is suggested instead of a dead end.
    let auth = match provider.auth {
        AuthMethod::OAuth2 if !sign_in_ready && provider.accepts_password => AuthMethod::Password,
        auth => auth,
    };
    let server = |protocol: Protocol, endpoint: &Endpoint| ServerSettings {
        protocol,
        host: endpoint.host.to_string(),
        port: endpoint.port,
        security: endpoint.security,
        auth,
        username: email.to_string(),
    };

    Detection {
        source: DetectionSource::BuiltIn,
        provider: Some(provider.name.to_string()),
        password_kind: provider.password_kind,
        sign_in_available: sign_in_ready,
        settings: AccountSettings {
            email: email.to_string(),
            display_name: String::new(),
            incoming: server(Protocol::Imap, &provider.imap),
            outgoing: server(Protocol::Smtp, &provider.smtp),
        },
        pop3: provider
            .pop3
            .as_ref()
            .map(|endpoint| server(Protocol::Pop3, endpoint)),
    }
}

/// A provider the user can add an account of without typing the address:
/// they pick the account on the provider's own page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProviderChoice {
    pub brand: Brand,
    pub provider: String,
}

/// The providers with a mark whose page this build can sign in on.
pub fn sign_in_choices() -> Vec<ProviderChoice> {
    sign_in_choices_with(mail_core::oauth::sign_in_ready)
}

fn sign_in_choices_with(sign_in_ready: impl Fn(&str) -> bool) -> Vec<ProviderChoice> {
    providers::PROVIDERS
        .iter()
        .filter(|provider| provider.auth == AuthMethod::OAuth2 && sign_in_ready(provider.imap.host))
        .filter_map(|provider| {
            provider.brand.map(|brand| ProviderChoice {
                brand,
                provider: provider.name.to_string(),
            })
        })
        .collect()
}

/// A well-known provider whose domain the address field can hold fixed, so
/// only the name before `@` is typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AddressPreset {
    pub provider: String,
    /// The provider's main domain, without `@`.
    pub domain: String,
    pub brand: Option<Brand>,
}

/// Every provider in the built-in table, in its order, with its main domain.
pub fn address_presets() -> Vec<AddressPreset> {
    providers::PROVIDERS
        .iter()
        .filter_map(|provider| {
            provider.domains.first().map(|domain| AddressPreset {
                provider: provider.name.to_string(),
                domain: (*domain).to_string(),
                brand: provider.brand,
            })
        })
        .collect()
}

/// The provider's settings for signing in on its page, without an address:
/// it is filled in from the account the user picks there. `None` when this
/// build cannot sign in there.
pub fn sign_in_settings(brand: Brand) -> Option<AccountSettings> {
    sign_in_settings_with(brand, mail_core::oauth::sign_in_ready)
}

fn sign_in_settings_with(
    brand: Brand,
    sign_in_ready: impl Fn(&str) -> bool,
) -> Option<AccountSettings> {
    let provider = providers::PROVIDERS.iter().find(|provider| {
        provider.brand == Some(brand)
            && provider.auth == AuthMethod::OAuth2
            && sign_in_ready(provider.imap.host)
    })?;

    Some(from_provider("", provider, true).settings)
}

fn guess(address: &EmailAddress) -> Detection {
    let server = |protocol: Protocol, prefix: &str, port: u16| ServerSettings {
        protocol,
        host: format!("{prefix}.{}", address.domain()),
        port,
        security: Security::Tls,
        auth: AuthMethod::Password,
        username: address.full(),
    };

    Detection {
        source: DetectionSource::Guess,
        provider: None,
        password_kind: PasswordKind::Account,
        sign_in_available: false,
        settings: AccountSettings {
            email: address.full(),
            display_name: String::new(),
            incoming: server(Protocol::Imap, "imap", 993),
            outgoing: server(Protocol::Smtp, "smtp", 465),
        },
        pop3: Some(server(Protocol::Pop3, "pop", 995)),
    }
}

#[cfg(test)]
mod tests {
    use mail_core::account::validate;

    use super::*;

    /// A build without any registered client id, whatever this one has.
    fn detect(email: &str) -> Result<Detection, AddressError> {
        detect_with(email, |_| false)
    }

    #[test]
    fn presets_start_with_gmail_and_are_all_detected_as_built_in() {
        let presets = address_presets();
        assert_eq!(
            presets.first(),
            Some(&AddressPreset {
                provider: "Gmail".to_string(),
                domain: "gmail.com".to_string(),
                brand: Some(Brand::Gmail),
            }),
        );
        assert!(presets.iter().any(|preset| preset.domain == "outlook.com"));
        for preset in presets {
            let detection = detect(&format!("john@{}", preset.domain)).unwrap();
            assert_eq!(
                detection.source,
                DetectionSource::BuiltIn,
                "{}",
                preset.domain
            );
        }
    }

    #[test]
    fn gmail_is_offered_without_an_address_only_where_the_build_can_sign_in() {
        assert_eq!(sign_in_choices_with(|_| false), Vec::new());
        assert_eq!(
            sign_in_choices_with(|host| host == "imap.gmail.com"),
            vec![ProviderChoice {
                brand: Brand::Gmail,
                provider: "Gmail".to_string(),
            }],
        );

        assert_eq!(sign_in_settings_with(Brand::Gmail, |_| false), None);
        let settings = sign_in_settings_with(Brand::Gmail, |_| true).unwrap();
        assert_eq!(settings.email, "");
        assert_eq!(settings.incoming.host, "imap.gmail.com");
        assert_eq!(settings.incoming.username, "");
        assert_eq!(settings.incoming.auth, AuthMethod::OAuth2);
        assert_eq!(settings.outgoing.host, "smtp.gmail.com");
        assert_eq!(settings.outgoing.auth, AuthMethod::OAuth2);
    }

    #[test]
    fn detects_gmail() {
        let detection = detect("John.Doe@Gmail.com").unwrap();

        assert_eq!(detection.source, DetectionSource::BuiltIn);
        assert_eq!(detection.provider.as_deref(), Some("Gmail"));

        let settings = &detection.settings;
        assert_eq!(settings.email, "John.Doe@gmail.com");
        assert_eq!(settings.incoming.protocol, Protocol::Imap);
        assert_eq!(settings.incoming.host, "imap.gmail.com");
        assert_eq!(settings.incoming.port, 993);
        assert_eq!(settings.incoming.security, Security::Tls);
        assert_eq!(settings.incoming.auth, AuthMethod::Password);
        assert_eq!(detection.password_kind, PasswordKind::AppPassword);
        assert_eq!(settings.incoming.username, "John.Doe@gmail.com");
        assert_eq!(settings.outgoing.protocol, Protocol::Smtp);
        assert_eq!(settings.outgoing.host, "smtp.gmail.com");
        assert_eq!(settings.outgoing.port, 465);
    }

    #[test]
    fn gmail_signs_in_on_googles_page_when_the_build_can() {
        let detection = detect_with("a@gmail.com", |host| host == "imap.gmail.com").unwrap();

        assert!(detection.sign_in_available);
        assert_eq!(detection.settings.incoming.auth, AuthMethod::OAuth2);
        assert_eq!(detection.settings.outgoing.auth, AuthMethod::OAuth2);
        // Still said for whoever switches to a password by hand.
        assert_eq!(detection.password_kind, PasswordKind::AppPassword);
    }

    #[test]
    fn a_provider_that_takes_no_password_keeps_its_sign_in_even_when_unavailable() {
        let detection = detect("a@outlook.com").unwrap();

        assert!(!detection.sign_in_available);
        assert_eq!(detection.settings.incoming.auth, AuthMethod::OAuth2);
    }

    #[test]
    fn detects_providers_by_any_of_their_domains() {
        let cases = [
            ("a@googlemail.com", "Gmail", "imap.gmail.com"),
            ("a@hotmail.com", "Outlook", "outlook.office365.com"),
            ("a@live.com", "Outlook", "outlook.office365.com"),
            ("a@ymail.com", "Yahoo Mail", "imap.mail.yahoo.com"),
            ("a@me.com", "iCloud Mail", "imap.mail.me.com"),
            ("a@gmx.de", "GMX", "imap.gmx.net"),
            ("a@gmx.com", "GMX", "imap.gmx.com"),
        ];

        for (email, provider, imap_host) in cases {
            let detection = detect(email).unwrap();
            assert_eq!(detection.provider.as_deref(), Some(provider), "{email}");
            assert_eq!(detection.settings.incoming.host, imap_host, "{email}");
        }
    }

    #[test]
    fn offers_pop3_only_where_the_provider_has_it() {
        let gmail = detect("a@gmail.com").unwrap();
        let pop3 = gmail.pop3.unwrap();
        assert_eq!(pop3.protocol, Protocol::Pop3);
        assert_eq!(pop3.host, "pop.gmail.com");
        assert_eq!(pop3.port, 995);

        let icloud = detect("a@icloud.com").unwrap();
        assert_eq!(icloud.pop3, None);
    }

    #[test]
    fn unknown_domain_falls_back_to_an_editable_guess() {
        let detection = detect("me@my-company.example").unwrap();

        assert_eq!(detection.source, DetectionSource::Guess);
        assert_eq!(detection.provider, None);
        assert_eq!(detection.settings.incoming.host, "imap.my-company.example");
        assert_eq!(detection.settings.incoming.port, 993);
        assert_eq!(detection.settings.outgoing.host, "smtp.my-company.example");
        assert_eq!(detection.settings.outgoing.port, 465);
        assert_eq!(detection.settings.incoming.auth, AuthMethod::Password);
        assert!(!detection.sign_in_available);
        assert_eq!(detection.pop3.unwrap().host, "pop.my-company.example");
    }

    #[test]
    fn invalid_address_is_an_error() {
        assert_eq!(detect("not an address"), Err(AddressError::MissingAt));
        assert_eq!(detect(""), Err(AddressError::Empty));
    }

    #[test]
    fn every_suggestion_passes_validation() {
        let mut emails: Vec<String> = providers::PROVIDERS
            .iter()
            .flat_map(|provider| provider.domains.iter())
            .map(|domain| format!("user@{domain}"))
            .collect();
        emails.push("user@unknown.example".to_string());

        for (email, ready) in emails
            .iter()
            .flat_map(|email| [(email, false), (email, true)])
        {
            let detection = detect_with(email, |_| ready).unwrap();
            assert_eq!(validate(&detection.settings), Vec::new(), "{email}");

            if let Some(pop3) = detection.pop3 {
                let mut settings = detection.settings.clone();
                settings.incoming = pop3;
                assert_eq!(validate(&settings), Vec::new(), "{email} (POP3)");
            }
        }
    }
}
