//! Turns raw messages into something the UI can show, and builds raw
//! messages from what the user wrote.

mod compose;
mod html;
mod parse;

pub use compose::{ComposeInput, compose};
pub use parse::{HeaderInfo, Rendered, attachment, parse_header, render};
