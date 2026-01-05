use crate::core::{CrateInfo, GenerationAction, WorkspaceInfo};
use crate::generation::{prepare_monolithic_temp_crate, run_monolithic};
use anyhow::{Result, bail};

/// Generates FTL files for a crate using the monolithic temp crate approach.
/// This is faster on subsequent runs because it reuses a single pre-built binary.
pub fn generate_for_crate_monolithic(
    krate: &CrateInfo,
    workspace: &WorkspaceInfo,
    action: &GenerationAction,
) -> Result<String> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    // Ensure monolithic temp crate is prepared (idempotent)
    prepare_monolithic_temp_crate(workspace)?;

    let (command, extra_args) = match action {
        GenerationAction::Generate { mode, dry_run } => {
            let mut args = vec![krate.i18n_config_path.display().to_string()];
            args.push("--mode".to_string());
            args.push(mode.to_string().to_lowercase());
            if *dry_run {
                args.push("--dry-run".to_string());
            }
            ("generate", args)
        },
        GenerationAction::Clean {
            all_locales,
            dry_run,
        } => {
            let mut args = vec![krate.i18n_config_path.display().to_string()];
            if *all_locales {
                args.push("--all".to_string());
            }
            if *dry_run {
                args.push("--dry-run".to_string());
            }
            ("clean", args)
        },
    };

    run_monolithic(workspace, command, &krate.name, &extra_args)
}
