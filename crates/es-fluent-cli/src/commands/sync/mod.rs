//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

mod locale;
mod merge;

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use super::dry_run::DryRunSummary;
use crate::core::CliError;
use crate::utils::ui;
use clap::Parser;
use es_fluent_shared::CanonicalLanguageIdentifierError;
use fs_err as fs;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

pub(crate) use locale::sync_crate;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SyncTextMode {
    Sync,
    AddLocale,
}

impl SyncTextMode {
    fn print_header(self) {
        match self {
            Self::Sync => ui::Ui::print_sync_header(),
            Self::AddLocale => ui::Ui::print_add_locale_header(),
        }
    }

    fn dry_run_summary(self, keys: usize, locales: usize) -> DryRunSummary {
        match self {
            Self::Sync => DryRunSummary::Sync { keys, locales },
            Self::AddLocale => DryRunSummary::AddLocale { keys, locales },
        }
    }

    fn print_summary(self, keys: usize, locales: usize) {
        match self {
            Self::Sync => ui::Ui::print_sync_summary(keys, locales),
            Self::AddLocale => ui::Ui::print_add_locale_summary(keys, locales),
        }
    }

    fn print_no_changes(self) {
        match self {
            Self::Sync => ui::Ui::print_all_in_sync(),
            Self::AddLocale => ui::Ui::print_no_locale_changes_needed(),
        }
    }

    fn text_error(self, error: impl ToString) -> CliError {
        let message = error.to_string();
        match self {
            Self::Sync => CliError::Other(message),
            Self::AddLocale => CliError::Other(
                message
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
    pub all: bool,

    /// Create missing target locale directories for explicit --locale targets; cannot be used with --all.
    #[arg(long)]
    pub create: bool,

    /// Dry run - show locale directories and keys that would be synced without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

fn collect_affected_locale_targets<'a>(
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
struct SyncJsonReport {
    dry_run: bool,
    keys_added: usize,
    locales_affected: usize,
    results: Vec<SyncResultJson>,
    error_count: usize,
    errors: Vec<String>,
}

#[derive(Serialize)]
struct SyncResultJson {
    crate_name: String,
    locale: String,
    locale_created: bool,
    keys_added: usize,
    added_keys: Vec<String>,
}

fn sync_json_error(
    output: OutputFormat,
    dry_run: bool,
    error: impl ToString,
) -> Result<(), CliError> {
    sync_json_error_with_results(output, dry_run, 0, 0, Vec::new(), error)
}

fn sync_json_error_with_results(
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

fn sync_json_error_for_workspace(
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

fn sync_json_error_with_results_for_workspace(
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

fn relative_sync_message(message: &str, base: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, base)
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

fn validate_explicit_targets_are_not_fallbacks(
    crates: &[crate::core::CrateInfo],
    targets: &HashSet<String>,
) -> Result<(), CliError> {
    let mut invalid_targets = Vec::new();

    for krate in crates {
        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;
        if targets.contains(&ctx.fallback) {
            invalid_targets.push(format!("{} for {}", ctx.fallback, krate.name));
        }
    }

    if !invalid_targets.is_empty() {
        invalid_targets.sort();
        return Err(CliError::Other(format!(
            "target locale must not be the fallback locale: {}",
            invalid_targets.join(", ")
        )));
    }

    Ok(())
}

fn validate_explicit_target_locales_exist(
    crates: &[crate::core::CrateInfo],
    targets: &HashSet<String>,
) -> Result<(), CliError> {
    let mut missing = Vec::new();
    let mut not_directories = Vec::new();

    for krate in crates {
        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;
        for target in targets {
            let target_dir = ctx.locale_dir(target);
            let target_path_exists = fs::symlink_metadata(&target_dir).is_ok();
            if target_path_exists && !crate::ftl::is_real_locale_directory(&target_dir) {
                not_directories.push(format!(
                    "{target} for {}: {}",
                    krate.name,
                    target_dir.display()
                ));
                continue;
            }

            if !target_path_exists {
                missing.push(format!("{target} for {}", krate.name));
            }
        }
    }

    if !not_directories.is_empty() {
        not_directories.sort();
        return Err(CliError::Other(format!(
            "target locale path(s) are not directories: {}",
            not_directories.join(", ")
        )));
    }

    if !missing.is_empty() {
        missing.sort();
        return Err(CliError::Other(format!(
            "target locale(s) do not exist for every selected crate: {}; pass --create to create missing target locale directories",
            missing.join(", ")
        )));
    }

    Ok(())
}

fn validate_created_target_locales_visible_to_all_scans(
    crates: &[crate::core::CrateInfo],
    targets: &HashSet<String>,
) -> Result<(), CliError> {
    let mut hidden_targets = Vec::new();

    for krate in crates {
        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;
        if ctx.assets_dir != krate.manifest_dir.as_path() {
            continue;
        }

        for target in targets {
            let target_dir = ctx.locale_dir(target);
            if !target_dir.exists()
                && es_fluent_toml::crate_root_asset_ignored_dir_names().contains(&target.as_str())
            {
                hidden_targets.push(format!("{target} for {}", krate.name));
            }
        }
    }

    if !hidden_targets.is_empty() {
        hidden_targets.sort();
        return Err(CliError::Other(format!(
            "cannot create target locale directory for locale name(s) {} because crate-root all-locale scans ignore common project directories with those names; choose a dedicated assets directory or a different locale",
            hidden_targets.join(", ")
        )));
    }

    Ok(())
}

fn validate_explicit_assets_dirs_are_directories(
    crates: &[crate::core::CrateInfo],
) -> Result<(), CliError> {
    let mut invalid_paths = Vec::new();

    for krate in crates {
        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;
        if !ctx.assets_dir.is_dir() {
            invalid_paths.push(format!(
                "assets_dir for {} is missing or not a directory: {}",
                krate.name,
                ctx.assets_dir.display()
            ));
        }
    }

    if !invalid_paths.is_empty() {
        invalid_paths.sort();
        return Err(CliError::Other(invalid_paths.join("; ")));
    }

    Ok(())
}

fn validate_all_locale_paths_are_directories(
    crates: &[crate::core::CrateInfo],
) -> Result<(), CliError> {
    let mut invalid_paths = Vec::new();

    for krate in crates {
        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;
        if !ctx.assets_dir.is_dir() {
            invalid_paths.push(format!(
                "assets_dir for {} is missing or not a directory: {}",
                krate.name,
                ctx.assets_dir.display()
            ));
            continue;
        }
        let issues = crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir)
            .map_err(|error| CliError::Other(error.to_string()))?;

        invalid_paths.extend(issues.into_iter().map(|issue| {
            format!(
                "{} for {}: {}",
                issue.locale,
                krate.name,
                issue.path.display()
            )
        }));
    }

    if !invalid_paths.is_empty() {
        invalid_paths.sort();
        return Err(CliError::Other(format!(
            "locale path(s) are not directories: {}",
            invalid_paths.join(", ")
        )));
    }

    Ok(())
}

/// Run the sync command.
pub fn run_sync(args: SyncArgs) -> Result<(), CliError> {
    run_sync_with_text_mode(args, SyncTextMode::Sync)
}

pub(crate) fn run_sync_with_text_mode(
    args: SyncArgs,
    text_mode: SyncTextMode,
) -> Result<(), CliError> {
    let output = args.output;
    let show_text = !output.is_json();

    validate_sync_target_selection(&args, output)?;

    let target_locales: Option<HashSet<String>> = if args.all {
        None // Will sync to all discovered locales.
    } else {
        match args
            .locale
            .iter()
            .map(|locale| canonical_locale(locale))
            .collect::<Result<HashSet<_>, _>>()
        {
            Ok(locales) => Some(locales),
            Err(error) => {
                if output.is_json() {
                    return sync_json_error(output, args.dry_run, error);
                }
                return Err(error);
            },
        }
    };

    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) if output.is_json() => return sync_json_error(output, args.dry_run, error),
        Err(error) => return Err(error),
    };

    let workspace_root = workspace.workspace_info.root_dir.clone();

    if workspace.crates.is_empty() {
        let reason = workspace
            .empty_selection_message()
            .unwrap_or_else(|| "no crates were selected".to_string());
        let error = if args.create {
            format!("cannot create target locale directories because {reason}")
        } else {
            format!("cannot sync locales because {reason}")
        };
        if output.is_json() {
            return sync_json_error_for_workspace(output, args.dry_run, error, &workspace_root);
        }
        if show_text {
            workspace.print_no_crates_found();
        }
        return Err(text_mode.text_error(error));
    }

    let crates = workspace.crates;

    if args.all
        && let Err(error) = validate_all_locale_paths_are_directories(&crates)
    {
        if args.output.is_json() {
            return sync_json_error_for_workspace(
                args.output,
                args.dry_run,
                error,
                &workspace_root,
            );
        }
        return Err(text_mode.text_error(error));
    }

    if show_text {
        text_mode.print_header();
    }

    if target_locales.is_some()
        && let Err(error) = validate_explicit_assets_dirs_are_directories(&crates)
    {
        if args.output.is_json() {
            return sync_json_error_for_workspace(
                args.output,
                args.dry_run,
                error,
                &workspace_root,
            );
        }
        return Err(text_mode.text_error(error));
    }

    if let Some(ref targets) = target_locales
        && let Err(error) = validate_explicit_targets_are_not_fallbacks(&crates, targets)
    {
        if args.output.is_json() {
            return sync_json_error_for_workspace(
                args.output,
                args.dry_run,
                error,
                &workspace_root,
            );
        }
        return Err(text_mode.text_error(error));
    }

    // Validate that specified locales exist
    if let Some(ref targets) = target_locales
        && !args.create
        && let Err(error) = validate_explicit_target_locales_exist(&crates, targets)
    {
        if args.output.is_json() {
            return sync_json_error_for_workspace(
                args.output,
                args.dry_run,
                error,
                &workspace_root,
            );
        }
        return Err(text_mode.text_error(error));
    }

    if let Some(ref targets) = target_locales
        && args.create
        && let Err(error) = validate_created_target_locales_visible_to_all_scans(&crates, targets)
    {
        if args.output.is_json() {
            return sync_json_error_for_workspace(
                args.output,
                args.dry_run,
                error,
                &workspace_root,
            );
        }
        return Err(text_mode.text_error(error));
    }

    let mut total_keys_added = 0;
    let mut affected_locale_targets: HashSet<(String, String)> = HashSet::new();
    let mut json_results = Vec::new();

    for krate in &crates {
        if let Err(error) =
            locale::preflight_sync_crate(krate, target_locales.as_ref(), args.create)
        {
            if args.output.is_json() {
                return sync_json_error_for_workspace(
                    args.output,
                    args.dry_run,
                    error,
                    &workspace_root,
                );
            }
            return Err(text_mode.text_error(error));
        }
    }

    let pb = if show_text {
        ui::Ui::create_progress_bar(crates.len() as u64, "Syncing crates...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    for krate in &crates {
        pb.set_message(format!("Syncing {}", krate.name));

        let results =
            match locale::sync_crate(krate, target_locales.as_ref(), args.dry_run, args.create) {
                Ok(results) => results,
                Err(error) => {
                    if args.output.is_json() {
                        return sync_json_error_with_results_for_workspace(
                            args.output,
                            args.dry_run,
                            total_keys_added,
                            affected_locale_targets.len(),
                            json_results,
                            error,
                            &workspace_root,
                        );
                    }
                    return Err(text_mode.text_error(error));
                },
            };
        affected_locale_targets.extend(collect_affected_locale_targets(
            krate.name.as_str(),
            results.iter(),
        ));

        for result in results {
            json_results.push(SyncResultJson {
                crate_name: krate.name.to_string(),
                locale: result.locale.clone(),
                locale_created: result.locale_created,
                keys_added: result.keys_added,
                added_keys: result.added_keys.clone(),
            });

            if result.locale_created && show_text {
                pb.suspend(|| {
                    if args.dry_run {
                        ui::Ui::print_would_create_locale(&result.locale, krate.name.as_str());
                    } else {
                        ui::Ui::print_created_locale(&result.locale, krate.name.as_str());
                    }
                });
            }

            if result.keys_added > 0 {
                total_keys_added += result.keys_added;

                if show_text {
                    pb.suspend(|| {
                        if args.dry_run {
                            ui::Ui::print_would_add_keys(
                                result.keys_added,
                                &result.locale,
                                krate.name.as_str(),
                            );
                            if let Some(diff) = &result.diff_info {
                                diff.print();
                            }
                        } else {
                            ui::Ui::print_added_keys(result.keys_added, &result.locale);
                            for key in &result.added_keys {
                                ui::Ui::print_synced_key(key);
                            }
                        }
                    });
                }
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();
    let total_locales_affected = affected_locale_targets.len();

    if args.output.is_json() {
        args.output.print_json(&SyncJsonReport {
            dry_run: args.dry_run,
            keys_added: total_keys_added,
            locales_affected: total_locales_affected,
            results: json_results,
            error_count: 0,
            errors: Vec::new(),
        })?;
        return Ok(());
    }

    if total_keys_added == 0 && total_locales_affected == 0 {
        text_mode.print_no_changes();
    } else if args.dry_run {
        text_mode
            .dry_run_summary(total_keys_added, total_locales_affected)
            .print();
    } else {
        text_mode.print_summary(total_keys_added, total_locales_affected);
    }

    Ok(())
}

fn validate_sync_target_selection(args: &SyncArgs, output: OutputFormat) -> Result<(), CliError> {
    let error = if args.all && !args.locale.is_empty() {
        Some("--all cannot be combined with --locale; pass one target selection mode".to_string())
    } else if args.create && (args.all || args.locale.is_empty()) {
        Some(
            "--create requires explicit --locale targets and cannot be used with --all".to_string(),
        )
    } else if !args.all && args.locale.is_empty() {
        Some("no target locales specified; pass --all or --locale <LOCALE>".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::parser;
    use fs_err as fs;

    #[test]
    fn relative_sync_message_strips_workspace_paths_for_json_errors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let message = format!(
            "target locale directory 'fr' is not a directory for test-app: {}",
            temp.path().join("i18n/fr").display()
        );

        let normalized = relative_sync_message(&message, temp.path());

        assert_eq!(
            normalized,
            "target locale directory 'fr' is not a directory for test-app: i18n/fr"
        );
    }

    fn write_sync_workspace_crate(root: &std::path::Path, name: &str, fallback: &str) {
        fs::create_dir_all(root.join(name).join("src")).expect("create src");
        fs::create_dir_all(root.join(name).join("i18n/en")).expect("create fallback locale");
        fs::write(
            root.join(name).join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n"
            ),
        )
        .expect("write manifest");
        fs::write(root.join(name).join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            root.join(name).join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        fs::write(
            root.join(name).join("i18n/en").join(format!("{name}.ftl")),
            fallback,
        )
        .expect("write fallback ftl");
    }

    #[test]
    fn test_extract_message_keys() {
        let content = r#"hello = Hello
world = World"#;
        let resource = parser::parse(content.to_string()).unwrap();
        let keys = crate::ftl::extract_message_keys(&resource);

        assert!(keys.contains("hello"));
        assert!(keys.contains("world"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn run_sync_returns_err_when_no_locales_specified() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: Vec::new(),
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(result.is_err());
    }

    #[test]
    fn run_sync_fails_when_no_crates_match_filter() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-package".to_string()),
            },
            locale: vec!["es".to_string()],
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("cannot sync locales") && message.contains("missing-package"))
        );
    }

    #[test]
    fn run_sync_create_fails_when_no_crates_match_filter() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[(
            "en",
            "hello = Hello\nworld = World\n",
        )]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-package".to_string()),
            },
            locale: vec!["fr-FR".to_string()],
            all: false,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-package"))
        );
        assert!(!temp.path().join("i18n/fr-FR").exists());
    }

    #[test]
    fn run_sync_rejects_missing_target_selection_before_workspace_discovery() {
        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(std::path::PathBuf::from("/definitely/missing/path")),
                package: None,
            },
            locale: Vec::new(),
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("no target locales specified"))
        );
    }

    #[test]
    fn run_sync_rejects_target_selection_conflicts_before_workspace_discovery() {
        let cases = [
            (
                true,
                true,
                Vec::new(),
                "--create requires explicit --locale targets",
            ),
            (
                false,
                true,
                Vec::new(),
                "--create requires explicit --locale targets",
            ),
            (
                true,
                false,
                vec!["fr-FR".to_string()],
                "--all cannot be combined with --locale",
            ),
        ];

        for (all, create, locale, expected) in cases {
            let result = run_sync(SyncArgs {
                workspace: WorkspaceArgs {
                    path: Some(std::path::PathBuf::from("/definitely/missing/path")),
                    package: None,
                },
                locale,
                all,
                create,
                dry_run: false,
                output: OutputFormat::Text,
            });

            assert!(
                matches!(&result, Err(CliError::Other(message)) if message.contains(expected)),
                "expected {expected:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn run_sync_fails_for_unknown_locale() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["zz-unknown".to_string()],
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("zz-unknown for test-app"))
        );
    }

    #[test]
    fn run_sync_trims_comma_separated_locale_values() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("fr", "hello = Bonjour\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec![" fr ".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
        let content = fs::read_to_string(temp.path().join("i18n/fr/test-app.ftl"))
            .expect("target FTL should remain readable");
        assert!(!content.contains("world = World"));
    }

    #[test]
    fn run_sync_rejects_empty_comma_separated_locale_values() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("fr", "hello = Bonjour\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec![" ".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("locale values must not be empty"))
        );
    }

    #[test]
    fn run_sync_rejects_noncanonical_locale_values_with_form_hint() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("fr", "hello = Bonjour\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["iw".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(matches!(
            result,
            Err(CliError::Other(message))
                if message.contains("locale 'iw' must use canonical BCP-47 form 'he'")
                    && !message.contains("casing")
        ));
    }

    #[test]
    fn run_sync_requires_explicit_locale_in_every_selected_crate() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("a/src")).expect("create a src");
        fs::create_dir_all(temp.path().join("a/i18n/en")).expect("create a en");
        fs::create_dir_all(temp.path().join("a/i18n/fr")).expect("create a fr");
        fs::create_dir_all(temp.path().join("b/src")).expect("create b src");
        fs::create_dir_all(temp.path().join("b/i18n/en")).expect("create b en");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");
        fs::write(
            temp.path().join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write a manifest");
        fs::write(
            temp.path().join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write b manifest");
        fs::write(temp.path().join("a/src/lib.rs"), "pub fn a() {}\n").expect("write a lib");
        fs::write(temp.path().join("b/src/lib.rs"), "pub fn b() {}\n").expect("write b lib");
        fs::write(
            temp.path().join("a/i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write a config");
        fs::write(
            temp.path().join("b/i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write b config");
        fs::write(
            temp.path().join("a/i18n/en/a.ftl"),
            "hello = Hello\nworld = World\n",
        )
        .expect("write a fallback");
        fs::write(temp.path().join("a/i18n/fr/a.ftl"), "hello = Bonjour\n").expect("write a fr");
        fs::write(
            temp.path().join("b/i18n/en/b.ftl"),
            "hello = Hello\nworld = World\n",
        )
        .expect("write b fallback");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("fr for b") && message.contains("--create"))
        );
        assert!(
            !temp.path().join("b/i18n/fr").exists(),
            "sync without --create must not create the missing locale"
        );
    }

    #[test]
    fn run_sync_rejects_explicit_target_locale_path_as_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n config");
        fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback ftl");
        fs::write(temp.path().join("i18n/fr"), "not a directory\n")
            .expect("write target locale file");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("target locale path") && message.contains("fr for test-app") && message.contains("not directories"))
        );
        assert!(temp.path().join("i18n/fr").is_file());
    }

    #[test]
    fn run_sync_all_rejects_locale_named_asset_path_as_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: Vec::new(),
            all: true,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("locale path") && message.contains("fr for test-app") && message.contains("not directories"))
        );
        assert!(temp.path().join("i18n/fr").is_file());
    }

    #[test]
    fn run_sync_all_rejects_missing_assets_dir() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: Vec::new(),
            all: true,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
        );
    }

    #[test]
    fn run_sync_all_rejects_assets_dir_path_as_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: Vec::new(),
            all: true,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
        );
        assert!(temp.path().join("i18n").is_file());
    }

    #[test]
    fn run_sync_explicit_target_rejects_assets_dir_path_as_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
        );
        assert!(temp.path().join("i18n").is_file());
    }

    #[test]
    fn run_sync_create_rejects_assets_dir_path_as_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            all: false,
            create: true,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("assets_dir for test-app") && message.contains("missing or not a directory"))
        );
        assert!(temp.path().join("i18n").is_file());
    }

    #[test]
    fn run_sync_rejects_fallback_target_locale() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["en".to_string()],
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("fallback locale"))
        );
    }

    #[test]
    fn run_sync_create_rejects_fallback_target_locale() {
        let temp =
            crate::test_fixtures::create_workspace_with_locales(&[("en", "hello = Hello\n")]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["en".to_string()],
            all: false,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("fallback locale"))
        );
    }

    #[test]
    fn run_sync_create_rejects_root_assets_locales_hidden_by_project_dir_ignores() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \".\"\n",
        )
        .expect("write config");
        fs::write(temp.path().join("en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback ftl");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["bin".to_string()],
            all: false,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("cannot create target locale") && message.contains("bin for test-app"))
        );
        assert!(
            !temp.path().join("bin").exists(),
            "sync --create must not create locales hidden from --all scans"
        );
    }

    #[test]
    fn run_sync_create_allows_existing_root_assets_locale_hidden_from_all_scans() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("en")).expect("create fallback locale");
        fs::create_dir_all(temp.path().join("bin")).expect("create existing target locale");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .expect("write manifest");
        fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \".\"\n",
        )
        .expect("write config");
        fs::write(
            temp.path().join("en/test-app.ftl"),
            "hello = Hello\nworld = World\n",
        )
        .expect("write fallback ftl");
        fs::write(temp.path().join("bin/test-app.ftl"), "hello = Hello\n")
            .expect("write existing target ftl");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["bin".to_string()],
            all: false,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
        let content =
            fs::read_to_string(temp.path().join("bin/test-app.ftl")).expect("read target ftl");
        assert!(content.contains("world = World"));
    }

    #[test]
    fn run_sync_rejects_create_with_all() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hola\n"),
        ]);

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: Vec::new(),
            all: true,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("--create requires explicit --locale"))
        );
    }

    #[test]
    fn run_sync_dry_run_does_not_write_missing_keys() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);
        let es_path = temp.path().join("i18n/es/test-app.ftl");
        let before = fs::read_to_string(&es_path).expect("read before");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["es".to_string()],
            all: false,
            create: false,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
        let after = fs::read_to_string(&es_path).expect("read after");
        assert_eq!(before, after, "dry-run should not modify locale files");
    }

    #[test]
    fn run_sync_writes_missing_keys_for_target_locale() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);
        let es_path = temp.path().join("i18n/es/test-app.ftl");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["es".to_string()],
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
        let es_content = fs::read_to_string(&es_path).expect("read synced es");
        assert!(es_content.contains("world = World"));
    }

    #[test]
    fn run_sync_create_writes_missing_target_locale() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[(
            "en",
            "hello = Hello\nworld = World\n",
        )]);
        let fr_path = temp.path().join("i18n/fr-FR/test-app.ftl");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            all: false,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
        let fr_content = fs::read_to_string(&fr_path).expect("read created locale");
        assert!(fr_content.contains("hello = Hello"));
        assert!(fr_content.contains("world = World"));
    }

    #[test]
    fn run_sync_create_preflights_selected_workspace_before_writing() {
        let temp = tempfile::tempdir().expect("workspace tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .expect("write workspace manifest");
        write_sync_workspace_crate(temp.path(), "a", "hello = Hello\n");
        write_sync_workspace_crate(temp.path(), "b", "hello = Hello\n");
        fs::write(temp.path().join("b/i18n/fr-FR"), "not a directory\n")
            .expect("write target locale blocker");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            all: false,
            create: true,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("target locale directory") && message.contains("b"))
        );
        assert!(
            !temp.path().join("a/i18n/fr-FR").exists(),
            "sync --create should not write earlier crates before preflighting later crates"
        );
        assert!(temp.path().join("b/i18n/fr-FR").is_file());
    }

    #[test]
    fn run_sync_explicit_target_ignores_unrelated_noncanonical_locale_dir() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[(
            "en",
            "hello = Hello\nworld = World\n",
        )]);
        fs::create_dir_all(temp.path().join("i18n/en-us")).expect("create unrelated bad locale");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["fr-FR".to_string()],
            all: false,
            create: true,
            dry_run: true,
            output: OutputFormat::Text,
        });

        assert!(
            result.is_ok(),
            "explicit sync targets should not scan unrelated locale dirs: {result:?}"
        );
        assert!(
            !temp.path().join("i18n/fr-FR").exists(),
            "dry-run explicit sync should preview creation without writing"
        );
    }

    #[test]
    fn run_sync_all_processes_non_fallback_locales() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);
        fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
        fs::write(temp.path().join("i18n/fr/test-app.ftl"), "hello = Salut\n").expect("write fr");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: Vec::new(),
            all: true,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(result.is_ok());
        let fr_content =
            fs::read_to_string(temp.path().join("i18n/fr/test-app.ftl")).expect("read fr");
        assert!(fr_content.contains("world = World"));
    }

    #[test]
    fn collect_affected_locale_targets_deduplicates_namespaced_file_results() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);
        fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create en namespace dir");
        fs::write(
            temp.path().join("i18n/en/test-app/ui.ftl"),
            "button = Button\n",
        )
        .expect("write en namespaced fallback");

        let workspace = WorkspaceCrates::discover(WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        })
        .expect("discover workspace");
        let krate = workspace.crates.first().expect("crate");
        let targets = HashSet::from(["es".to_string()]);

        let results = locale::sync_crate(krate, Some(&targets), true, false).expect("sync crate");

        assert_eq!(
            results
                .iter()
                .filter(|result| result.keys_added > 0)
                .count(),
            2,
            "both locale files should report changes"
        );
        assert_eq!(
            collect_affected_locale_targets(krate.name.as_str(), results.iter()).len(),
            1
        );
    }

    #[test]
    fn collect_affected_locale_targets_counts_the_same_locale_in_different_crates() {
        let results = [
            locale::SyncLocaleResult {
                locale: "fr".to_string(),
                locale_created: false,
                keys_added: 1,
                added_keys: vec!["hello".to_string()],
                diff_info: None,
            },
            locale::SyncLocaleResult {
                locale: "fr".to_string(),
                locale_created: true,
                keys_added: 0,
                added_keys: Vec::new(),
                diff_info: None,
            },
        ];

        let mut affected = HashSet::new();
        affected.extend(collect_affected_locale_targets("a", results.iter()));
        affected.extend(collect_affected_locale_targets("b", results.iter()));

        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&("a".to_string(), "fr".to_string())));
        assert!(affected.contains(&("b".to_string(), "fr".to_string())));
    }
}
