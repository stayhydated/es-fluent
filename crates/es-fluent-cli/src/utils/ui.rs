// CLI output formatting with consistent styling using indicatif and colored.
// We stick to standard println!/eprintln! for textual output to ensure ANSI color compatibility.

use crate::core::CrateInfo;
use colored::Colorize as _;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const PD_TICK: Duration = Duration::from_millis(100);
const FRIENDLY_DURATION_PRINTER: jiff::fmt::friendly::SpanPrinter =
    jiff::fmt::friendly::SpanPrinter::new();

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

/// Whether terminal hyperlinks should be emitted.
pub fn terminal_links_enabled() -> bool {
    if let Ok(force) = std::env::var("FORCE_HYPERLINK") {
        return force.trim() != "0";
    }
    if is_e2e() {
        return false;
    }
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        return false;
    }
    std::io::stderr().is_terminal()
}

pub(crate) fn format_duration(duration: Duration) -> String {
    if is_e2e() {
        "[DURATION]".to_string()
    } else {
        FRIENDLY_DURATION_PRINTER.unsigned_duration_to_string(&duration)
    }
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

pub fn print_tree_header() {
    println!("{}", "Fluent FTL Tree".dimmed());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_crate(name: &str) -> CrateInfo {
        CrateInfo {
            name: name.to_string(),
            manifest_dir: std::path::PathBuf::from("/tmp/test"),
            src_dir: std::path::PathBuf::from("/tmp/test/src"),
            i18n_config_path: std::path::PathBuf::from("/tmp/test/i18n.toml"),
            ftl_output_dir: std::path::PathBuf::from("/tmp/test/i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    #[test]
    fn terminal_links_enabled_honors_env_and_modes() {
        let _guard = env_lock().lock().unwrap();

        set_e2e_mode(false);
        // SAFETY: We serialize env var mutations with a global mutex.
        unsafe {
            std::env::remove_var("FORCE_HYPERLINK");
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CI");
            std::env::remove_var("GITHUB_ACTIONS");
            std::env::set_var("FORCE_HYPERLINK", "1");
        }
        assert!(terminal_links_enabled());

        // SAFETY: Protected by the same global mutex.
        unsafe {
            std::env::set_var("FORCE_HYPERLINK", "0");
        }
        assert!(!terminal_links_enabled());

        // SAFETY: Protected by the same global mutex.
        unsafe {
            std::env::remove_var("FORCE_HYPERLINK");
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!terminal_links_enabled());

        // SAFETY: Protected by the same global mutex.
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::set_var("CI", "1");
        }
        assert!(!terminal_links_enabled());

        // SAFETY: Protected by the same global mutex.
        unsafe {
            std::env::remove_var("CI");
        }
        set_e2e_mode(true);
        assert!(!terminal_links_enabled());

        set_e2e_mode(false);
    }

    #[test]
    fn duration_and_progress_helpers_cover_e2e_and_default_modes() {
        set_e2e_mode(true);
        assert_eq!(format_duration(Duration::from_millis(5)), "[DURATION]");
        assert!(create_spinner("spin").is_hidden());
        assert!(create_progress_bar(3, "progress").is_hidden());

        set_e2e_mode(false);
        let formatted = format_duration(Duration::from_millis(5));
        assert!(!formatted.is_empty());

        let spinner = create_spinner("spin");
        spinner.finish_and_clear();

        let pb = create_progress_bar(3, "progress");
        pb.finish_and_clear();
    }

    #[test]
    fn terminal_links_enabled_falls_back_to_terminal_probe_branch() {
        let _guard = env_lock().lock().unwrap();
        set_e2e_mode(false);
        // SAFETY: Protected by env_lock mutex.
        unsafe {
            std::env::remove_var("FORCE_HYPERLINK");
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CI");
            std::env::remove_var("GITHUB_ACTIONS");
        }

        // Environment-dependent; the assertion is that the code path executes without panicking.
        let _ = terminal_links_enabled();
    }

    #[test]
    fn print_helpers_do_not_panic() {
        let crates = vec![test_crate("crate-a"), test_crate("crate-b")];

        print_header();
        print_discovered(&crates);
        print_discovered(&[]);
        print_missing_lib_rs("crate-missing");
        print_generating("crate-a");
        print_generated("crate-a", Duration::from_millis(1), 2);
        print_cleaning("crate-a");
        print_cleaned("crate-a", Duration::from_millis(1), 2);
        print_generation_error("crate-a", "boom");
        print_package_not_found("crate-z");

        print_check_header();
        print_checking("crate-a");
        print_check_error("crate-a", "bad check");
        print_check_success();

        print_format_header();
        print_tree_header();
        print_would_format(Path::new("i18n/en/test.ftl"));
        print_formatted(Path::new("i18n/en/test.ftl"));
        print_format_dry_run_summary(1);
        print_format_summary(2, 3);

        print_sync_header();
        print_syncing("crate-a");
        print_would_add_keys(2, "es", "crate-a");
        print_added_keys(2, "es");
        print_synced_key("hello_world");
        print_all_in_sync();
        print_sync_dry_run_summary(3, 2);
        print_sync_summary(3, 2);
        print_no_locales_specified();
        print_no_crates_found();
        print_locale_not_found("zz", &["en".to_string(), "es".to_string()]);
        print_locale_not_found("zz", &[]);

        print_diff("a = 1\nb = 2\n", "a = 1\nc = 3\n");
        print_diff(
            "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n",
            "l1\nx2\nl3\nl4\nl5\nl6\nl7\nx8\nl9\nl10\n",
        );
    }
}
