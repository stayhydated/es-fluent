//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

mod locale;
mod merge;

use super::common::{WorkspaceArgs, WorkspaceCrates};
use super::dry_run::DryRunSummary;
use crate::core::{CliError, LocaleNotFoundError, SyncMissingKey};
use crate::ftl::collect_all_available_locales;
use crate::utils::ui;
use clap::Parser;
use std::collections::HashSet;

/// Arguments for the sync command.
#[derive(Debug, Parser)]
pub struct SyncArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Specific locale(s) to sync to (can be specified multiple times).
    #[arg(short, long)]
    pub locale: Vec<String>,

    /// Sync to all locales (excluding the fallback language).
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would be synced without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

/// Run the sync command.
pub fn run_sync(args: SyncArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    ui::print_sync_header();

    let crates = workspace.crates;

    if crates.is_empty() {
        ui::print_no_crates_found();
        return Ok(());
    }

    let target_locales: Option<HashSet<String>> = if args.all {
        None // Will sync to all locales
    } else if args.locale.is_empty() {
        ui::print_no_locales_specified();
        return Ok(());
    } else {
        Some(args.locale.iter().cloned().collect())
    };

    // Validate that specified locales exist
    if let Some(ref targets) = target_locales {
        let all_available_locales = collect_all_available_locales(&crates)?;

        for locale in targets {
            if !all_available_locales.contains(locale) {
                let mut available: Vec<String> = all_available_locales.into_iter().collect();
                available.sort();
                ui::print_locale_not_found(locale, &available);
                return Err(CliError::LocaleNotFound(LocaleNotFoundError {
                    locale: locale.clone(),
                    available: available.join(", "),
                }));
            }
        }
    }

    let mut total_keys_added = 0;
    let mut total_locales_affected = 0;
    let mut all_synced_keys: Vec<SyncMissingKey> = Vec::new();

    let pb = ui::create_progress_bar(crates.len() as u64, "Syncing crates...");

    for krate in &crates {
        pb.set_message(format!("Syncing {}", krate.name));

        let results = locale::sync_crate(krate, target_locales.as_ref(), args.dry_run)?;

        for result in results {
            if result.keys_added > 0 {
                total_locales_affected += 1;
                total_keys_added += result.keys_added;

                pb.suspend(|| {
                    if args.dry_run {
                        ui::print_would_add_keys(result.keys_added, &result.locale, &krate.name);
                        if let Some(diff) = &result.diff_info {
                            diff.print();
                        }
                    } else {
                        ui::print_added_keys(result.keys_added, &result.locale);
                        for key in &result.added_keys {
                            ui::print_synced_key(key);
                            all_synced_keys.push(SyncMissingKey {
                                key: key.clone(),
                                target_locale: result.locale.clone(),
                                source_locale: "fallback".to_string(),
                            });
                        }
                    }
                });
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    if total_keys_added == 0 {
        ui::print_all_in_sync();
        Ok(())
    } else if args.dry_run {
        DryRunSummary::Sync {
            keys: total_keys_added,
            locales: total_locales_affected,
        }
        .print();
        Ok(())
    } else {
        ui::print_sync_summary(total_keys_added, total_locales_affected);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ftl::extract_message_keys;
    use crate::test_fixtures::create_workspace_with_locales;
    use fluent_syntax::parser;
    use std::fs;

    #[test]
    fn test_extract_message_keys() {
        let content = r#"hello = Hello
world = World"#;
        let resource = parser::parse(content.to_string()).unwrap();
        let keys = extract_message_keys(&resource);

        assert!(keys.contains("hello"));
        assert!(keys.contains("world"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn run_sync_returns_ok_when_no_locales_specified() {
        let temp = create_workspace_with_locales(&[
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
            dry_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_sync_returns_ok_when_no_crates_match_filter() {
        let temp = create_workspace_with_locales(&[
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
            dry_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_sync_fails_for_unknown_locale() {
        let temp = create_workspace_with_locales(&[
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
            dry_run: false,
        });

        assert!(matches!(result, Err(CliError::LocaleNotFound(_))));
    }

    #[test]
    fn run_sync_dry_run_does_not_write_missing_keys() {
        let temp = create_workspace_with_locales(&[
            ("en", "hello = Hello\nworld = World\n"),
            ("es", "hello = Hola\n"),
        ]);
        let es_path = temp.path().join("i18n/es/test-app.ftl");
        let before = std::fs::read_to_string(&es_path).expect("read before");

        let result = run_sync(SyncArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            locale: vec!["es".to_string()],
            all: false,
            dry_run: true,
        });

        assert!(result.is_ok());
        let after = std::fs::read_to_string(&es_path).expect("read after");
        assert_eq!(before, after, "dry-run should not modify locale files");
    }

    #[test]
    fn run_sync_writes_missing_keys_for_target_locale() {
        let temp = create_workspace_with_locales(&[
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
            dry_run: false,
        });

        assert!(result.is_ok());
        let es_content = fs::read_to_string(&es_path).expect("read synced es");
        assert!(es_content.contains("world = World"));
    }

    #[test]
    fn run_sync_all_processes_non_fallback_locales() {
        let temp = create_workspace_with_locales(&[
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
            dry_run: false,
        });

        assert!(result.is_ok());
        let fr_content =
            fs::read_to_string(temp.path().join("i18n/fr/test-app.ftl")).expect("read fr");
        assert!(fr_content.contains("world = World"));
    }
}
