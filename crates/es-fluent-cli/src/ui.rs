//! UI module for console output with colored status display.

use crate::types::{CrateInfo, CrateState};
use colored::Colorize;
use std::collections::HashMap;

const PREFIX: &str = "[es-fluent]";

/// Prints the CLI header.
pub fn print_header() {
    println!(
        "{} {}",
        PREFIX.cyan().bold(),
        "Fluent FTL Generator".dimmed()
    );
}

/// Prints a message about discovered crates.
pub fn print_discovered(crates: &[CrateInfo]) {
    if crates.is_empty() {
        println!(
            "{} {}",
            PREFIX.red().bold(),
            "No crates with i18n.toml found.".red()
        );
    } else {
        println!(
            "{} {} {}",
            PREFIX.cyan().bold(),
            "Discovered".dimmed(),
            format!("{} crate(s)", crates.len()).green()
        );
    }
}

/// Prints a file change notification.
pub fn print_file_changed(crate_name: &str, file_name: &str) {
    println!(
        "{} {} {} in {}",
        PREFIX.cyan().bold(),
        "File changed:".dimmed(),
        file_name.yellow(),
        crate_name.white().bold()
    );
}

/// Prints a generation started message.
pub fn print_generating(crate_name: &str) {
    println!(
        "{} {} {}",
        PREFIX.cyan().bold(),
        "Generating FTL for".dimmed(),
        crate_name.green()
    );
}

/// Prints a generation completed message with duration.
pub fn print_generated(crate_name: &str, duration: std::time::Duration, resource_count: usize) {
    println!(
        "{} {} {} ({} resources)",
        PREFIX.cyan().bold(),
        format!("{} generated in", crate_name).dimmed(),
        humantime::format_duration(duration).to_string().green(),
        resource_count.to_string().cyan()
    );
}

/// Prints a generation error message.
pub fn print_generation_error(crate_name: &str, error: &str) {
    eprintln!(
        "{} {} {}: {}",
        PREFIX.red().bold(),
        "Generation failed for".red(),
        crate_name.white().bold(),
        error
    );
}

/// Prints the watching status message.
pub fn print_watching() {
    println!(
        "{} {}",
        PREFIX.cyan().bold(),
        "Watching for changes... (Ctrl+C to stop)".dimmed()
    );
}

/// Prints a shutdown message.
pub fn print_shutdown() {
    println!("\n{} {}", PREFIX.cyan().bold(), "Shutting down...".dimmed());
}

/// Prints a summary of all crate states.
pub fn print_summary(crates: &[CrateInfo], states: &HashMap<String, CrateState>) {
    println!();
    for krate in crates {
        let state = states.get(&krate.name);
        let (symbol, status) = match state {
            Some(CrateState::MissingLibRs) => ("!".red().bold(), "missing lib.rs".red()),
            Some(CrateState::Generating) => ("*".yellow().bold(), "generating...".yellow()),
            Some(CrateState::Watching { resource_count }) => (
                "✓".green().bold(),
                format!("watching ({} resources)", resource_count).green(),
            ),
            Some(CrateState::Error { message }) => {
                ("✗".red().bold(), format!("error: {}", message).red())
            },
            None => ("-".dimmed(), "pending".dimmed()),
        };

        println!("  {} {} {}", symbol, krate.name.white().bold(), status);
    }
    println!();
}
