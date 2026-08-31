use super::{
    locale,
    preflight::{
        validate_all_locale_paths_are_directories,
        validate_created_target_locales_visible_to_all_scans,
        validate_explicit_assets_dirs_are_directories, validate_explicit_target_locales_exist,
        validate_explicit_targets_are_not_fallbacks,
    },
    report::{
        SyncJsonReport, SyncResultJson, SyncTextMode, collect_affected_locale_targets,
        sync_json_error, sync_json_error_for_workspace,
    },
    selection::{SyncArgs, canonical_locale, validate_sync_target_selection},
};

use super::super::common::WorkspaceCrates;

use crate::{core::CliError, utils::ui};

use std::collections::HashSet;

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

    let target_locales: Option<HashSet<String>> = if args.all_locales {
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

    if args.all_locales
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

    let pb = if show_text {
        ui::Ui::create_progress_bar(crates.len() as u64, "Planning locale changes...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    let mut transaction = es_fluent_runner::FileTransaction::default();
    let mut planned_results = Vec::new();
    for krate in &crates {
        pb.set_message(format!("Planning {}", krate.name));
        let results = match locale::plan_sync_crate(
            krate,
            target_locales.as_ref(),
            args.dry_run,
            args.create,
            &mut transaction,
        ) {
            Ok(results) => results,
            Err(error) => {
                pb.finish_and_clear();
                if args.output.is_json() {
                    return sync_json_error_for_workspace(
                        args.output,
                        args.dry_run,
                        error,
                        &workspace_root,
                    );
                }
                return Err(text_mode.text_error(error));
            },
        };
        planned_results.push((krate.name.to_string(), results));
        pb.inc(1);
    }
    pb.finish_and_clear();

    if !args.dry_run
        && let Err(error) = transaction.commit()
    {
        let error = format!("sync transaction failed: {error}");
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
    for (crate_name, results) in planned_results {
        affected_locale_targets
            .extend(collect_affected_locale_targets(&crate_name, results.iter()));

        for result in results {
            let path = result.path.as_ref().map(|path| {
                crate::utils::paths::relative_slash_path(path.as_path(), &workspace_root)
            });
            json_results.push(SyncResultJson {
                crate_name: crate_name.clone(),
                locale: result.locale.clone(),
                path: path.clone(),
                locale_created: result.locale_created,
                keys_added: result.keys_added,
                added_keys: result.added_keys.clone(),
            });

            if result.locale_created && show_text {
                if args.dry_run {
                    ui::Ui::print_would_create_locale(&result.locale, &crate_name);
                } else {
                    ui::Ui::print_created_locale(&result.locale, &crate_name);
                }
            }

            if result.keys_added > 0 {
                total_keys_added += result.keys_added;

                if show_text {
                    if args.dry_run {
                        ui::Ui::print_would_add_keys(
                            result.keys_added,
                            &result.locale,
                            &crate_name,
                            path.as_deref(),
                        );
                        if let Some(diff) = &result.diff_info {
                            diff.print();
                        }
                    } else {
                        ui::Ui::print_added_keys(
                            result.keys_added,
                            &result.locale,
                            &crate_name,
                            path.as_deref(),
                        );
                        for key in &result.added_keys {
                            ui::Ui::print_synced_key(key);
                        }
                    }
                }
            }
        }
    }
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
