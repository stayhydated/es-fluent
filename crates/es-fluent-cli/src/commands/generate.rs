//! Generate command implementation.

use super::common::{GenerationVerb, WorkspaceArgs, run_generation_command};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use clap::Parser;

/// Arguments for the generate command.
#[derive(Parser)]
pub struct GenerateArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,

    /// Dry run - show what would be generated without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,
}

/// Run the generate command.
pub fn run_generate(args: GenerateArgs) -> Result<(), CliError> {
    run_generation_command(
        args.workspace,
        GenerationAction::Generate {
            mode: args.mode,
            dry_run: args.dry_run,
        },
        args.force_run,
        args.dry_run,
        GenerationVerb::Generate,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{
        FakeRunnerBehavior, create_test_crate_workspace, setup_fake_runner_and_cache,
    };

    #[test]
    fn run_generate_returns_ok_when_package_filter_matches_nothing() {
        let temp = create_test_crate_workspace();
        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_generate_executes_with_fake_runner() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::stdout("generated\n"));

        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(result.is_ok());
    }
}
