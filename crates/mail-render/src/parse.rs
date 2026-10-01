use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::mail::{Attachment, Contact};
use mail_parser::{Address, HeaderValue, Message, MessageParser, MessagePart, MimeHeaders};

use crate::html::sanitise;

/// What the message list needs, taken from the header block alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderInfo {
    pub subject: String,
    pub from: Vec<Contact>,
    pub to: Vec<Contact>,
    pub cc: Vec<Contact>,
    /// Unix seconds from the Date header, when present and readable.
    pub date: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub text: Option<String>,
    pub html: Option<String>,
    pub attachments: Vec<Attachment>,
    pub remote_content_blocked: bool,
    pub message_id: Option<String>,
    pub references: Vec<String>,
    pub reply_to: Vec<Contact>,
}

fn unreadable() -> MailError {
    MailError::new(ErrorKind::Protocol, "the message could not be parsed")
}

fn contacts(address: Option<&Address<'_>>) -> Vec<Contact> {
    let Some(address) = address else {
        return Vec::new();
    };

    address
        .iter()
        .filter_map(|entry| {
            let email = entry.address()?.trim();
            if email.is_empty() {
                return None;
            }

            Some(Contact {
                name: entry
                    .name()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string),
                email: email.to_string(),
            })
        })
        .collect()
}

fn id_list(value: &HeaderValue<'_>) -> Vec<String> {
    match value {
        HeaderValue::Text(id) => vec![id.to_string()],
        HeaderValue::TextList(ids) => ids.iter().map(|id| id.to_string()).collect(),
        _ => Vec::new(),
    }
}

pub fn parse_header(raw: &[u8]) -> HeaderInfo {
    let parsed = MessageParser::default().parse(raw);
    let Some(message) = parsed else {
        return HeaderInfo {
            subject: String::new(),
            from: Vec::new(),
            to: Vec::new(),
            cc: Vec::new(),
            date: None,
        };
    };

    HeaderInfo {
        subject: message.subject().unwrap_or_default().trim().to_string(),
        from: contacts(message.from()),
        to: contacts(message.to()),
        cc: contacts(message.cc()),
        date: message.date().map(|date| date.to_timestamp()),
    }
}

fn mime_type(part: &MessagePart<'_>) -> String {
    match part.content_type() {
        Some(content_type) => match &content_type.c_subtype {
            Some(subtype) => format!("{}/{subtype}", content_type.c_type).to_lowercase(),
            None => content_type.c_type.to_lowercase(),
        },
        None => "application/octet-stream".to_string(),
    }
}

fn describe(index: usize, part: &MessagePart<'_>) -> Attachment {
    let name = part
        .attachment_name()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("attachment-{}", index + 1));

    Attachment {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        name,
        mime: mime_type(part),
        size: u32::try_from(part.contents().len()).unwrap_or(u32::MAX),
    }
}

fn inline_images(message: &Message<'_>) -> HashMap<String, String> {
    let mut images = HashMap::new();
    for part in message.attachments() {
        let mime = mime_type(part);
        if !mime.starts_with("image/") {
            continue;
        }
        if let Some(content_id) = part.content_id() {
            let content_id = content_id.trim_matches(|c| c == '<' || c == '>');
            let uri = format!("data:{mime};base64,{}", STANDARD.encode(part.contents()));
            images.insert(content_id.to_string(), uri);
        }
    }

    images
}

/// Prepares a raw message for display. Remote images are held back unless
/// `allow_remote` is set.
pub fn render(raw: &[u8], allow_remote: bool) -> MailResult<Rendered> {
    let message = MessageParser::default().parse(raw).ok_or_else(unreadable)?;

    // mail-parser synthesises the missing alternative; only keep what the
    // sender actually wrote, so a plain-text mail stays plain text.
    let has_html = message
        .html_bodies()
        .any(|part| part.is_content_type("text", "html"));
    let has_text = message
        .text_bodies()
        .any(|part| part.is_content_type("text", "plain"));

    let mut remote_content_blocked = false;
    let html = if has_html {
        message.body_html(0).map(|html| {
            let sanitised = sanitise(&html, &inline_images(&message), allow_remote);
            remote_content_blocked = sanitised.remote_blocked;

            sanitised.html
        })
    } else {
        None
    };

    // The text is always kept, even when it is derived from the HTML:
    // replies quote it.
    let text = message.body_text(0).map(|text| text.into_owned());
    let html = if has_text && !has_html { None } else { html };

    Ok(Rendered {
        text,
        html,
        attachments: message
            .attachments()
            .enumerate()
            .map(|(index, part)| describe(index, part))
            .collect(),
        remote_content_blocked,
        message_id: message.message_id().map(str::to_string),
        references: id_list(message.references()),
        reply_to: contacts(message.reply_to()),
    })
}

/// The attachment at `index` with its bytes.
pub fn attachment(raw: &[u8], index: u32) -> MailResult<(Attachment, Vec<u8>)> {
    let message = MessageParser::default().parse(raw).ok_or_else(unreadable)?;
    let position = usize::try_from(index).map_err(|_| MailError::not_found("attachment"))?;
    let part = message
        .attachments()
        .nth(position)
        .ok_or_else(|| MailError::not_found("attachment"))?;

    Ok((describe(position, part), part.contents().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: &[u8] = b"From: =?UTF-8?B?0JDQvdC90LA=?= <anna@example.com>\r\n\
To: Me <me@example.com>, bob@example.com\r\n\
Cc: carol@example.com\r\n\
Reply-To: list@example.com\r\n\
Subject: =?UTF-8?Q?Caf=C3=A9_at_noon=3F?=\r\n\
Date: Tue, 14 Nov 2023 22:13:20 +0000\r\n\
Message-ID: <m2@example.com>\r\n\
References: <m0@example.com> <m1@example.com>\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
See you there.\r\n";

    const MIXED: &[u8] = b"From: ann@example.com\r\n\
Subject: Report\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"outer\"\r\n\
\r\n\
--outer\r\n\
Content-Type: multipart/related; boundary=\"inner\"\r\n\
\r\n\
--inner\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>Hello <img src=\"cid:logo\"><img src=\"https://tracker.example/p.gif\"></p><script>x()</script>\r\n\
--inner\r\n\
Content-Type: image/png\r\n\
Content-ID: <logo>\r\n\
Content-Transfer-Encoding: base64\r\n\
Content-Disposition: inline\r\n\
\r\n\
iVBORw0KGgo=\r\n\
--inner--\r\n\
--outer\r\n\
Content-Type: application/pdf; name=\"report.pdf\"\r\n\
Content-Disposition: attachment; filename=\"report.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
JVBERi0xLjQ=\r\n\
--outer--\r\n";

    #[test]
    fn header_is_decoded() {
        let header = parse_header(PLAIN);

        assert_eq!(header.subject, "Café at noon?");
        assert_eq!(header.from.len(), 1);
        assert_eq!(header.from[0].name.as_deref(), Some("Анна"));
        assert_eq!(header.from[0].email, "anna@example.com");
        assert_eq!(header.to.len(), 2);
        assert_eq!(header.to[1].name, None);
        assert_eq!(header.to[1].email, "bob@example.com");
        assert_eq!(header.cc[0].email, "carol@example.com");
        assert_eq!(header.date, Some(1_700_000_000));
    }

    #[test]
    fn garbage_header_gives_empty_info_instead_of_failing() {
        let header = parse_header(b"");

        assert_eq!(header.subject, "");
        assert_eq!(header.from, Vec::new());
        assert_eq!(header.date, None);
    }

    #[test]
    fn plain_text_message_stays_plain_text() {
        let rendered = render(PLAIN, false).unwrap();

        assert_eq!(
            rendered.text.as_deref().map(str::trim_end),
            Some("See you there.")
        );
        assert_eq!(rendered.html, None);
        assert_eq!(rendered.attachments, Vec::new());
        assert!(!rendered.remote_content_blocked);
        assert_eq!(rendered.message_id.as_deref(), Some("m2@example.com"));
        assert_eq!(
            rendered.references,
            vec!["m0@example.com", "m1@example.com"]
        );
        assert_eq!(rendered.reply_to[0].email, "list@example.com");
    }

    #[test]
    fn html_message_is_sanitised_with_inline_images_embedded() {
        let rendered = render(MIXED, false).unwrap();
        let html = rendered.html.unwrap();

        assert!(html.contains("Hello"));
        assert!(html.contains("src=\"data:image/png;base64,iVBORw0KGgo=\""));
        assert!(!html.contains("script"));
        assert!(!html.contains("tracker.example"));
        assert!(rendered.remote_content_blocked);
    }

    #[test]
    fn remote_images_show_when_allowed() {
        let rendered = render(MIXED, true).unwrap();

        assert!(
            rendered
                .html
                .unwrap()
                .contains("https://tracker.example/p.gif")
        );
        assert!(!rendered.remote_content_blocked);
    }

    #[test]
    fn attachments_are_listed_and_extracted() {
        let rendered = render(MIXED, false).unwrap();
        let pdf = rendered
            .attachments
            .iter()
            .find(|attachment| attachment.name == "report.pdf")
            .unwrap();
        assert_eq!(pdf.mime, "application/pdf");
        assert_eq!(pdf.size, 8);

        let (info, bytes) = attachment(MIXED, pdf.index).unwrap();
        assert_eq!(info.name, "report.pdf");
        assert_eq!(bytes, b"%PDF-1.4");

        assert_eq!(attachment(MIXED, 99).unwrap_err().kind, ErrorKind::NotFound);
    }
}
