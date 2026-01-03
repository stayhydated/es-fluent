use crate::core::FluentParseMode;

/// The action to perform during generation.
#[derive(Clone, Debug)]
pub enum GenerationAction {
    /// Generate FTL files with the specified mode.
    Generate(FluentParseMode),
    /// Clean orphan keys from FTL files.
    Clean {
        all_locales: bool,
        dry_run: bool,
    },
}
