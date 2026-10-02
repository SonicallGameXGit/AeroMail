//! The mail core as a process: one JSON request per line on stdin, one JSON
//! response per line on stdout. It lives as long as its stdin is open, so it
//! ends together with the shell that started it.

mod secrets;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_engine::Engine;
use mail_host::handle_line;
use mail_protocols::{NetConnector, NetLogos, NetSignIn};
use mail_store::Store;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Same folder name as the keychain service, so the two stay together.
const DATA_FOLDER: &str = "app.mailclient.desktop";

/// `--data-dir <path>` wins; otherwise the per-user data folder of the OS.
fn data_directory() -> MailResult<PathBuf> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--data-dir"
            && let Some(path) = arguments.next()
        {
            return Ok(PathBuf::from(path));
        }
    }

    dirs::data_dir()
        .map(|directory| directory.join(DATA_FOLDER))
        .ok_or_else(|| MailError::new(ErrorKind::Storage, "no data folder on this system"))
}

fn build_engine() -> MailResult<Engine> {
    let directory = data_directory()?;
    std::fs::create_dir_all(&directory).map_err(|error| {
        MailError::new(
            ErrorKind::Storage,
            format!("{}: {error}", directory.display()),
        )
    })?;
    let store = Store::open(&directory.join("mail.db"))?;

    Ok(Engine::new(
        store,
        Arc::new(NetConnector),
        Arc::new(secrets::KeychainSecrets),
        Arc::new(NetLogos),
        Arc::new(NetSignIn),
    ))
}

#[tokio::main]
async fn main() -> ExitCode {
    let engine = match build_engine() {
        Ok(engine) => Arc::new(engine),
        Err(error) => {
            eprintln!("failed to start the mail core: {error}");

            return ExitCode::FAILURE;
        }
    };

    // Requests run side by side; one task owns stdout so lines never mix.
    let (responses, mut outbox) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(mut line) = outbox.recv().await {
            line.push('\n');
            if stdout.write_all(line.as_bytes()).await.is_err() || stdout.flush().await.is_err() {
                break;
            }
        }
    });

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let engine = Arc::clone(&engine);
        let responses = responses.clone();
        tokio::spawn(async move {
            match handle_line(&engine, &line).await {
                Some(response) => {
                    let _ = responses.send(response);
                }
                // The line itself is not printed: it may hold a password.
                None => eprintln!("ignored a line that is not a request"),
            }
        });
    }

    ExitCode::SUCCESS
}
