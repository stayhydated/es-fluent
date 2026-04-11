use super::CLI_VERSION;
use super::config::TempCrateConfig;
use super::exec::RunnerCrate;
use crate::core::WorkspaceInfo;
use crate::generation::templates::{
    ConfigTomlTemplate, GitignoreTemplate, MonolithicCargoTomlTemplate, MonolithicCrateDep,
    MonolithicMainRsTemplate,
};
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use es_fluent_derive_core::get_es_fluent_temp_dir;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

pub(super) struct MonolithicRunner<'a> {
    pub(super) workspace: &'a WorkspaceInfo,
    pub(super) temp_dir: PathBuf,
    pub(super) binary_path: PathBuf,
}

impl<'a> MonolithicRunner<'a> {
    pub(super) fn new(workspace: &'a WorkspaceInfo) -> Self {
        Self {
            workspace,
            temp_dir: get_es_fluent_temp_dir(&workspace.root_dir),
            binary_path: get_monolithic_binary_path(workspace),
        }
    }

    pub(super) fn is_stale(&self) -> bool {
        use crate::generation::cache::{RunnerCache, compute_content_hash};

        let runner_mtime = match fs::metadata(&self.binary_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return true,
        };

        let runner_mtime_secs = runner_mtime
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut current_hashes = indexmap::IndexMap::new();
        for krate in &self.workspace.crates {
            if krate.src_dir.exists() {
                let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
                current_hashes.insert(krate.name.clone(), hash);
            }
        }

        if let Some(cache) = RunnerCache::load(&self.temp_dir) {
            if cache.cli_version != CLI_VERSION {
                return true;
            }

            if cache.runner_mtime == runner_mtime_secs {
                for (name, current_hash) in &current_hashes {
                    match cache.crate_hashes.get(name) {
                        Some(cached_hash) if cached_hash == current_hash => continue,
                        _ => return true,
                    }
                }
                for cached_name in cache.crate_hashes.keys() {
                    if !current_hashes.contains_key(cached_name) {
                        return true;
                    }
                }
                return false;
            }

            let new_cache = RunnerCache {
                crate_hashes: current_hashes,
                runner_mtime: runner_mtime_secs,
                cli_version: CLI_VERSION.to_string(),
            };
            let _ = new_cache.save(&self.temp_dir);
            return false;
        }

        true
    }
}

/// Prepare the monolithic runner crate at workspace root that links all workspace crates.
pub fn prepare_monolithic_runner_crate(workspace: &WorkspaceInfo) -> Result<PathBuf> {
    let temp_dir = get_es_fluent_temp_dir(&workspace.root_dir);
    let src_dir = temp_dir.join("src");
    let runner_crate = RunnerCrate::new(&temp_dir);

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    fs::write(
        temp_dir.join(".gitignore"),
        GitignoreTemplate.render().unwrap(),
    )
    .context("Failed to write .es-fluent/.gitignore")?;

    let root_manifest = workspace.root_dir.join("Cargo.toml");
    let config = TempCrateConfig::from_manifest(&root_manifest);

    let crate_deps: Vec<MonolithicCrateDep> = workspace
        .crates
        .iter()
        .filter(|c| c.has_lib_rs)
        .map(|c| MonolithicCrateDep {
            name: &c.name,
            path: c.manifest_dir.display().to_string(),
            ident: c.name.replace('-', "_"),
            has_features: !c.fluent_features.is_empty(),
            features: &c.fluent_features,
        })
        .collect();

    let cargo_toml = MonolithicCargoTomlTemplate {
        crates: crate_deps.clone(),
        es_fluent_dep: &config.es_fluent_dep,
        es_fluent_cli_helpers_dep: &config.es_fluent_cli_helpers_dep,
        manifest_overrides: &config.manifest_overrides,
    };
    runner_crate.write_cargo_toml(&cargo_toml.render().unwrap())?;

    let config_toml = ConfigTomlTemplate {
        target_dir: &config.target_dir,
    };
    runner_crate.write_cargo_config(&config_toml.render().unwrap())?;

    let main_rs = MonolithicMainRsTemplate { crates: crate_deps };
    fs::write(src_dir.join("main.rs"), main_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    let workspace_lock = workspace.root_dir.join("Cargo.lock");
    let runner_lock = temp_dir.join("Cargo.lock");
    if workspace_lock.exists() {
        fs::copy(&workspace_lock, &runner_lock)
            .context("Failed to copy Cargo.lock to .es-fluent/")?;
    }

    Ok(temp_dir)
}

/// Get the path to the monolithic binary if it exists.
pub fn get_monolithic_binary_path(workspace: &WorkspaceInfo) -> PathBuf {
    workspace.target_dir.join("debug").join("es-fluent-runner")
}

/// Run the monolithic binary directly (fast path) or build+run (slow path).
pub fn run_monolithic(
    workspace: &WorkspaceInfo,
    command: &str,
    crate_name: &str,
    extra_args: &[String],
    force_run: bool,
) -> Result<String> {
    let runner = MonolithicRunner::new(workspace);

    if !force_run && runner.binary_path.exists() && !runner.is_stale() {
        let mut cmd = Command::new(&runner.binary_path);
        cmd.arg(command)
            .args(extra_args)
            .arg("--crate")
            .arg(crate_name)
            .current_dir(&runner.temp_dir);

        if env::var("NO_COLOR").is_err() {
            cmd.env("CLICOLOR_FORCE", "1");
        }

        let output = cmd.output().context("Failed to run monolithic binary")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Monolithic binary failed: {}", stderr);
        }

        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let mut args = vec![command.to_string()];
    args.extend(extra_args.iter().cloned());
    args.push("--crate".to_string());
    args.push(crate_name.to_string());
    let result = RunnerCrate::new(&runner.temp_dir).run_cargo(Some("es-fluent-runner"), &args)?;

    write_runner_cache(&runner);

    Ok(result)
}

fn write_runner_cache(runner: &MonolithicRunner<'_>) {
    use crate::generation::cache::{RunnerCache, compute_content_hash};

    if let Ok(meta) = fs::metadata(&runner.binary_path)
        && let Ok(mtime) = meta.modified()
    {
        let runner_mtime_secs = mtime
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut crate_hashes = indexmap::IndexMap::new();
        for krate in &runner.workspace.crates {
            if krate.src_dir.exists() {
                let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
                crate_hashes.insert(krate.name.clone(), hash);
            }
        }

        let cache = RunnerCache {
            crate_hashes,
            runner_mtime: runner_mtime_secs,
            cli_version: CLI_VERSION.to_string(),
        };
        let _ = cache.save(&runner.temp_dir);
    }
}
