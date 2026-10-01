//! Built-in table of well-known mail providers.
//!
//! Adding a provider is adding one entry here; the table tests check every
//! entry for consistency.

use mail_core::account::{AuthMethod, Security};

use crate::PasswordKind;

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
    pub auth: AuthMethod,
    pub password_kind: PasswordKind,
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
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "Outlook",
        domains: &["outlook.com", "hotmail.com", "live.com", "msn.com"],
        imap: tls("outlook.office365.com", 993),
        pop3: Some(tls("outlook.office365.com", 995)),
        smtp: start_tls("smtp-mail.outlook.com", 587),
        auth: AuthMethod::OAuth2,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "Yahoo Mail",
        domains: &["yahoo.com", "ymail.com", "rocketmail.com"],
        imap: tls("imap.mail.yahoo.com", 993),
        pop3: Some(tls("pop.mail.yahoo.com", 995)),
        smtp: tls("smtp.mail.yahoo.com", 465),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "iCloud Mail",
        domains: &["icloud.com", "me.com", "mac.com"],
        imap: tls("imap.mail.me.com", 993),
        pop3: None,
        smtp: start_tls("smtp.mail.me.com", 587),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "Fastmail",
        domains: &["fastmail.com", "fastmail.fm"],
        imap: tls("imap.fastmail.com", 993),
        pop3: Some(tls("pop.fastmail.com", 995)),
        smtp: tls("smtp.fastmail.com", 465),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "AOL Mail",
        domains: &["aol.com"],
        imap: tls("imap.aol.com", 993),
        pop3: Some(tls("pop.aol.com", 995)),
        smtp: tls("smtp.aol.com", 465),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "GMX",
        domains: &["gmx.com", "gmx.us"],
        imap: tls("imap.gmx.com", 993),
        pop3: Some(tls("pop.gmx.com", 995)),
        smtp: start_tls("mail.gmx.com", 587),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::Account,
    },
    Provider {
        name: "GMX",
        domains: &["gmx.net", "gmx.de", "gmx.at", "gmx.ch"],
        imap: tls("imap.gmx.net", 993),
        pop3: Some(tls("pop.gmx.net", 995)),
        smtp: start_tls("mail.gmx.net", 587),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::Account,
    },
    Provider {
        name: "Yandex Mail",
        domains: &["yandex.com", "yandex.ru", "ya.ru"],
        imap: tls("imap.yandex.com", 993),
        pop3: Some(tls("pop.yandex.com", 995)),
        smtp: tls("smtp.yandex.com", 465),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "Mail.ru",
        domains: &["mail.ru", "inbox.ru", "list.ru", "bk.ru"],
        imap: tls("imap.mail.ru", 993),
        pop3: Some(tls("pop.mail.ru", 995)),
        smtp: tls("smtp.mail.ru", 465),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::AppPassword,
    },
    Provider {
        name: "Zoho Mail",
        domains: &["zoho.com", "zohomail.com"],
        imap: tls("imap.zoho.com", 993),
        pop3: Some(tls("pop.zoho.com", 995)),
        smtp: tls("smtp.zoho.com", 465),
        auth: AuthMethod::Password,
        password_kind: PasswordKind::Account,
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
