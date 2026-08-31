use crate::core::CliError;

use fs_err as fs;

use std::collections::HashSet;

pub(super) fn validate_explicit_targets_are_not_fallbacks(
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

pub(super) fn validate_explicit_target_locales_exist(
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

pub(super) fn validate_created_target_locales_visible_to_all_scans(
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

pub(super) fn validate_explicit_assets_dirs_are_directories(
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

pub(super) fn validate_all_locale_paths_are_directories(
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
