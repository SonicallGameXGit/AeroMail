use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize, TS)]
#[ts(export)]
pub enum AddressError {
    #[error("the address is empty")]
    Empty,
    #[error("the address has no @")]
    MissingAt,
    #[error("the address has more than one @")]
    MultipleAt,
    #[error("the part before @ is empty")]
    EmptyLocalPart,
    #[error("the part before @ is not valid")]
    InvalidLocalPart,
    #[error("the domain after @ is not valid")]
    InvalidDomain,
}

/// An address the user typed for an account, split and normalised.
///
/// This is deliberately stricter than RFC 5322 (no quoted local parts, no
/// comments): it validates what a person types into a sign-in field, not
/// arbitrary header addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAddress {
    local: String,
    domain: String,
}

impl EmailAddress {
    pub fn parse(input: &str) -> Result<Self, AddressError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AddressError::Empty);
        }

        let mut parts = trimmed.split('@');
        let local = parts.next().unwrap_or_default();
        let domain = parts.next().ok_or(AddressError::MissingAt)?;
        if parts.next().is_some() {
            return Err(AddressError::MultipleAt);
        }

        if local.is_empty() {
            return Err(AddressError::EmptyLocalPart);
        }
        if !is_valid_local_part(local) {
            return Err(AddressError::InvalidLocalPart);
        }

        let domain = domain.to_lowercase();
        if !is_valid_host(&domain) || !domain.contains('.') {
            return Err(AddressError::InvalidDomain);
        }

        Ok(Self {
            local: local.to_string(),
            domain,
        })
    }

    pub fn local(&self) -> &str {
        &self.local
    }

    /// Always lowercase.
    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn full(&self) -> String {
        format!("{}@{}", self.local, self.domain)
    }
}

fn is_valid_local_part(local: &str) -> bool {
    local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .chars()
            .all(|c| !c.is_whitespace() && !c.is_control() && !"()<>[]:;,\\\"".contains(c))
}

/// Host name check shared by addresses and server settings: dot-separated
/// labels of letters, digits and inner hyphens.
pub fn is_valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_alphanumeric() || c == '-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalises() {
        let address = EmailAddress::parse("  John.Doe@Example.COM ").unwrap();
        assert_eq!(address.local(), "John.Doe");
        assert_eq!(address.domain(), "example.com");
        assert_eq!(address.full(), "John.Doe@example.com");
    }

    #[test]
    fn accepts_plus_tags_and_subdomains() {
        let address = EmailAddress::parse("me+news@mail.example.co.uk").unwrap();
        assert_eq!(address.domain(), "mail.example.co.uk");
    }

    #[test]
    fn rejects_bad_input() {
        let cases = [
            ("", AddressError::Empty),
            ("   ", AddressError::Empty),
            ("john", AddressError::MissingAt),
            ("a@b@c.com", AddressError::MultipleAt),
            ("@example.com", AddressError::EmptyLocalPart),
            ("jo hn@example.com", AddressError::InvalidLocalPart),
            (".john@example.com", AddressError::InvalidLocalPart),
            ("jo..hn@example.com", AddressError::InvalidLocalPart),
            ("john@", AddressError::InvalidDomain),
            ("john@localhost", AddressError::InvalidDomain),
            ("john@exa mple.com", AddressError::InvalidDomain),
            ("john@-example.com", AddressError::InvalidDomain),
            ("john@example..com", AddressError::InvalidDomain),
        ];

        for (input, expected) in cases {
            assert_eq!(
                EmailAddress::parse(input),
                Err(expected),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn host_check() {
        assert!(is_valid_host("imap.example.com"));
        assert!(is_valid_host("localhost"));
        assert!(is_valid_host("192.168.1.10"));
        assert!(!is_valid_host(""));
        assert!(!is_valid_host("imap .example.com"));
        assert!(!is_valid_host("imap.example.com:993"));
        assert!(!is_valid_host("https://example.com"));
    }
}
