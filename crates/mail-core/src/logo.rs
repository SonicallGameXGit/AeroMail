//! Sender pictures: the logo a brand publishes for its own mail (BIMI).
//!
//! Only the rules live here; asking DNS and downloading is behind
//! `transport::LogoSource`. Everything a record or a download says comes from
//! the sender, so it is all treated as untrusted.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::address::is_valid_host;

/// A logo is a small drawing. Anything bigger is not shown.
pub const MAX_LOGO_BYTES: usize = 512 * 1024;

/// How many domains are asked for one sender: its own and two parents.
const MAX_CANDIDATES: usize = 3;

/// A name that can be looked up on the internet: not an address in numbers,
/// not a name that only means something on this machine or network.
fn is_public_name(host: &str) -> bool {
    let last_is_word = host
        .rsplit('.')
        .next()
        .is_some_and(|label| label.chars().any(|c| c.is_ascii_alphabetic()));

    host.is_ascii()
        && is_valid_host(host)
        && host.contains('.')
        && last_is_word
        && !host.ends_with(".localhost")
        && !host.ends_with(".local")
        && !host.ends_with(".internal")
}

/// The domain a message claims to come from, lower-cased.
pub fn sender_domain(email: &str) -> Option<String> {
    let (_, domain) = email.trim().rsplit_once('@')?;
    let domain = domain.to_ascii_lowercase();

    is_public_name(&domain).then_some(domain)
}

/// The domains to ask, the sender's own first. A company publishes one logo
/// on its main domain for all of its subdomains, so parents are asked too,
/// down to two labels.
pub fn candidate_domains(domain: &str) -> Vec<String> {
    let labels: Vec<&str> = domain.split('.').collect();

    (0..labels.len().saturating_sub(1))
        .take(MAX_CANDIDATES)
        .map(|skip| labels[skip..].join("."))
        .collect()
}

pub fn bimi_name(domain: &str) -> String {
    format!("default._bimi.{domain}")
}

pub fn dmarc_name(domain: &str) -> String {
    format!("_dmarc.{domain}")
}

/// `name=value` pairs of a DNS policy record, names lower-cased.
fn tags(record: &str) -> Vec<(String, &str)> {
    record
        .split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim()))
        .collect()
}

fn tag<'a>(tags: &[(String, &'a str)], name: &str) -> Option<&'a str> {
    tags.iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| *value)
}

fn is_version(tags: &[(String, &str)], version: &str) -> bool {
    tags.first()
        .is_some_and(|(name, value)| name == "v" && value.eq_ignore_ascii_case(version))
}

/// What a BIMI record says about the domain's logo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BimiRecord {
    /// Not a BIMI record, or one that cannot be used.
    Unusable,
    /// The domain says it has no logo; its parents are not asked.
    Declined,
    /// Where the logo is. Always an `https` address.
    Logo(String),
}

pub fn parse_bimi(record: &str) -> BimiRecord {
    let tags = tags(record);
    if !is_version(&tags, "BIMI1") {
        return BimiRecord::Unusable;
    }

    match tag(&tags, "l") {
        Some("") => BimiRecord::Declined,
        Some(url) if logo_host(url).is_some() => BimiRecord::Logo(url.to_string()),
        _ => BimiRecord::Unusable,
    }
}

/// Whether the domain tells receivers to refuse mail that only pretends to
/// come from it. Without that anyone could borrow the domain's logo by
/// forging the sender, so a logo is shown only for domains that do.
///
/// `inherited` is true when the record is a parent's: its `sp` then speaks
/// for the subdomain, and `p` only when there is no `sp`.
pub fn dmarc_enforced(record: &str, inherited: bool) -> bool {
    let tags = tags(record);
    if !is_version(&tags, "DMARC1") {
        return false;
    }

    let policy = match (inherited, tag(&tags, "sp")) {
        (true, Some(subdomains)) => Some(subdomains),
        _ => tag(&tags, "p"),
    };

    policy.is_some_and(|policy| {
        policy.eq_ignore_ascii_case("quarantine") || policy.eq_ignore_ascii_case("reject")
    })
}

/// The host of a logo address. `None` unless it is plain `https` to a
/// public host name on the standard port: no numbers for an address, no
/// sign-in part, nothing that could point into the user's own network.
pub fn logo_host(url: &str) -> Option<String> {
    let scheme = url.get(..8)?;
    if !scheme.eq_ignore_ascii_case("https://") {
        return None;
    }

    let rest = url.get(8..)?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest.get(..end)?.to_ascii_lowercase();
    // Nothing but the address itself: no spaces or control characters.
    let clean = url.chars().all(|c| c.is_ascii_graphic());

    (clean && is_public_name(&host)).then_some(host)
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    // 100.64.0.0/10 (carrier networks) and 240.0.0.0/4 (reserved) have no
    // method of their own.
    let shared = a == 100 && (64..128).contains(&b);

    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || shared
        || a == 0
        || a >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(inner) = ip.to_ipv4_mapped() {
        return is_public_v4(inner);
    }
    let first = ip.segments()[0];
    let unique_local = first & 0xfe00 == 0xfc00;
    let link_local = first & 0xffc0 == 0xfe80;

    !(ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || unique_local || link_local)
}

/// A logo is only ever downloaded from the public internet: a sender must
/// not be able to make the app knock on a machine in the user's network.
pub fn is_public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_v4(ip),
        IpAddr::V6(ip) => is_public_v6(ip),
    }
}

/// Whether a download is an SVG drawing of acceptable size. It is only ever
/// shown as a picture, where a drawing cannot run scripts or load anything.
pub fn is_logo(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > MAX_LOGO_BYTES {
        return false;
    }

    std::str::from_utf8(bytes).is_ok_and(|text| {
        let start = text.trim_start_matches('\u{feff}').trim_start();
        (start.starts_with("<?xml") || start.starts_with("<svg") || start.starts_with("<!"))
            && text.contains("<svg")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_domain_is_what_follows_the_last_at() {
        assert_eq!(
            sender_domain("No-Reply@Revolut.com").as_deref(),
            Some("revolut.com")
        );
        assert_eq!(
            sender_domain(" \"a@b\"@mail.example.co.uk ").as_deref(),
            Some("mail.example.co.uk")
        );
        assert_eq!(sender_domain("nobody"), None);
        assert_eq!(sender_domain("me@localhost"), None);
        assert_eq!(sender_domain("me@192.168.1.10"), None);
        assert_eq!(sender_domain("me@printer.local"), None);
        assert_eq!(sender_domain("me@exa mple.com"), None);
    }

    #[test]
    fn parents_are_asked_after_the_domain_itself_but_never_a_bare_ending() {
        assert_eq!(candidate_domains("revolut.com"), ["revolut.com"]);
        assert_eq!(
            candidate_domains("mail.revolut.com"),
            ["mail.revolut.com", "revolut.com"]
        );
        assert_eq!(
            candidate_domains("a.b.c.d.example.com"),
            [
                "a.b.c.d.example.com",
                "b.c.d.example.com",
                "c.d.example.com"
            ]
        );
        assert_eq!(bimi_name("revolut.com"), "default._bimi.revolut.com");
        assert_eq!(dmarc_name("revolut.com"), "_dmarc.revolut.com");
    }

    #[test]
    fn a_bimi_record_gives_the_logo_address() {
        assert_eq!(
            parse_bimi(
                "v=BIMI1; l=https://cdn.example.com/logo.svg; a=https://cdn.example.com/vmc.pem"
            ),
            BimiRecord::Logo("https://cdn.example.com/logo.svg".to_string())
        );
        assert_eq!(
            parse_bimi("V=bimi1;L=https://example.com/l.svg"),
            BimiRecord::Logo("https://example.com/l.svg".to_string())
        );
    }

    #[test]
    fn a_domain_can_decline_and_broken_records_are_ignored() {
        assert_eq!(parse_bimi("v=BIMI1; l=; a="), BimiRecord::Declined);
        assert_eq!(parse_bimi("v=BIMI1"), BimiRecord::Unusable);
        assert_eq!(
            parse_bimi("v=spf1 include:example.com"),
            BimiRecord::Unusable
        );
        assert_eq!(
            parse_bimi("l=https://example.com/l.svg; v=BIMI1"),
            BimiRecord::Unusable
        );
        assert_eq!(
            parse_bimi("v=BIMI1; l=http://example.com/l.svg"),
            BimiRecord::Unusable
        );
        assert_eq!(parse_bimi(""), BimiRecord::Unusable);
    }

    #[test]
    fn a_logo_address_must_be_plain_https_to_a_public_name() {
        assert_eq!(
            logo_host("https://CDN.Example.com/a/b.svg?x=1").as_deref(),
            Some("cdn.example.com")
        );
        assert_eq!(
            logo_host("https://example.com").as_deref(),
            Some("example.com")
        );

        for url in [
            "http://example.com/l.svg",
            "ftp://example.com/l.svg",
            "https://127.0.0.1/l.svg",
            "https://192.168.1.1/l.svg",
            "https://[::1]/l.svg",
            "https://localhost/l.svg",
            "https://router.local/l.svg",
            "https://example.com:8443/l.svg",
            "https://user:pass@example.com/l.svg",
            "https://example.com/l.svg\r\nHost: other",
            "https://",
            "https",
            "",
        ] {
            assert_eq!(logo_host(url), None, "{url}");
        }
    }

    #[test]
    fn only_domains_that_refuse_forged_mail_count() {
        assert!(dmarc_enforced(
            "v=DMARC1; p=reject; rua=mailto:a@example.com",
            false
        ));
        assert!(dmarc_enforced("v=DMARC1;p=Quarantine", false));
        assert!(!dmarc_enforced("v=DMARC1; p=none", false));
        assert!(!dmarc_enforced("v=DMARC1", false));
        assert!(!dmarc_enforced("v=spf1 -all", false));
        assert!(!dmarc_enforced("p=reject; v=DMARC1", false));
    }

    #[test]
    fn a_parent_speaks_for_subdomains_with_its_own_rule_for_them() {
        assert!(dmarc_enforced("v=DMARC1; p=reject", true));
        assert!(!dmarc_enforced("v=DMARC1; p=reject; sp=none", true));
        assert!(dmarc_enforced("v=DMARC1; p=none; sp=reject", true));
        // The rule for subdomains says nothing about the domain itself.
        assert!(!dmarc_enforced("v=DMARC1; p=none; sp=reject", false));
    }

    #[test]
    fn only_the_public_internet_is_downloaded_from() {
        let public = ["8.8.8.8", "151.101.1.140", "2606:4700::1111"];
        let private = [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.3.4",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "240.0.0.1",
            "::1",
            "::",
            "fe80::1",
            "fd00::1",
            "::ffff:192.168.1.1",
        ];

        for ip in public {
            assert!(is_public_address(ip.parse().unwrap()), "{ip}");
        }
        for ip in private {
            assert!(!is_public_address(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn a_logo_is_a_small_svg_drawing() {
        assert!(is_logo(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"));
        assert!(is_logo(b"<?xml version=\"1.0\"?>\n<svg></svg>"));
        assert!(is_logo("\u{feff}  <svg></svg>".as_bytes()));

        assert!(!is_logo(b""));
        assert!(!is_logo(b"<html><body>Not found</body></html>"));
        assert!(!is_logo(b"\x89PNG\r\n\x1a\n"));
        assert!(!is_logo(&[0xff, 0xfe, 0x00]));

        let mut huge = b"<svg>".to_vec();
        huge.resize(MAX_LOGO_BYTES + 1, b' ');
        assert!(!is_logo(&huge));
    }
}
