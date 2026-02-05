use crate::utils::ui;

#[derive(Debug, Clone)]
pub struct DryRunDiff {
    before: String,
    after: String,
}

impl DryRunDiff {
    pub fn new(before: String, after: String) -> Self {
        Self { before, after }
    }

    pub fn print(&self) {
        ui::print_diff(&self.before, &self.after);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DryRunSummary {
    Format { formatted: usize },
    Sync { keys: usize, locales: usize },
}

impl DryRunSummary {
    pub fn print(self) {
        match self {
            DryRunSummary::Format { formatted } => {
                ui::print_format_dry_run_summary(formatted);
            },
            DryRunSummary::Sync { keys, locales } => {
                ui::print_sync_dry_run_summary(keys, locales);
            },
        }
    }
}
