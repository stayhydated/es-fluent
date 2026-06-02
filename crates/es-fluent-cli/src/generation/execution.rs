use crate::core::{CrateInfo, GenerateResult, GenerationAction, WorkspaceInfo};
use anyhow::{Result, bail};
use es_fluent_runner::{I18nTomlPath, RunnerMetadataStore, RunnerRequest};
use std::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerExecution {
    pub output: String,
    pub changed: bool,
}

impl GenerationAction {
    pub(crate) fn to_runner_request(&self, krate: &CrateInfo) -> RunnerRequest {
        let crate_name = krate.name.clone();
        let i18n_toml_path = I18nTomlPath::new(krate.i18n_config_path.clone())
            .expect("discovered i18n.toml paths should be non-empty");
        match self {
            GenerationAction::Generate { mode, dry_run } => RunnerRequest::Generate {
                crate_name,
                i18n_toml_path,
                mode: *mode,
                dry_run: *dry_run,
            },
            GenerationAction::Clean {
                all_locales,
                dry_run,
            } => RunnerRequest::Clean {
                crate_name,
                i18n_toml_path,
                all_locales: *all_locales,
                dry_run: *dry_run,
            },
        }
    }
}

impl CrateInfo {
    pub(crate) fn check_request(&self) -> RunnerRequest {
        RunnerRequest::Check {
            crate_name: self.name.clone(),
            manifest_dir: self.manifest_dir.to_path_buf(),
        }
    }

    fn ensure_inventory_library_target(&self) -> Result<()> {
        if !self.has_lib_rs {
            bail!(
                "Crate '{}' has no lib.rs - inventory requires a library target for linking",
                self.name
            );
        }

        Ok(())
    }
}

pub struct MonolithicExecutor<'a> {
    workspace: &'a WorkspaceInfo,
    metadata_store: RunnerMetadataStore,
}

impl<'a> MonolithicExecutor<'a> {
    pub(crate) fn new(workspace: &'a WorkspaceInfo) -> Self {
        Self {
            workspace,
            metadata_store: RunnerMetadataStore::temp_for_workspace(&workspace.root_dir),
        }
    }

    pub(crate) fn execute_request(
        &self,
        request: &RunnerRequest,
        force_run: bool,
    ) -> Result<RunnerExecution> {
        let output = super::runner::run_monolithic(self.workspace, request, force_run)?;
        let changed = match request {
            RunnerRequest::Generate { crate_name, .. }
            | RunnerRequest::Clean { crate_name, .. } => {
                self.metadata_store.result_changed(crate_name)
            },
            RunnerRequest::Check { .. } => false,
        };

        Ok(RunnerExecution { output, changed })
    }

    pub(crate) fn execute_generation_action(
        &self,
        krate: &CrateInfo,
        action: &GenerationAction,
        force_run: bool,
    ) -> GenerateResult {
        let start = Instant::now();
        let execution = self.try_execute_generation_action(krate, action, force_run);
        let duration = start.elapsed();

        match execution {
            Ok(execution) => GenerateResult::success(
                krate.name.to_string(),
                duration,
                crate::utils::count_ftl_resources(&krate.ftl_output_dir, krate.name.as_str()),
                normalize_output(execution.output),
                execution.changed,
            ),
            Err(error) => {
                GenerateResult::failure(krate.name.to_string(), duration, format!("{error:#}"))
            },
        }
    }

    fn try_execute_generation_action(
        &self,
        krate: &CrateInfo,
        action: &GenerationAction,
        force_run: bool,
    ) -> Result<RunnerExecution> {
        krate.ensure_inventory_library_target()?;
        let request = action.to_runner_request(krate);
        self.execute_request(&request, force_run)
    }
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
    use es_fluent_runner::PackageName;
    use std::fs;
    use std::path::PathBuf;

    fn package(name: &str) -> PackageName {
        PackageName::try_new(name).expect("package")
    }

    fn i18n_path(path: &str) -> I18nTomlPath {
        I18nTomlPath::new(path).expect("i18n path")
    }

    fn test_crate_info(has_lib_rs: bool) -> CrateInfo {
        CrateInfo {
            name: es_fluent_runner::PackageName::try_new("test-crate").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from(
                "/tmp/test-crate",
            )),
            src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/test-crate/src")),
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
    fn generation_action_builds_generate_request() {
        let krate = test_crate_info(true);
        let action = GenerationAction::Generate {
            mode: FluentParseMode::Conservative,
            dry_run: true,
        };

        let request = action.to_runner_request(&krate);
        assert_eq!(
            request,
            RunnerRequest::Generate {
                crate_name: package("test-crate"),
                i18n_toml_path: i18n_path("/tmp/test-crate/i18n.toml"),
                mode: FluentParseMode::Conservative,
                dry_run: true,
            }
        );
    }

    #[test]
    fn generation_action_builds_clean_request() {
        let krate = test_crate_info(true);
        let action = GenerationAction::Clean {
            all_locales: true,
            dry_run: true,
        };

        let request = action.to_runner_request(&krate);
        assert_eq!(
            request,
            RunnerRequest::Clean {
                crate_name: package("test-crate"),
                i18n_toml_path: i18n_path("/tmp/test-crate/i18n.toml"),
                all_locales: true,
                dry_run: true,
            }
        );
    }

    #[test]
    fn crate_info_builds_check_request() {
        let krate = test_crate_info(true);
        assert_eq!(
            krate.check_request(),
            RunnerRequest::Check {
                crate_name: package("test-crate"),
                manifest_dir: PathBuf::from("/tmp/test-crate"),
            }
        );
    }

    #[test]
    fn metadata_store_handles_missing_invalid_and_valid_changed_status() {
        let temp = tempfile::tempdir().expect("tempdir");
        let crate_name = "demo";
        let package_name = package(crate_name);
        let store = RunnerMetadataStore::new(temp.path());
        let result_path = store.result_path(&package_name);
        fs::create_dir_all(result_path.parent().expect("result parent")).expect("create dir");

        assert!(!store.result_changed(&package_name));

        fs::write(&result_path, "{not-json").expect("write invalid json");
        assert!(!store.result_changed(&package_name));

        fs::write(&result_path, r#"{"changed":true}"#).expect("write valid json");
        assert!(store.result_changed(&package_name));
    }

    #[test]
    fn execute_generation_action_fails_without_lib_rs() {
        let krate = test_crate_info(false);
        let workspace = test_workspace_info();
        let action = GenerationAction::Generate {
            mode: FluentParseMode::default(),
            dry_run: false,
        };
        let executor = MonolithicExecutor::new(&workspace);

        let result = executor.execute_generation_action(&krate, &action, false);
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
