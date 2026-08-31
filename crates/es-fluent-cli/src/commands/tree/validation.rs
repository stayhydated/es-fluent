use crate::ftl::LocaleContext;

use anyhow::{Context as _, Result};

use std::{fs, path::Path};

use super::super::common::WorkspaceCrates;

use crate::core::CliError;

pub(super) fn validate_tree_workspace_setup(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> Result<(), CliError> {
    for krate in &workspace.crates {
        let ctx = LocaleContext::from_crate(krate, all_locales)
            .map_err(|error| CliError::Other(format!("{}: {}", krate.name, error)))?;
        validate_tree_locale_setup(&ctx, all_locales)
            .map_err(|error| CliError::Other(format!("{}: {}", krate.name, error)))?;

        for locale in &ctx.locales {
            let locale_dir = ctx.locale_dir(locale);
            validate_tree_locale_dir(locale, &locale_dir)
                .map_err(|error| CliError::Other(format!("{}: {}", krate.name, error)))?;
            let ftl_files = ctx
                .discover_files(locale)
                .map_err(|error| CliError::Other(format!("{}: {}", krate.name, error)))?;
            for file_info in ftl_files {
                let relative_path = crate::utils::paths::slash_path(&file_info.relative_path);
                crate::ftl::parse_ftl_file(&file_info.abs_path).map_err(|error| {
                    CliError::Other(format!(
                        "{}: failed to parse FTL file '{relative_path}': {error}",
                        krate.name
                    ))
                })?;
            }
        }
    }

    Ok(())
}

pub(super) fn validate_tree_locale_dir(locale: &str, locale_dir: &Path) -> Result<()> {
    match fs::symlink_metadata(locale_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!(
                "locale directory '{locale}' must be a real directory, not a symlink: {}",
                locale_dir.display()
            )
        },
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => {
            anyhow::bail!(
                "locale directory '{locale}' is missing or not a directory: {}",
                locale_dir.display()
            )
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "locale directory '{locale}' is missing or not a directory: {}",
                locale_dir.display()
            )
        },
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to inspect locale directory '{locale}': {}",
                locale_dir.display()
            )
        }),
    }
}

pub(super) fn validate_tree_locale_setup(ctx: &LocaleContext, all_locales: bool) -> Result<()> {
    match fs::symlink_metadata(&ctx.assets_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!(
                "assets_dir must be a real directory, not a symlink: {}",
                ctx.assets_dir.display()
            );
        },
        Ok(metadata) if metadata.is_dir() => {},
        Ok(_) => {
            anyhow::bail!(
                "assets_dir is missing or not a directory: {}",
                ctx.assets_dir.display()
            );
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "assets_dir is missing or not a directory: {}",
                ctx.assets_dir.display()
            );
        },
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to inspect assets_dir: {}", ctx.assets_dir.display())
            });
        },
    }

    let fallback_dir = ctx.locale_dir(&ctx.fallback);
    validate_tree_locale_dir(&ctx.fallback, &fallback_dir)?;

    if !all_locales {
        return Ok(());
    }

    let issues = crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir)?;
    if let Some(issue) = issues.first() {
        validate_tree_locale_dir(&issue.locale, &issue.path)?;
    }

    Ok(())
}

pub(super) fn relative_tree_message(message: &str, base: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, base)
}
