#![doc = include_str!("../README.md")]

mod commands;
mod core;
mod ftl;
mod generation;
mod tui;
mod utils;

pub use commands::{
    CheckArgs, CleanArgs, DryRunDiff, DryRunSummary, FormatArgs, GenerateArgs, SyncArgs, TreeArgs,
    WatchArgs, WorkspaceArgs, run_check, run_clean, run_format, run_generate, run_sync, run_tree,
    run_watch,
};
pub use core::{CliError, FluentParseMode};
pub use utils::ui::Ui;

pub fn set_e2e_mode(enabled: bool) {
    Ui::set_e2e_mode(enabled);
}

pub fn is_e2e() -> bool {
    Ui::is_e2e()
}

pub fn terminal_links_enabled() -> bool {
    Ui::terminal_links_enabled()
}

#[cfg(test)]
pub(crate) mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e2e_mode_facade_delegates_to_ui_state() {
        set_e2e_mode(false);
        assert!(!is_e2e());

        set_e2e_mode(true);
        assert!(is_e2e());

        set_e2e_mode(false);
        assert!(!is_e2e());
    }

    #[test]
    fn terminal_links_facade_delegates_to_ui_state() {
        let _ = terminal_links_enabled();
    }
}
