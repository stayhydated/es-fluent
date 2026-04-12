//! TUI application state and rendering.

use crate::core::{CrateInfo, CrateState};
use crate::tui::Message;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use indexmap::IndexMap;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;
use std::time::{Duration, Instant};
use throbber_widgets_tui::{BRAILLE_SIX, ThrobberState};

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(100);

/// The TUI application state.
pub struct TuiApp<'a> {
    /// The crates being watched.
    pub crates: &'a [CrateInfo],
    /// The current state of each crate.
    pub states: IndexMap<String, CrateState>,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Throbber state for the "generating" animation.
    pub throbber_state: ThrobberState,
    /// How often to advance the animation.
    pub tick_interval: Duration,
    /// Last time the animation was advanced.
    last_tick: Instant,
}

impl<'a> TuiApp<'a> {
    /// Creates a new TUI app.
    pub fn new(crates: &'a [CrateInfo]) -> Self {
        let mut states = IndexMap::new();
        for krate in crates {
            if krate.has_lib_rs {
                states.insert(krate.name.clone(), CrateState::Generating);
            } else {
                states.insert(krate.name.clone(), CrateState::MissingLibRs);
            }
        }

        Self {
            crates,
            states,
            should_quit: false,
            throbber_state: ThrobberState::default(),
            tick_interval: DEFAULT_TICK_INTERVAL,
            last_tick: Instant::now(),
        }
    }

    /// Updates the state of a crate.
    pub fn set_state(&mut self, crate_name: &str, state: CrateState) {
        self.states.insert(crate_name.to_string(), state);
    }

    /// Advance the throbber animation if enough time has passed.
    pub fn tick(&mut self) {
        if self.last_tick.elapsed() >= self.tick_interval {
            self.throbber_state.calc_next();
            self.last_tick = Instant::now();
        }
    }

    /// Process a message and update application state (Elm-style update).
    ///
    /// Returns `true` if the message was handled and requires a redraw.
    pub fn update(&mut self, msg: Message) -> bool {
        match msg {
            Message::Tick => {
                self.tick();
                true
            },
            Message::Quit => {
                self.should_quit = true;
                false
            },
            Message::FileChanged { crate_name } => {
                // Only matters if we're not already generating
                if !matches!(self.states.get(&crate_name), Some(CrateState::Generating)) {
                    self.set_state(&crate_name, CrateState::Generating);
                    true
                } else {
                    false
                }
            },
            Message::GenerationStarted { crate_name } => {
                self.set_state(&crate_name, CrateState::Generating);
                true
            },
            Message::GenerationComplete { result } => {
                if let Some(ref error) = result.error {
                    self.set_state(
                        &result.name,
                        CrateState::Error {
                            message: error.clone(),
                        },
                    );
                } else {
                    self.set_state(
                        &result.name,
                        CrateState::Watching {
                            resource_count: result.resource_count,
                        },
                    );
                }
                true
            },
            Message::WatchError { error } => {
                // Errors are already visible in the crate state
                drop(error);
                false
            },
        }
    }
}

/// Get the current throbber symbol based on state.
fn get_throbber_symbol(state: &ThrobberState) -> &'static str {
    let symbols = BRAILLE_SIX.symbols;
    let idx = state.index().rem_euclid(symbols.len() as i8) as usize;
    symbols[idx]
}

/// Draws the TUI.
pub fn draw(frame: &mut Frame, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Crate list
        ])
        .split(frame.area());

    // Header
    let header = Paragraph::new("es-fluent watch (q to quit)")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    // Crate list
    let throbber_symbol = get_throbber_symbol(&app.throbber_state);

    let items: Vec<ListItem> = app
        .crates
        .iter()
        .map(|krate| {
            let state = app.states.get(&krate.name);
            let (symbol, status_text, status_color) = match state {
                Some(CrateState::MissingLibRs) => ("!", "missing lib.rs", Color::Red),
                Some(CrateState::Generating) => (throbber_symbol, "generating", Color::Yellow),
                Some(CrateState::Watching { resource_count }) => {
                    let text = format!("watching ({} resources)", resource_count);
                    return ListItem::new(Line::from(vec![
                        Span::styled(
                            "✓ ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            krate.name.clone(),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(text, Style::default().fg(Color::Green)),
                    ]));
                },
                Some(CrateState::Error { message }) => {
                    return ListItem::new(Line::from(vec![
                        Span::styled(
                            "✗ ",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            krate.name.clone(),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("error: {}", message),
                            Style::default().fg(Color::Red),
                        ),
                    ]));
                },
                None => ("-", "pending", Color::DarkGray),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", symbol),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    krate.name.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]))
        })
        .collect();

    let crate_list = List::new(items).block(Block::default().borders(Borders::ALL).title("Crates"));
    frame.render_widget(crate_list, chunks[1]);
}

/// Polls for keyboard events with a timeout.
/// Returns true if the user wants to quit.
pub fn poll_quit_event(timeout: Duration) -> io::Result<bool> {
    if event::poll(timeout)?
        && let Event::Key(key) = event::read()?
        && (key.code == KeyCode::Char('q')
            || (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c')))
    {
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GenerateResult;
    use ratatui::{Terminal, backend::TestBackend};
    use std::path::PathBuf;

    fn test_crate(name: &str, has_lib_rs: bool) -> CrateInfo {
        CrateInfo {
            name: name.to_string(),
            manifest_dir: PathBuf::from("/tmp/test"),
            src_dir: PathBuf::from("/tmp/test/src"),
            i18n_config_path: PathBuf::from("/tmp/test/i18n.toml"),
            ftl_output_dir: PathBuf::from("/tmp/test/i18n/en"),
            has_lib_rs,
            fluent_features: Vec::new(),
        }
    }

    #[test]
    fn app_new_initializes_states_from_crates() {
        let crates = vec![test_crate("a", true), test_crate("b", false)];
        let app = TuiApp::new(&crates);

        assert!(matches!(app.states.get("a"), Some(CrateState::Generating)));
        assert!(matches!(
            app.states.get("b"),
            Some(CrateState::MissingLibRs)
        ));
        assert!(!app.should_quit);
    }

    #[test]
    fn app_update_covers_message_transitions() {
        let crates = vec![test_crate("a", true)];
        let mut app = TuiApp::new(&crates);

        assert!(app.update(Message::Tick));

        // Already generating => no redraw request.
        assert!(!app.update(Message::FileChanged {
            crate_name: "a".to_string(),
        }));

        app.set_state("a", CrateState::Watching { resource_count: 1 });
        assert!(app.update(Message::FileChanged {
            crate_name: "a".to_string(),
        }));
        assert!(matches!(app.states.get("a"), Some(CrateState::Generating)));

        assert!(app.update(Message::GenerationStarted {
            crate_name: "a".to_string(),
        }));

        assert!(app.update(Message::GenerationComplete {
            result: GenerateResult::success(
                "a".to_string(),
                Duration::from_millis(1),
                3,
                None,
                true,
            ),
        }));
        assert!(matches!(
            app.states.get("a"),
            Some(CrateState::Watching { resource_count: 3 })
        ));

        assert!(app.update(Message::GenerationComplete {
            result: GenerateResult::failure(
                "a".to_string(),
                Duration::from_millis(1),
                "boom".to_string(),
            ),
        }));
        assert!(matches!(
            app.states.get("a"),
            Some(CrateState::Error { message }) if message == "boom"
        ));

        assert!(!app.update(Message::WatchError {
            error: "watch failed".to_string(),
        }));
        assert!(!app.update(Message::Quit));
        assert!(app.should_quit);
    }

    #[test]
    fn draw_renders_without_panicking() {
        let crates = vec![test_crate("a", true)];
        let app = TuiApp::new(&crates);
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("create terminal");

        terminal.draw(|f| draw(f, &app)).expect("draw");
    }

    #[test]
    fn poll_quit_event_times_out_to_false() {
        match poll_quit_event(Duration::from_millis(0)) {
            Ok(quit) => assert!(!quit),
            Err(err) => assert!(
                err.to_string()
                    .contains("Failed to initialize input reader"),
                "unexpected poll error: {err}"
            ),
        }
    }

    #[test]
    fn tick_advances_throbber_when_interval_elapsed() {
        let crates = vec![test_crate("a", true)];
        let mut app = TuiApp::new(&crates);
        app.tick_interval = Duration::ZERO;

        let before = app.throbber_state.index();
        app.tick();
        let after = app.throbber_state.index();

        assert_ne!(before, after, "tick should advance throbber frame");
    }

    #[test]
    fn draw_covers_watching_error_and_pending_states() {
        let crates = vec![
            test_crate("watching", true),
            test_crate("error", true),
            test_crate("pending", true),
        ];
        let mut app = TuiApp::new(&crates);
        app.set_state("watching", CrateState::Watching { resource_count: 2 });
        app.set_state(
            "error",
            CrateState::Error {
                message: "boom".to_string(),
            },
        );
        app.states.shift_remove("pending");

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("create terminal");
        terminal.draw(|f| draw(f, &app)).expect("draw");
    }
}
