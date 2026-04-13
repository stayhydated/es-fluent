use crate::utils::ui;

#[derive(Clone, Debug)]
pub struct DryRunDiff {
    before: String,
    after: String,
}

impl DryRunDiff {
    pub fn new(before: String, after: String) -> Self {
        Self { before, after }
    }

    pub fn print(&self) {
        ui::Ui::print_diff(&self.before, &self.after);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DryRunSummary {
    Format { formatted: usize },
    Sync { keys: usize, locales: usize },
}

impl DryRunSummary {
    pub fn print(self) {
        match self {
            DryRunSummary::Format { formatted } => {
                ui::Ui::print_format_dry_run_summary(formatted);
            },
            DryRunSummary::Sync { keys, locales } => {
                ui::Ui::print_sync_dry_run_summary(keys, locales);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_diff_new_stores_before_and_after() {
        let diff = DryRunDiff::new("old".to_string(), "new".to_string());
        assert_eq!(diff.before, "old");
        assert_eq!(diff.after, "new");
    }

    #[test]
    fn dry_run_diff_print_and_summary_print_do_not_panic() {
        let diff = DryRunDiff::new("a = 1\n".to_string(), "a = 2\n".to_string());
        diff.print();

        DryRunSummary::Format { formatted: 3 }.print();
        DryRunSummary::Sync {
            keys: 5,
            locales: 2,
        }
        .print();
    }
}
