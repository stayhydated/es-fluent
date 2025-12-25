//! UI module for console output with colored status display.
//! Used by the `generate` command for raw terminal output.

use crate::types::CrateInfo;
use colored::Colorize;

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

/// Prints a message that a crate is missing lib.rs.
pub fn print_missing_lib_rs(crate_name: &str) {
    println!(
        "{} {} {}",
        PREFIX.yellow().bold(),
        "Skipping".dimmed(),
        format!("{} (missing lib.rs)", crate_name).yellow()
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
