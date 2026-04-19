use clap::{Parser, Subcommand};
use es_fluent_cli::{
    CheckArgs, CleanArgs, CliError, FormatArgs, GenerateArgs, SyncArgs, TreeArgs, WatchArgs,
    run_check, run_clean, run_format, run_generate, run_sync, run_tree, run_watch,
};
use miette::Result as MietteResult;

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

    /// Watch for changes and regenerate FTL files (TUI mode)
    Watch(WatchArgs),

    /// Clean orphan keys from FTL files
    Clean(CleanArgs),

    /// Format FTL files (sort keys A-Z)
    #[command(name = "format")]
    Fmt(FormatArgs),

    /// Check FTL files for missing keys and variables
    Check(CheckArgs),

    /// Sync missing translations from fallback to other locales
    Sync(SyncArgs),

    /// Display a tree view of FTL items for each crate
    Tree(TreeArgs),
}

fn main() -> MietteResult<()> {
    // Parse first to check for e2e flag before setting up miette/logging
    let cli = Cli::parse();
    let CargoCommand::EsFluent { command, e2e } = cli.command;

    if e2e {
        es_fluent_cli::set_e2e_mode(true);
    }

    let no_color = std::env::var("NO_COLOR").is_ok();
    let use_color = !es_fluent_cli::is_e2e() && !no_color;
    let use_links = es_fluent_cli::terminal_links_enabled();

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

    let result = dispatch(command);

    result.map_err(miette::Report::new)
}

fn dispatch(command: Commands) -> Result<(), CliError> {
    match command {
        Commands::Generate(args) => run_generate(args),
        Commands::Watch(args) => run_watch(args),
        Commands::Clean(args) => run_clean(args),
        Commands::Fmt(args) => run_format(args),
        Commands::Check(args) => run_check(args),
        Commands::Sync(args) => run_sync(args),
        Commands::Tree(args) => run_tree(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_cli::WorkspaceArgs;
    mod fixtures {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mod.rs"
        ));
    }

    use fixtures::create_workspace;

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
    fn cli_parses_format_command() {
        let cli = Cli::try_parse_from(["cargo", "es-fluent", "format"]).expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);
        assert!(matches!(command, Commands::Fmt(_)));
    }

    #[test]
    fn dispatch_handles_all_commands_without_matching_packages() {
        let temp = create_workspace();
        let workspace = missing_package_workspace_args(temp.path());

        assert!(
            dispatch(Commands::Generate(GenerateArgs {
                workspace: workspace.clone(),
                mode: es_fluent_cli::FluentParseMode::default(),
                dry_run: false,
                force_run: false,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Watch(WatchArgs {
                workspace: workspace.clone(),
                mode: es_fluent_cli::FluentParseMode::default(),
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
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Check(CheckArgs {
                workspace: workspace.clone(),
                all: false,
                ignore: Vec::new(),
                force_run: false,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Sync(SyncArgs {
                workspace: workspace.clone(),
                locale: vec!["en".to_string()],
                all: false,
                dry_run: false,
            }))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Tree(TreeArgs {
                workspace,
                all: false,
                attributes: false,
                variables: false,
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
            mode: es_fluent_cli::FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        }));

        assert!(result.is_err());
    }
}
