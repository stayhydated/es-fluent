mod context;
mod loaded;

#[cfg(test)]
mod tests;

use self::context::ValidationContext;
use super::inventory::ExpectedKeys;
use crate::core::{CrateInfo, ValidationIssue};
use crate::ftl::LocaleContext;
use anyhow::Result;
use std::path::Path;

pub(crate) fn validate_crate(
    krate: &CrateInfo,
    workspace_root: &Path,
    temp_dir: &Path,
    check_all: bool,
    check_fallback_copies: bool,
) -> Result<Vec<ValidationIssue>> {
    let expected_keys = super::inventory::read_inventory_file(temp_dir, &krate.name)?;
    validate_ftl_files(
        krate,
        workspace_root,
        &expected_keys,
        check_all,
        check_fallback_copies,
    )
}

fn validate_ftl_files(
    krate: &CrateInfo,
    workspace_root: &Path,
    expected_keys: &ExpectedKeys,
    check_all: bool,
    check_fallback_copies: bool,
) -> Result<Vec<ValidationIssue>> {
    let locale_ctx = LocaleContext::from_crate(krate, check_all)?;
    let ctx = ValidationContext {
        expected_keys,
        owner: krate.name.as_str(),
        workspace_root,
        manifest_dir: &krate.manifest_dir,
    };
    let check_fallback_copies =
        check_all && check_fallback_copies && locale_ctx.check_fallback_copies;
    // Loaded once, then used for the fallback-copy comparison and as the
    // fallback locale's own files in the loop below.
    let mut fallback_files =
        check_fallback_copies.then(|| locale_ctx.discover_and_load_files(&locale_ctx.fallback));
    let fallback_keys = fallback_files
        .as_ref()
        .and_then(|files| files.as_ref().ok())
        .map(|files| loaded::collect_fallback_keys(&ctx, files));

    let mut issues = Vec::new();

    for locale in &locale_ctx.locales {
        let loaded_files = fallback_files
            .take_if(|_| locale == &locale_ctx.fallback)
            .unwrap_or_else(|| locale_ctx.discover_and_load_files(locale));
        match loaded_files {
            Ok(loaded_files) if loaded_files.is_empty() => {
                let ftl_abs_path = crate::ftl::main_ftl_path(
                    &locale_ctx.assets_dir,
                    locale,
                    &locale_ctx.crate_name,
                );
                let ftl_relative_path = ctx.to_relative_path(&ftl_abs_path);
                let ftl_header_link = ctx.format_terminal_link(
                    &ftl_relative_path,
                    &format!("file://{}", ftl_abs_path.display()),
                );

                issues.extend(ctx.missing_file_issues(locale, &ftl_header_link));
            },
            Ok(loaded_files) => {
                issues.extend(loaded::validate_loaded_ftl_files(
                    &ctx,
                    loaded_files,
                    locale,
                    &locale_ctx.fallback,
                    fallback_keys.as_ref(),
                ));
            },
            Err(error) => {
                let ftl_abs_path = crate::ftl::main_ftl_path(
                    &locale_ctx.assets_dir,
                    locale,
                    &locale_ctx.crate_name,
                );
                issues.push(ctx.syntax_error_issue(
                    locale,
                    &ftl_abs_path,
                    format!("Failed to discover FTL files: {}", error),
                ));
            },
        }
    }

    Ok(issues)
}
