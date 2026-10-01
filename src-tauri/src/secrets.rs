//! Passwords in the OS keychain: Secret Service on Linux, Keychain on macOS,
//! Credential Manager on Windows.

use keyring::Entry;
use mail_core::error::{ErrorKind, MailError, MailResult};
use mail_core::transport::SecretStore;

const SERVICE: &str = "app.mailclient.desktop";

pub struct KeychainSecrets;

fn keychain(error: keyring::Error) -> MailError {
    MailError::new(ErrorKind::Keychain, error.to_string())
}

/// The keychain library blocks, and on Linux drives its own async runtime,
/// so it must not run on one of the app's async worker threads.
fn off_runtime<T: Send>(work: impl FnOnce() -> MailResult<T> + Send) -> MailResult<T> {
    std::thread::scope(|scope| {
        scope.spawn(work).join().unwrap_or_else(|_| {
            Err(MailError::new(
                ErrorKind::Keychain,
                "the keychain request crashed",
            ))
        })
    })
}

impl SecretStore for KeychainSecrets {
    fn get(&self, key: &str) -> MailResult<Option<String>> {
        off_runtime(|| {
            let entry = Entry::new(SERVICE, key).map_err(keychain)?;
            match entry.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(keychain(error)),
            }
        })
    }

    fn set(&self, key: &str, secret: &str) -> MailResult<()> {
        off_runtime(|| {
            Entry::new(SERVICE, key)
                .and_then(|entry| entry.set_password(secret))
                .map_err(keychain)
        })
    }

    fn delete(&self, key: &str) -> MailResult<()> {
        off_runtime(|| {
            let entry = Entry::new(SERVICE, key).map_err(keychain)?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(keychain(error)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Talks to the real OS keychain, which may ask the user to unlock it.
    /// Run with `cargo test -p mailclient -- --ignored`.
    #[test]
    #[ignore = "needs an unlocked OS keychain"]
    fn round_trip_through_the_real_keychain() {
        let secrets = KeychainSecrets;
        let key = "self-test";

        secrets.set(key, "s3cret").unwrap();
        assert_eq!(secrets.get(key).unwrap().as_deref(), Some("s3cret"));
        secrets.delete(key).unwrap();
        assert_eq!(secrets.get(key).unwrap(), None);
        // Deleting what is not there is not an error.
        secrets.delete(key).unwrap();
    }
}
