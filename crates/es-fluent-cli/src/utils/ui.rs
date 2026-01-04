// CLI output formatting with consistent styling using indicatif and colored.
// We stick to standard println!/eprintln! for textual output to ensure ANSI color compatibility.

use crate::core::CrateInfo;
use colored::Colorize as _;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const PD_TICK: Duration = Duration::from_millis(100);

static E2E_MODE: AtomicBool = AtomicBool::new(false);

/// Enable E2E mode for deterministic output (no colors, fixed durations, hidden progress bars).
pub fn set_e2e_mode(enabled: bool) {
    E2E_MODE.store(enabled, Ordering::SeqCst);
    if enabled {
        colored::control::set_override(false);
    }
}

pub fn is_e2e() -> bool {
    E2E_MODE.load(Ordering::SeqCst)
}

fn format_duration(duration: Duration) -> String {
    if is_e2e() {
        "[DURATION]".to_string()
    } else {
        humantime::format_duration(duration).to_string()
    }
}

pub fn init_logging() {
    // No-op: we rely on standard output for CLI presentation.
    // Kept to avoid breaking main.rs calls.
}

pub fn create_spinner(msg: &str) -> ProgressBar {
    if is_e2e() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(PD_TICK);
    pb
}

pub fn create_progress_bar(len: u64, msg: &str) -> ProgressBar {
    if is_e2e() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} {msg} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(PD_TICK);
    pb
}

// Deprecated/Legacy output helpers - redirected to println/eprintln to preserve formatting
// Tracing proved problematic for raw ANSI passthrough in some environments or configs.

pub fn print_header() {
    println!("{}", "Fluent FTL Generator".dimmed());
}

pub fn print_discovered(crates: &[CrateInfo]) {
    if crates.is_empty() {
        eprintln!("{}", "No crates with i18n.toml found.".red());
    } else {
        println!(
            "{} {}",
            "Discovered".dimmed(),
            format!("{} crate(s)", crates.len()).green()
        );
    }
}

pub fn print_missing_lib_rs(crate_name: &str) {
    println!(
        "{} {}",
        "Skipping".dimmed(),
        format!("{} (missing lib.rs)", crate_name).yellow()
    );
}

// Action-specific printers

pub fn print_generating(crate_name: &str) {
    println!("{} {}", "Generating FTL for".dimmed(), crate_name.green());
}

pub fn print_generated(crate_name: &str, duration: Duration, resource_count: usize) {
    println!(
        "{} {} ({} resources)",
        format!("{} generated in", crate_name).dimmed(),
        format_duration(duration).green(),
        resource_count.to_string().cyan()
    );
}

pub fn print_cleaning(crate_name: &str) {
    println!("{} {}", "Cleaning FTL for".dimmed(), crate_name.green());
}

pub fn print_cleaned(crate_name: &str, duration: Duration, resource_count: usize) {
    println!(
        "{} {} ({} resources)",
        format!("{} cleaned in", crate_name).dimmed(),
        format_duration(duration).green(),
        resource_count.to_string().cyan()
    );
}

pub fn print_generation_error(crate_name: &str, error: &str) {
    eprintln!(
        "{} {}: {}",
        "Generation failed for".red(),
        crate_name.white().bold(),
        error
    );
}

pub fn print_package_not_found(package: &str) {
    println!(
        "{} '{}'",
        "No crate found matching package filter:".yellow(),
        package.white().bold()
    );
}

pub fn print_check_header() {
    println!("{}", "Fluent FTL Checker".dimmed());
}

pub fn print_checking(crate_name: &str) {
    println!("{} {}", "Checking".dimmed(), crate_name.green());
}

pub fn print_check_error(crate_name: &str, error: &str) {
    eprintln!(
        "{} {}: {}",
        "Check failed for".red(),
        crate_name.white().bold(),
        error
    );
}

pub fn print_check_success() {
    println!("{}", "No issues found!".green());
}

pub fn print_format_header() {
    println!("{}", "Fluent FTL Formatter".dimmed());
}

pub fn print_would_format(path: &Path) {
    println!("{} {}", "Would format:".yellow(), path.display());
}

pub fn print_formatted(path: &Path) {
    println!("{} {}", "Formatted:".green(), path.display());
}

pub fn print_format_dry_run_summary(count: usize) {
    println!(
        "{} {} file(s) would be formatted",
        "Dry run:".yellow(),
        count
    );
}

pub fn print_format_summary(formatted: usize, unchanged: usize) {
    println!(
        "{} {} formatted, {} unchanged",
        "Done:".green(),
        formatted,
        unchanged
    );
}

pub fn print_sync_header() {
    println!("{}", "Fluent FTL Sync".dimmed());
}

pub fn print_syncing(crate_name: &str) {
    println!("{} {}", "Syncing".dimmed(), crate_name.green());
}

pub fn print_would_add_keys(count: usize, locale: &str, crate_name: &str) {
    println!(
        "{} {} key(s) to {} ({})",
        "Would add".yellow(),
        count,
        locale.cyan(),
        crate_name.bold()
    );
}

pub fn print_added_keys(count: usize, locale: &str) {
    println!("{} {} key(s) to {}", "Added".green(), count, locale.cyan());
}

pub fn print_synced_key(key: &str) {
    println!("  {} {}", "->".dimmed(), key);
}

pub fn print_all_in_sync() {
    println!("{}", "All locales are in sync!".green());
}

pub fn print_sync_dry_run_summary(keys: usize, locales: usize) {
    println!(
        "{} {} key(s) across {} locale(s)",
        "Would sync".yellow(),
        keys,
        locales
    );
}

pub fn print_sync_summary(keys: usize, locales: usize) {
    println!(
        "{} {} key(s) synced to {} locale(s)",
        "Done:".green(),
        keys,
        locales
    );
}

pub fn print_no_locales_specified() {
    println!(
        "{}",
        "No locales specified. Use --locale <LOCALE> or --all".yellow()
    );
}

pub fn print_no_crates_found() {
    eprintln!("{}", "No crates with i18n.toml found.".red());
}

pub fn print_locale_not_found(locale: &str, available: &[String]) {
    let available_str = if available.is_empty() {
        "none".to_string()
    } else {
        available.join(", ")
    };
    eprintln!(
        "{} '{}'. Available locales: {}",
        "Locale not found:".red(),
        locale.white().bold(),
        available_str.cyan()
    );
}

pub fn print_diff(old: &str, new: &str) {
    // If e2e mode, just print a marker or simplified diff to avoid colored crate dependency affecting things
    // But we still want to see the diff content.
    // Use the existing logic but colors will be suppressed by `colored::control::set_override(false)`.

    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{}", "  ...".dimmed());
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                let line = format!("{} {}", sign, change);
                match change.tag() {
                    ChangeTag::Delete => print!("{}", line.red()),
                    ChangeTag::Insert => print!("{}", line.green()),
                    ChangeTag::Equal => print!("{}", line.dimmed()),
                }
            }
        }
    }
}
