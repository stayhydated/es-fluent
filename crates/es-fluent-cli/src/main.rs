mod app_state;
mod consts;
mod core;
mod error;
mod ui;

use crate::{
    app_state::{AppEvent, AppState},
    consts::{DEBOUNCE_DURATION, TICK_RATE},
    core::{self as fluent_core, BuildOutcome, CrateInfo},
    error::CliError,
};
use clap::Parser;
use colored::*;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use es_fluent_generate::FluentParseMode;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, Stdout},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(name = "es-fluent-cli")]
#[command(about = "A CLI tool for watching and building Fluent localization files")]
#[command(version)]
struct Cli {
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::Conservative)]
    mode: FluentParseMode,
    #[arg(short, long)]
    directory: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Run once and print to console instead of entering watch mode"
    )]
    once: bool,
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    let root_dir = cli
        .directory
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let mode = cli.mode.clone();

    let discovered_crates = fluent_core::discover_crates(&root_dir)?;

    if discovered_crates.is_empty() {
        eprintln!(
            "{}",
            "No crates with i18n.toml found. Exiting.".bright_red()
        );
        return Err(CliError::Internal(
            "No crates with i18n.toml found".to_string(),
        ));
    }

    if cli.once {
        let initial_build_results = if discovered_crates.is_empty() {
            std::collections::HashMap::new()
        } else {
            fluent_core::build_all_crates(&discovered_crates, mode.clone()).await?
        };

        let output_lines =
            generate_formatted_crate_and_build_info(&discovered_crates, &initial_build_results);

        for line in output_lines {
            println!("{}", line);
        }

        if !discovered_crates.is_empty() {
            println!();
        }
        return Ok(());
    }

    let initial_build_results =
        fluent_core::build_all_crates(&discovered_crates, mode.clone()).await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = AppState::new(discovered_crates.clone(), initial_build_results.clone());

    let (raw_file_event_tx, raw_file_event_rx) = mpsc::channel::<CrateInfo>();
    let (app_event_tx, app_event_rx) = mpsc::channel::<AppEvent>();

    let shutdown_signal = Arc::new(AtomicBool::new(false));

    let watcher_crates_clone = discovered_crates.clone();
    let watcher_shutdown_signal = Arc::clone(&shutdown_signal);
    let watcher_thread_handle = thread::spawn(move || {
        log::debug!("File watcher thread started.");
        if let Err(e) = fluent_core::watch_crates_sender(
            &watcher_crates_clone,
            raw_file_event_tx,
            watcher_shutdown_signal,
        ) {
            log::error!("File watcher thread error: {}", e);
        }
        log::debug!("File watcher thread finished.");
    });

    let tick_app_event_tx = app_event_tx.clone();
    let event_polling_thread_handle = thread::spawn(move || {
        log::debug!("Event polling thread started.");
        let mut last_tick = Instant::now();
        loop {
            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_millis(1));

            match event::poll(timeout) {
                Ok(true) => match event::read() {
                    Ok(CEvent::Key(key_event)) => {
                        if tick_app_event_tx.send(AppEvent::Input(key_event)).is_err() {
                            log::debug!(
                                "Event polling thread: AppEvent channel for Input closed, exiting."
                            );
                            break;
                        }
                    },
                    Ok(_) => {},
                    Err(e) => {
                        log::error!(
                            "Event polling thread: Failed to read crossterm event: {}. Exiting.",
                            e
                        );
                        break;
                    },
                },
                Ok(false) => {},
                Err(e) => {
                    log::error!(
                        "Event polling thread: crossterm event::poll error: {}. Exiting.",
                        e
                    );
                    break;
                },
            }

            if last_tick.elapsed() >= TICK_RATE {
                if tick_app_event_tx.send(AppEvent::Tick).is_err() {
                    log::debug!("Event polling thread: AppEvent channel for Tick closed, exiting.");
                    break;
                }
                last_tick = Instant::now();
            }
        }
        log::debug!("Event polling thread finished.");
    });

    let fwd_app_event_tx = app_event_tx.clone();
    tokio::spawn(async move {
        log::debug!("Raw file event forwarding task started.");
        for received_crate_info in raw_file_event_rx {
            if fwd_app_event_tx
                .send(AppEvent::FileChange(received_crate_info))
                .is_err()
            {
                log::debug!("Raw file event forwarding task: AppEvent channel closed, exiting.");
                break;
            }
        }
        log::debug!("Raw file event forwarding task finished.");
    });

    let run_result = run_app_loop(&mut terminal, app, mode, app_event_rx).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = run_result {
        eprintln!("Application error: {}", e.to_string().red());
    }

    log::debug!("Signalling threads to shutdown...");
    shutdown_signal.store(true, Ordering::Relaxed);

    log::debug!("Attempting to join event polling thread...");
    if event_polling_thread_handle.join().is_err() {
        log::error!("Failed to join the event polling thread. It might have panicked.");
    } else {
        log::debug!("Event polling thread joined successfully.");
    }

    log::debug!("Attempting to join file watcher thread...");
    if watcher_thread_handle.join().is_err() {
        log::error!("Failed to join the file watcher thread. It might have panicked.");
    } else {
        log::debug!("File watcher thread joined successfully.");
    }
    Ok(())
}

fn generate_formatted_crate_and_build_info(
    discovered_crates: &[CrateInfo],
    initial_build_results: &std::collections::HashMap<String, BuildOutcome>,
) -> Vec<String> {
    let mut lines = Vec::new();

    if discovered_crates.is_empty() {
        let msg = "No crates with i18n.toml found.";
        lines.push(msg.to_string());
        return lines;
    }

    let discovered_msg_str = format!("Discovered {} crates:", discovered_crates.len());
    lines.push(discovered_msg_str);

    for krate in discovered_crates {
        let crate_line = format!("• {} ({})", krate.name(), krate.manifest_dir().display());
        lines.push(crate_line);
    }
    lines.push("".to_string());

    for krate in discovered_crates {
        let build_line = match initial_build_results.get(krate.name()) {
            Some(BuildOutcome::Success { duration }) => {
                format!(
                    "{} built in {}",
                    krate.name(),
                    fluent_core::format_duration(*duration)
                )
            },
            Some(BuildOutcome::Failure {
                error_message,
                duration,
            }) => {
                format!(
                    "{} failed: {} (took {})",
                    krate.name(),
                    error_message,
                    fluent_core::format_duration(*duration)
                )
            },
            None => {
                format!("{} status unknown after initial build", krate.name())
            },
        };
        lines.push(build_line);
    }
    lines
}

async fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut app: AppState,
    mode: FluentParseMode,
    app_event_rx: mpsc::Receiver<AppEvent>,
) -> Result<(), CliError> {
    log::debug!("Starting main application event loop.");
    loop {
        if app.should_quit {
            log::debug!("App should_quit is true, breaking main loop.");
            break;
        }

        terminal.draw(|f| ui::ui(f, &app))?;

        match app_event_rx.recv() {
            Ok(AppEvent::Input(key_event)) => {
                log::trace!("Received input event: {:?}", key_event);
                if key_event.modifiers == KeyModifiers::CONTROL
                    && key_event.code == KeyCode::Char('c')
                {
                    log::info!("Ctrl+C detected, setting should_quit to true.");
                    app.should_quit = true;
                }
            },
            Ok(AppEvent::FileChange(crate_info)) => {
                log::debug!(
                    "Received file change event for crate: {}",
                    crate_info.name()
                );
                app.pending_builds_debouncer
                    .insert(crate_info.name().clone(), (crate_info, Instant::now()));
            },
            Ok(AppEvent::Tick) => {
                log::trace!("Received tick event.");
                let mut crates_to_build_now: Vec<CrateInfo> = Vec::new();

                {
                    let mut active_builds_locked = app.active_builds.lock().unwrap();
                    app.pending_builds_debouncer.retain(
                        |crate_name, (crate_info, last_event_time)| {
                            if last_event_time.elapsed() >= DEBOUNCE_DURATION {
                                if !active_builds_locked.contains(crate_name) {
                                    log::debug!(
                                        "Debounce expired for {}, adding to build queue.",
                                        crate_name
                                    );
                                    crates_to_build_now.push(crate_info.clone());
                                    active_builds_locked.insert(crate_name.clone());
                                } else {
                                    log::trace!(
                                        "Debounce expired for {}, but already in active_builds.",
                                        crate_name
                                    );
                                }
                                false
                            } else {
                                true
                            }
                        },
                    );
                }

                for krate_to_build in crates_to_build_now {
                    log::info!("Spawning build task for crate: {}", krate_to_build.name());
                    let build_statuses_arc = Arc::clone(&app.build_statuses);
                    let active_builds_arc = Arc::clone(&app.active_builds);
                    let mode_clone = mode.clone();
                    let krate_name_captured = krate_to_build.name().clone();

                    tokio::spawn(async move {
                        log::debug!("Build task started for {}", krate_name_captured);
                        let outcome = fluent_core::build_crate(&krate_to_build, mode_clone).await;
                        log::debug!(
                            "Build task finished for {}. Outcome: {:?}",
                            krate_name_captured,
                            outcome
                        );

                        let mut statuses_locked = build_statuses_arc.lock().unwrap();
                        match outcome {
                            Ok(build_outcome) => {
                                statuses_locked.insert(krate_name_captured.clone(), build_outcome);
                            },
                            Err(cli_err) => {
                                log::error!(
                                    "Error from build_crate for {}: {}",
                                    krate_name_captured,
                                    cli_err
                                );
                                let failure_outcome = BuildOutcome::Failure {
                                    error_message: format!("Build fn error: {}", cli_err),
                                    duration: Duration::ZERO,
                                };
                                statuses_locked
                                    .insert(krate_name_captured.clone(), failure_outcome);
                            },
                        };

                        let mut active_builds_locked_after = active_builds_arc.lock().unwrap();
                        active_builds_locked_after.remove(&krate_name_captured);
                        log::debug!("Removed {} from active builds.", krate_name_captured);
                    });
                }
            },
            Err(_) => {
                log::info!(
                    "AppEvent channel closed (all senders dropped), setting should_quit to true."
                );
                app.should_quit = true;
            },
        }
    }

    log::debug!("Exiting main application event loop.");
    Ok(())
}
