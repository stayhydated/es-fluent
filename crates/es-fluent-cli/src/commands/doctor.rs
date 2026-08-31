mod catalog;
mod diagnosis;
mod manifest;
mod model;
mod render;

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::CliError;
use clap::Args;

use diagnosis::diagnose_crate;
use model::DoctorReport;
use render::render_report;

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

pub fn run_doctor(args: DoctorArgs) -> Result<(), CliError> {
    let output = args.output;
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) => {
            let report = DoctorReport::new(0, vec![error.to_string()], Vec::new());
            render_report(&report, output)?;
            return Err(CliError::Exit(1));
        },
    };

    let mut workspace_errors = workspace
        .empty_selection_message()
        .into_iter()
        .collect::<Vec<_>>();
    let checks = workspace
        .crates
        .iter()
        .flat_map(|krate| diagnose_crate(krate, &workspace.workspace_info.root_dir))
        .collect::<Vec<_>>();
    if workspace.crates.is_empty() && workspace_errors.is_empty() {
        workspace_errors.push("no configured crates were selected".to_string());
    }
    let report = DoctorReport::new(workspace.crates.len(), workspace_errors, checks);
    render_report(&report, output)?;

    if report.healthy {
        Ok(())
    } else {
        Err(CliError::Exit(1))
    }
}

#[cfg(test)]
use catalog::fallback_catalog_inputs;
#[cfg(test)]
use es_fluent_toml::ResolvedI18nLayout;
#[cfg(test)]
use manifest::dependency_specs;
#[cfg(test)]
use model::{DoctorCheck, DoctorStatus};

#[cfg(test)]
mod tests;
