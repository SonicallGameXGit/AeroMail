//! Which messages belong to one conversation. Plain rules over what a
//! message says about itself; the engine applies them to the cached mail.
//!
//! Two rules, both easy to predict:
//! - a reply belongs with what it answers: messages are joined when one
//!   names another (`Message-ID` in `In-Reply-To` / `References`), or when
//!   both answer the same message;
//! - optionally, messages from the same sender with the same subject (once
//!   "Re:" and the like are taken off) belong together, which gathers
//!   repeated notifications that carry no reply headers.

use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;

use crate::category::Category;
use crate::mail::{Contact, MessageSummary};

/// One row of the message list: a conversation, or a single message when
/// grouping is off or nothing else belongs with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Conversation {
    /// The newest of `message_ids`: what the row shows and what opens it.
    pub latest: MessageSummary,
    /// The conversation's messages in the listed folder, newest first:
    /// what moving, deleting or marking the whole row applies to.
    #[ts(type = "Array<number>")]
    pub message_ids: Vec<i64>,
    /// Every message of the conversation the app has, including the ones
    /// kept in other folders (the user's own replies in Sent).
    pub count: u32,
    /// How many of `message_ids` are unread.
    pub unread: u32,
    /// Whether any of `message_ids` is flagged.
    pub flagged: bool,
    /// Who wrote in it, each once, in the order they first wrote.
    pub senders: Vec<Contact>,
    /// The inbox tab it is shown in, decided by its newest message here;
    /// `None` outside the inbox or while the inbox is not sorted.
    pub category: Option<Category>,
}

/// What a message says about where it belongs.
#[derive(Debug, Clone, Copy)]
pub struct Thread<'a> {
    pub message_id: Option<&'a str>,
    /// The messages it answers: `In-Reply-To` and `References`.
    pub parents: &'a [String],
    pub subject: &'a str,
    pub sender: Option<&'a str>,
}

/// Words that mark a reply or a forward, in the languages mail is commonly
/// written in. Compared without case.
const PREFIXES: [&str; 18] = [
    "re",
    "fw",
    "fwd",
    "aw",
    "wg",
    "sv",
    "vs",
    "antw",
    "odp",
    "rv",
    "tr",
    "enc",
    "res",
    "ynt",
    "отв",
    "ответ",
    "пересл",
    "відп",
];

/// The subject without the "Re:" / "Fwd:" marks in front of it, its spaces
/// made single and its letters small. Counted marks like "Re[2]:" count too.
pub fn base_subject(subject: &str) -> String {
    let mut rest = subject.trim();
    while let Some(colon) = rest.find(':') {
        let word = rest[..colon]
            .trim()
            .trim_end_matches(|c: char| c.is_ascii_digit() || matches!(c, '[' | ']' | '(' | ')'))
            .to_lowercase();
        if !PREFIXES.contains(&word.as_str()) {
            break;
        }
        rest = rest[colon + 1..].trim_start();
    }

    rest.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A message id the way two headers can be compared: without the angle
/// brackets some headers keep around it.
fn id_key(id: &str) -> &str {
    id.trim().trim_start_matches('<').trim_end_matches('>')
}

fn root(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }

    index
}

/// The lower index stays the root, so a group is named by its first member.
fn join(parents: &mut [usize], a: usize, b: usize) {
    let (a, b) = (root(parents, a), root(parents, b));
    if a != b {
        parents[a.max(b)] = a.min(b);
    }
}

/// Joins `index` with the first message that had the same `key`.
fn link(seen: &mut HashMap<String, usize>, parents: &mut [usize], key: String, index: usize) {
    match seen.get(&key) {
        Some(&first) => join(parents, index, first),
        None => {
            seen.insert(key, index);
        }
    }
}

/// The conversations among `threads`, as positions into it. Each group is
/// in ascending order, and the groups are in the order of their first
/// message, so the order of `threads` decides the order of everything.
pub fn group(threads: &[Thread<'_>], same_subject: bool) -> Vec<Vec<usize>> {
    let mut parents: Vec<usize> = (0..threads.len()).collect();
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut subjects: HashMap<String, usize> = HashMap::new();

    for (index, thread) in threads.iter().enumerate() {
        let named = thread
            .message_id
            .into_iter()
            .chain(thread.parents.iter().map(String::as_str))
            .map(id_key)
            .filter(|id| !id.is_empty());
        for id in named {
            link(&mut ids, &mut parents, id.to_string(), index);
        }

        if !same_subject {
            continue;
        }
        let subject = base_subject(thread.subject);
        if let Some(sender) = thread.sender
            && !subject.is_empty()
        {
            let key = format!("{}\n{subject}", sender.trim().to_lowercase());
            link(&mut subjects, &mut parents, key, index);
        }
    }

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut positions: HashMap<usize, usize> = HashMap::new();
    for index in 0..threads.len() {
        let first = root(&mut parents, index);
        match positions.get(&first) {
            Some(&position) => groups[position].push(index),
            None => {
                positions.insert(first, groups.len());
                groups.push(vec![index]);
            }
        }
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mail {
        id: &'static str,
        parents: Vec<String>,
        subject: &'static str,
        sender: &'static str,
    }

    fn mail(
        id: &'static str,
        parents: &[&str],
        subject: &'static str,
        sender: &'static str,
    ) -> Mail {
        Mail {
            id,
            parents: parents.iter().map(|parent| parent.to_string()).collect(),
            subject,
            sender,
        }
    }

    fn grouped(mails: &[Mail], same_subject: bool) -> Vec<Vec<usize>> {
        let threads: Vec<Thread<'_>> = mails
            .iter()
            .map(|mail| Thread {
                message_id: Some(mail.id).filter(|id| !id.is_empty()),
                parents: &mail.parents,
                subject: mail.subject,
                sender: Some(mail.sender),
            })
            .collect();

        group(&threads, same_subject)
    }

    #[test]
    fn reply_marks_are_taken_off_the_subject() {
        assert_eq!(base_subject("Re: Lunch?"), "lunch?");
        assert_eq!(
            base_subject("RE: Fwd:  re:Lunch   on  Friday"),
            "lunch on friday"
        );
        assert_eq!(base_subject("Re[2]: AW: Lunch"), "lunch");
        assert_eq!(base_subject("Ответ: Обед"), "обед");
        assert_eq!(base_subject("Meeting: notes"), "meeting: notes");
        assert_eq!(base_subject("Re:"), "");
        assert_eq!(base_subject("Reply: soon"), "reply: soon");
    }

    #[test]
    fn a_reply_joins_what_it_answers_in_any_order() {
        let mails = [
            mail("c@x", &["a@x", "b@x"], "Re: Lunch", "ann@x"),
            mail("other@x", &[], "Unrelated", "bob@x"),
            mail("a@x", &[], "Lunch", "me@x"),
            mail("b@x", &["<a@x>"], "Re: Lunch", "ann@x"),
        ];

        assert_eq!(grouped(&mails, false), vec![vec![0, 2, 3], vec![1]]);
    }

    #[test]
    fn two_answers_to_a_message_that_is_not_here_still_belong_together() {
        let mails = [
            mail("b@x", &["a@x"], "Re: Plan", "ann@x"),
            mail("c@x", &["a@x"], "Re: Plan", "bob@x"),
        ];

        assert_eq!(grouped(&mails, false), vec![vec![0, 1]]);
    }

    #[test]
    fn the_same_subject_joins_only_the_same_sender_and_only_when_asked() {
        let mails = [
            mail("1@x", &[], "Your code", "bank@x"),
            mail("2@x", &[], "Re: your  CODE", "BANK@x"),
            mail("3@x", &[], "Your code", "shop@x"),
            mail("4@x", &[], "", "bank@x"),
            mail("5@x", &[], "", "bank@x"),
        ];

        assert_eq!(
            grouped(&mails, false),
            vec![vec![0], vec![1], vec![2], vec![3], vec![4]]
        );
        assert_eq!(
            grouped(&mails, true),
            vec![vec![0, 1], vec![2], vec![3], vec![4]]
        );
    }

    #[test]
    fn messages_without_any_ids_stay_alone() {
        let mails = [mail("", &[], "Hi", "a@x"), mail("", &[" "], "Hi", "b@x")];

        assert_eq!(grouped(&mails, false), vec![vec![0], vec![1]]);
        assert_eq!(grouped(&[], true), Vec::<Vec<usize>>::new());
    }
}
