//! SQLite storage: accounts, folders, the local cache of messages, settings
//! and the sender pictures that were looked up.
//!
//! Secrets never come here; they live in the OS keychain.

use std::path::Path;

use mail_core::account::AccountSettings;
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::mail::{Account, Contact, Folder, FolderRole, MessageFlags, MessageSummary};
use mail_core::transport::RemoteFolder;
use rusqlite::{Connection, OptionalExtension, Row, params};

// Ids are never reused (AUTOINCREMENT): a stale id held by the UI or a
// keychain entry must not start pointing at something else.
const SCHEMA: &str = "
    CREATE TABLE accounts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        settings TEXT NOT NULL
    );

    CREATE TABLE folders (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
        path TEXT NOT NULL,
        name TEXT NOT NULL,
        depth INTEGER NOT NULL,
        role TEXT NOT NULL,
        selectable INTEGER NOT NULL,
        unread INTEGER NOT NULL,
        total INTEGER NOT NULL,
        uid_validity INTEGER,
        UNIQUE (account_id, path)
    );

    CREATE TABLE messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
        uid INTEGER NOT NULL,
        subject TEXT NOT NULL,
        sender TEXT NOT NULL,
        recipients TEXT NOT NULL,
        cc TEXT NOT NULL,
        date INTEGER NOT NULL,
        seen INTEGER NOT NULL,
        flagged INTEGER NOT NULL,
        answered INTEGER NOT NULL,
        size INTEGER NOT NULL,
        raw BLOB,
        UNIQUE (folder_id, uid)
    );
";

// Added later; a database made before them gets them on its next start.
const SCHEMA_2: &str = "
    CREATE TABLE settings (
        name TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE logos (
        domain TEXT PRIMARY KEY,
        svg BLOB,
        checked INTEGER NOT NULL
    );
";

/// What was found out about a sender domain's logo, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedLogo {
    /// `None` when the domain was asked and has no logo.
    pub svg: Option<Vec<u8>>,
    /// Unix time in seconds of the lookup.
    pub checked: i64,
}

/// A message as it arrives from the server, before it has a local id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMessage {
    pub uid: u32,
    pub subject: String,
    pub from: Vec<Contact>,
    pub to: Vec<Contact>,
    pub cc: Vec<Contact>,
    pub date: i64,
    pub flags: MessageFlags,
    pub size: u32,
}

pub struct Store {
    connection: Connection,
}

fn storage(error: rusqlite::Error) -> MailError {
    MailError::new(ErrorKind::Storage, error.to_string())
}

fn json(error: serde_json::Error) -> MailError {
    MailError::new(ErrorKind::Storage, error.to_string())
}

fn role_to_text(role: FolderRole) -> MailResult<String> {
    serde_json::to_string(&role).map_err(json)
}

fn conversion(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn contacts(row: &Row<'_>, column: &str) -> rusqlite::Result<Vec<Contact>> {
    let text: String = row.get(column)?;

    serde_json::from_str(&text).map_err(conversion)
}

fn folder_from_row(row: &Row<'_>) -> rusqlite::Result<Folder> {
    let role: String = row.get("role")?;

    Ok(Folder {
        id: row.get("id")?,
        account_id: row.get("account_id")?,
        path: row.get("path")?,
        name: row.get("name")?,
        depth: row.get("depth")?,
        role: serde_json::from_str(&role).map_err(conversion)?,
        selectable: row.get("selectable")?,
        unread: row.get("unread")?,
        total: row.get("total")?,
    })
}

fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<MessageSummary> {
    Ok(MessageSummary {
        id: row.get("id")?,
        folder_id: row.get("folder_id")?,
        uid: row.get("uid")?,
        subject: row.get("subject")?,
        from: contacts(row, "sender")?,
        to: contacts(row, "recipients")?,
        cc: contacts(row, "cc")?,
        date: row.get("date")?,
        flags: MessageFlags {
            seen: row.get("seen")?,
            flagged: row.get("flagged")?,
            answered: row.get("answered")?,
        },
        size: row.get("size")?,
    })
}

const SUMMARY_COLUMNS: &str =
    "id, folder_id, uid, subject, sender, recipients, cc, date, seen, flagged, answered, size";

impl Store {
    pub fn open(path: &Path) -> MailResult<Self> {
        Self::prepare(Connection::open(path).map_err(storage)?)
    }

    pub fn open_in_memory() -> MailResult<Self> {
        Self::prepare(Connection::open_in_memory().map_err(storage)?)
    }

    fn prepare(connection: Connection) -> MailResult<Self> {
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(storage)?;

        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage)?;
        if version == 0 {
            connection.execute_batch(SCHEMA).map_err(storage)?;
        }
        if version < 2 {
            connection.execute_batch(SCHEMA_2).map_err(storage)?;
            connection
                .pragma_update(None, "user_version", 2)
                .map_err(storage)?;
        }

        Ok(Self { connection })
    }

    // Accounts

    pub fn insert_account(&self, settings: &AccountSettings) -> MailResult<Account> {
        let text = serde_json::to_string(settings).map_err(json)?;
        self.connection
            .execute("INSERT INTO accounts (settings) VALUES (?1)", params![text])
            .map_err(storage)?;

        Ok(Account {
            id: self.connection.last_insert_rowid(),
            settings: settings.clone(),
        })
    }

    pub fn accounts(&self) -> MailResult<Vec<Account>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, settings FROM accounts ORDER BY id")
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?;

        let mut accounts = Vec::new();
        for row in rows {
            let (id, text) = row.map_err(storage)?;
            accounts.push(Account {
                id,
                settings: serde_json::from_str(&text).map_err(json)?,
            });
        }

        Ok(accounts)
    }

    pub fn account(&self, id: i64) -> MailResult<Account> {
        self.accounts()?
            .into_iter()
            .find(|account| account.id == id)
            .ok_or_else(|| MailError::not_found("account"))
    }

    /// Removes the account with its folders and cached messages.
    pub fn delete_account(&self, id: i64) -> MailResult<()> {
        self.connection
            .execute("DELETE FROM accounts WHERE id = ?1", params![id])
            .map_err(storage)?;

        Ok(())
    }

    // Folders

    /// Makes the stored folder list match the server's: new folders are added,
    /// known ones updated in place (keeping their id and cache), gone ones removed.
    pub fn replace_folders(&mut self, account_id: i64, remote: &[RemoteFolder]) -> MailResult<()> {
        let transaction = self.connection.transaction().map_err(storage)?;

        let known: Vec<String> = {
            let mut statement = transaction
                .prepare("SELECT path FROM folders WHERE account_id = ?1")
                .map_err(storage)?;
            let rows = statement
                .query_map(params![account_id], |row| row.get(0))
                .map_err(storage)?;
            rows.collect::<rusqlite::Result<_>>().map_err(storage)?
        };

        for path in &known {
            if !remote.iter().any(|folder| &folder.path == path) {
                transaction
                    .execute(
                        "DELETE FROM folders WHERE account_id = ?1 AND path = ?2",
                        params![account_id, path],
                    )
                    .map_err(storage)?;
            }
        }

        for folder in remote {
            transaction
                .execute(
                    "INSERT INTO folders (account_id, path, name, depth, role, selectable, unread, total)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT (account_id, path) DO UPDATE SET
                         name = excluded.name,
                         depth = excluded.depth,
                         role = excluded.role,
                         selectable = excluded.selectable,
                         unread = excluded.unread,
                         total = excluded.total",
                    params![
                        account_id,
                        folder.path,
                        folder.name,
                        folder.depth,
                        role_to_text(folder.role)?,
                        folder.selectable,
                        folder.unread,
                        folder.total,
                    ],
                )
                .map_err(storage)?;
        }

        transaction.commit().map_err(storage)
    }

    pub fn folders(&self, account_id: i64) -> MailResult<Vec<Folder>> {
        let mut statement = self
            .connection
            .prepare("SELECT * FROM folders WHERE account_id = ?1")
            .map_err(storage)?;
        let rows = statement
            .query_map(params![account_id], folder_from_row)
            .map_err(storage)?;
        let mut folders: Vec<Folder> = rows.collect::<rusqlite::Result<_>>().map_err(storage)?;

        // Well-known folders first; everything else alphabetically by path so
        // that children follow their parent.
        folders.sort_by(|a, b| {
            let key = |folder: &Folder| {
                let order = if folder.depth == 0 {
                    folder.role.order()
                } else {
                    FolderRole::Other.order()
                };
                (order, folder.path.to_lowercase())
            };
            key(a).cmp(&key(b))
        });

        Ok(folders)
    }

    pub fn folder(&self, id: i64) -> MailResult<Folder> {
        self.connection
            .query_row(
                "SELECT * FROM folders WHERE id = ?1",
                params![id],
                folder_from_row,
            )
            .optional()
            .map_err(storage)?
            .ok_or_else(|| MailError::not_found("folder"))
    }

    pub fn set_folder_counts(&self, folder_id: i64, unread: u32, total: u32) -> MailResult<()> {
        self.connection
            .execute(
                "UPDATE folders SET unread = ?2, total = ?3 WHERE id = ?1",
                params![folder_id, unread, total],
            )
            .map_err(storage)?;

        Ok(())
    }

    /// Records the folder's UID validity. When it differs from the stored
    /// one, the cached messages are meaningless and are dropped.
    pub fn set_uid_validity(&self, folder_id: i64, uid_validity: u32) -> MailResult<()> {
        let stored: Option<u32> = self
            .connection
            .query_row(
                "SELECT uid_validity FROM folders WHERE id = ?1",
                params![folder_id],
                |row| row.get(0),
            )
            .map_err(storage)?;

        if stored.is_some_and(|stored| stored != uid_validity) {
            self.connection
                .execute(
                    "DELETE FROM messages WHERE folder_id = ?1",
                    params![folder_id],
                )
                .map_err(storage)?;
        }

        self.connection
            .execute(
                "UPDATE folders SET uid_validity = ?2 WHERE id = ?1",
                params![folder_id, uid_validity],
            )
            .map_err(storage)?;

        Ok(())
    }

    // Messages

    /// UIDs of the cached messages of a folder, ascending.
    pub fn uids(&self, folder_id: i64) -> MailResult<Vec<u32>> {
        let mut statement = self
            .connection
            .prepare("SELECT uid FROM messages WHERE folder_id = ?1 ORDER BY uid")
            .map_err(storage)?;
        let rows = statement
            .query_map(params![folder_id], |row| row.get(0))
            .map_err(storage)?;

        rows.collect::<rusqlite::Result<_>>().map_err(storage)
    }

    pub fn upsert_message(&self, folder_id: i64, message: &NewMessage) -> MailResult<()> {
        self.connection
            .execute(
                "INSERT INTO messages
                     (folder_id, uid, subject, sender, recipients, cc, date, seen, flagged, answered, size)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT (folder_id, uid) DO UPDATE SET
                     seen = excluded.seen,
                     flagged = excluded.flagged,
                     answered = excluded.answered",
                params![
                    folder_id,
                    message.uid,
                    message.subject,
                    serde_json::to_string(&message.from).map_err(json)?,
                    serde_json::to_string(&message.to).map_err(json)?,
                    serde_json::to_string(&message.cc).map_err(json)?,
                    message.date,
                    message.flags.seen,
                    message.flags.flagged,
                    message.flags.answered,
                    message.size,
                ],
            )
            .map_err(storage)?;

        Ok(())
    }

    pub fn set_flags(&self, folder_id: i64, uid: u32, flags: MessageFlags) -> MailResult<()> {
        self.connection
            .execute(
                "UPDATE messages SET seen = ?3, flagged = ?4, answered = ?5
                 WHERE folder_id = ?1 AND uid = ?2",
                params![folder_id, uid, flags.seen, flags.flagged, flags.answered],
            )
            .map_err(storage)?;

        Ok(())
    }

    pub fn delete_uids(&self, folder_id: i64, uids: &[u32]) -> MailResult<()> {
        for uid in uids {
            self.connection
                .execute(
                    "DELETE FROM messages WHERE folder_id = ?1 AND uid = ?2",
                    params![folder_id, uid],
                )
                .map_err(storage)?;
        }

        Ok(())
    }

    /// Cached messages of a folder, newest first.
    pub fn messages(&self, folder_id: i64) -> MailResult<Vec<MessageSummary>> {
        let sql = format!(
            "SELECT {SUMMARY_COLUMNS} FROM messages WHERE folder_id = ?1 ORDER BY uid DESC"
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage)?;
        let rows = statement
            .query_map(params![folder_id], summary_from_row)
            .map_err(storage)?;

        rows.collect::<rusqlite::Result<_>>().map_err(storage)
    }

    pub fn message(&self, id: i64) -> MailResult<MessageSummary> {
        let sql = format!("SELECT {SUMMARY_COLUMNS} FROM messages WHERE id = ?1");
        self.connection
            .query_row(&sql, params![id], summary_from_row)
            .optional()
            .map_err(storage)?
            .ok_or_else(|| MailError::not_found("message"))
    }

    /// The raw message, when its body was downloaded before.
    pub fn raw(&self, id: i64) -> MailResult<Option<Vec<u8>>> {
        self.connection
            .query_row(
                "SELECT raw FROM messages WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or_else(|| MailError::not_found("message"))
    }

    pub fn set_raw(&self, id: i64, raw: &[u8]) -> MailResult<()> {
        self.connection
            .execute(
                "UPDATE messages SET raw = ?2 WHERE id = ?1",
                params![id, raw],
            )
            .map_err(storage)?;

        Ok(())
    }

    // Settings

    pub fn setting(&self, name: &str) -> MailResult<Option<String>> {
        self.connection
            .query_row(
                "SELECT value FROM settings WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)
    }

    pub fn set_setting(&self, name: &str, value: &str) -> MailResult<()> {
        self.connection
            .execute(
                "INSERT INTO settings (name, value) VALUES (?1, ?2)
                 ON CONFLICT (name) DO UPDATE SET value = excluded.value",
                params![name, value],
            )
            .map_err(storage)?;

        Ok(())
    }

    // Sender pictures

    pub fn logo(&self, domain: &str) -> MailResult<Option<CachedLogo>> {
        self.connection
            .query_row(
                "SELECT svg, checked FROM logos WHERE domain = ?1",
                params![domain],
                |row| {
                    Ok(CachedLogo {
                        svg: row.get(0)?,
                        checked: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(storage)
    }

    /// Remembers the answer for a domain, replacing an older one.
    pub fn put_logo(&self, domain: &str, svg: Option<&[u8]>, checked: i64) -> MailResult<()> {
        self.connection
            .execute(
                "INSERT INTO logos (domain, svg, checked) VALUES (?1, ?2, ?3)
                 ON CONFLICT (domain) DO UPDATE SET svg = excluded.svg, checked = excluded.checked",
                params![domain, svg, checked],
            )
            .map_err(storage)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mail_core::account::{AuthMethod, Protocol, Security, ServerSettings};

    use super::*;

    fn settings() -> AccountSettings {
        let server = |protocol, host: &str, port| ServerSettings {
            protocol,
            host: host.to_string(),
            port,
            security: Security::Tls,
            auth: AuthMethod::Password,
            username: "me@example.com".to_string(),
        };

        AccountSettings {
            email: "me@example.com".to_string(),
            display_name: "Me".to_string(),
            incoming: server(Protocol::Imap, "imap.example.com", 993),
            outgoing: server(Protocol::Smtp, "smtp.example.com", 465),
        }
    }

    fn remote(path: &str, role: FolderRole) -> RemoteFolder {
        RemoteFolder {
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap().to_string(),
            depth: path.matches('/').count() as u32,
            role,
            selectable: true,
            unread: 1,
            total: 2,
        }
    }

    fn new_message(uid: u32) -> NewMessage {
        NewMessage {
            uid,
            subject: format!("Message {uid}"),
            from: vec![Contact {
                name: Some("Ann".to_string()),
                email: "ann@example.com".to_string(),
            }],
            to: Vec::new(),
            cc: Vec::new(),
            date: 1_700_000_000 + i64::from(uid),
            flags: MessageFlags::default(),
            size: 100,
        }
    }

    fn store_with_inbox() -> (Store, Folder) {
        let mut store = Store::open_in_memory().unwrap();
        let account = store.insert_account(&settings()).unwrap();
        store
            .replace_folders(account.id, &[remote("INBOX", FolderRole::Inbox)])
            .unwrap();
        let inbox = store.folders(account.id).unwrap().remove(0);

        (store, inbox)
    }

    #[test]
    fn accounts_round_trip() {
        let store = Store::open_in_memory().unwrap();
        let account = store.insert_account(&settings()).unwrap();

        assert_eq!(store.accounts().unwrap(), vec![account.clone()]);
        assert_eq!(store.account(account.id).unwrap(), account);
        assert_eq!(store.account(99).unwrap_err().kind, ErrorKind::NotFound);
    }

    #[test]
    fn folders_are_ordered_and_keep_their_id_across_refreshes() {
        let mut store = Store::open_in_memory().unwrap();
        let account = store.insert_account(&settings()).unwrap();
        let first = [
            remote("Work", FolderRole::Other),
            remote("Trash", FolderRole::Trash),
            remote("INBOX", FolderRole::Inbox),
            remote("Work/Bills", FolderRole::Other),
        ];
        store.replace_folders(account.id, &first).unwrap();

        let folders = store.folders(account.id).unwrap();
        let paths: Vec<&str> = folders.iter().map(|folder| folder.path.as_str()).collect();
        assert_eq!(paths, vec!["INBOX", "Trash", "Work", "Work/Bills"]);
        let inbox_id = folders[0].id;

        let second = [
            remote("INBOX", FolderRole::Inbox),
            remote("Sent", FolderRole::Sent),
        ];
        store.replace_folders(account.id, &second).unwrap();

        let folders = store.folders(account.id).unwrap();
        let paths: Vec<&str> = folders.iter().map(|folder| folder.path.as_str()).collect();
        assert_eq!(paths, vec!["INBOX", "Sent"]);
        assert_eq!(folders[0].id, inbox_id);
    }

    #[test]
    fn messages_are_listed_newest_first_and_updated_in_place() {
        let (store, inbox) = store_with_inbox();
        store.upsert_message(inbox.id, &new_message(1)).unwrap();
        store.upsert_message(inbox.id, &new_message(2)).unwrap();

        let mut seen = new_message(1);
        seen.flags.seen = true;
        store.upsert_message(inbox.id, &seen).unwrap();

        let messages = store.messages(inbox.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].uid, 2);
        assert_eq!(messages[1].uid, 1);
        assert!(messages[1].flags.seen);
        assert_eq!(messages[1].from[0].name.as_deref(), Some("Ann"));
        assert_eq!(store.uids(inbox.id).unwrap(), vec![1, 2]);
    }

    #[test]
    fn raw_body_is_cached_separately() {
        let (store, inbox) = store_with_inbox();
        store.upsert_message(inbox.id, &new_message(1)).unwrap();
        let id = store.messages(inbox.id).unwrap()[0].id;

        assert_eq!(store.raw(id).unwrap(), None);
        store.set_raw(id, b"raw message").unwrap();
        assert_eq!(store.raw(id).unwrap().as_deref(), Some(&b"raw message"[..]));
        assert_eq!(store.raw(999).unwrap_err().kind, ErrorKind::NotFound);
    }

    #[test]
    fn changed_uid_validity_drops_the_cache() {
        let (store, inbox) = store_with_inbox();
        store.set_uid_validity(inbox.id, 10).unwrap();
        store.upsert_message(inbox.id, &new_message(1)).unwrap();

        store.set_uid_validity(inbox.id, 10).unwrap();
        assert_eq!(store.uids(inbox.id).unwrap(), vec![1]);

        store.set_uid_validity(inbox.id, 11).unwrap();
        assert_eq!(store.uids(inbox.id).unwrap(), Vec::<u32>::new());
    }

    #[test]
    fn deleting_an_account_removes_its_folders_and_messages() {
        let (store, inbox) = store_with_inbox();
        store.upsert_message(inbox.id, &new_message(1)).unwrap();
        let id = store.messages(inbox.id).unwrap()[0].id;

        store.delete_account(inbox.account_id).unwrap();

        assert_eq!(store.accounts().unwrap(), Vec::new());
        assert_eq!(
            store.folder(inbox.id).unwrap_err().kind,
            ErrorKind::NotFound
        );
        assert_eq!(store.message(id).unwrap_err().kind, ErrorKind::NotFound);
    }

    #[test]
    fn settings_are_kept_and_replaced() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.setting("sender_pictures").unwrap(), None);

        store.set_setting("sender_pictures", "off").unwrap();
        store.set_setting("sender_pictures", "on").unwrap();
        assert_eq!(
            store.setting("sender_pictures").unwrap().as_deref(),
            Some("on")
        );
    }

    #[test]
    fn a_logo_answer_is_kept_per_domain_including_no_logo() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.logo("revolut.com").unwrap(), None);

        store.put_logo("revolut.com", Some(b"<svg/>"), 10).unwrap();
        store.put_logo("example.com", None, 20).unwrap();
        assert_eq!(
            store.logo("revolut.com").unwrap(),
            Some(CachedLogo {
                svg: Some(b"<svg/>".to_vec()),
                checked: 10
            })
        );
        assert_eq!(
            store.logo("example.com").unwrap(),
            Some(CachedLogo {
                svg: None,
                checked: 20
            })
        );

        store.put_logo("revolut.com", None, 30).unwrap();
        assert_eq!(store.logo("revolut.com").unwrap().unwrap().svg, None);
    }

    #[test]
    fn a_database_from_before_settings_and_logos_gains_them() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(SCHEMA).unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        connection
            .execute(
                "INSERT INTO accounts (settings) VALUES (?1)",
                params![serde_json::to_string(&settings()).unwrap()],
            )
            .unwrap();

        let store = Store::prepare(connection).unwrap();

        assert_eq!(store.accounts().unwrap().len(), 1);
        store.set_setting("sender_pictures", "off").unwrap();
        store.put_logo("example.com", None, 1).unwrap();
    }
}
