use anstream::println;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct StatusReport {
    pub(super) crates_discovered: usize,
    pub(super) crates_checked: usize,
    pub(super) workspace_warnings: Vec<String>,
    pub(super) setup_errors: Vec<String>,
    pub(super) generation_stale_crates: usize,
    pub(super) generation_errors: Vec<String>,
    pub(super) cleanup_stale_crates: usize,
    pub(super) cleanup_errors: Vec<String>,
    pub(super) files_need_formatting: usize,
    pub(super) format_errors: Vec<String>,
    pub(super) missing_synced_keys: usize,
    pub(super) locales_need_sync: usize,
    pub(super) orphaned_files: Vec<String>,
    pub(super) validation_errors: usize,
    pub(super) validation_warnings: usize,
    pub(super) clean: bool,
}

pub(super) fn print_status_report(report: &StatusReport) {
    println!("Crates discovered: {}", report.crates_discovered);
    println!("Crates checked: {}", report.crates_checked);
    for warning in &report.workspace_warnings {
        println!("workspace warning: {warning}");
    }
    println!(
        "Generation-stale crates: {}",
        report.generation_stale_crates
    );
    println!("Cleanup-stale crates: {}", report.cleanup_stale_crates);
    println!("Files needing formatting: {}", report.files_need_formatting);
    println!("Missing synced keys: {}", report.missing_synced_keys);
    println!("Locale targets needing sync: {}", report.locales_need_sync);
    println!("Orphaned files: {}", report.orphaned_files.len());
    println!("Validation errors: {}", report.validation_errors);
    println!("Validation warnings: {}", report.validation_warnings);

    for error in &report.setup_errors {
        println!("setup error: {error}");
    }
    for error in &report.generation_errors {
        println!("generation error: {error}");
    }
    for error in &report.cleanup_errors {
        println!("cleanup error: {error}");
    }
    for error in &report.format_errors {
        println!("format error: {error}");
    }
    for path in &report.orphaned_files {
        println!("orphaned: {path}");
    }

    if report.clean {
        println!("Status: clean");
    } else {
        println!("Status: attention required");
    }
}
