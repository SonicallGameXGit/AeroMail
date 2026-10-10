//! Builds the starting point of a reply or a forward from an opened message.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::mail::{Contact, MessageContent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ReplyMode {
    Reply,
    ReplyAll,
    Forward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Draft {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
}

pub fn reply_draft(message: &MessageContent, own_email: &str, mode: ReplyMode) -> Draft {
    let summary = &message.summary;
    let is_own = |contact: &&Contact| contact.email.eq_ignore_ascii_case(own_email);

    let (to, cc) = match mode {
        ReplyMode::Forward => (Vec::new(), Vec::new()),
        ReplyMode::Reply => (emails(reply_targets(message).iter()), Vec::new()),
        ReplyMode::ReplyAll => {
            let to = reply_targets(message);
            let cc = summary
                .to
                .iter()
                .chain(summary.cc.iter())
                .filter(|contact| !is_own(contact))
                .filter(|contact| {
                    !to.iter()
                        .any(|target| target.email.eq_ignore_ascii_case(&contact.email))
                });

            (emails(to.iter()), emails(cc))
        }
    };

    let (prefix, threaded) = match mode {
        ReplyMode::Forward => ("Fwd: ", false),
        ReplyMode::Reply | ReplyMode::ReplyAll => ("Re: ", true),
    };

    let mut references = Vec::new();
    if threaded {
        references.extend(message.references.iter().cloned());
        references.extend(message.message_id.iter().cloned());
    }

    Draft {
        to,
        cc,
        subject: prefixed(&summary.subject, prefix),
        body: quoted_body(message, mode),
        in_reply_to: message.message_id.clone().filter(|_| threaded),
        references,
    }
}

fn reply_targets(message: &MessageContent) -> &[Contact] {
    if message.reply_to.is_empty() {
        &message.summary.from
    } else {
        &message.reply_to
    }
}

fn emails<'a>(contacts: impl Iterator<Item = &'a Contact>) -> Vec<String> {
    let mut emails: Vec<String> = Vec::new();
    for contact in contacts {
        if !emails
            .iter()
            .any(|email| email.eq_ignore_ascii_case(&contact.email))
        {
            emails.push(contact.email.clone());
        }
    }

    emails
}

/// Adds "Re: " / "Fwd: " unless the subject already starts with it.
fn prefixed(subject: &str, prefix: &str) -> String {
    let trimmed = subject.trim();
    let already = trimmed
        .get(..prefix.trim_end().len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix.trim_end()));
    if already {
        return trimmed.to_string();
    }

    format!("{prefix}{trimmed}")
}

fn display(contacts: &[Contact]) -> String {
    contacts
        .iter()
        .map(|contact| match &contact.name {
            Some(name) => format!("{name} <{}>", contact.email),
            None => contact.email.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn quoted_body(message: &MessageContent, mode: ReplyMode) -> String {
    let summary = &message.summary;
    let original = message.text.as_deref().unwrap_or_default().trim_end();

    match mode {
        ReplyMode::Forward => format!(
            "\n\n---------- Forwarded message ----------\nFrom: {}\nTo: {}\nSubject: {}\n\n{original}\n",
            display(&summary.from),
            display(&summary.to),
            summary.subject,
        ),
        ReplyMode::Reply | ReplyMode::ReplyAll => {
            let quoted: Vec<String> = original
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        ">".to_string()
                    } else {
                        format!("> {line}")
                    }
                })
                .collect();

            format!(
                "\n\n{} wrote:\n{}\n",
                display(&summary.from),
                quoted.join("\n")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::{MessageFlags, MessageSummary};

    fn contact(name: Option<&str>, email: &str) -> Contact {
        Contact {
            name: name.map(str::to_string),
            email: email.to_string(),
        }
    }

    fn message() -> MessageContent {
        MessageContent {
            summary: MessageSummary {
                id: 1,
                folder_id: 1,
                uid: 7,
                subject: "Lunch?".to_string(),
                from: vec![contact(Some("Ann"), "ann@example.com")],
                to: vec![
                    contact(None, "me@example.com"),
                    contact(None, "bob@example.com"),
                ],
                cc: vec![contact(None, "carol@example.com")],
                date: 0,
                flags: MessageFlags::default(),
                size: 0,
            },
            text: Some("Hi,\n\nNoon?\n".to_string()),
            text_without_quote: None,
            html: None,
            attachments: Vec::new(),
            remote_content_blocked: false,
            message_id: Some("<a1@example.com>".to_string()),
            references: vec!["<a0@example.com>".to_string()],
            reply_to: Vec::new(),
        }
    }

    #[test]
    fn reply_goes_to_the_sender_and_keeps_the_thread() {
        let draft = reply_draft(&message(), "me@example.com", ReplyMode::Reply);

        assert_eq!(draft.to, vec!["ann@example.com"]);
        assert_eq!(draft.cc, Vec::<String>::new());
        assert_eq!(draft.subject, "Re: Lunch?");
        assert_eq!(draft.in_reply_to.as_deref(), Some("<a1@example.com>"));
        assert_eq!(
            draft.references,
            vec!["<a0@example.com>", "<a1@example.com>"]
        );
        assert_eq!(
            draft.body,
            "\n\nAnn <ann@example.com> wrote:\n> Hi,\n>\n> Noon?\n"
        );
    }

    #[test]
    fn reply_all_copies_everyone_except_me() {
        let draft = reply_draft(&message(), "ME@example.com", ReplyMode::ReplyAll);

        assert_eq!(draft.to, vec!["ann@example.com"]);
        assert_eq!(draft.cc, vec!["bob@example.com", "carol@example.com"]);
    }

    #[test]
    fn reply_prefers_reply_to() {
        let mut message = message();
        message.reply_to = vec![contact(None, "list@example.com")];

        let draft = reply_draft(&message, "me@example.com", ReplyMode::Reply);
        assert_eq!(draft.to, vec!["list@example.com"]);
    }

    #[test]
    fn subject_prefix_is_not_repeated() {
        let mut message = message();
        message.summary.subject = "RE: Lunch?".to_string();

        let draft = reply_draft(&message, "me@example.com", ReplyMode::Reply);
        assert_eq!(draft.subject, "RE: Lunch?");
    }

    #[test]
    fn forward_has_no_recipients_and_starts_a_new_thread() {
        let draft = reply_draft(&message(), "me@example.com", ReplyMode::Forward);

        assert_eq!(draft.to, Vec::<String>::new());
        assert_eq!(draft.subject, "Fwd: Lunch?");
        assert_eq!(draft.in_reply_to, None);
        assert_eq!(draft.references, Vec::<String>::new());
        assert!(
            draft
                .body
                .contains("---------- Forwarded message ----------")
        );
        assert!(draft.body.contains("From: Ann <ann@example.com>"));
        assert!(draft.body.contains("Noon?"));
    }
}
