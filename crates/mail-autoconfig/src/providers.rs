//! Built-in table of well-known mail providers.
//!
//! Adding a provider is adding one entry here; the table tests check every
//! entry for consistency.

use mail_core::account::{AuthMethod, Security};

use crate::{Brand, PasswordKind};

pub(crate) struct Endpoint {
    pub host: &'static str,
    pub port: u16,
    pub security: Security,
}

pub(crate) struct Provider {
    pub name: &'static str,
    pub domains: &'static [&'static str],
    pub imap: Endpoint,
    pub pop3: Option<Endpoint>,
    pub smtp: Endpoint,
    /// The sign-in the provider wants.
    pub auth: AuthMethod,
    /// Whether a password also works where `auth` is OAuth2. It is then the
    /// manual way in, and the only one in a build that cannot sign in there.
    pub accepts_password: bool,
    pub password_kind: PasswordKind,
    /// The provider's own mark, shown while its address is being typed.
    pub brand: Option<Brand>,
    /// Whether `domains` is every domain the provider gives out, and no other
    /// domain starts with the same name. The name with another ending (Gmail
    /// has no `gmail.ru`) is then almost surely a typo. Not true where a
    /// provider has country domains of its own (`outlook.de`, `yahoo.fr`), or
    /// a name others use too (`mail.example.com`).
    pub closed_domains: bool,
}

const fn tls(host: &'static str, port: u16) -> Endpoint {
    Endpoint {
        host,
        port,
        security: Security::Tls,
    }
}

const fn start_tls(host: &'static str, port: u16) -> Endpoint {
    Endpoint {
        host,
        port,
        security: Security::StartTls,
    }
}

pub(crate) const PROVIDERS: &[Provider] = &[
    Provider {
        name: "Gmail",
        domains: &["gmail.com", "googlemail.com"],
        imap: tls("imap.gmail.com", 993),
        pop3: Some(tls("pop.gmail.com", 995)),
        smtp: tls("smtp.gmail.com", 465),
        auth: AuthMethod::OAuth2,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: Some(Brand::Gmail),
        closed_domains: true,
    },
    Provider {
        name: "Outlook",
        domains: &["outlook.com", "hotmail.com", "live.com", "msn.com"],
        imap: tls("outlook.office365.com", 993),
        pop3: Some(tls("outlook.office365.com", 995)),
        smtp: start_tls("smtp-mail.outlook.com", 587),
        auth: AuthMethod::OAuth2,
        accepts_password: false,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "Yahoo Mail",
        domains: &["yahoo.com", "ymail.com", "rocketmail.com"],
        imap: tls("imap.mail.yahoo.com", 993),
        pop3: Some(tls("pop.mail.yahoo.com", 995)),
        smtp: tls("smtp.mail.yahoo.com", 465),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "iCloud Mail",
        domains: &["icloud.com", "me.com", "mac.com"],
        imap: tls("imap.mail.me.com", 993),
        pop3: None,
        smtp: start_tls("smtp.mail.me.com", 587),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "Fastmail",
        domains: &["fastmail.com", "fastmail.fm"],
        imap: tls("imap.fastmail.com", 993),
        pop3: Some(tls("pop.fastmail.com", 995)),
        smtp: tls("smtp.fastmail.com", 465),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "AOL Mail",
        domains: &["aol.com"],
        imap: tls("imap.aol.com", 993),
        pop3: Some(tls("pop.aol.com", 995)),
        smtp: tls("smtp.aol.com", 465),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "GMX",
        domains: &["gmx.com", "gmx.us"],
        imap: tls("imap.gmx.com", 993),
        pop3: Some(tls("pop.gmx.com", 995)),
        smtp: start_tls("mail.gmx.com", 587),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::Account,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "GMX",
        domains: &["gmx.net", "gmx.de", "gmx.at", "gmx.ch"],
        imap: tls("imap.gmx.net", 993),
        pop3: Some(tls("pop.gmx.net", 995)),
        smtp: start_tls("mail.gmx.net", 587),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::Account,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "Yandex Mail",
        domains: &["yandex.com", "yandex.ru", "ya.ru"],
        imap: tls("imap.yandex.com", 993),
        pop3: Some(tls("pop.yandex.com", 995)),
        smtp: tls("smtp.yandex.com", 465),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "Mail.ru",
        domains: &["mail.ru", "inbox.ru", "list.ru", "bk.ru"],
        imap: tls("imap.mail.ru", 993),
        pop3: Some(tls("pop.mail.ru", 995)),
        smtp: tls("smtp.mail.ru", 465),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::AppPassword,
        brand: None,
        closed_domains: false,
    },
    Provider {
        name: "Zoho Mail",
        domains: &["zoho.com", "zohomail.com"],
        imap: tls("imap.zoho.com", 993),
        pop3: Some(tls("pop.zoho.com", 995)),
        smtp: tls("smtp.zoho.com", 465),
        auth: AuthMethod::Password,
        accepts_password: true,
        password_kind: PasswordKind::Account,
        brand: None,
        closed_domains: false,
    },
];

pub(crate) fn find(domain: &str) -> Option<&'static Provider> {
    PROVIDERS
        .iter()
        .find(|provider| provider.domains.contains(&domain))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use mail_core::address::is_valid_host;

    use super::*;

    #[test]
    fn no_domain_is_listed_twice() {
        let mut seen = HashSet::new();
        for provider in PROVIDERS {
            for domain in provider.domains {
                assert!(seen.insert(*domain), "{domain} is listed twice");
            }
        }
    }

    #[test]
    fn every_entry_is_well_formed() {
        for provider in PROVIDERS {
            assert!(!provider.name.is_empty());
            assert!(
                !provider.domains.is_empty(),
                "{} has no domains",
                provider.name
            );

            for domain in provider.domains {
                assert_eq!(*domain, domain.to_lowercase(), "{domain} must be lowercase");
                assert!(is_valid_host(domain), "{domain} is not a valid domain");
            }

            let endpoints = [
                Some(&provider.imap),
                provider.pop3.as_ref(),
                Some(&provider.smtp),
            ];
            for endpoint in endpoints.into_iter().flatten() {
                assert!(
                    is_valid_host(endpoint.host),
                    "{} is not a valid host",
                    endpoint.host
                );
                assert_ne!(endpoint.port, 0);
                assert_ne!(
                    endpoint.security,
                    Security::None,
                    "{} must not be listed without encryption",
                    endpoint.host,
                );
            }
        }
    }
}
