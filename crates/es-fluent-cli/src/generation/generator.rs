use crate::core::{CrateInfo, GenerationAction, WorkspaceInfo};
use crate::generation::{prepare_monolithic_runner_crate, run_monolithic};
use anyhow::{Result, bail};

fn action_command_and_args(
    krate: &CrateInfo,
    action: &GenerationAction,
) -> (&'static str, Vec<String>) {
    match action {
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
    }
}

/// Generates FTL files for a crate using the monolithic temp crate approach.
/// This is faster on subsequent runs because it reuses a single pre-built binary.
///
/// If `force_run` is true, the staleness check is skipped and the runner is always rebuilt.
pub fn generate_for_crate_monolithic(
    krate: &CrateInfo,
    workspace: &WorkspaceInfo,
    action: &GenerationAction,
    force_run: bool,
) -> Result<String> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    // Ensure monolithic temp crate is prepared (idempotent)
    prepare_monolithic_runner_crate(workspace)?;

    let (command, extra_args) = action_command_and_args(krate, action);

    run_monolithic(workspace, command, &krate.name, &extra_args, force_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FluentParseMode;
    use std::path::PathBuf;

    fn test_crate_info(has_lib_rs: bool) -> CrateInfo {
        CrateInfo {
            name: "test-crate".to_string(),
            manifest_dir: PathBuf::from("/tmp/test-crate"),
            src_dir: PathBuf::from("/tmp/test-crate/src"),
            i18n_config_path: PathBuf::from("/tmp/test-crate/i18n.toml"),
            ftl_output_dir: PathBuf::from("/tmp/test-crate/i18n/en"),
            has_lib_rs,
            fluent_features: Vec::new(),
        }
    }

    fn test_workspace_info() -> WorkspaceInfo {
        WorkspaceInfo {
            root_dir: PathBuf::from("/tmp/workspace"),
            target_dir: PathBuf::from("/tmp/workspace/target"),
            crates: Vec::new(),
        }
    }

    #[test]
    fn action_command_and_args_builds_generate_arguments() {
        let krate = test_crate_info(true);
        let action = GenerationAction::Generate {
            mode: FluentParseMode::Conservative,
            dry_run: true,
        };

        let (command, args) = action_command_and_args(&krate, &action);
        assert_eq!(command, "generate");
        assert_eq!(args[0], "/tmp/test-crate/i18n.toml");
        assert_eq!(args[1], "--mode");
        assert_eq!(args[2], "conservative");
        assert_eq!(args[3], "--dry-run");
    }

    #[test]
    fn action_command_and_args_builds_clean_arguments() {
        let krate = test_crate_info(true);
        let action = GenerationAction::Clean {
            all_locales: true,
            dry_run: true,
        };

        let (command, args) = action_command_and_args(&krate, &action);
        assert_eq!(command, "clean");
        assert_eq!(args[0], "/tmp/test-crate/i18n.toml");
        assert!(args.contains(&"--all".to_string()));
        assert!(args.contains(&"--dry-run".to_string()));
    }

    #[test]
    fn generate_for_crate_monolithic_fails_without_lib_rs() {
        let krate = test_crate_info(false);
        let workspace = test_workspace_info();
        let action = GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: false,
        };

        let err = generate_for_crate_monolithic(&krate, &workspace, &action, false)
            .err()
            .expect("expected missing-lib error");
        assert!(
            err.to_string()
                .contains("inventory requires a library target for linking")
        );
    }
}
