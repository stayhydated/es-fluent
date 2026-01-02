//! Terminal UI for watch mode.

mod app;
mod watcher;

pub use app::{TuiApp, draw, init_terminal, poll_quit_event, restore_terminal};
pub use watcher::watch_all;
