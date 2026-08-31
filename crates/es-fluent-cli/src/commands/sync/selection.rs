use super::report::sync_json_error;

use super::super::common::{OutputFormat, WorkspaceArgs};

use crate::core::CliError;

use clap::Parser;

use es_fluent_shared::CanonicalLanguageIdentifierError;

/// Arguments for the sync command.
#[derive(Debug, Parser)]
pub struct SyncArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Specific locale(s) to sync to. Can be specified multiple times or comma-separated.
    #[arg(short, long, value_delimiter = ',')]
    pub locale: Vec<String>,

    /// Sync to all discovered locale directories, excluding the fallback language; cannot be used with --locale.
    #[arg(long)]
    pub all_locales: bool,

    /// Create missing target locale directories for explicit --locale targets; cannot be used with --all-locales.
    #[arg(long)]
    pub create: bool,

    /// Dry run - show locale directories and keys that would be synced without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}
pub(crate) fn canonical_locale(locale: &str) -> Result<String, CliError> {
    let locale = locale.trim();
    if locale.is_empty() {
        return Err(CliError::Other(
            "locale values must not be empty; remove empty entries from comma-separated lists"
                .to_string(),
        ));
    }

    es_fluent_shared::parse_canonical_language_identifier(locale).map_err(|error| match error {
        CanonicalLanguageIdentifierError::Invalid { source, .. } => {
            CliError::Other(format!("invalid locale '{locale}': {source}"))
        },
        CanonicalLanguageIdentifierError::IcuInvalid { details, .. } => {
            CliError::Other(format!("invalid locale '{locale}': {details}"))
        },
        CanonicalLanguageIdentifierError::NonCanonical { canonical, .. } => CliError::Other(
            format!("locale '{locale}' must use canonical BCP-47 form '{canonical}'"),
        ),
    })?;

    Ok(locale.to_string())
}
pub(super) fn validate_sync_target_selection(
    args: &SyncArgs,
    output: OutputFormat,
) -> Result<(), CliError> {
    let error = if args.all_locales && !args.locale.is_empty() {
        Some(
            "--all-locales cannot be combined with --locale; pass one target selection mode"
                .to_string(),
        )
    } else if args.create && args.all_locales {
        Some("--create conflicts with --all-locales; pass explicit --locale targets".to_string())
    } else if args.create && args.locale.is_empty() {
        Some("--create requires explicit --locale targets".to_string())
    } else if !args.all_locales && args.locale.is_empty() {
        Some("no target locales specified; pass --all-locales or --locale <LOCALE>".to_string())
    } else {
        None
    };

    if let Some(error) = error {
        if output.is_json() {
            return sync_json_error(output, args.dry_run, error);
        }
        return Err(CliError::Other(error));
    }

    Ok(())
}
