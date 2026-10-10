//! What an address says about its provider while it is still being typed,
//! so the setup screen can show the provider's mark, or warn about a typo,
//! before the user goes on. Nothing here decides anything: the finished
//! address is detected again (`detect`), whatever this said.

use serde::Serialize;
use ts_rs::TS;

use crate::providers::PROVIDERS;

/// A provider whose own mark the UI can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum Brand {
    Gmail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum TypedDomain {
    /// No domain yet, or none of a provider with a mark.
    Unknown,
    /// One of the provider's domains, typed in full or on its way there.
    Known { brand: Brand, provider: String },
    /// The provider's name with an ending it does not use.
    Mistyped {
        provider: String,
        /// The provider's domain with the same name.
        suggestion: String,
    },
}

/// Reads the domain typed so far. Its name alone (`gmail`) is enough to be
/// known, as long as the rest is still on the way to a real domain
/// (`gmail.c`).
pub fn recognize(email: &str) -> TypedDomain {
    let Some((_, domain)) = email.trim().rsplit_once('@') else {
        return TypedDomain::Unknown;
    };
    let domain = domain.to_lowercase();

    let branded = PROVIDERS
        .iter()
        .filter_map(|provider| provider.brand.map(|brand| (provider, brand)));
    for (provider, brand) in branded.clone() {
        for known in provider.domains {
            if domain.starts_with(name_of(known)) && known.starts_with(domain.as_str()) {
                return TypedDomain::Known {
                    brand,
                    provider: provider.name.to_string(),
                };
            }
        }
    }

    for (provider, _) in branded.filter(|(provider, _)| provider.closed_domains) {
        for known in provider.domains {
            if domain.starts_with(&format!("{}.", name_of(known))) {
                return TypedDomain::Mistyped {
                    provider: provider.name.to_string(),
                    suggestion: known.to_string(),
                };
            }
        }
    }

    TypedDomain::Unknown
}

/// `gmail` for `gmail.com`.
fn name_of(domain: &str) -> &str {
    domain.split('.').next().unwrap_or(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gmail() -> TypedDomain {
        TypedDomain::Known {
            brand: Brand::Gmail,
            provider: "Gmail".to_string(),
        }
    }

    fn mistyped(suggestion: &str) -> TypedDomain {
        TypedDomain::Mistyped {
            provider: "Gmail".to_string(),
            suggestion: suggestion.to_string(),
        }
    }

    #[test]
    fn knows_gmail_from_its_name_on() {
        for email in [
            "john@gmail",
            "john@gmail.",
            "john@gmail.c",
            "john@gmail.co",
            "john@gmail.com",
            "John@GMail.Com",
            " john@gmail.com ",
            "john@googlemail.com",
            "john@googlemail",
        ] {
            assert_eq!(recognize(email), gmail(), "{email}");
        }
    }

    #[test]
    fn says_nothing_before_the_name_is_complete_or_for_other_domains() {
        for email in [
            "",
            "john",
            "john@",
            "john@g",
            "john@gmai",
            "john@gmailx.com",
            "john@mail.gmail.com",
            "john@example.com",
            // Known providers without a mark of their own.
            "john@outlook.com",
            "john@yahoo.com",
        ] {
            assert_eq!(recognize(email), TypedDomain::Unknown, "{email}");
        }
    }

    #[test]
    fn suggests_the_real_domain_for_another_ending() {
        let cases = [
            ("john@gmail.ru", "gmail.com"),
            ("john@gmail.con", "gmail.com"),
            ("john@gmail.comm", "gmail.com"),
            ("john@gmail.co.uk", "gmail.com"),
            ("john@googlemail.de", "googlemail.com"),
        ];

        for (email, suggestion) in cases {
            assert_eq!(recognize(email), mistyped(suggestion), "{email}");
        }
    }

    #[test]
    fn the_last_at_sign_starts_the_domain() {
        assert_eq!(recognize("a@b@gmail.com"), gmail());
    }
}
