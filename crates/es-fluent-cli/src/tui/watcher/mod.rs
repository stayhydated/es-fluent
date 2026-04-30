//! File watcher and main TUI event loop.

mod events;
mod generation;
mod runtime;

#[cfg(test)]
mod tests;

use self::runtime::WatchRuntime;
use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::tui::{self, TuiApp};
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, RecvTimeoutError};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, RecommendedCache};
use ratatui::{Terminal, backend::Backend};
use std::time::Duration;

/// Watch for changes and regenerate FTL files for all discovered crates.
pub fn watch_all(
    crates: &[CrateInfo],
    workspace: &WorkspaceInfo,
    mode: &FluentParseMode,
) -> Result<()> {
    if crates.is_empty() {
        anyhow::bail!("No crates to watch");
    }

    crate::generation::prepare_monolithic_runner_crate(workspace)?;

    let mut terminal = ratatui::init();
    let poll = tui::poll_quit_event;
    let result = run_watch_loop_with_poll(&mut terminal, crates, workspace, mode, poll, None);
    ratatui::restore();

    result
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
    let (_debouncer, file_rx) = configure_file_watcher(runtime.valid_crates())?;
    run_watch_loop_with_runtime(
        terminal,
        &mut app,
        &mut runtime,
        file_rx,
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
        poll_quit,
        max_iterations,
    )
}

fn run_watch_loop_with_runtime<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
    runtime: &mut WatchRuntime<'_>,
    file_rx: Receiver<DebounceEventResult>,
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
            Ok(Ok(events)) => runtime.handle_file_events(app, &events),
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

    Ok(())
}

fn configure_file_watcher(
    valid_crates: &[&CrateInfo],
) -> Result<(
    notify_debouncer_full::Debouncer<RecommendedWatcher, RecommendedCache>,
    Receiver<DebounceEventResult>,
)> {
    let (file_tx, file_rx) = crossbeam_channel::unbounded();
    let mut debouncer =
        notify_debouncer_full::new_debouncer(Duration::from_millis(300), None, file_tx)
            .context("Failed to create file watcher")?;

    for krate in valid_crates {
        debouncer
            .watch(&krate.src_dir, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", krate.src_dir.display()))?;

        debouncer
            .watch(&krate.manifest_dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("Failed to watch {}", krate.manifest_dir.display()))?;
    }

    Ok((debouncer, file_rx))
}
