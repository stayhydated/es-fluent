use super::model::DoctorReport;
use crate::{commands::common::OutputFormat, core::CliError};
use anstream::println;
use std::path::Path;

pub(super) fn render_report(report: &DoctorReport, output: OutputFormat) -> Result<(), CliError> {
    if output.is_json() {
        return output.print_json(report);
    }

    println!("Fluent Setup Doctor");
    println!("Discovered {} crate(s)", report.crates_discovered);
    for error in &report.workspace_errors {
        println!("ERROR workspace: {error}");
    }
    let mut current_package = None;
    for check in &report.checks {
        if current_package != Some(check.package.as_str()) {
            println!();
            println!("{}", check.package);
            current_package = Some(check.package.as_str());
        }
        println!(
            "  {} {}: {}",
            check.status.label(),
            check.category,
            check.message
        );
        if let Some(help) = &check.help {
            println!("    help: {help}");
        }
    }
    println!();
    println!(
        "Summary: {} error(s), {} warning(s)",
        report.error_count, report.warning_count
    );
    Ok(())
}

pub(super) fn relative_path(path: &Path, root: &Path) -> String {
    crate::utils::paths::relative_slash_path(path, root)
}

pub(super) fn relative_message(message: &str, root: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, root)
}
