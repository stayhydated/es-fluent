use crate::types::{CrateInfo, CrateState};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::time::Duration;

/// The TUI application state.
pub struct TuiApp<'a> {
    /// The crates being watched.
    pub crates: &'a [CrateInfo],
    /// The current state of each crate.
    pub states: HashMap<String, CrateState>,
    /// Whether the app should quit.
    pub should_quit: bool,
}

impl<'a> TuiApp<'a> {
    /// Creates a new TUI app.
    pub fn new(crates: &'a [CrateInfo]) -> Self {
        let mut states = HashMap::new();
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
        }
    }

    /// Updates the state of a crate.
    pub fn set_state(&mut self, crate_name: &str, state: CrateState) {
        self.states.insert(crate_name.to_string(), state);
    }
}

/// Initializes the terminal for TUI mode.
pub fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restores the terminal to normal mode.
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
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

    let header = Paragraph::new("es-fluent watch (q to quit)")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = app
        .crates
        .iter()
        .map(|krate| {
            let state = app.states.get(&krate.name);
            let (symbol, status_text, status_color) = match state {
                Some(CrateState::MissingLibRs) => ("!", "missing lib.rs", Color::Red),
                Some(CrateState::Generating) => ("*", "generating...", Color::Yellow),
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
