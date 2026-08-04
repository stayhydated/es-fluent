use crate::core::FluentParseMode;

/// The action to perform during generation.
#[derive(Clone, Debug)]
pub enum GenerationAction {
    /// Generate FTL files with the specified mode.
    Generate {
        mode: FluentParseMode,
        dry_run: bool,
    },
    /// Remove FTL entries and package-owned files absent from Rust inventory.
    Clean { all_locales: bool, dry_run: bool },
}

impl GenerationAction {
    pub(crate) fn is_dry_run(&self) -> bool {
        match self {
            Self::Generate { dry_run, .. } | Self::Clean { dry_run, .. } => *dry_run,
        }
    }
}
