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
use std::ffi::{OsStr, OsString};

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

    /// Clean stale generated keys from locale files
    Clean(CleanArgs),

    /// Format FTL files (sort keys A-Z)
    #[command(name = "fmt", visible_alias = "format")]
    Fmt(FormatArgs),

    /// Validate FTL files, Rust-derived keys, and locale setup
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
    let cli = Cli::parse_from(normalized_cli_args());
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

fn normalized_cli_args() -> Vec<OsString> {
    let mut args = std::env::args_os();
    let binary = args
        .next()
        .unwrap_or_else(|| OsString::from("cargo-es-fluent"));
    let mut rest = args.collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(rest.len() + 2);
    normalized.push(binary);

    let starts_with_wrapper = rest
        .first()
        .is_some_and(|arg| arg == OsStr::new("es-fluent"));
    if !starts_with_wrapper {
        normalized.push(OsString::from("es-fluent"));
    }

    let help_index = usize::from(starts_with_wrapper);
    if rest
        .get(help_index)
        .is_some_and(|arg| arg == OsStr::new("help"))
        && rest
            .get(help_index + 1)
            .is_some_and(|arg| arg == OsStr::new("es-fluent"))
    {
        rest.remove(help_index + 1);
    }

    normalized.extend(rest);
    normalized
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
    use crate::commands::{InitManager, OutputFormat, WorkspaceArgs};
    use crate::core::FluentParseMode;
    use clap::CommandFactory as _;
    mod fixtures {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mod.rs"
        ));
    }

    const EXPECTED_SUBCOMMANDS: &[&str] = &[
        "generate",
        "init",
        "watch",
        "clean",
        "fmt",
        "check",
        "doctor",
        "status",
        "sync",
        "add-locale",
        "tree",
    ];

    fn missing_package_workspace_args(path: &std::path::Path) -> WorkspaceArgs {
        WorkspaceArgs {
            path: Some(path.to_path_buf()),
            package: Some("missing-package".to_string()),
        }
    }

    fn parsed_subcommand_name(args: &[&str]) -> &'static str {
        let cli = Cli::try_parse_from(
            ["cargo", "es-fluent"]
                .into_iter()
                .chain(args.iter().copied()),
        )
        .expect("subcommand should parse");

        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);

        match command {
            Commands::Generate(_) => "generate",
            Commands::Init(_) => "init",
            Commands::Watch(_) => "watch",
            Commands::Clean(_) => "clean",
            Commands::Fmt(_) => "fmt",
            Commands::Check(_) => "check",
            Commands::Doctor(_) => "doctor",
            Commands::Status(_) => "status",
            Commands::Sync(_) => "sync",
            Commands::AddLocale(_) => "add-locale",
            Commands::Tree(_) => "tree",
        }
    }

    #[test]
    fn clap_help_exposes_the_expected_subcommand_surface() {
        let cli = Cli::command();
        let es_fluent = cli
            .get_subcommands()
            .find(|command| command.get_name() == "es-fluent")
            .expect("cargo es-fluent subcommand should exist");
        let actual = es_fluent
            .get_subcommands()
            .map(clap::Command::get_name)
            .collect::<Vec<_>>();

        assert_eq!(actual, EXPECTED_SUBCOMMANDS);
    }

    #[test]
    fn clap_help_describes_workspace_path_subdirectory_support() {
        let mut cli = Cli::command();
        let generate = cli
            .find_subcommand_mut("es-fluent")
            .and_then(|command| command.find_subcommand_mut("generate"))
            .expect("generate subcommand should exist");
        let help = generate.render_long_help().to_string();

        assert!(
            help.contains(
                "Existing path to a crate/workspace root, its Cargo.toml, or a path inside a crate"
            ),
            "unexpected generate help:\n{help}"
        );
    }

    #[test]
    fn clap_help_describes_all_locale_discovery() {
        let mut cli = Cli::command();
        let es_fluent = cli
            .find_subcommand_mut("es-fluent")
            .expect("es-fluent subcommand should exist");

        let cases = [
            ("clean", "Clean all discovered locale directories"),
            ("fmt", "Format all discovered locale directories"),
            (
                "check",
                "Include non-fallback validation, fallback-copy warnings, and orphan-file checks",
            ),
            (
                "status",
                "Include non-fallback formatting, sync, orphan-file, and validation checks",
            ),
            (
                "sync",
                "Sync to all discovered locale directories, excluding the fallback language",
            ),
            ("tree", "Show all discovered locale directories"),
        ];

        for (subcommand, expected) in cases {
            let help = es_fluent
                .find_subcommand_mut(subcommand)
                .expect("subcommand should exist")
                .render_long_help()
                .to_string();
            assert!(
                help.contains(expected),
                "unexpected {subcommand} help:\n{help}"
            );
        }
    }

    #[test]
    fn clap_help_describes_clean_without_implying_orphan_cleanup_by_default() {
        let mut cli = Cli::command();
        let es_fluent = cli
            .find_subcommand_mut("es-fluent")
            .expect("es-fluent subcommand should exist");
        let help = es_fluent.render_long_help().to_string();

        assert!(
            help.contains("clean       Clean stale generated keys from locale files"),
            "unexpected top-level help:\n{help}"
        );
        assert!(
            !help.contains("clean       Clean stale generated keys and orphaned FTL files"),
            "clean summary should not imply orphan cleanup runs by default:\n{help}"
        );
    }

    #[test]
    fn cli_parses_every_public_subcommand() {
        let cases: &[(&[&str], &str)] = &[
            (&["generate"], "generate"),
            (&["init"], "init"),
            (&["watch"], "watch"),
            (&["clean"], "clean"),
            (&["fmt"], "fmt"),
            (&["check"], "check"),
            (&["doctor"], "doctor"),
            (&["status"], "status"),
            (&["sync", "--all"], "sync"),
            (&["add-locale", "fr-FR"], "add-locale"),
            (&["tree"], "tree"),
        ];

        let parsed = cases
            .iter()
            .map(|(args, _)| parsed_subcommand_name(args))
            .collect::<Vec<_>>();

        assert_eq!(parsed, EXPECTED_SUBCOMMANDS);
        for (args, expected) in cases {
            assert_eq!(parsed_subcommand_name(args), *expected);
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
    fn cli_parses_format_alias() {
        let cli = Cli::try_parse_from(["cargo", "es-fluent", "format"]).expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);
        assert!(matches!(command, Commands::Fmt(_)));
    }

    #[test]
    fn cli_parses_sync_comma_separated_locales() {
        let cli = Cli::try_parse_from(["cargo", "es-fluent", "sync", "--locale", "es, fr-FR"])
            .expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);

        let Commands::Sync(args) = command else {
            panic!("expected sync command");
        };
        assert_eq!(args.locale, ["es", " fr-FR"]);
    }

    #[test]
    fn cli_parses_add_locale_comma_separated_locales() {
        let cli =
            Cli::try_parse_from(["cargo", "es-fluent", "add-locale", "es, fr-FR"]).expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);

        let Commands::AddLocale(args) = command else {
            panic!("expected add-locale command");
        };
        assert_eq!(args.locale, ["es", " fr-FR"]);
    }

    #[test]
    fn cli_parses_dioxus_runtime_comma_separated_values_with_spaces() {
        let cli = Cli::try_parse_from([
            "cargo",
            "es-fluent",
            "init",
            "--manager",
            "dioxus",
            "--update-cargo-toml",
            "--dioxus-runtime",
            "client, ssr",
        ])
        .expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);

        let Commands::Init(args) = command else {
            panic!("expected init command");
        };
        assert_eq!(format!("{:?}", args.dioxus_runtime), "[Client, Ssr]");
    }

    #[test]
    fn cli_parses_status_force_run_flag() {
        let cli =
            Cli::try_parse_from(["cargo", "es-fluent", "status", "--force-run"]).expect("parse");
        let CargoCommand::EsFluent { command, e2e } = cli.command;
        assert!(!e2e);

        let Commands::Status(args) = command else {
            panic!("expected status command");
        };
        assert!(args.force_run);
    }

    #[test]
    fn cli_rejects_generate_only_flags_for_watch() {
        for flag in ["--dry-run", "--force-run"] {
            let error = match Cli::try_parse_from(["cargo", "es-fluent", "watch", flag]) {
                Ok(_) => panic!("{flag} should not parse for watch"),
                Err(error) => error,
            };
            assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
        }
    }

    #[test]
    fn dispatch_handles_all_commands_on_noninteractive_paths() {
        let temp = fixtures::create_workspace();
        let missing_workspace = missing_package_workspace_args(temp.path());
        let selected_workspace = WorkspaceArgs {
            path: Some(temp.path().to_path_buf()),
            package: None,
        };

        assert!(
            dispatch(Commands::Generate(GenerateArgs {
                workspace: selected_workspace.clone(),
                mode: FluentParseMode::default(),
                dry_run: true,
                force_run: false,
            }))
            .is_ok()
        );

        let init_root = tempfile::tempdir().expect("init tempdir");
        fs_err::write(init_root.path().join("Cargo.toml"), fixtures::CARGO_TOML)
            .expect("write init Cargo.toml");
        assert!(
            dispatch(Commands::Init(
                InitArgs::builder()
                    .path(init_root.path())
                    .fallback_language("en")
                    .locales(vec!["fr-FR".to_string()])
                    .assets_dir("assets/locales")
                    .namespaces(vec!["ui".to_string()])
                    .manager(InitManager::Embedded)
                    .dioxus_runtime(Vec::new())
                    .build_rs(true)
                    .update_cargo_toml(false)
                    .dry_run(true)
                    .force(false)
                    .build(),
            ))
            .is_ok()
        );

        let watch_result = dispatch(Commands::Watch(WatchArgs {
            workspace: missing_workspace.clone(),
            mode: FluentParseMode::default(),
        }));
        assert!(watch_result.is_err());

        assert!(
            dispatch(Commands::Clean(CleanArgs {
                workspace: selected_workspace.clone(),
                all: false,
                dry_run: true,
                force_run: false,
                orphaned: false,
            }))
            .is_ok()
        );

        let clean_result = dispatch(Commands::Clean(CleanArgs {
            workspace: missing_workspace.clone(),
            all: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        }));
        assert!(clean_result.is_err());

        assert!(
            dispatch(Commands::Fmt(FormatArgs {
                workspace: selected_workspace.clone(),
                all: false,
                dry_run: false,
                output: OutputFormat::Text,
            }))
            .is_ok()
        );

        let fmt_result = dispatch(Commands::Fmt(FormatArgs {
            workspace: missing_workspace.clone(),
            all: false,
            dry_run: false,
            output: OutputFormat::Text,
        }));
        assert!(fmt_result.is_err());

        let generate_result = dispatch(Commands::Generate(GenerateArgs {
            workspace: missing_workspace.clone(),
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        }));
        assert!(generate_result.is_err());

        assert!(
            dispatch(Commands::Check(
                CheckArgs::builder()
                    .workspace(missing_workspace.clone())
                    .all(false)
                    .ignore(Vec::new())
                    .force_run(false)
                    .output(OutputFormat::Text)
                    .build(),
            ))
            .is_ok()
        );

        assert!(
            dispatch(Commands::Doctor(DoctorArgs {
                workspace: missing_workspace.clone(),
                output: OutputFormat::Text,
            }))
            .is_ok()
        );

        let status_result = dispatch(Commands::Status(StatusArgs {
            workspace: missing_workspace.clone(),
            all: false,
            force_run: false,
            output: OutputFormat::Text,
        }));
        assert!(matches!(status_result, Err(CliError::Exit(1))));

        let sync_result = dispatch(Commands::Sync(SyncArgs {
            workspace: missing_workspace.clone(),
            locale: vec!["en".to_string()],
            all: false,
            create: false,
            dry_run: false,
            output: OutputFormat::Text,
        }));
        assert!(sync_result.is_err());

        assert!(
            dispatch(Commands::AddLocale(AddLocaleArgs {
                workspace: selected_workspace,
                locale: vec!["fr-FR".to_string()],
                dry_run: true,
            }))
            .is_ok()
        );

        let tree_result = dispatch(Commands::Tree(TreeArgs {
            workspace: missing_workspace,
            all: false,
            attributes: true,
            variables: true,
            link_mode: "rust".to_string(),
            output: OutputFormat::Text,
        }));
        assert!(matches!(tree_result, Err(CliError::Exit(1))));
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
