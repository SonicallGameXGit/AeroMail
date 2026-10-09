use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::account::AccountSettings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Account {
    #[ts(type = "number")]
    pub id: i64,
    pub settings: AccountSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FolderRole {
    Inbox,
    Drafts,
    Sent,
    Archive,
    Junk,
    Trash,
    All,
    Flagged,
    Other,
}

impl FolderRole {
    /// Fallback for servers that do not announce special folders: the
    /// conventional English names of the last path segment.
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "inbox" => FolderRole::Inbox,
            "drafts" | "draft" => FolderRole::Drafts,
            "sent" | "sent mail" | "sent items" | "sent messages" => FolderRole::Sent,
            "archive" | "archives" => FolderRole::Archive,
            "junk" | "spam" | "junk e-mail" | "junk email" | "bulk mail" => FolderRole::Junk,
            "trash" | "deleted" | "deleted items" | "deleted messages" | "bin" => FolderRole::Trash,
            _ => FolderRole::Other,
        }
    }

    /// Position in the folder list: well-known folders first, in a fixed order.
    pub fn order(self) -> u8 {
        match self {
            FolderRole::Inbox => 0,
            FolderRole::Flagged => 1,
            FolderRole::Drafts => 2,
            FolderRole::Sent => 3,
            FolderRole::Archive => 4,
            FolderRole::All => 5,
            FolderRole::Junk => 6,
            FolderRole::Trash => 7,
            FolderRole::Other => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Folder {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub account_id: i64,
    /// The server's own name of the folder, used in commands.
    pub path: String,
    /// Readable name of the last path segment.
    pub name: String,
    /// Nesting level, 0 for top-level folders.
    pub depth: u32,
    pub role: FolderRole,
    /// False for folders that only contain other folders.
    pub selectable: bool,
    pub unread: u32,
    pub total: u32,
}

/// A person in a message header: optional display name plus address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Contact {
    pub name: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MessageFlags {
    pub seen: bool,
    pub flagged: bool,
    pub answered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageSummary {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub folder_id: i64,
    pub uid: u32,
    pub subject: String,
    pub from: Vec<Contact>,
    pub to: Vec<Contact>,
    pub cc: Vec<Contact>,
    /// Unix time in seconds.
    #[ts(type = "number")]
    pub date: i64,
    pub flags: MessageFlags,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Attachment {
    /// Position among the message's attachments; identifies it when saving.
    pub index: u32,
    pub name: String,
    pub mime: String,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageContent {
    pub summary: MessageSummary,
    pub text: Option<String>,
    /// Already sanitised; still shown isolated from the app.
    pub html: Option<String>,
    pub attachments: Vec<Attachment>,
    /// True when remote images were held back and the user can ask for them.
    pub remote_content_blocked: bool,
    pub message_id: Option<String>,
    pub references: Vec<String>,
    pub reply_to: Vec<Contact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OutgoingMessage {
    #[ts(type = "number")]
    pub account_id: i64,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
}

/// What a notification shows about one new message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewMailItem {
    #[ts(type = "number")]
    pub id: i64,
    pub subject: String,
    pub from: Option<Contact>,
    /// The start of the text, when the notification shows it (see
    /// `NotificationSettings::shows_preview`) and it could be read.
    pub preview: Option<String>,
}

/// Something the engine found out by itself, without being asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum MailEvent {
    /// A folder's cached messages or counts changed; the lists can be read
    /// again from the cache, no server is needed for that.
    FolderChanged {
        #[ts(type = "number")]
        account_id: i64,
        #[ts(type = "number")]
        folder_id: i64,
    },
    /// Unread messages arrived in the inbox (never sent when the user turned
    /// notifications off).
    NewMail {
        #[ts(type = "number")]
        account_id: i64,
        #[ts(type = "number")]
        folder_id: i64,
        /// The account's address, so a notification says where mail arrived.
        account: String,
        messages: Vec<NewMailItem>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_conventional_folder_names() {
        assert_eq!(FolderRole::from_name("INBOX"), FolderRole::Inbox);
        assert_eq!(FolderRole::from_name("Sent Items"), FolderRole::Sent);
        assert_eq!(FolderRole::from_name("Spam"), FolderRole::Junk);
        assert_eq!(FolderRole::from_name("Deleted Messages"), FolderRole::Trash);
        assert_eq!(FolderRole::from_name("Receipts"), FolderRole::Other);
    }
}
