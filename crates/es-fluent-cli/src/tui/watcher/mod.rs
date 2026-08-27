//! File watcher and main TUI event loop.

mod events;
mod generation;
mod runtime;

#[cfg(test)]
mod tests;

use self::runtime::{BuildSourceWatchUpdate, WatchRuntime};
use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::tui::{self, TuiApp};
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, RecvTimeoutError};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, RecommendedCache};
use ratatui::{Terminal, backend::Backend};
use std::collections::BTreeSet;
use std::time::Duration;

type FileDebouncer = notify_debouncer_full::Debouncer<RecommendedWatcher, RecommendedCache>;

/// Watch for changes and regenerate FTL files for all discovered crates.
pub fn watch_all(
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
) -> Result<()> {
    if crates.is_empty() {
        anyhow::bail!("No crates to watch");
    }

    let runner_workspace = workspace_for_crates(workspace, crates);
    {
        let _runner_lock =
            crate::generation::acquire_monolithic_runner_lock(&runner_workspace.root_dir)?;
        crate::generation::prepare_monolithic_runner_crate(&runner_workspace)?;
    }

    run_watch_terminal(crates, &runner_workspace, mode)
}

pub(super) fn workspace_for_crates(
    workspace: &WorkspaceInfo,
    crates: &[CrateInfo],
) -> WorkspaceInfo {
    WorkspaceInfo {
        root_dir: workspace.root_dir.clone(),
        target_dir: workspace.target_dir.clone(),
        crates: crates.to_vec(),
    }
}

#[cfg(not(any(test, coverage)))]
fn run_watch_terminal(
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
) -> Result<()> {
    let mut terminal = ratatui::init();
    let poll = tui::poll_quit_event;
    let result = run_watch_loop_with_poll(&mut terminal, crates, workspace, mode, poll, None);
    ratatui::restore();

    result
}

#[cfg(any(test, coverage))]
fn run_watch_terminal(
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
) -> Result<()> {
    let backend = ratatui::backend::TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend)?;
    run_watch_loop_with_poll(
        &mut terminal,
        crates,
        workspace,
        mode,
        quit_immediately,
        Some(1),
    )
}

#[cfg(any(test, coverage))]
fn quit_immediately(_timeout: Duration) -> std::io::Result<bool> {
    Ok(true)
}

fn run_watch_loop_with_poll<B: Backend>(
    terminal: &mut Terminal<B>,
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
    poll_quit: fn(Duration) -> std::io::Result<bool>,
    max_iterations: Option<usize>,
) -> Result<()> {
    let mut app = TuiApp::new(crates);
    let mut runtime = WatchRuntime::new(crates, workspace, mode);
    let valid_crates = runtime.valid_crates();
    let (mut debouncer, file_rx) = configure_file_watcher(&valid_crates, &workspace.root_dir)?;
    run_watch_loop_with_runtime(
        terminal,
        &mut app,
        &mut runtime,
        file_rx,
        Some(&mut debouncer),
        poll_quit,
        max_iterations,
    )
}

#[cfg(test)]
fn run_watch_loop_with_file_rx<B: Backend>(
    terminal: &mut Terminal<B>,
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
    file_rx: Receiver<DebounceEventResult>,
    poll_quit: fn(Duration) -> std::io::Result<bool>,
    max_iterations: Option<usize>,
) -> Result<()> {
    let mut app = TuiApp::new(crates);
    let mut runtime = WatchRuntime::new(crates, workspace, mode);
    run_watch_loop_with_runtime(
        terminal,
        &mut app,
        &mut runtime,
        file_rx,
        None,
        poll_quit,
        max_iterations,
    )
}

fn run_watch_loop_with_runtime<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
    runtime: &mut WatchRuntime,
    file_rx: Receiver<DebounceEventResult>,
    mut debouncer: Option<&mut FileDebouncer>,
    poll_quit: fn(Duration) -> std::io::Result<bool>,
    max_iterations: Option<usize>,
) -> Result<()> {
    let mut iterations = 0usize;

    terminal
        .draw(|f| tui::draw(f, app))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if runtime.spawn_initial_generations(app) {
        terminal
            .draw(|f| tui::draw(f, app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    while !app.should_quit {
        if let Some(max) = max_iterations
            && iterations >= max
        {
            break;
        }
        iterations += 1;

        app.update(tui::Message::Tick);

        if poll_quit(Duration::from_millis(16))? {
            app.update(tui::Message::Quit);
            break;
        }

        runtime.handle_generation_results(app);

        match file_rx.recv_timeout(Duration::from_millis(16)) {
            Ok(Ok(events)) => handle_watch_events(app, runtime, &events, debouncer.as_deref_mut())?,
            Ok(Err(errors)) => {
                for error in errors {
                    app.update(tui::Message::WatchError {
                        error: format!("{:?}", error),
                    });
                }
            },
            Err(RecvTimeoutError::Timeout) => {},
            Err(RecvTimeoutError::Disconnected) => break,
        }

        terminal
            .draw(|f| tui::draw(f, app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    runtime.finish_pending_generations(app)?;
    terminal
        .draw(|f| tui::draw(f, app))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    Ok(())
}

fn handle_watch_events(
    app: &mut TuiApp,
    runtime: &mut WatchRuntime,
    events: &[notify_debouncer_full::DebouncedEvent],
    debouncer: Option<&mut FileDebouncer>,
) -> Result<()> {
    let mut affected_crates = runtime.affected_crates_for_events(events);
    match runtime.refresh_build_sources_if_needed(events) {
        Ok(Some(update)) => {
            if let Some(debouncer) = debouncer {
                update_custom_build_watches(debouncer, update)?;
            }
            affected_crates.extend(runtime.affected_crates_for_events(events));
        },
        Ok(None) => {},
        Err(error) => {
            app.update(tui::Message::WatchError {
                error: format!(
                    "failed to rediscover Cargo metadata; retaining previous build-source watches: {error:#}"
                ),
            });
        },
    }
    affected_crates.sort();
    affected_crates.dedup();
    runtime.handle_affected_crates(app, affected_crates);
    Ok(())
}

fn configure_file_watcher(
    valid_crates: &[&CrateInfo],
    workspace_root: &std::path::Path,
) -> Result<(FileDebouncer, Receiver<DebounceEventResult>)> {
    let (file_tx, file_rx) = crossbeam_channel::unbounded();
    let mut debouncer =
        notify_debouncer_full::new_debouncer(Duration::from_millis(300), None, file_tx)
            .context("Failed to create file watcher")?;

    debouncer
        .watch(workspace_root, RecursiveMode::NonRecursive)
        .with_context(|| format!("Failed to watch {}", workspace_root.display()))?;

    let path_to_crate = events::build_path_to_crate(valid_crates, workspace_root);
    let custom_build_dirs = path_to_crate.build_source_watch_dirs();
    for krate in valid_crates {
        debouncer
            .watch(&krate.src_dir, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", krate.src_dir.display()))?;

        debouncer
            .watch(
                &krate.manifest_dir,
                if custom_build_dirs.contains(krate.manifest_dir.as_path()) {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                },
            )
            .with_context(|| format!("Failed to watch {}", krate.manifest_dir.display()))?;
    }
    for directory in custom_build_dirs {
        if valid_crates
            .iter()
            .any(|krate| krate.manifest_dir.as_path() == directory)
        {
            continue;
        }
        debouncer
            .watch(&directory, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", directory.display()))?;
    }

    Ok((debouncer, file_rx))
}

fn update_custom_build_watches(
    debouncer: &mut FileDebouncer,
    update: BuildSourceWatchUpdate,
) -> Result<()> {
    for directory in update.removed {
        if let Err(error) = debouncer.unwatch(&directory) {
            if matches!(
                &error.kind,
                notify::ErrorKind::PathNotFound | notify::ErrorKind::WatchNotFound
            ) {
                continue;
            }
            return Err(error)
                .with_context(|| format!("Failed to stop watching {}", directory.display()));
        }
    }
    let directories_to_watch = update
        .added
        .into_iter()
        .chain(update.rearmed)
        .collect::<BTreeSet<_>>();
    for directory in directories_to_watch {
        debouncer
            .watch(&directory, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", directory.display()))?;
    }
    Ok(())
}
