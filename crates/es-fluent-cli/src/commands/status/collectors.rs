use super::super::common::WorkspaceCrates;
use crate::core::{CliError, GenerateResult, ValidationIssue};
use std::path::Path;

pub(super) fn collect_status_setup_errors(workspace: &WorkspaceCrates) -> Vec<String> {
    let mut setup_errors = Vec::new();

    for krate in &workspace.skipped {
        setup_errors.push(format!(
            "{}: crate has i18n.toml but no Cargo library target",
            krate.name
        ));
    }

    for krate in &workspace.crates {
        let ctx = match crate::ftl::LocaleContext::from_crate(krate, true) {
            Ok(ctx) => ctx,
            Err(error) => {
                setup_errors.push(format!("{}: {}", krate.name, error));
                continue;
            },
        };

        let fallback_dir = ctx.locale_dir(&ctx.fallback);
        let fallback_path_invalid = !crate::ftl::is_real_locale_directory(&fallback_dir);
        if fallback_path_invalid {
            setup_errors.push(format!(
                "{}: fallback locale directory '{}' is missing or not a directory: {}",
                krate.name,
                ctx.fallback,
                fallback_dir.display()
            ));
        }

        match crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir) {
            Ok(issues) => {
                setup_errors.extend(
                    issues
                        .into_iter()
                        .filter(|issue| !(fallback_path_invalid && issue.locale == ctx.fallback))
                        .map(|issue| {
                            format!(
                                "{}: locale path '{}' is not a directory: {}",
                                krate.name,
                                issue.locale,
                                issue.path.display()
                            )
                        }),
                );
            },
            Err(error) => setup_errors.push(format!("{}: {}", krate.name, error)),
        }

        for locale in &ctx.locales {
            let locale_dir = ctx.locale_dir(locale);
            if !crate::ftl::is_real_locale_directory(&locale_dir) {
                continue;
            }

            if let Err(error) = crate::ftl::CrateFtlLayout::from_assets_dir(
                &ctx.assets_dir,
                locale,
                &ctx.crate_name,
            )
            .discover_files()
            {
                setup_errors.push(format!("{}: {}", krate.name, error));
            }
        }
    }

    setup_errors.sort();
    setup_errors
}

pub(super) fn collect_orphaned_status_paths(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> Result<Vec<String>, CliError> {
    Ok(
        super::super::clean::orphaned::find_orphaned_files(workspace, all_locales)?
            .into_iter()
            .map(|path| relative_status_path(&path, &workspace.workspace_info.root_dir))
            .collect(),
    )
}

pub(super) fn collect_format_status_results(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> (usize, Vec<String>) {
    let mut files_need_formatting = 0;
    let mut format_errors = Vec::new();

    for krate in &workspace.crates {
        match super::super::format::format_crate(krate, all_locales, true) {
            Ok(results) => {
                for result in results {
                    if let Some(error) = result.error {
                        let path =
                            relative_status_path(&result.path, &workspace.workspace_info.root_dir);
                        format_errors.push(format!("{path}: {error}"));
                    } else if result.changed {
                        files_need_formatting += 1;
                    }
                }
            },
            Err(error) => {
                format_errors.push(format!("{}: {}", krate.name, error));
            },
        }
    }

    (files_need_formatting, format_errors)
}

pub(super) fn collect_status_generation_errors(
    results: &[GenerateResult],
    base: &Path,
) -> Vec<String> {
    results
        .iter()
        .filter_map(|result| {
            result
                .error
                .as_ref()
                .map(|error| relative_status_message(&format!("{}: {error}", result.name), base))
        })
        .collect()
}

pub(super) fn relative_status_path(path: &Path, base: &Path) -> String {
    crate::utils::paths::relative_slash_path(path, base)
}

pub(super) fn relative_status_message(message: &str, base: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, base)
}

pub(super) fn normalize_status_setup_errors(errors: Vec<String>, base: &Path) -> Vec<String> {
    errors
        .into_iter()
        .map(|error| relative_status_message(&error, base))
        .collect()
}

pub(super) fn count_status_validation_issues(issues: &[ValidationIssue]) -> (usize, usize) {
    let error_count = issues
        .iter()
        .filter(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingKey(_)
                    | ValidationIssue::DuplicateKey(_)
                    | ValidationIssue::UnexpectedVariable(_)
                    | ValidationIssue::ValidationExecution(_)
                    | ValidationIssue::SyntaxError(_)
            )
        })
        .count();
    let warning_count = issues
        .iter()
        .filter(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingVariable(_) | ValidationIssue::UntranslatedMessage(_)
            )
        })
        .count();

    (error_count, warning_count)
}

pub(super) fn count_generation_stale_crates(results: &[GenerateResult]) -> usize {
    results.iter().filter(|result| result.changed).count()
}
