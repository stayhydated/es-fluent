#![doc = include_str!("../README.md")]

use clap::{Parser, Subcommand};
use commands::{
    AddLocaleArgs, CheckArgs, CleanArgs, DoctorArgs, FormatArgs, GenerateArgs, InitArgs,
    StatusArgs, SyncArgs, TreeArgs, WatchArgs,
};
use miette::Result as MietteResult;

mod commands;
mod core;
mod ftl;
mod generation;
mod tui;
mod utils;

use crate::core::CliError;
use crate::utils::ui::Ui;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: CargoCommand,
}

#[derive(Subcommand)]
enum CargoCommand {
    /// CLI for generating FTL files from es-fluent derive macros
    #[command(name = "es-fluent", version)]
    EsFluent {
        #[command(subcommand)]
        command: Commands,

        /// Enable E2E testing mode
        #[arg(long, global = true, hide = true)]
        e2e: bool,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Generate FTL files once for all crates with i18n.toml
    Generate(GenerateArgs),

    /// Scaffold i18n.toml, locale folders, and a crate-local i18n module
    Init(InitArgs),

    /// Watch for changes and regenerate FTL files (TUI mode)
    Watch(WatchArgs),

    /// Clean orphan keys from FTL files
    Clean(CleanArgs),

    /// Format FTL files (sort keys A-Z)
    #[command(name = "fmt")]
    Fmt(FormatArgs),

    /// Check FTL files for missing keys and variables
    Check(CheckArgs),

    /// Diagnose es-fluent setup issues
    Doctor(DoctorArgs),

    /// Report whether generated, formatted, synced, cleaned, and checked surfaces are current
    Status(StatusArgs),

    /// Sync missing translations from fallback to other locales
    Sync(SyncArgs),

    /// Create locale directories and seed them from the fallback language
    AddLocale(AddLocaleArgs),

    /// Display a tree view of FTL items for each crate
    Tree(TreeArgs),
}

#[doc(hidden)]
pub fn run_cli() -> MietteResult<()> {
    // Parse first to check for e2e flag before setting up miette/logging.
    let cli = Cli::parse();
    let CargoCommand::EsFluent { command, e2e } = cli.command;

    if e2e {
        Ui::set_e2e_mode(true);
    }

    let no_color = std::env::var("NO_COLOR").is_ok();
    let use_color = !Ui::is_e2e() && !no_color;
    let use_links = Ui::terminal_links_enabled();

    miette::set_hook(Box::new(move |_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(use_links)
                .unicode(true)
                .context_lines(2)
                .tab_width(4)
                .color(use_color)
                .build(),
        )
    }))
    .ok();

    match dispatch(command) {
        Ok(()) => Ok(()),
        Err(CliError::Exit(code)) => std::process::exit(code),
        Err(error) => Err(miette::Report::new(error)),
    }
}

fn dispatch(command: Commands) -> Result<(), CliError> {
    match command {
        Commands::Generate(args) => commands::run_generate(args),
        Commands::Init(args) => commands::run_init(args),
        Commands::Watch(args) => commands::run_watch(args),
        Commands::Clean(args) => commands::run_clean(args),
        Commands::Fmt(args) => commands::run_format(args),
        Commands::Check(args) => commands::run_check(args),
        Commands::Doctor(args) => commands::run_doctor(args),
        Commands::Status(args) => commands::run_status(args),
        Commands::Sync(args) => commands::run_sync(args),
        Commands::AddLocale(args) => commands::run_add_locale(args),
        Commands::Tree(args) => commands::run_tree(args),
    }
}

#[cfg(test)]
pub(crate) mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{OutputFormat, WorkspaceArgs};
    use crate::core::FluentParseMode;
    mod fixtures {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mod.rs"
        ));
    }

    fn missing_package_workspace_args(path: &std::path::Path) -> WorkspaceArgs {
        WorkspaceArgs {
            path: Some(path.to_path_buf()),
            package: Some("missing-package".to_string()),
        }
    }

    #[test]
    fn cli_parses_e2e_flag_and_generate_subcommand() {
        let cli = Cli::try_parse_from(["cargo", "es-fluent", "generate", "--e2e"]).expect("parse");

        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(e2e);
        assert!(matches!(command, Commands::Generate(_)));
    }

    #[test]
    fn cli_parses_fmt_command() {
        let cli = Cli::try_parse_from(["cargo", "es-fluent", "fmt"]).expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);
        assert!(matches!(command, Commands::Fmt(_)));
    }

    #[test]
    fn cli_rejects_old_format_command_name() {
        let error = match Cli::try_parse_from(["cargo", "es-fluent", "format"]) {
            Ok(_) => panic!("old format command name should be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn dispatch_handles_all_commands_without_matching_packages() {
        let temp = fixtures::create_workspace();
        let workspace = missing_package_workspace_args(temp.path());

        assert!(
            dispatch(Commands::Generate(GenerateArgs {
                workspace: workspace.clone(),
                mode: FluentParseMode::default(),
                dry_run: false,
                force_run: false,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Watch(WatchArgs {
                workspace: workspace.clone(),
                mode: FluentParseMode::default(),
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Clean(CleanArgs {
                workspace: workspace.clone(),
                all: false,
                dry_run: false,
                force_run: false,
                orphaned: false,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Fmt(FormatArgs {
                workspace: workspace.clone(),
                all: false,
                dry_run: false,
                output: OutputFormat::Text,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Check(CheckArgs {
                workspace: workspace.clone(),
                all: false,
                ignore: Vec::new(),
                force_run: false,
                output: OutputFormat::Text,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Doctor(DoctorArgs {
                workspace: workspace.clone(),
                output: OutputFormat::Text,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Sync(SyncArgs {
                workspace: workspace.clone(),
                locale: vec!["en".to_string()],
                all: false,
                create: false,
                dry_run: false,
                output: OutputFormat::Text,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::AddLocale(AddLocaleArgs {
                workspace: workspace.clone(),
                locale: vec!["fr-FR".to_string()],
                dry_run: true,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Tree(TreeArgs {
                workspace,
                all: false,
                attributes: false,
                variables: false,
                output: OutputFormat::Text,
            }))
            .is_ok()
        );
    }

    #[test]
    fn dispatch_propagates_errors_for_invalid_workspace_paths() {
        let invalid_workspace = WorkspaceArgs {
            path: Some(std::path::PathBuf::from("/definitely/missing/path")),
            package: None,
        };

        let result = dispatch(Commands::Generate(GenerateArgs {
            workspace: invalid_workspace,
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        }));

        assert!(result.is_err());
    }
}
