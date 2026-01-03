//! Terminal UI for watch mode.

mod app;
mod message;
mod watcher;

pub use app::{TuiApp, draw, poll_quit_event};
pub use message::Message;
pub use watcher::watch_all;
