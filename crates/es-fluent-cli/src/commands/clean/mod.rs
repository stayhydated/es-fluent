//! Clean command implementation.

mod orphaned;

use super::common::{GenerationVerb, WorkspaceArgs, WorkspaceCrates, run_generation_command};
use crate::core::{CliError, GenerationAction};
use clap::Parser;

use orphaned::clean_orphaned_files;

/// Arguments for the clean command.
#[derive(Parser)]
pub struct CleanArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Clean all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would be cleaned without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Force rebuild of the runner, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Remove orphaned FTL files that are no longer tied to any types.
    /// This removes files that don't correspond to any registered types
    /// (e.g., when all items are now namespaced or the crate was deleted).
    #[arg(long)]
    pub orphaned: bool,
}

/// Run the clean command.
pub fn run_clean(args: CleanArgs) -> Result<(), CliError> {
    // Handle orphaned file removal first if requested
    if args.orphaned {
        let workspace = WorkspaceCrates::discover(args.workspace)?;
        if !workspace.print_discovery(crate::utils::ui::Ui::print_header) {
            return Ok(());
        }
        return clean_orphaned_files(&workspace, args.all, args.dry_run);
    }

    run_generation_command(
        args.workspace,
        GenerationAction::Clean {
            all_locales: args.all,
            dry_run: args.dry_run,
        },
        args.force_run,
        args.dry_run,
        GenerationVerb::Clean,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{
        FakeRunnerBehavior, create_test_crate_workspace, setup_fake_runner_and_cache,
    };

    #[test]
    fn run_clean_returns_ok_when_package_filter_matches_nothing() {
        let temp = create_test_crate_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            all: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_clean_executes_with_fake_runner() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache(&temp, FakeRunnerBehavior::stdout("cleaned\n"));

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(result.is_ok());
    }
}
