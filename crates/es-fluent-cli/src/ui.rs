use crate::app_state::AppState;
use crate::core::{self as fluent_core, BuildOutcome};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

fn build_list_items(app: &'_ AppState) -> Vec<ListItem<'_>> {
    let mut items: Vec<ListItem> = Vec::new();
    let build_statuses_locked = app.build_statuses.lock().unwrap();
    let active_builds_locked = app.active_builds.lock().unwrap();

    for krate in &app.crates {
        let crate_name_str = krate.name();
        let status_span: Span;

        if active_builds_locked.contains(crate_name_str) {
            status_span = Span::styled("Building...", Style::default().fg(Color::Yellow));
        } else {
            match build_statuses_locked.get(crate_name_str) {
                Some(BuildOutcome::Success { duration }) => {
                    status_span = Span::styled(
                        format!("Built in {}", fluent_core::format_duration(*duration)),
                        Style::default().fg(Color::Green),
                    );
                },
                Some(BuildOutcome::Failure {
                    error_message,
                    duration,
                }) => {
                    status_span = Span::styled(
                        format!(
                            "Failed: {} (took {})",
                            error_message,
                            fluent_core::format_duration(*duration)
                        ),
                        Style::default().fg(Color::Red),
                    );
                },
                None => {
                    status_span =
                        Span::styled("Status Unknown", Style::default().fg(Color::DarkGray));
                },
            }
        }

        let line_content = Line::from(vec![
            Span::raw("• "),
            Span::styled(
                crate_name_str.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(": "),
            status_span,
        ]);
        items.push(ListItem::new(line_content));
    }

    items
}

/// Renders the UI for the application.
pub fn ui(f: &mut Frame, app: &AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)].as_ref())
        .split(f.area());

    let list_items = build_list_items(app);

    let crate_list_widget = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title("Crates"))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol("❯ ");
    f.render_widget(crate_list_widget, main_chunks[0]);
}
