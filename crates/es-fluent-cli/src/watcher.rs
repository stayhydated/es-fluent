//! Watcher module - watches for file changes and triggers regeneration

use crate::discovery::count_ftl_resources;
use crate::generator;
use crate::types::{CrateInfo, CrateState};
use crate::ui;
use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Compute a hash of all .rs files in the src directory.
fn compute_src_hash(src_dir: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();

    let entries: Vec<_> = walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .collect();

    // Sort for deterministic ordering
    let mut paths: Vec<_> = entries.iter().map(|e| e.path().to_path_buf()).collect();
    paths.sort();

    for path in paths {
        if let Ok(content) = fs::read_to_string(&path) {
            path.hash(&mut hasher);
            content.hash(&mut hasher);
        }
    }

    hasher.finish()
}

/// Watch for changes and regenerate FTL files for all discovered crates.
pub fn watch_all(crates: &[CrateInfo], running: Arc<AtomicBool>) -> Result<()> {
    if crates.is_empty() {
        anyhow::bail!("No crates to watch");
    }

    // Initialize states for all crates
    let mut states: HashMap<String, CrateState> = HashMap::new();
    let mut src_hashes: HashMap<String, u64> = HashMap::new();

    // Map from watched path to crate name
    let mut path_to_crate: HashMap<std::path::PathBuf, String> = HashMap::new();

    // Check which crates are valid (have lib.rs)
    for krate in crates {
        if !krate.has_lib_rs {
            states.insert(krate.name.clone(), CrateState::MissingLibRs);
        } else {
            states.insert(krate.name.clone(), CrateState::Generating);
            src_hashes.insert(krate.name.clone(), compute_src_hash(&krate.src_dir));
            path_to_crate.insert(krate.src_dir.clone(), krate.name.clone());
        }
    }

    // Print initial state
    ui::print_summary(crates, &states);

    // Initial generation for all valid crates
    for krate in crates {
        if !krate.has_lib_rs {
            continue;
        }

        ui::print_generating(&krate.name);
        let start = Instant::now();

        match generator::generate_for_crate(krate) {
            Ok(()) => {
                let duration = start.elapsed();
                let resource_count = count_ftl_resources(&krate.ftl_output_dir, &krate.name);
                ui::print_generated(&krate.name, duration, resource_count);
                states.insert(krate.name.clone(), CrateState::Watching { resource_count });
            },
            Err(e) => {
                ui::print_generation_error(&krate.name, &e.to_string());
                states.insert(
                    krate.name.clone(),
                    CrateState::Error {
                        message: e.to_string(),
                    },
                );
            },
        }
    }

    // Print summary after initial generation
    ui::print_summary(crates, &states);
    ui::print_watching();

    // Set up file watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        new_debouncer(Duration::from_millis(300), tx).context("Failed to create file watcher")?;

    // Watch all src directories and i18n.toml files
    for krate in crates {
        if !krate.has_lib_rs {
            continue;
        }

        debouncer
            .watcher()
            .watch(&krate.src_dir, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", krate.src_dir.display()))?;

        if krate.i18n_config_path.exists() {
            debouncer
                .watcher()
                .watch(&krate.i18n_config_path, RecursiveMode::NonRecursive)
                .with_context(|| format!("Failed to watch {}", krate.i18n_config_path.display()))?;
        }
    }

    // Track pending rebuilds per crate
    let mut pending_rebuilds: HashMap<String, bool> = HashMap::new();
    let mut is_building: HashMap<String, bool> = HashMap::new();

    // Main watch loop
    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(events)) => {
                // Group events by crate
                let mut affected_crates: HashMap<String, Vec<String>> = HashMap::new();

                for event in &events {
                    let path = &event.path;

                    // Skip files in any .es-fluent temp directory
                    if path.components().any(|c| c.as_os_str() == ".es-fluent") {
                        continue;
                    }

                    // Skip .ftl files
                    if path.extension().is_some_and(|ext| ext == "ftl") {
                        continue;
                    }

                    // Find which crate this file belongs to
                    for (src_dir, crate_name) in &path_to_crate {
                        if path.starts_with(src_dir) || path.ends_with("i18n.toml") {
                            // Check if it's a relevant file
                            let is_rs = path.extension().is_some_and(|ext| ext == "rs");
                            let is_i18n = path.file_name().is_some_and(|n| n == "i18n.toml");

                            if is_rs || is_i18n {
                                let file_name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "file".to_string());

                                affected_crates
                                    .entry(crate_name.clone())
                                    .or_default()
                                    .push(file_name);
                            }
                            break;
                        }
                    }
                }

                // Process each affected crate
                for (crate_name, changed_files) in affected_crates {
                    // If already building, mark as pending
                    if *is_building.get(&crate_name).unwrap_or(&false) {
                        pending_rebuilds.insert(crate_name.clone(), true);
                        continue;
                    }

                    // Find the crate info
                    let Some(krate) = crates.iter().find(|k| k.name == crate_name) else {
                        continue;
                    };

                    // Check if content actually changed
                    let new_hash = compute_src_hash(&krate.src_dir);
                    let old_hash = src_hashes.get(&crate_name).copied().unwrap_or(0);
                    if new_hash == old_hash {
                        continue;
                    }

                    // Print file change notification
                    let file_name = changed_files.first().map(String::as_str).unwrap_or("file");
                    ui::print_file_changed(&crate_name, file_name);

                    // Mark as building
                    is_building.insert(crate_name.clone(), true);
                    states.insert(crate_name.clone(), CrateState::Generating);

                    // Generate
                    let start = Instant::now();
                    match generator::generate_for_crate(krate) {
                        Ok(()) => {
                            let duration = start.elapsed();
                            let resource_count =
                                count_ftl_resources(&krate.ftl_output_dir, &krate.name);
                            ui::print_generated(&crate_name, duration, resource_count);
                            states.insert(
                                crate_name.clone(),
                                CrateState::Watching { resource_count },
                            );
                        },
                        Err(e) => {
                            ui::print_generation_error(&crate_name, &e.to_string());
                            states.insert(
                                crate_name.clone(),
                                CrateState::Error {
                                    message: e.to_string(),
                                },
                            );
                        },
                    }

                    // Update hash
                    src_hashes.insert(crate_name.clone(), compute_src_hash(&krate.src_dir));
                    is_building.insert(crate_name.clone(), false);

                    // Check for pending rebuild
                    if pending_rebuilds.get(&crate_name).copied().unwrap_or(false) {
                        pending_rebuilds.insert(crate_name.clone(), false);
                        // Will be handled on next iteration if hash changed
                    }
                }
            },
            Ok(Err(e)) => {
                eprintln!("Watch error: {:?}", e);
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            },
        }
    }

    Ok(())
}
