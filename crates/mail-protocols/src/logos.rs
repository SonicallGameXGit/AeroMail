//! Sender pictures from the network: the BIMI record of a domain in DNS,
//! then the logo it points to over HTTPS. What is acceptable is decided by
//! `mail_core::logo`; this file only asks and downloads.

use std::net::SocketAddr;
use std::time::Duration;

use async_trait::async_trait;
use hickory_resolver::TokioResolver;
use hickory_resolver::proto::rr::RData;
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::logo::{
    BimiRecord, MAX_LOGO_BYTES, bimi_name, candidate_domains, dmarc_enforced, dmarc_name, is_logo,
    is_public_address, logo_host, parse_bimi,
};
use mail_core::transport::LogoSource;

use crate::tls;

/// A picture is decoration: it is not worth a long wait.
const TIMEOUT: Duration = Duration::from_secs(8);

/// Finds logos on the real internet.
#[derive(Default)]
pub struct NetLogos;

fn unreachable(detail: impl ToString) -> MailError {
    MailError::new(ErrorKind::Connect, detail.to_string())
}

fn resolver() -> MailResult<TokioResolver> {
    let mut builder = TokioResolver::builder_tokio().map_err(unreachable)?;
    builder.options_mut().timeout = TIMEOUT / 2;
    builder.options_mut().attempts = 2;

    builder.build().map_err(unreachable)
}

/// The text records of a name; none when the name has none.
async fn text_records(resolver: &TokioResolver, name: &str) -> MailResult<Vec<String>> {
    // The final dot stops the system's search domains from being appended.
    let lookup = match resolver.txt_lookup(format!("{name}.")).await {
        Ok(lookup) => lookup,
        Err(error) if error.is_no_records_found() => return Ok(Vec::new()),
        Err(error) => return Err(unreachable(format!("{name}: {error}"))),
    };

    Ok(lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            // One record can arrive cut in pieces; they belong together.
            RData::TXT(text) => Some(
                text.txt_data
                    .iter()
                    .map(|piece| String::from_utf8_lossy(piece))
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect())
}

/// Whether the domain, or the parent that speaks for it, refuses forged mail.
async fn refuses_forgery(resolver: &TokioResolver, domain: &str) -> MailResult<bool> {
    for (index, candidate) in candidate_domains(domain).iter().enumerate() {
        let records = text_records(resolver, &dmarc_name(candidate)).await?;
        if let Some(record) = records.iter().find(|record| record.contains("DMARC1")) {
            return Ok(dmarc_enforced(record, index > 0));
        }
    }

    Ok(false)
}

/// The address of the domain's logo, from its own record or a parent's.
async fn logo_address(resolver: &TokioResolver, domain: &str) -> MailResult<Option<String>> {
    for candidate in candidate_domains(domain) {
        for record in text_records(resolver, &bimi_name(&candidate)).await? {
            match parse_bimi(&record) {
                BimiRecord::Logo(url) => return Ok(Some(url)),
                BimiRecord::Declined => return Ok(None),
                BimiRecord::Unusable => {}
            }
        }
    }

    Ok(None)
}

async fn download(resolver: &TokioResolver, url: &str) -> MailResult<Option<Vec<u8>>> {
    let Some(host) = logo_host(url) else {
        return Ok(None);
    };

    // The addresses are checked here and the download is tied to exactly
    // these, so the name cannot be pointed somewhere else in between.
    let addresses: Vec<SocketAddr> = resolver
        .lookup_ip(format!("{host}."))
        .await
        .map_err(|error| unreachable(format!("{host}: {error}")))?
        .iter()
        .filter(|ip| is_public_address(*ip))
        .map(|ip| SocketAddr::new(ip, 443))
        .collect();
    if addresses.is_empty() {
        return Ok(None);
    }

    let client = reqwest::Client::builder()
        .tls_backend_preconfigured(tls::client_config()?)
        .https_only(true)
        // A redirect would leave the checked addresses.
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(&host, &addresses)
        .timeout(TIMEOUT)
        .build()
        .map_err(unreachable)?;

    let mut response = client.get(url).send().await.map_err(unreachable)?;
    if !response.status().is_success() {
        return Ok(None);
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(unreachable)? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > MAX_LOGO_BYTES {
            return Ok(None);
        }
    }

    Ok(is_logo(&bytes).then_some(bytes))
}

#[async_trait]
impl LogoSource for NetLogos {
    async fn logo(&self, domain: &str) -> MailResult<Option<Vec<u8>>> {
        let resolver = resolver()?;

        let Some(url) = logo_address(&resolver, domain).await? else {
            return Ok(None);
        };
        if !refuses_forgery(&resolver, domain).await? {
            return Ok(None);
        }

        download(&resolver, &url).await
    }
}

/// Against the real internet. Run with `cargo test -p mail-protocols -- --ignored`.
#[cfg(test)]
mod live_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "needs the internet"]
    async fn a_brand_that_publishes_a_logo_has_one() {
        let logo = NetLogos.logo("revolut.com").await.unwrap();

        assert!(is_logo(&logo.unwrap()));
    }

    #[tokio::test]
    #[ignore = "needs the internet"]
    async fn a_subdomain_gets_the_logo_of_its_brand() {
        let logo = NetLogos.logo("mail.revolut.com").await.unwrap();

        assert!(logo.is_some());
    }

    #[tokio::test]
    #[ignore = "needs the internet"]
    async fn a_domain_without_a_logo_has_none() {
        assert_eq!(NetLogos.logo("example.com").await.unwrap(), None);
    }
}
