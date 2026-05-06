//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

mod locale;
mod merge;

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use super::dry_run::DryRunSummary;
use crate::core::{CliError, LocaleNotFoundError};
use crate::utils::ui;
use clap::Parser;
use serde::Serialize;
use std::collections::HashSet;
use unic_langid::LanguageIdentifier;

pub(crate) use locale::sync_crate;

/// Arguments for the sync command.
#[derive(Debug, Parser)]
pub struct SyncArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Specific locale(s) to sync to (can be specified multiple times).
    #[arg(short, long)]
    pub locale: Vec<String>,

    /// Sync to all locales (excluding the fallback language).
    #[arg(long, conflicts_with = "locale")]
    pub all: bool,

    /// Create target locale directories when they do not already exist.
    #[arg(long)]
    pub create: bool,

    /// Dry run - show what would be synced without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

fn collect_affected_locales<'a>(
    results: impl IntoIterator<Item = &'a locale::SyncLocaleResult>,
) -> HashSet<String> {
    results
        .into_iter()
        .filter(|result| result.keys_added > 0)
        .map(|result| result.locale.clone())
        .collect()
}

#[derive(Serialize)]
struct SyncJsonReport {
    keys_added: usize,
    locales_affected: usize,
    results: Vec<SyncResultJson>,
}

#[derive(Serialize)]
struct SyncResultJson {
    crate_name: String,
    locale: String,
    keys_added: usize,
    added_keys: Vec<String>,
}

pub(crate) fn canonical_locale(locale: &str) -> Result<String, CliError> {
    let language = locale
        .parse::<LanguageIdentifier>()
        .map_err(|error| CliError::Other(format!("invalid locale '{locale}': {error}")))?;
    let canonical = language.to_string();

    if canonical != locale {
        return Err(CliError::Other(format!(
            "locale '{locale}' must use canonical BCP-47 casing '{canonical}'"
        )));
    }

    Ok(canonical)
}

/// Run the sync command.
pub fn run_sync(args: SyncArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;
    let show_text = !args.output.is_json();

    if show_text {
        ui::Ui::print_sync_header();
    }

    let crates = workspace.crates;

    if crates.is_empty() {
        if show_text {
            ui::Ui::print_no_crates_found();
        }
        return Ok(());
    }

    let target_locales: Option<HashSet<String>> = if args.all {
        None // Will sync to all locales
    } else if args.locale.is_empty() {
        if show_text {
            ui::Ui::print_no_locales_specified();
        }
        return Err(CliError::Other(
            "no target locales specified; pass --all or --locale <LOCALE>".to_string(),
        ));
    } else {
        Some(
            args.locale
                .iter()
                .map(|locale| canonical_locale(locale))
                .collect::<Result<HashSet<_>, _>>()?,
        )
    };

    // Validate that specified locales exist
    if let Some(ref targets) = target_locales
        && !args.create
    {
        let all_available_locales = crate::ftl::collect_all_available_locales(&crates)?;

        for locale in targets {
            if !all_available_locales.contains(locale) {
                let mut available: Vec<String> = all_available_locales.into_iter().collect();
                available.sort();
                if show_text {
                    ui::Ui::print_locale_not_found(locale, &available);
                }
                return Err(CliError::LocaleNotFound(LocaleNotFoundError {
                    locale: locale.clone(),
                    available: available.join(", "),
                }));
            }
        }
    }

    let mut total_keys_added = 0;
    let mut affected_locales: HashSet<String> = HashSet::new();
    let mut json_results = Vec::new();
    let pb = if show_text {
        ui::Ui::create_progress_bar(crates.len() as u64, "Syncing crates...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    for krate in &crates {
        pb.set_message(format!("Syncing {}", krate.name));

        let results =
            locale::sync_crate(krate, target_locales.as_ref(), args.dry_run, args.create)?;
        affected_locales.extend(collect_affected_locales(results.iter()));

        for result in results {
            json_results.push(SyncResultJson {
                crate_name: krate.name.clone(),
                locale: result.locale.clone(),
                keys_added: result.keys_added,
                added_keys: result.added_keys.clone(),
            });

            if result.keys_added > 0 {
                total_keys_added += result.keys_added;

                if show_text {
                    pb.suspend(|| {
                        if args.dry_run {
                            ui::Ui::print_would_add_keys(
                                result.keys_added,
                                &result.locale,
                                &krate.name,
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
    let total_locales_affected = affected_locales.len();

    if args.output.is_json() {
        args.output.print_json(&SyncJsonReport {
            keys_added: total_keys_added,
            locales_affected: total_locales_affected,
            results: json_results,
        })?;
        return Ok(());
    }

    if total_keys_added == 0 {
        ui::Ui::print_all_in_sync();
        Ok(())
    } else if args.dry_run {
        DryRunSummary::Sync {
            keys: total_keys_added,
            locales: total_locales_affected,
        }
        .print();
        Ok(())
    } else {
        ui::Ui::print_sync_summary(total_keys_added, total_locales_affected);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::parser;
    use fs_err as fs;

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
    fn run_sync_returns_ok_when_no_crates_match_filter() {
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

        assert!(result.is_ok());
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

        assert!(matches!(result, Err(CliError::LocaleNotFound(_))));
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
    fn collect_affected_locales_deduplicates_namespaced_file_results() {
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
        assert_eq!(collect_affected_locales(results.iter()).len(), 1);
    }
}
