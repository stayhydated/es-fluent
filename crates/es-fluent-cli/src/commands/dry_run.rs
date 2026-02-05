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
