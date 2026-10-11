use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mail_core::category::{Category, Signals, classify};
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
    pub message_id: Option<String>,
    /// The messages this one answers: `In-Reply-To`, then `References`,
    /// each id once.
    pub parents: Vec<String>,
    /// The inbox tab the header points to (`mail_core::category`).
    pub category: Category,
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
            message_id: None,
            parents: Vec::new(),
            category: Category::Primary,
        };
    };

    let mut parents: Vec<String> = Vec::new();
    for id in id_list(message.in_reply_to())
        .into_iter()
        .chain(id_list(message.references()))
    {
        if !parents.contains(&id) {
            parents.push(id);
        }
    }

    let subject = message.subject().unwrap_or_default().trim().to_string();
    let from = contacts(message.from());
    let category = classify(&signals(&message, &subject, &from));

    HeaderInfo {
        subject,
        to: contacts(message.to()),
        cc: contacts(message.cc()),
        date: message.date().map(|date| date.to_timestamp()),
        message_id: message.message_id().map(str::to_string),
        parents,
        category,
        from,
    }
}

/// Headers marketing platforms put on their campaigns (Mailchimp, Brevo,
/// Salesforce Marketing Cloud and the generic ones). Fetched with the list's
/// header fields (`mail-protocols`), so a name added here is added there.
const CAMPAIGN_HEADERS: [&str; 6] = [
    "X-Campaign",
    "X-CampaignID",
    "X-Campaign-ID",
    "X-MC-User",
    "X-Mailin-Campaign",
    "X-SFMC-Stack",
];

/// What the header says about where the message comes from.
fn signals<'a>(message: &'a Message<'a>, subject: &'a str, from: &'a [Contact]) -> Signals<'a> {
    let value = |name: &'static str| message.header_raw(name).map(str::trim);
    let lower = |name: &'static str| value(name).map(str::to_ascii_lowercase);

    Signals {
        sender: from.first().map_or("", |sender| sender.email.as_str()),
        subject,
        list_id: value("List-Id").is_some(),
        // "NO" says the list cannot be written to (RFC 2369).
        list_post: value("List-Post").is_some_and(|post| !post.eq_ignore_ascii_case("no")),
        list_unsubscribe: value("List-Unsubscribe").is_some(),
        bulk: lower("Precedence")
            .is_some_and(|precedence| matches!(precedence.as_str(), "bulk" | "junk")),
        auto_submitted: lower("Auto-Submitted").is_some_and(|kind| kind != "no"),
        campaign: CAMPAIGN_HEADERS.iter().any(|name| value(name).is_some()),
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

/// The most a preview holds; notifications show a part of it.
const PREVIEW_LIMIT: usize = 600;

/// Characters that take no room: newsletters fill their hidden preheader
/// with them so that mail apps show nothing after the visible teaser.
fn is_invisible(character: char) -> bool {
    matches!(
        character,
        '\u{00ad}' | '\u{034f}' | '\u{200b}'..='\u{200f}' | '\u{2060}' | '\u{feff}'
    )
}

/// The start of what the message says, on one line: quoted text and the
/// signature left out, every run of spaces and line breaks made one space.
/// `None` for a message without any text.
pub fn preview(raw: &[u8]) -> Option<String> {
    let message = MessageParser::default().parse(raw)?;
    let text = message.body_text(0)?;

    let mut words: Vec<&str> = Vec::new();
    for line in text.lines() {
        // The usual signature separator; what follows is not the message.
        if line == "-- " {
            break;
        }
        if line.trim_start().starts_with('>') {
            continue;
        }
        words.extend(line.split_whitespace());
    }

    let joined: String = words
        .join(" ")
        .chars()
        .filter(|character| !is_invisible(*character))
        .collect();
    let single: Vec<&str> = joined.split_whitespace().collect();
    let preview: String = single.join(" ").chars().take(PREVIEW_LIMIT).collect();

    Some(preview).filter(|preview| !preview.is_empty())
}

/// Whether a line introduces the quote under it: "On Monday, Ann wrote:".
/// Any line ending in a colon counts, because the wording differs between
/// every mail app and language.
fn is_attribution(line: &str) -> bool {
    line.trim_end().ends_with(':')
}

/// Whether the line at `index` starts an earlier message pasted under the
/// reply by apps that do not quote with `>` (Outlook): their marker line, or
/// a rule of underscores with the earlier message's "From:" right under it.
fn starts_pasted_message(lines: &[&str], index: usize) -> bool {
    let line = lines[index].trim();
    if line.eq_ignore_ascii_case("-----Original Message-----") {
        return true;
    }
    let rule = line.len() >= 10 && line.chars().all(|character| character == '_');
    let next = lines[index + 1..]
        .iter()
        .find(|next| !next.trim().is_empty());

    rule && next.is_some_and(|next| next.trim_start().starts_with("From:"))
}

/// The text without the earlier messages it repeats at its end: the quoted
/// lines (`>`), the line that introduces them, and an earlier message pasted
/// in by apps that do not quote. `None` when nothing is repeated, or when
/// nothing would be left. A quote with the reply written under or between
/// it is not at the end, so that text stays whole.
pub fn without_quote(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let pasted = (0..lines.len()).find(|index| starts_pasted_message(&lines, *index));

    // From the end, past blank and quoted lines.
    let mut start = lines.len();
    while start > 0 {
        let line = lines[start - 1].trim_start();
        if !line.is_empty() && !line.starts_with('>') {
            break;
        }
        start -= 1;
    }
    let quoted = lines[start..]
        .iter()
        .any(|line| line.trim_start().starts_with('>'));
    if quoted {
        while start > 0 && lines[start - 1].trim().is_empty() {
            start -= 1;
        }
        if start > 0 && is_attribution(lines[start - 1]) {
            start -= 1;
        }
    } else {
        start = lines.len();
    }
    let cut = match pasted {
        Some(pasted) => start.min(pasted),
        None => start,
    };
    if cut == lines.len() {
        return None;
    }

    let kept = lines[..cut].join("\n").trim_end().to_string();

    Some(kept).filter(|kept| !kept.trim().is_empty())
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
        assert_eq!(header.message_id.as_deref(), Some("m2@example.com"));
        assert_eq!(header.parents, vec!["m0@example.com", "m1@example.com"]);
    }

    #[test]
    fn the_header_says_which_tab_a_message_belongs_in() {
        assert_eq!(parse_header(PLAIN).category, Category::Primary);

        let list = parse_header(
            b"From: ann@example.com\r\nSubject: Borrowing\r\nList-Id: <rust.lists.example>\r\n\
List-Post: <mailto:rust@lists.example>\r\n\r\n",
        );
        assert_eq!(list.category, Category::Forums);

        let announcements = parse_header(
            b"From: hello@shop.example\r\nSubject: New in\r\nList-Id: <shop>\r\nList-Post: NO\r\n\r\n",
        );
        assert_eq!(announcements.category, Category::Promotions);

        let campaign =
            parse_header(b"From: hello@shop.example\r\nSubject: Sale\r\nx-mc-user: 12ab\r\n\r\n");
        assert_eq!(campaign.category, Category::Promotions);

        let robot = parse_header(
            b"From: ci@build.example\r\nSubject: Build done\r\nAuto-Submitted: auto-generated\r\n\r\n",
        );
        assert_eq!(robot.category, Category::Updates);

        let person = parse_header(
            b"From: ann@example.com\r\nSubject: Hi\r\nAuto-Submitted: no\r\nPrecedence: list\r\n\r\n",
        );
        assert_eq!(person.category, Category::Primary);
    }

    #[test]
    fn the_answered_message_comes_first_and_each_id_once() {
        let header = parse_header(
            b"Subject: Re: Plan\r\nIn-Reply-To: <b@x>\r\nReferences: <a@x> <b@x>\r\n\r\n",
        );

        assert_eq!(header.message_id, None);
        assert_eq!(header.parents, vec!["b@x", "a@x"]);
    }

    #[test]
    fn a_reply_without_its_quote_is_what_was_written() {
        let reply = "Sounds good.\nSee you.\n\nOn Mon, 5 Oct 2026, Ann <ann@x> wrote:\n> Lunch?\n>\n> Ann\n";
        assert_eq!(
            without_quote(reply).as_deref(),
            Some("Sounds good.\nSee you.")
        );

        let outlook =
            "Yes.\n\n________________________________\nFrom: Ann\nSent: Monday\n\nLunch?\n";
        assert_eq!(without_quote(outlook).as_deref(), Some("Yes."));

        let original = "Hi,\n\n-----Original Message-----\nFrom: Ann\n";
        assert_eq!(without_quote(original).as_deref(), Some("Hi,"));
    }

    #[test]
    fn text_that_repeats_nothing_at_its_end_is_left_whole() {
        assert_eq!(without_quote("Just text.\nNothing quoted:\n"), None);

        let interleaved = "> Lunch?\nYes!\n> Where?\nThe usual place.\n";
        assert_eq!(without_quote(interleaved), None);

        assert_eq!(without_quote("Ann wrote:\n> only a quote\n"), None);
        assert_eq!(without_quote(""), None);
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
    fn newsletter_keeps_its_class_based_layout() {
        let raw = include_bytes!("../../../tests/fixtures/class-styled-newsletter.eml");
        let html = render(raw, false).unwrap().html.unwrap();

        assert!(html.contains(".col-pdd{padding:24px 32px}"));
        assert!(html.contains("class=\"col-pdd f-size-22\""));
        assert!(!html.contains("Shop title"));
        assert!(html.contains("Zamawiaj taniej"));
    }

    #[test]
    fn page_styled_on_its_body_keeps_its_page() {
        let raw = include_bytes!("../../../tests/fixtures/dark-aware-page-style.eml");
        let rendered = render(raw, false).unwrap();
        let html = rendered.html.unwrap();

        assert!(html.starts_with(
            "<body style=\"background-color:#f5f7fa;word-spacing: normal; margin: 0;\">"
        ));
        assert!(html.contains("media=\"screen and (min-width:480px)\""));
        // The dark-mode rules stay; the frame tells the message it is light.
        assert!(html.contains("prefers-color-scheme: dark"));
        assert!(html.contains("font-size: 40px; color: #191C1F;\">Build a savings habit</h1>"));
        assert!(html.contains("<img src=\"data:,\" width=\"1\" height=\"1\" alt=\"\">"));
        assert!(rendered.remote_content_blocked);
    }

    #[test]
    fn a_preview_is_the_start_of_the_text_on_one_line() {
        let raw = b"From: a@example.com\r\nSubject: Hi\r\n\r\nHello  there,\r\n\r\nsee you\tat\r\nnoon.\r\n\r\nOn Monday Bob wrote:\r\n> old text\r\n-- \r\nAnn's signature\r\n";

        assert_eq!(
            preview(raw).as_deref(),
            Some("Hello there, see you at noon. On Monday Bob wrote:")
        );
    }

    #[test]
    fn a_preview_of_html_mail_is_its_text() {
        let raw = b"From: a@example.com\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><head><style>p { color: red }</style></head><body><p>Your\xe2\x80\x8c \xcd\x8f order</p><p>has <b>shipped</b></p></body></html>\r\n";

        assert_eq!(preview(raw).as_deref(), Some("Your order has shipped"));
    }

    #[test]
    fn a_message_without_text_has_no_preview() {
        let raw = b"From: a@example.com\r\nSubject: Empty\r\n\r\n \r\n";

        assert_eq!(preview(raw), None);
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
