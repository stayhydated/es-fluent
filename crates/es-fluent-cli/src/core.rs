use crate::error::CliError;
use cargo_metadata::{MetadataCommand, Package};
use es_fluent_generate::FluentParseMode;
use getset::Getters;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct CrateInfo {
    name: String,
    manifest_dir: PathBuf,
    src_dir: PathBuf,
    i18n_output_path: PathBuf,
}

#[derive(Clone, Debug)]
pub enum BuildOutcome {
    Success {
        duration: Duration,
    },
    Failure {
        error_message: String,
        duration: Duration,
    },
}

pub fn discover_crates(root_dir: &Path) -> Result<Vec<CrateInfo>, CliError> {
    let mut crates = Vec::new();

    if let Ok(metadata) = MetadataCommand::new().current_dir(root_dir).exec() {
        for package in metadata.workspace_packages() {
            let manifest_dir = package.manifest_path.parent().unwrap();
            if manifest_dir.starts_with(root_dir)
                && let Some(crate_info) = check_crate_for_i18n(package)?
            {
                crates.push(crate_info);
            }
        }
    } else {
        let manifest_path = root_dir.join("Cargo.toml");
        if manifest_path.exists() {
            let metadata = MetadataCommand::new()
                .manifest_path(&manifest_path)
                .exec()?;

            if let Some(package) = metadata.packages.first()
                && let Some(crate_info) = check_crate_for_i18n(package)?
            {
                crates.push(crate_info);
            }
        }
    }

    Ok(crates)
}

fn check_crate_for_i18n(package: &Package) -> Result<Option<CrateInfo>, CliError> {
    let manifest_dir: PathBuf = package.manifest_path.parent().unwrap().into();
    let i18n_config_path = manifest_dir.join("i18n.toml");

    if !i18n_config_path.exists() {
        return Ok(None);
    }

    let i18n_config = i18n_config::I18nConfig::from_file(&i18n_config_path)?;

    let i18n_output_path = match &i18n_config.fluent {
        Some(fluent_config) => {
            let assets_dir = manifest_dir.join(&fluent_config.assets_dir);
            assets_dir.join(i18n_config.fallback_language.to_string())
        },
        None => return Ok(None),
    };

    let src_dir = manifest_dir.join("src");
    if !src_dir.exists() {
        return Ok(None);
    }

    Ok(Some(CrateInfo {
        name: package.name.clone(),
        manifest_dir,
        src_dir,
        i18n_output_path,
    }))
}

pub async fn build_all_crates(
    crates: &[CrateInfo],
    mode: FluentParseMode,
) -> Result<HashMap<String, BuildOutcome>, CliError> {
    let mut results = HashMap::new();
    for krate in crates {
        let outcome = build_crate(krate, mode.clone()).await?;
        results.insert(krate.name.clone(), outcome);
    }
    Ok(results)
}

pub async fn build_crate(
    krate: &CrateInfo,
    mode: FluentParseMode,
) -> Result<BuildOutcome, CliError> {
    let start = Instant::now();
    let crate_name = &krate.name;

    let result = (|| -> Result<(), CliError> {
        if let Some(parent) = krate.i18n_output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = es_fluent_sc_parser::parse_directory(&krate.src_dir)?;
        es_fluent_generate::generate(crate_name, &krate.i18n_output_path, data, mode)?;
        Ok(())
    })();

    let duration = start.elapsed();
    match result {
        Ok(()) => Ok(BuildOutcome::Success { duration }),
        Err(e) => Ok(BuildOutcome::Failure {
            error_message: e.to_string(),
            duration,
        }),
    }
}

pub fn watch_crates_sender(
    crates: &[CrateInfo],
    event_tx: mpsc::Sender<CrateInfo>,
    shutdown_signal: Arc<AtomicBool>,
) -> Result<(), CliError> {
    let (notify_tx, notify_rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())?;

    let mut path_to_crate_map: HashMap<PathBuf, CrateInfo> = HashMap::new();

    for krate in crates {
        if let Err(e) = watcher.watch(&krate.src_dir, RecursiveMode::Recursive) {
            log::error!("Failed to watch directory {:?}: {}", krate.src_dir, e);

            return Err(e.into());
        }
        path_to_crate_map.insert(krate.src_dir.clone(), krate.clone());
    }

    let recv_timeout_duration = Duration::from_millis(250);

    loop {
        if shutdown_signal.load(Ordering::Relaxed) {
            log::debug!("Shutdown signal received in watcher thread. Exiting.");
            break;
        }

        match notify_rx.recv_timeout(recv_timeout_duration) {
            Ok(Ok(event)) => {
                if should_rebuild(&event) {
                    if let Some(affected_crate) = find_affected_crate(&event, &path_to_crate_map) {
                        if event_tx.send(affected_crate.clone()).is_err() {
                            log::error!(
                                "Watch event channel (AppEvent::FileChange) closed. File watcher stopping."
                            );
                            break;
                        }
                    }
                }
            },
            Ok(Err(e)) => {
                log::error!("File watch error: {:?}. File watcher stopping.", e);
                break;
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                log::error!("File watcher's internal channel disconnected. File watcher stopping.");
                break;
            },
        }
    }

    for krate in crates {
        if let Err(e) = watcher.unwatch(&krate.src_dir) {
            log::warn!("Failed to unwatch directory {:?}: {}", krate.src_dir, e);
        }
    }
    log::debug!("File watcher thread has completed cleanup and is exiting.");
    Ok(())
}

fn should_rebuild(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    })
}

fn find_affected_crate(
    event: &Event,
    path_to_crate: &HashMap<PathBuf, CrateInfo>,
) -> Option<CrateInfo> {
    for path in &event.paths {
        for (watched_path, crate_info) in path_to_crate {
            if path.starts_with(watched_path) {
                return Some(crate_info.clone());
            }
        }
    }
    None
}

pub fn format_duration(duration: Duration) -> String {
    if duration.as_nanos() == 0 {
        return "0s".to_string();
    }
    let formatted = humantime::format_duration(duration).to_string();
    formatted
        .split(' ')
        .next()
        .unwrap_or(&formatted)
        .to_string()
}
