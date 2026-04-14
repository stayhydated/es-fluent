use super::CLI_VERSION;
use super::config::TempCrateConfig;
use super::exec::RunnerCrate;
use crate::core::WorkspaceInfo;
use crate::generation::templates::{GitignoreTemplate, MonolithicCrateDep, MonolithicMainRsTemplate};
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use es_fluent_runner::{RunnerMetadataStore, RunnerRequest};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};
use toml::{map::Map, Value};

pub(super) struct MonolithicRunner<'a> {
    pub(super) workspace: &'a WorkspaceInfo,
    pub(super) temp_store: RunnerMetadataStore,
    pub(super) binary_path: PathBuf,
}

impl<'a> MonolithicRunner<'a> {
    pub(super) fn new(workspace: &'a WorkspaceInfo) -> Self {
        Self {
            workspace,
            temp_store: RunnerMetadataStore::temp_for_workspace(&workspace.root_dir),
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

        if let Some(cache) = RunnerCache::load(self.temp_store.base_dir()) {
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
            let _ = new_cache.save(self.temp_store.base_dir());
            return false;
        }

        true
    }
}

/// Prepare the monolithic runner crate at workspace root that links all workspace crates.
pub fn prepare_monolithic_runner_crate(workspace: &WorkspaceInfo) -> Result<PathBuf> {
    let temp_store = RunnerMetadataStore::temp_for_workspace(&workspace.root_dir);
    let src_dir = temp_store.base_dir().join("src");
    let runner_crate = RunnerCrate::new(temp_store.base_dir());

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    fs::write(
        temp_store.base_dir().join(".gitignore"),
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
            path: c.manifest_dir.to_string_lossy().into_owned(),
            ident: c.name.replace('-', "_"),
            has_features: !c.fluent_features.is_empty(),
            features: &c.fluent_features,
        })
        .collect();

    runner_crate.write_cargo_toml(&render_monolithic_cargo_toml(&crate_deps, &config)?)?;

    runner_crate.write_cargo_config(&render_cargo_config_toml(&config.target_dir)?)?;

    let main_rs = MonolithicMainRsTemplate { crates: crate_deps };
    fs::write(src_dir.join("main.rs"), main_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    let workspace_lock = workspace.root_dir.join("Cargo.lock");
    let runner_lock = temp_store.base_dir().join("Cargo.lock");
    if workspace_lock.exists() {
        fs::copy(&workspace_lock, &runner_lock)
            .context("Failed to copy Cargo.lock to .es-fluent/")?;
    }

    Ok(temp_store.base_dir().to_path_buf())
}

/// Get the path to the monolithic binary if it exists.
pub fn get_monolithic_binary_path(workspace: &WorkspaceInfo) -> PathBuf {
    workspace
        .target_dir
        .join("debug")
        .join(format!("es-fluent-runner{}", std::env::consts::EXE_SUFFIX))
}

fn render_monolithic_cargo_toml(
    crate_deps: &[MonolithicCrateDep<'_>],
    config: &TempCrateConfig,
) -> Result<String> {
    let mut package = Map::new();
    package.insert("name".to_string(), Value::String("es-fluent-temp".to_string()));
    package.insert("version".to_string(), Value::String("0.0.0".to_string()));
    package.insert("edition".to_string(), Value::String("2024".to_string()));
    package.insert("publish".to_string(), Value::Boolean(false));

    let mut dependencies = Map::new();
    for dep in crate_deps {
        dependencies.insert(dep.name.to_string(), monolithic_dep_value(dep));
    }
    dependencies.insert(
        "es-fluent".to_string(),
        Value::Table(config.es_fluent_dep.clone()),
    );
    dependencies.insert(
        "es-fluent-cli-helpers".to_string(),
        Value::Table(config.es_fluent_cli_helpers_dep.clone()),
    );

    let mut bin = Map::new();
    bin.insert(
        "name".to_string(),
        Value::String("es-fluent-runner".to_string()),
    );
    bin.insert("path".to_string(), Value::String("src/main.rs".to_string()));

    let mut manifest = Map::new();
    manifest.insert("package".to_string(), Value::Table(package));
    manifest.insert("workspace".to_string(), Value::Table(Map::new()));
    manifest.insert("dependencies".to_string(), Value::Table(dependencies));
    manifest.insert("bin".to_string(), Value::Array(vec![Value::Table(bin)]));
    for (key, value) in &config.manifest_overrides {
        manifest.insert(key.clone(), value.clone());
    }

    toml::to_string(&Value::Table(manifest)).context("Failed to serialize runner Cargo.toml")
}

fn render_cargo_config_toml(target_dir: &str) -> Result<String> {
    let mut build = Map::new();
    build.insert(
        "target-dir".to_string(),
        Value::String(target_dir.to_string()),
    );

    let mut config = Map::new();
    config.insert("build".to_string(), Value::Table(build));

    toml::to_string(&Value::Table(config)).context("Failed to serialize runner config.toml")
}

fn monolithic_dep_value(dep: &MonolithicCrateDep<'_>) -> Value {
    let mut table = Map::new();
    table.insert("path".to_string(), Value::String(dep.path.clone()));
    if dep.has_features {
        table.insert(
            "features".to_string(),
            Value::Array(
                dep.features
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    Value::Table(table)
}

/// Run the monolithic binary directly (fast path) or build+run (slow path).
pub fn run_monolithic(
    workspace: &WorkspaceInfo,
    request: &RunnerRequest,
    force_run: bool,
) -> Result<String> {
    let runner = MonolithicRunner::new(workspace);
    let encoded_request = request.encode()?;

    if !force_run && runner.binary_path.exists() && !runner.is_stale() {
        let mut cmd = Command::new(&runner.binary_path);
        cmd.arg(&encoded_request)
            .current_dir(runner.temp_store.base_dir());

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

    let args = vec![encoded_request];
    let result = RunnerCrate::new(runner.temp_store.base_dir())
        .run_cargo(Some("es-fluent-runner"), &args)?;

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
        let _ = cache.save(runner.temp_store.base_dir());
    }
}
