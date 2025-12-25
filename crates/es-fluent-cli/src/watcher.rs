//! Watcher module - watches for file changes and triggers regeneration

use crate::generator;
use anyhow::{Context, Result};
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

/// Compute a hash of all .rs files in the src directory
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

/// Watch for changes and regenerate FTL files
pub fn watch(crate_path: &Path, package: Option<&str>) -> Result<()> {
    let crate_path = crate_path
        .canonicalize()
        .context("Failed to canonicalize crate path")?;

    let src_dir = crate_path.join("src");
    if !src_dir.exists() {
        anyhow::bail!("No src directory found at {}", crate_path.display());
    }

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!(
            "\n{} {}",
            "[es-fluent]".cyan().bold(),
            "Shutting down...".dimmed()
        );
        r.store(false, Ordering::SeqCst);
        std::process::exit(0);
    })
    .context("Failed to set Ctrl+C handler")?;

    println!(
        "{} {} {}",
        "[es-fluent]".cyan().bold(),
        "Watching".dimmed(),
        src_dir.display().to_string().green()
    );
    println!(
        "{} {}",
        "[es-fluent]".cyan().bold(),
        "Press Ctrl+C to stop".dimmed()
    );

    // Track last known hash of source files
    let mut last_hash = compute_src_hash(&src_dir);

    // Initial generation
    if let Err(e) = generator::generate_once(&crate_path, package) {
        eprintln!(
            "{} {} {}",
            "[es-fluent]".red().bold(),
            "Initial generation failed:".red(),
            e
        );
    }

    // Set up file watcher
    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)
        .context("Failed to create file watcher")?;

    debouncer
        .watcher()
        .watch(&src_dir, RecursiveMode::Recursive)
        .context("Failed to watch src directory")?;

    // Also watch i18n.toml if it exists
    let i18n_toml = crate_path.join("i18n.toml");
    if i18n_toml.exists() {
        debouncer
            .watcher()
            .watch(&i18n_toml, RecursiveMode::NonRecursive)
            .context("Failed to watch i18n.toml")?;
    }

    // Track if we're currently building and if changes happened during build
    let mut is_building = false;
    let mut pending_rebuild = false;

    // Main watch loop
    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(events)) => {
                // If building, mark as pending and continue
                if is_building {
                    pending_rebuild = true;
                    continue;
                }

                // Filter out non-source files
                let es_fluent_dir = crate_path.join(".es-fluent");
                let has_relevant_changes = events.iter().any(|e| {
                    let path = &e.path;
                    
                    // Skip files in the .es-fluent temp directory
                    if path.starts_with(&es_fluent_dir) {
                        return false;
                    }
                    
                    // Skip .ftl files
                    if path.extension().is_some_and(|ext| ext == "ftl") {
                        return false;
                    }
                    
                    // Accept .rs files or i18n.toml
                    path.extension().is_some_and(|ext| ext == "rs")
                        || path.file_name().is_some_and(|name| name == "i18n.toml")
                });

                if has_relevant_changes {
                    // Check if content actually changed using hash
                    let new_hash = compute_src_hash(&src_dir);
                    if new_hash == last_hash {
                        // Content unchanged, skip rebuild
                        continue;
                    }
                    
                    // Get changed file for display
                    let changed_file = events
                        .iter()
                        .filter(|e| e.path.extension().is_some_and(|ext| ext == "rs"))
                        .filter_map(|e| e.path.file_name())
                        .next()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "source files".to_string());

                    println!(
                        "{} {} {}",
                        "[es-fluent]".cyan().bold(),
                        "File changed:".dimmed(),
                        changed_file.yellow()
                    );

                    is_building = true;
                    
                    if let Err(e) = generator::generate_once(&crate_path, package) {
                        eprintln!(
                            "{} {} {}",
                            "[es-fluent]".red().bold(),
                            "Generation failed:".red(),
                            e
                        );
                    }
                    
                    // Update hash after build
                    last_hash = compute_src_hash(&src_dir);
                    is_building = false;
                    
                    // Drain any events that accumulated during build
                    while rx.try_recv().is_ok() {}
                    
                    // If changes happened during build, rebuild
                    if pending_rebuild {
                        pending_rebuild = false;
                        // Check if content actually changed
                        let new_hash = compute_src_hash(&src_dir);
                        if new_hash != last_hash {
                            println!(
                                "{} {}",
                                "[es-fluent]".cyan().bold(),
                                "Changes detected during build, rebuilding...".yellow()
                            );
                            
                            if let Err(e) = generator::generate_once(&crate_path, package) {
                                eprintln!(
                                    "{} {} {}",
                                    "[es-fluent]".red().bold(),
                                    "Generation failed:".red(),
                                    e
                                );
                            }
                            last_hash = compute_src_hash(&src_dir);
                            while rx.try_recv().is_ok() {}
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!(
                    "{} {} {:?}",
                    "[es-fluent]".red().bold(),
                    "Watch error:".red(),
                    e
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}
