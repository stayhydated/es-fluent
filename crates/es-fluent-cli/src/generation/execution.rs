use super::runner::run_monolithic;
use crate::core::{CrateInfo, GenerateResult, GenerationAction, WorkspaceInfo};
use crate::utils::count_ftl_resources;
use anyhow::{Result, bail};
use es_fluent_runner::{RunnerParseMode, RunnerRequest};
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerExecution {
    pub output: String,
    pub changed: bool,
}

pub fn build_action_request(krate: &CrateInfo, action: &GenerationAction) -> RunnerRequest {
    match action {
        GenerationAction::Generate { mode, dry_run } => RunnerRequest::Generate {
            crate_name: krate.name.clone(),
            i18n_toml_path: krate.i18n_config_path.display().to_string(),
            mode: match mode {
                crate::core::FluentParseMode::Conservative => RunnerParseMode::Conservative,
                crate::core::FluentParseMode::Aggressive => RunnerParseMode::Aggressive,
            },
            dry_run: *dry_run,
        },
        GenerationAction::Clean {
            all_locales,
            dry_run,
        } => RunnerRequest::Clean {
            crate_name: krate.name.clone(),
            i18n_toml_path: krate.i18n_config_path.display().to_string(),
            all_locales: *all_locales,
            dry_run: *dry_run,
        },
    }
}

pub fn build_check_request(krate: &CrateInfo) -> RunnerRequest {
    RunnerRequest::Check {
        crate_name: krate.name.clone(),
    }
}

pub fn read_changed_status(temp_dir: &Path, crate_name: &str) -> bool {
    es_fluent_runner::read_result(temp_dir, crate_name)
        .map(|result| result.changed)
        .unwrap_or(false)
}

pub fn execute_request_monolithic(
    workspace: &WorkspaceInfo,
    request: &RunnerRequest,
    force_run: bool,
) -> Result<RunnerExecution> {
    let output = run_monolithic(workspace, request, force_run)?;
    let changed = match request {
        RunnerRequest::Generate { crate_name, .. } | RunnerRequest::Clean { crate_name, .. } => {
            let temp_dir = es_fluent_runner::get_es_fluent_temp_dir(&workspace.root_dir);
            read_changed_status(&temp_dir, crate_name)
        },
        RunnerRequest::Check { .. } => false,
    };

    Ok(RunnerExecution { output, changed })
}

pub fn execute_generation_action_monolithic(
    krate: &CrateInfo,
    workspace: &WorkspaceInfo,
    action: &GenerationAction,
    force_run: bool,
) -> GenerateResult {
    let start = Instant::now();
    let execution = try_execute_generation_action_monolithic(krate, workspace, action, force_run);
    let duration = start.elapsed();

    match execution {
        Ok(execution) => GenerateResult::success(
            krate.name.clone(),
            duration,
            count_ftl_resources(&krate.ftl_output_dir, &krate.name),
            normalize_output(execution.output),
            execution.changed,
        ),
        Err(error) => GenerateResult::failure(krate.name.clone(), duration, error.to_string()),
    }
}

fn try_execute_generation_action_monolithic(
    krate: &CrateInfo,
    workspace: &WorkspaceInfo,
    action: &GenerationAction,
    force_run: bool,
) -> Result<RunnerExecution> {
    ensure_inventory_library_target(krate)?;
    let request = build_action_request(krate, action);
    execute_request_monolithic(workspace, &request, force_run)
}

fn ensure_inventory_library_target(krate: &CrateInfo) -> Result<()> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    Ok(())
}

fn normalize_output(output: String) -> Option<String> {
    if output.trim().is_empty() {
        None
    } else {
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FluentParseMode;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

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
    fn build_action_request_builds_generate_request() {
        let krate = test_crate_info(true);
        let action = GenerationAction::Generate {
            mode: FluentParseMode::Conservative,
            dry_run: true,
        };

        let request = build_action_request(&krate, &action);
        assert_eq!(
            request,
            RunnerRequest::Generate {
                crate_name: "test-crate".to_string(),
                i18n_toml_path: "/tmp/test-crate/i18n.toml".to_string(),
                mode: RunnerParseMode::Conservative,
                dry_run: true,
            }
        );
    }

    #[test]
    fn build_action_request_builds_clean_request() {
        let krate = test_crate_info(true);
        let action = GenerationAction::Clean {
            all_locales: true,
            dry_run: true,
        };

        let request = build_action_request(&krate, &action);
        assert_eq!(
            request,
            RunnerRequest::Clean {
                crate_name: "test-crate".to_string(),
                i18n_toml_path: "/tmp/test-crate/i18n.toml".to_string(),
                all_locales: true,
                dry_run: true,
            }
        );
    }

    #[test]
    fn build_check_request_uses_crate_name() {
        let krate = test_crate_info(true);
        assert_eq!(
            build_check_request(&krate),
            RunnerRequest::Check {
                crate_name: "test-crate".to_string(),
            }
        );
    }

    #[test]
    fn read_changed_status_handles_missing_invalid_and_valid_json() {
        let temp = tempdir().expect("tempdir");
        let crate_name = "demo";
        let result_path = es_fluent_runner::result_path(temp.path(), crate_name);
        fs::create_dir_all(result_path.parent().expect("result parent")).expect("create dir");

        assert!(!read_changed_status(temp.path(), crate_name));

        fs::write(&result_path, "{not-json").expect("write invalid json");
        assert!(!read_changed_status(temp.path(), crate_name));

        fs::write(&result_path, r#"{"changed":true}"#).expect("write valid json");
        assert!(read_changed_status(temp.path(), crate_name));
    }

    #[test]
    fn execute_generation_action_monolithic_fails_without_lib_rs() {
        let krate = test_crate_info(false);
        let workspace = test_workspace_info();
        let action = GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: false,
        };

        let result = execute_generation_action_monolithic(&krate, &workspace, &action, false);
        assert_eq!(result.name, "test-crate");
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .as_ref()
                .expect("missing-lib error")
                .contains("inventory requires a library target for linking")
        );
    }

    #[test]
    fn normalize_output_ignores_whitespace_only_output() {
        assert_eq!(normalize_output("   \n\t".to_string()), None);
        assert_eq!(
            normalize_output("generated output\n".to_string()),
            Some("generated output\n".to_string())
        );
    }
}
