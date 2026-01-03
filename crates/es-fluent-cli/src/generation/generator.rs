use crate::core::{CrateInfo, GenerationAction};
use crate::generation::{prepare_temp_crate, run_cargo};
use anyhow::{Result, bail};

/// Generates FTL files for a crate using the CrateInfo struct.
pub fn generate_for_crate(krate: &CrateInfo, action: &GenerationAction) -> Result<()> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    let temp_dir = prepare_temp_crate(krate)?;

    let args = match action {
        GenerationAction::Generate(mode) => {
            // FluentParseMode Display implementation typically matches clap ValueEnum (lowercase)
            vec![
                "generate".to_string(),
                "--mode".to_string(),
                mode.to_string().to_lowercase(),
            ]
        },
        GenerationAction::Clean {
            all_locales,
            dry_run,
        } => {
            let mut args = vec!["clean".to_string()];
            if *all_locales {
                args.push("--all".to_string());
            }
            if *dry_run {
                args.push("--dry-run".to_string());
            }
            args
        },
    };

    run_cargo(&temp_dir, Some("generate"), &args)
}
