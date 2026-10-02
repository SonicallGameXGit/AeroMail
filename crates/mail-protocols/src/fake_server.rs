//! Scripted local servers for the protocol tests. They speak just enough
//! IMAP / SMTP for the clients to run against real sockets.

use std::sync::{Arc, Mutex};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

// base64 of "\0me@example.com\0secret", the AUTH PLAIN form of the test login.
const SMTP_PLAIN_OK: &str = "AG1lQGV4YW1wbGUuY29tAHNlY3JldA==";

/// The XOAUTH2 answer of the test login with the access token "token".
fn xoauth2_ok() -> String {
    STANDARD.encode("user=me@example.com\x01auth=Bearer token\x01\x01")
}

// What a server says about a refused token, before it says no.
const XOAUTH2_REFUSED: &str = "eyJzdGF0dXMiOiI0MDAifQ==";

const FIRST: &str = "Subject: First\r\n\r\nBody of the first\r\n";

#[derive(Default)]
struct ImapLog {
    commands: Vec<String>,
    appended: Vec<u8>,
}

pub(crate) struct FakeImap {
    pub port: u16,
    log: Arc<Mutex<ImapLog>>,
}

fn literal(text: &str) -> String {
    format!("{{{}}}\r\n{text}", text.len())
}

impl FakeImap {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let log = Arc::new(Mutex::new(ImapLog::default()));

        let shared = Arc::clone(&log);
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve_imap(stream, Arc::clone(&shared)));
            }
        });

        Self { port, log }
    }

    pub fn commands(&self) -> Vec<String> {
        self.log.lock().unwrap().commands.clone()
    }

    pub fn appended(&self) -> Vec<u8> {
        self.log.lock().unwrap().appended.clone()
    }
}

async fn serve_imap(stream: TcpStream, log: Arc<Mutex<ImapLog>>) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read);
    write.write_all(b"* OK ready\r\n").await.unwrap();

    loop {
        let mut line = String::new();
        if lines.read_line(&mut line).await.unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim_end();
        let Some((tag, command)) = line.split_once(' ') else {
            return;
        };
        log.lock().unwrap().commands.push(command.to_string());

        let upper = command.to_uppercase();
        let mut untagged = String::new();
        let mut status = "OK done".to_string();

        if upper.starts_with("LOGIN") {
            if !command.contains("\"secret\"") {
                status = "NO [AUTHENTICATIONFAILED] invalid credentials".to_string();
            }
        } else if upper.starts_with("AUTHENTICATE XOAUTH2") {
            write.write_all(b"+ \r\n").await.unwrap();
            let mut answer = String::new();
            lines.read_line(&mut answer).await.unwrap();
            if answer.trim_end() != xoauth2_ok() {
                let report = format!("+ {XOAUTH2_REFUSED}\r\n");
                write.write_all(report.as_bytes()).await.unwrap();
                let mut empty = String::new();
                lines.read_line(&mut empty).await.unwrap();
                status = "NO [AUTHENTICATIONFAILED] invalid credentials".to_string();
            }
        } else if upper.starts_with("CAPABILITY") {
            untagged.push_str("* CAPABILITY IMAP4rev1 MOVE UIDPLUS\r\n");
        } else if upper.starts_with("LIST") {
            untagged.push_str("* LIST (\\HasNoChildren) \"/\" \"INBOX\"\r\n");
            untagged.push_str("* LIST (\\Noselect \\HasChildren) \"/\" \"[Gmail]\"\r\n");
            untagged.push_str("* LIST (\\HasNoChildren \\Sent) \"/\" \"[Gmail]/Sent Mail\"\r\n");
            untagged.push_str("* LIST (\\HasNoChildren) \"/\" \"Spam\"\r\n");
            untagged.push_str("* LIST (\\HasNoChildren) \"/\" \"&BB8EQAQ4BDIENQRC-\"\r\n");
        } else if upper.starts_with("STATUS") {
            let name = command.split('"').nth(1).unwrap_or("INBOX");
            untagged.push_str(&format!("* STATUS \"{name}\" (MESSAGES 2 UNSEEN 1)\r\n"));
        } else if upper.starts_with("SELECT") {
            untagged.push_str("* 2 EXISTS\r\n* 0 RECENT\r\n* OK [UIDVALIDITY 42] valid\r\n");
            status = "OK [READ-WRITE] selected".to_string();
        } else if upper.starts_with("UID SEARCH") {
            if upper.contains("UNSEEN") {
                untagged.push_str("* SEARCH 9\r\n");
            } else {
                untagged.push_str("* SEARCH 5 9\r\n");
            }
        } else if upper.starts_with("UID FETCH") {
            let date = "INTERNALDATE \"14-Nov-2023 22:13:20 +0000\"";
            if upper.contains("BODY.PEEK[HEADER]") {
                untagged.push_str(&format!(
                    "* 1 FETCH (UID 5 FLAGS (\\Seen) RFC822.SIZE 120 {date} BODY[HEADER] {})\r\n",
                    literal("Subject: First\r\n\r\n"),
                ));
                untagged.push_str(&format!(
                    "* 2 FETCH (UID 9 FLAGS (\\Flagged) RFC822.SIZE 80 {date} BODY[HEADER] {})\r\n",
                    literal("Subject: Second\r\n\r\n"),
                ));
            } else if upper.contains("BODY.PEEK[]") {
                untagged.push_str(&format!("* 1 FETCH (UID 5 BODY[] {})\r\n", literal(FIRST)));
            } else {
                untagged.push_str("* 1 FETCH (UID 5 FLAGS (\\Seen))\r\n");
            }
        } else if upper.starts_with("APPEND") {
            let size: usize = command
                .rsplit('{')
                .next()
                .and_then(|rest| rest.trim_end_matches('}').parse().ok())
                .unwrap_or(0);
            write.write_all(b"+ go ahead\r\n").await.unwrap();
            let mut content = vec![0; size];
            lines.read_exact(&mut content).await.unwrap();
            let mut end = String::new();
            lines.read_line(&mut end).await.unwrap();
            log.lock().unwrap().appended = content;
        }

        let answer = format!("{untagged}{tag} {status}\r\n");
        if write.write_all(answer.as_bytes()).await.is_err() {
            return;
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct ReceivedMail {
    pub from: String,
    pub recipients: Vec<String>,
    pub data: String,
}

pub(crate) struct FakeSmtp {
    pub port: u16,
    received: Arc<Mutex<ReceivedMail>>,
}

impl FakeSmtp {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let received = Arc::new(Mutex::new(ReceivedMail::default()));

        let shared = Arc::clone(&received);
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve_smtp(stream, Arc::clone(&shared)));
            }
        });

        Self { port, received }
    }

    pub fn received(&self) -> ReceivedMail {
        self.received.lock().unwrap().clone()
    }
}

async fn serve_smtp(stream: TcpStream, received: Arc<Mutex<ReceivedMail>>) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read);
    write.write_all(b"220 fake ESMTP\r\n").await.unwrap();

    let mut in_data = false;
    loop {
        let mut line = String::new();
        if lines.read_line(&mut line).await.unwrap_or(0) == 0 {
            return;
        }

        if in_data {
            if line == ".\r\n" {
                in_data = false;
                write.write_all(b"250 queued\r\n").await.unwrap();
            } else {
                received.lock().unwrap().data.push_str(&line);
            }
            continue;
        }

        let line = line.trim_end();
        let upper = line.to_uppercase();
        let answer = if upper.starts_with("EHLO") {
            "250-fake\r\n250 AUTH PLAIN XOAUTH2\r\n".to_string()
        } else if upper.starts_with("AUTH XOAUTH2") {
            if line.ends_with(&xoauth2_ok()) {
                "235 welcome\r\n".to_string()
            } else {
                format!("334 {XOAUTH2_REFUSED}\r\n")
            }
        } else if upper.starts_with("AUTH PLAIN") {
            if line.ends_with(SMTP_PLAIN_OK) {
                "235 welcome\r\n".to_string()
            } else {
                "535 5.7.8 invalid credentials\r\n".to_string()
            }
        } else if upper.starts_with("MAIL FROM:") {
            let from = line[10..].split(' ').next().unwrap_or_default();
            received.lock().unwrap().from = from.to_string();
            "250 ok\r\n".to_string()
        } else if upper.starts_with("RCPT TO:") {
            received
                .lock()
                .unwrap()
                .recipients
                .push(line[8..].to_string());
            "250 ok\r\n".to_string()
        } else if upper.starts_with("DATA") {
            in_data = true;
            "354 go ahead\r\n".to_string()
        } else if upper.starts_with("QUIT") {
            let _ = write.write_all(b"221 bye\r\n").await;
            return;
        } else {
            "250 ok\r\n".to_string()
        };

        if write.write_all(answer.as_bytes()).await.is_err() {
            return;
        }
    }
}
