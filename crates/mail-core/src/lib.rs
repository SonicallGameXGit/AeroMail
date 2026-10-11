//! Headless core of the mail client: domain types and the rules around them.
//!
//! Nothing in here knows about the UI or the app shell.

pub mod account;
pub mod address;
pub mod category;
pub mod conversation;
pub mod draft;
pub mod error;
pub mod logo;
pub mod mail;
pub mod oauth;
pub mod settings;
pub mod transport;
