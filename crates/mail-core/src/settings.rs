//! What the user can change about how the app behaves, as one value.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppSettings {
    /// A brand's own logo is looked up for its mail. Off: nothing is asked.
    pub sender_pictures: bool,
    /// Remote images load in every message without asking for each one.
    pub remote_images: bool,
}

impl Default for AppSettings {
    /// What a fresh install does. Remote images stay blocked until the user
    /// decides otherwise: loading one tells the sender the mail was read.
    fn default() -> Self {
        Self {
            sender_pictures: true,
            remote_images: false,
        }
    }
}
