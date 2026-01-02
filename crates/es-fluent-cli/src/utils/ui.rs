//! CLI output formatting with consistent styling.
//!
//! This module provides a `Status` enum for consistent, styled CLI output.

use crate::core::CrateInfo;
use colored::{ColoredString, Colorize as _};
use std::fmt;
use std::path::Path;
use std::time::Duration;

const PREFIX: &str = "[es-fluent]";

/// Status level for CLI output messages.
#[derive(Clone, Copy)]
pub enum Status {
    Info,
    Success,
    Warning,
    Error,
}

impl Status {
    /// Returns the colored prefix for this status level.
    fn prefix(self) -> ColoredString {
        match self {
            Status::Info => PREFIX.cyan().bold(),
            Status::Success => PREFIX.green().bold(),
            Status::Warning => PREFIX.yellow().bold(),
            Status::Error => PREFIX.red().bold(),
        }
    }

    /// Print a message with this status level.
    pub fn print(self, message: impl fmt::Display) {
        println!("{} {}", self.prefix(), message);
    }

    /// Print a message to stderr with this status level.
    pub fn eprint(self, message: impl fmt::Display) {
        eprintln!("{} {}", self.prefix(), message);
    }
}

pub fn print_header() {
    Status::Info.print("Fluent FTL Generator".dimmed());
}

pub fn print_discovered(crates: &[CrateInfo]) {
    if crates.is_empty() {
        Status::Error.print("No crates with i18n.toml found.".red());
    } else {
        Status::Info.print(format!(
            "{} {}",
            "Discovered".dimmed(),
            format!("{} crate(s)", crates.len()).green()
        ));
    }
}

pub fn print_missing_lib_rs(crate_name: &str) {
    Status::Warning.print(format!(
        "{} {}",
        "Skipping".dimmed(),
        format!("{} (missing lib.rs)", crate_name).yellow()
    ));
}

pub fn print_generating(crate_name: &str) {
    Status::Info.print(format!(
        "{} {}",
        "Generating FTL for".dimmed(),
        crate_name.green()
    ));
}

pub fn print_generated(crate_name: &str, duration: Duration, resource_count: usize) {
    Status::Info.print(format!(
        "{} {} ({} resources)",
        format!("{} generated in", crate_name).dimmed(),
        humantime::format_duration(duration).to_string().green(),
        resource_count.to_string().cyan()
    ));
}

pub fn print_cleaning(crate_name: &str) {
    Status::Info.print(format!(
        "{} {}",
        "Cleaning FTL for".dimmed(),
        crate_name.green()
    ));
}

pub fn print_cleaned(crate_name: &str, duration: Duration, resource_count: usize) {
    Status::Info.print(format!(
        "{} {} ({} resources)",
        format!("{} cleaned in", crate_name).dimmed(),
        humantime::format_duration(duration).to_string().green(),
        resource_count.to_string().cyan()
    ));
}

pub fn print_generation_error(crate_name: &str, error: &str) {
    Status::Error.eprint(format!(
        "{} {}: {}",
        "Generation failed for".red(),
        crate_name.white().bold(),
        error
    ));
}

pub fn print_package_not_found(package: &str) {
    Status::Warning.print(format!(
        "{} '{}'",
        "No crate found matching package filter:".yellow(),
        package.white().bold()
    ));
}

pub fn print_check_header() {
    Status::Info.print("Fluent FTL Checker".dimmed());
}

pub fn print_checking(crate_name: &str) {
    Status::Info.print(format!("{} {}", "Checking".dimmed(), crate_name.green()));
}

pub fn print_check_error(crate_name: &str, error: &str) {
    Status::Error.eprint(format!(
        "{} {}: {}",
        "Check failed for".red(),
        crate_name.white().bold(),
        error
    ));
}

pub fn print_check_success() {
    Status::Success.print("No issues found!".green());
}

pub fn print_format_header() {
    Status::Info.print("Fluent FTL Formatter".dimmed());
}

pub fn print_would_format(path: &Path) {
    Status::Warning.print(format!("{} {}", "Would format:".yellow(), path.display()));
}

pub fn print_formatted(path: &Path) {
    Status::Success.print(format!("{} {}", "Formatted:".green(), path.display()));
}

pub fn print_format_dry_run_summary(count: usize) {
    Status::Warning.print(format!(
        "{} {} file(s) would be formatted",
        "Dry run:".yellow(),
        count
    ));
}

pub fn print_format_summary(formatted: usize, unchanged: usize) {
    Status::Success.print(format!(
        "{} {} formatted, {} unchanged",
        "Done:".green(),
        formatted,
        unchanged
    ));
}

pub fn print_sync_header() {
    Status::Info.print("Fluent FTL Sync".dimmed());
}

pub fn print_syncing(crate_name: &str) {
    Status::Info.print(format!("{} {}", "Syncing".dimmed(), crate_name.green()));
}

pub fn print_would_add_keys(count: usize, locale: &str) {
    Status::Warning.print(format!(
        "{} {} key(s) to {}",
        "Would add".yellow(),
        count,
        locale.cyan()
    ));
}

pub fn print_added_keys(count: usize, locale: &str) {
    Status::Success.print(format!(
        "{} {} key(s) to {}",
        "Added".green(),
        count,
        locale.cyan()
    ));
}

pub fn print_synced_key(key: &str) {
    println!("  {} {}", "->".dimmed(), key);
}

pub fn print_all_in_sync() {
    Status::Success.print("All locales are in sync!".green());
}

pub fn print_sync_dry_run_summary(keys: usize, locales: usize) {
    Status::Warning.print(format!(
        "{} {} key(s) across {} locale(s)",
        "Would sync".yellow(),
        keys,
        locales
    ));
}

pub fn print_sync_summary(keys: usize, locales: usize) {
    Status::Success.print(format!(
        "{} {} key(s) synced to {} locale(s)",
        "Done:".green(),
        keys,
        locales
    ));
}

pub fn print_no_locales_specified() {
    Status::Warning.print("No locales specified. Use --locale <LOCALE> or --all".yellow());
}

pub fn print_no_crates_found() {
    Status::Error.print("No crates with i18n.toml found.".red());
}
