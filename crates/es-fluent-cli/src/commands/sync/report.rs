use super::locale;

use super::super::{common::OutputFormat, dry_run::DryRunSummary};

use crate::{core::CliError, utils::ui};

use serde::Serialize;

use std::{collections::HashSet, path::Path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SyncTextMode {
    Sync,
    AddLocale,
}

impl SyncTextMode {
    pub(super) fn print_header(self) {
        match self {
            Self::Sync => ui::Ui::print_sync_header(),
            Self::AddLocale => ui::Ui::print_add_locale_header(),
        }
    }

    pub(super) fn dry_run_summary(self, keys: usize, locales: usize) -> DryRunSummary {
        match self {
            Self::Sync => DryRunSummary::Sync { keys, locales },
            Self::AddLocale => DryRunSummary::AddLocale { keys, locales },
        }
    }

    pub(super) fn print_summary(self, keys: usize, locales: usize) {
        match self {
            Self::Sync => ui::Ui::print_sync_summary(keys, locales),
            Self::AddLocale => ui::Ui::print_add_locale_summary(keys, locales),
        }
    }

    pub(super) fn print_no_changes(self) {
        match self {
            Self::Sync => ui::Ui::print_all_in_sync(),
            Self::AddLocale => ui::Ui::print_no_locale_changes_needed(),
        }
    }

    pub(super) fn text_error(self, error: impl ToString) -> CliError {
        let message = error.to_string();
        match self {
            Self::Sync => CliError::Other(message),
            Self::AddLocale => CliError::Other(
                message
                    .replace("sync transaction failed", "add-locale transaction failed")
                    .replace("Refusing to sync ", "Refusing to add locale data to ")
                    .replace("target FTL", "requested-locale FTL")
                    .replace(
                        "target parent directories",
                        "requested-locale parent directories",
                    )
                    .replace("parent path", "requested-locale parent path")
                    .replace("target locale", "requested locale"),
            ),
        }
    }
}
pub(super) fn collect_affected_locale_targets<'a>(
    crate_name: &str,
    results: impl IntoIterator<Item = &'a locale::SyncLocaleResult>,
) -> HashSet<(String, String)> {
    results
        .into_iter()
        .filter(|result| result.keys_added > 0 || result.locale_created)
        .map(|result| (crate_name.to_string(), result.locale.clone()))
        .collect()
}

#[derive(Serialize)]
pub(super) struct SyncJsonReport {
    pub(super) dry_run: bool,
    pub(super) keys_added: usize,
    pub(super) locales_affected: usize,
    pub(super) results: Vec<SyncResultJson>,
    pub(super) error_count: usize,
    pub(super) errors: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct SyncResultJson {
    pub(super) crate_name: String,
    pub(super) locale: String,
    pub(super) path: Option<String>,
    pub(super) locale_created: bool,
    pub(super) keys_added: usize,
    pub(super) added_keys: Vec<String>,
}

pub(super) fn sync_json_error(
    output: OutputFormat,
    dry_run: bool,
    error: impl ToString,
) -> Result<(), CliError> {
    sync_json_error_with_results(output, dry_run, 0, 0, Vec::new(), error)
}

pub(super) fn sync_json_error_with_results(
    output: OutputFormat,
    dry_run: bool,
    keys_added: usize,
    locales_affected: usize,
    results: Vec<SyncResultJson>,
    error: impl ToString,
) -> Result<(), CliError> {
    output.print_json(&SyncJsonReport {
        dry_run,
        keys_added,
        locales_affected,
        results,
        error_count: 1,
        errors: vec![error.to_string()],
    })?;
    Err(CliError::Exit(1))
}

pub(super) fn sync_json_error_for_workspace(
    output: OutputFormat,
    dry_run: bool,
    error: impl ToString,
    workspace_root: &Path,
) -> Result<(), CliError> {
    sync_json_error_with_results_for_workspace(
        output,
        dry_run,
        0,
        0,
        Vec::new(),
        error,
        workspace_root,
    )
}

pub(super) fn sync_json_error_with_results_for_workspace(
    output: OutputFormat,
    dry_run: bool,
    keys_added: usize,
    locales_affected: usize,
    results: Vec<SyncResultJson>,
    error: impl ToString,
    workspace_root: &Path,
) -> Result<(), CliError> {
    let error = relative_sync_message(&error.to_string(), workspace_root);
    sync_json_error_with_results(
        output,
        dry_run,
        keys_added,
        locales_affected,
        results,
        error,
    )
}

pub(super) fn relative_sync_message(message: &str, base: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, base)
}
