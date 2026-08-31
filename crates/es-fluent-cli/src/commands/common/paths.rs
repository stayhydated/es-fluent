use crate::core::{CliError, CrateInfo};
use std::path::{Path, PathBuf};

pub(crate) fn validate_generation_paths(
    crates: &[CrateInfo],
    validate_fallback_locale: bool,
) -> Result<(), CliError> {
    let mut invalid_paths = Vec::new();

    for krate in crates {
        let ctx = crate::ftl::LocaleContext::from_crate(krate, false)
            .map_err(|error| CliError::Other(error.to_string()))?;

        if ctx.assets_dir.exists() && !ctx.assets_dir.is_dir() {
            invalid_paths.push(format!(
                "assets_dir for {} is not a directory: {}",
                krate.name,
                ctx.assets_dir.display()
            ));
        }

        if let Some(blocked_path) = non_directory_ancestor(&ctx.assets_dir) {
            invalid_paths.push(format!(
                "assets_dir for {} cannot be created because a path component is not a directory: {}",
                krate.name,
                blocked_path.display()
            ));
        }

        let fallback_dir = ctx.locale_dir(&ctx.fallback);
        if validate_fallback_locale && fallback_dir.exists() && !fallback_dir.is_dir() {
            invalid_paths.push(format!(
                "fallback locale path '{}' for {} is not a directory: {}",
                ctx.fallback,
                krate.name,
                fallback_dir.display()
            ));
        }

        if validate_fallback_locale
            && let Some(blocked_path) = non_directory_ancestor(&fallback_dir)
        {
            invalid_paths.push(format!(
                "fallback locale path '{}' for {} cannot be created because a path component is not a directory: {}",
                ctx.fallback,
                krate.name,
                blocked_path.display()
            ));
        }

        if validate_fallback_locale && fallback_dir.is_dir() {
            let layout = crate::ftl::CrateFtlLayout::from_assets_dir(
                &ctx.assets_dir,
                &ctx.fallback,
                krate.name.as_str(),
            );
            if let Err(error) = layout.discover_files() {
                invalid_paths.push(format!(
                    "fallback locale FTL layout for {} could not be read: {}",
                    krate.name, error
                ));
            }
        }
    }

    if !invalid_paths.is_empty() {
        invalid_paths.sort();
        return Err(CliError::Other(format!(
            "generation path setup error(s): {}",
            invalid_paths.join(", ")
        )));
    }

    Ok(())
}

fn non_directory_ancestor(path: &Path) -> Option<PathBuf> {
    path.parent()?
        .ancestors()
        .find(|ancestor| ancestor.exists() && !ancestor.is_dir())
        .map(Path::to_path_buf)
}
