use mail_builder::MessageBuilder;
use mail_builder::headers::address::Address;
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::mail::Contact;

fn strip(id: &str) -> String {
    id.trim_matches(|c| c == '<' || c == '>').to_string()
}

pub struct ComposeInput<'a> {
    pub from: &'a Contact,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub subject: &'a str,
    pub body: &'a str,
    /// Without angle brackets.
    pub message_id: &'a str,
    pub in_reply_to: Option<&'a str>,
    pub references: &'a [String],
    /// Unix seconds.
    pub date: i64,
}

/// Builds the raw message. Bcc recipients are deliberately not part of it:
/// they only exist in the envelope given to the outgoing server.
pub fn compose(input: &ComposeInput<'_>) -> MailResult<Vec<u8>> {
    let from = match input.from.name.as_deref().map(str::trim) {
        Some(name) if !name.is_empty() => {
            Address::from((name.to_string(), input.from.email.clone()))
        }
        _ => Address::from(input.from.email.clone()),
    };

    let mut builder = MessageBuilder::new()
        .from(from)
        .to(input.to.to_vec())
        .subject(input.subject)
        .date(input.date)
        .message_id(strip(input.message_id))
        .text_body(input.body);

    if !input.cc.is_empty() {
        builder = builder.cc(input.cc.to_vec());
    }
    if let Some(in_reply_to) = input.in_reply_to {
        builder = builder.in_reply_to(strip(in_reply_to));
    }
    if !input.references.is_empty() {
        let references: Vec<String> = input.references.iter().map(|id| strip(id)).collect();
        builder = builder.references(references);
    }

    builder
        .write_to_vec()
        .map_err(|error| MailError::new(ErrorKind::Invalid, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{parse_header, render};

    #[test]
    fn composed_message_reads_back_the_same() {
        let from = Contact {
            name: Some("Müller".to_string()),
            email: "me@example.com".to_string(),
        };
        let to = ["ann@example.com".to_string(), "bob@example.com".to_string()];
        let cc = ["carol@example.com".to_string()];
        let references = ["<m0@example.com>".to_string(), "m1@example.com".to_string()];

        let raw = compose(&ComposeInput {
            from: &from,
            to: &to,
            cc: &cc,
            subject: "Привіт — café",
            body: "Line one\nLine two — ünïcode\n",
            message_id: "new@example.com",
            in_reply_to: Some("<m1@example.com>"),
            references: &references,
            date: 1_700_000_000,
        })
        .unwrap();

        let header = parse_header(&raw);
        assert_eq!(header.subject, "Привіт — café");
        assert_eq!(header.from[0].name.as_deref(), Some("Müller"));
        assert_eq!(header.from[0].email, "me@example.com");
        assert_eq!(header.to.len(), 2);
        assert_eq!(header.cc[0].email, "carol@example.com");
        assert_eq!(header.date, Some(1_700_000_000));

        let rendered = render(&raw, false).unwrap();
        assert_eq!(
            rendered.text.as_deref().map(str::trim_end),
            Some("Line one\nLine two — ünïcode")
        );
        assert_eq!(rendered.html, None);
        assert_eq!(rendered.message_id.as_deref(), Some("new@example.com"));
        assert_eq!(
            rendered.references,
            vec!["m0@example.com", "m1@example.com"]
        );
    }

    #[test]
    fn bcc_never_appears_in_the_message() {
        let from = Contact {
            name: None,
            email: "me@example.com".to_string(),
        };
        let to = ["ann@example.com".to_string()];

        let raw = compose(&ComposeInput {
            from: &from,
            to: &to,
            cc: &[],
            subject: "Hi",
            body: "Hello",
            message_id: "x@example.com",
            in_reply_to: None,
            references: &[],
            date: 0,
        })
        .unwrap();

        let text = String::from_utf8(raw).unwrap();
        assert!(!text.to_lowercase().contains("bcc"));
        assert!(text.contains("From: <me@example.com>"));
    }
}
