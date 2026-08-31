use super::{
    json::{TreeErrorJson, TreeJsonReport, build_crate_tree_json},
    links::{TreeLinkMode, collect_rust_link_indexes},
    text::print_crate_tree,
    validation::relative_tree_message,
};

use super::super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};

use crate::{core::CliError, utils::ui};

use clap::{ArgAction, Parser};

/// Arguments for the tree command.
#[derive(Debug, Parser)]
pub struct TreeArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Show all discovered locale directories, not just the fallback language.
    #[arg(long)]
    pub all_locales: bool,

    /// Hide attributes under message and term entries.
    #[arg(long = "no-attributes", action = ArgAction::SetFalse, default_value_t = true)]
    pub attributes: bool,

    /// Hide variables used by each message or term entry.
    #[arg(long = "no-variables", action = ArgAction::SetFalse, default_value_t = true)]
    pub variables: bool,

    /// Text-output hyperlink target mode for message, attribute, and variable rows: rust
    /// or ftl. Defaults to rust; cannot be used with --output json.
    #[arg(long = "link-mode", value_name = "MODE")]
    pub link_mode: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}
/// Run the tree command.
pub fn run_tree(args: TreeArgs) -> Result<(), CliError> {
    let output = args.output;
    let link_mode = if output.is_json() {
        if args.link_mode.is_some() {
            let error = CliError::Other(
                "--link-mode cannot be used with --output json because JSON tree output has no hyperlinks"
                    .to_string(),
            );
            output.print_json(&TreeJsonReport {
                crates: Vec::new(),
                error_count: 1,
                errors: vec![TreeErrorJson {
                    crate_name: "workspace".to_string(),
                    message: error.to_string(),
                }],
            })?;
            return Err(CliError::Exit(1));
        }
        TreeLinkMode::default()
    } else {
        TreeLinkMode::parse_arg(args.link_mode.as_deref().unwrap_or("rust"))?
    };

    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) if output.is_json() => {
            output.print_json(&TreeJsonReport {
                crates: Vec::new(),
                error_count: 1,
                errors: vec![TreeErrorJson {
                    crate_name: "workspace".to_string(),
                    message: error.to_string(),
                }],
            })?;
            return Err(CliError::Exit(1));
        },
        Err(error) => return Err(error),
    };
    let show_text = !output.is_json();
    let terminal_links = show_text && ui::Ui::terminal_links_enabled();

    if show_text {
        ui::Ui::print_tree_header();
    }

    if workspace.crates.is_empty() {
        let message = workspace
            .empty_selection_message()
            .unwrap_or_else(|| "no crates with i18n.toml were found".to_string());
        if output.is_json() {
            output.print_json(&TreeJsonReport {
                crates: Vec::new(),
                error_count: 1,
                errors: vec![TreeErrorJson {
                    crate_name: "workspace".to_string(),
                    message,
                }],
            })?;
            return Err(CliError::Exit(1));
        }
        if show_text {
            workspace.print_no_crates_found();
        }
        return Err(CliError::Exit(1));
    }

    if output.is_json() {
        let mut crates = Vec::new();
        let mut errors = Vec::new();

        for krate in &workspace.crates {
            match build_crate_tree_json(krate, args.all_locales, args.attributes, args.variables) {
                Ok((tree, parse_errors)) => {
                    crates.push(tree);
                    errors.extend(parse_errors.into_iter().map(|message| TreeErrorJson {
                        crate_name: krate.name.to_string(),
                        message: relative_tree_message(
                            &message,
                            &workspace.workspace_info.root_dir,
                        ),
                    }));
                },
                Err(error) => errors.push(TreeErrorJson {
                    crate_name: krate.name.to_string(),
                    message: relative_tree_message(
                        &error.to_string(),
                        &workspace.workspace_info.root_dir,
                    ),
                }),
            }
        }

        let report = TreeJsonReport {
            crates,
            error_count: errors.len(),
            errors,
        };
        output.print_json(&report)?;
        return if report.error_count > 0 {
            Err(CliError::Exit(1))
        } else {
            Ok(())
        };
    }

    let rust_link_indexes =
        collect_rust_link_indexes(&workspace, link_mode, terminal_links, args.all_locales)?;

    for krate in &workspace.crates {
        print_crate_tree(
            krate,
            args.all_locales,
            args.attributes,
            args.variables,
            terminal_links,
            link_mode,
            rust_link_indexes.get(krate.name.as_str()),
        )?;
    }

    Ok(())
}
