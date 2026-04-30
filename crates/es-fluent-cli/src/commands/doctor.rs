//! Doctor command implementation.

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo};
use crate::ftl::LocaleContext;
use clap::Parser;
use fs_err as fs;
use serde::Serialize;
use std::path::Path;

/// Arguments for the doctor command.
#[derive(Debug, Parser)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

#[derive(Serialize)]
struct DoctorReport {
    crates_discovered: usize,
    error_count: usize,
    warning_count: usize,
    issues: Vec<DoctorIssue>,
}

#[derive(Serialize)]
struct DoctorIssue {
    severity: &'static str,
    crate_name: Option<String>,
    message: String,
    help: String,
}

/// Run the doctor command.
pub fn run_doctor(args: DoctorArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;
    let mut issues = Vec::new();

    if workspace.crates.is_empty() {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: None,
            message: "no crates with i18n.toml were found".to_string(),
            help: "Run `cargo es-fluent init` in a crate that should use es-fluent.".to_string(),
        });
    }

    for krate in &workspace.skipped {
        issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.clone()),
            message: "crate has i18n.toml but no library target".to_string(),
            help: "Add src/lib.rs or move localizable types into a library crate.".to_string(),
        });
    }

    for krate in &workspace.crates {
        inspect_crate(krate, &mut issues);
    }

    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warning_count = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();
    let report = DoctorReport {
        crates_discovered: workspace.crates.len(),
        error_count,
        warning_count,
        issues,
    };

    if args.output.is_json() {
        args.output.print_json(&report)?;
    } else {
        print_doctor_report(&report);
    }

    if report.error_count > 0 {
        Err(CliError::Exit(1))
    } else {
        Ok(())
    }
}

fn inspect_crate(krate: &CrateInfo, issues: &mut Vec<DoctorIssue>) {
    match LocaleContext::from_crate(krate, true) {
        Ok(ctx) => {
            let fallback_dir = ctx.locale_dir(&ctx.fallback);
            if !fallback_dir.exists() {
                issues.push(DoctorIssue {
                    severity: "error",
                    crate_name: Some(krate.name.clone()),
                    message: format!("fallback locale directory '{}' is missing", ctx.fallback),
                    help: format!("Create {}", fallback_dir.display()),
                });
            }
        },
        Err(error) => issues.push(DoctorIssue {
            severity: "error",
            crate_name: Some(krate.name.clone()),
            message: "i18n.toml or locale assets could not be read".to_string(),
            help: error.to_string(),
        }),
    }

    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap_or_default();
    let i18n_module = fs::read_to_string(krate.src_dir.join("i18n.rs")).unwrap_or_default();

    if let Some(manager_dependency) = manager_dependency_from_module(&i18n_module)
        && !manifest.contains(manager_dependency)
    {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.clone()),
            message: format!(
                "src/i18n.rs references {manager_dependency}, but Cargo.toml does not"
            ),
            help: format!("Add `{manager_dependency}` under [dependencies]."),
        });
    }

    inspect_build_script(krate, &i18n_module, issues);
}

fn manager_dependency_from_module(module: &str) -> Option<&'static str> {
    if module.contains("es_fluent_manager_embedded") {
        Some("es-fluent-manager-embedded")
    } else if module.contains("es_fluent_manager_dioxus") {
        Some("es-fluent-manager-dioxus")
    } else if module.contains("es_fluent_manager_bevy") {
        Some("es-fluent-manager-bevy")
    } else {
        None
    }
}

fn inspect_build_script(krate: &CrateInfo, i18n_module: &str, issues: &mut Vec<DoctorIssue>) {
    if !i18n_module.contains("define_i18n_module!") {
        return;
    }

    let build_rs = krate.manifest_dir.join("build.rs");
    if !build_rs.exists() {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.clone()),
            message: "build.rs does not track locale asset changes".to_string(),
            help: "Run `cargo es-fluent init --build-rs --force` or add `es_fluent::build::track_i18n_assets();` manually.".to_string(),
        });
        return;
    }

    if !file_contains(&build_rs, "track_i18n_assets") {
        issues.push(DoctorIssue {
            severity: "warning",
            crate_name: Some(krate.name.clone()),
            message: "build.rs exists but does not call track_i18n_assets".to_string(),
            help: "Call `es_fluent::build::track_i18n_assets();` from build.rs.".to_string(),
        });
    }
}

fn file_contains(path: &Path, needle: &str) -> bool {
    fs::read_to_string(path).is_ok_and(|contents| contents.contains(needle))
}

fn print_doctor_report(report: &DoctorReport) {
    println!("Fluent FTL Doctor");
    if report.issues.is_empty() {
        println!("No setup issues found.");
        return;
    }

    for issue in &report.issues {
        let crate_label = issue
            .crate_name
            .as_deref()
            .map(|name| format!("{name}: "))
            .unwrap_or_default();
        println!(
            "{}: {}{}",
            issue.severity.to_uppercase(),
            crate_label,
            issue.message
        );
        println!("  help: {}", issue.help);
    }

    println!(
        "{} error(s), {} warning(s)",
        report.error_count, report.warning_count
    );
}

#[cfg(test)]
mod tests;
