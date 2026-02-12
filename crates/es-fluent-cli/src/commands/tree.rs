//! Tree command for displaying FTL structure.
//!
//! This module provides functionality to display a tree view of FTL items
//! for each FTL file associated with a crate.

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::CliError;
use crate::ftl::{LocaleContext, parse_ftl_file};
use crate::utils::{discover_ftl_files, ui};
use anyhow::Result;
use clap::Parser;
use colored::Colorize as _;
use fluent_syntax::ast;
use std::path::Path;

/// Arguments for the tree command.
#[derive(Debug, Parser)]
pub struct TreeArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Show all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Show attributes under each message.
    #[arg(long)]
    pub attributes: bool,

    /// Show variables used in each message.
    #[arg(long)]
    pub variables: bool,
}

/// Run the tree command.
pub fn run_tree(args: TreeArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    ui::print_tree_header();

    if workspace.crates.is_empty() {
        ui::print_no_crates_found();
        return Ok(());
    }

    for krate in &workspace.crates {
        print_crate_tree(krate, args.all, args.attributes, args.variables)?;
    }

    Ok(())
}

/// Print the tree for a single crate.
fn print_crate_tree(
    krate: &crate::core::CrateInfo,
    all_locales: bool,
    show_attributes: bool,
    show_variables: bool,
) -> Result<()> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;

    println!("{}", krate.name.bold().cyan());

    let mut first_locale = true;
    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        if !locale_dir.exists() {
            continue;
        }

        let ftl_files = discover_ftl_files(&ctx.assets_dir, locale, &ctx.crate_name)?;

        if ftl_files.is_empty() {
            continue;
        }

        let prefix = if first_locale { "" } else { "\n" };
        println!("{}  {}", prefix, locale.green());
        first_locale = false;

        for (idx, file_info) in ftl_files.iter().enumerate() {
            let is_last_file = idx == ftl_files.len() - 1;
            print_file_tree(
                &file_info.relative_path.display().to_string(),
                &file_info.abs_path,
                is_last_file,
                show_attributes,
                show_variables,
            );
        }
    }

    Ok(())
}

/// Print the tree for a single FTL file.
fn print_file_tree(
    relative_path: &str,
    abs_path: &Path,
    is_last_file: bool,
    show_attributes: bool,
    show_variables: bool,
) {
    let file_prefix = if is_last_file {
        "  └── "
    } else {
        "  ├── "
    };
    println!("{}{}", file_prefix, relative_path.yellow());

    let resource = match parse_ftl_file(abs_path) {
        Ok(res) => res,
        Err(_) => {
            println!("      {}", "<parse error>".red());
            return;
        },
    };

    let entries: Vec<_> = resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(msg) => Some(FtlEntry::Message {
                id: msg.id.name.clone(),
                attributes: msg.attributes.iter().map(|a| a.id.name.clone()).collect(),
                variables: extract_variables_from_message(msg),
            }),
            ast::Entry::Term(term) => Some(FtlEntry::Term {
                id: term.id.name.clone(),
                attributes: term.attributes.iter().map(|a| a.id.name.clone()).collect(),
                variables: extract_variables_from_pattern(&term.value),
            }),
            ast::Entry::Comment(_) => None,
            ast::Entry::GroupComment(_) => None,
            ast::Entry::ResourceComment(_) => None,
            ast::Entry::Junk { .. } => None,
        })
        .collect();

    let entry_prefix = if is_last_file { "      " } else { "  │   " };

    for (idx, entry) in entries.iter().enumerate() {
        let is_last_entry = idx == entries.len() - 1;
        let entry_connector = if is_last_entry {
            "└── "
        } else {
            "├── "
        };

        match entry {
            FtlEntry::Message {
                id,
                attributes,
                variables,
            } => {
                println!("{}{}{}", entry_prefix, entry_connector, id);
                print_details(
                    entry_prefix,
                    is_last_entry,
                    show_attributes,
                    show_variables,
                    attributes,
                    variables,
                );
            },
            FtlEntry::Term {
                id,
                attributes,
                variables,
            } => {
                println!("{}{}{} {}", entry_prefix, entry_connector, "-".dimmed(), id);
                print_details(
                    entry_prefix,
                    is_last_entry,
                    show_attributes,
                    show_variables,
                    attributes,
                    variables,
                );
            },
        }
    }
}

/// Print attributes and variables for an entry.
fn print_details(
    entry_prefix: &str,
    is_last_entry: bool,
    show_attributes: bool,
    show_variables: bool,
    attributes: &[String],
    variables: &[String],
) {
    if !show_attributes && !show_variables {
        return;
    }

    let detail_prefix = if is_last_entry {
        format!("{}    ", entry_prefix)
    } else {
        format!("{}│   ", entry_prefix)
    };

    let mut has_details = false;

    if show_attributes && !attributes.is_empty() {
        for (idx, attr) in attributes.iter().enumerate() {
            let is_last_attr =
                !show_variables || (idx == attributes.len() - 1 && variables.is_empty());
            let connector = if is_last_attr {
                "└── "
            } else {
                "├── "
            };
            println!("{}{}@{}", detail_prefix, connector, attr.dimmed());
        }
        has_details = true;
    }

    if show_variables && !variables.is_empty() {
        let vars_str = variables.join(", ");
        let prefix = if has_details {
            format!("{}    ", detail_prefix)
        } else {
            detail_prefix.clone()
        };
        println!("{}${}", prefix, vars_str.magenta());
    }
}

/// An entry in an FTL file.
enum FtlEntry {
    Message {
        id: String,
        attributes: Vec<String>,
        variables: Vec<String>,
    },
    Term {
        id: String,
        attributes: Vec<String>,
        variables: Vec<String>,
    },
}

/// Extract variable names from a message.
fn extract_variables_from_message(msg: &ast::Message<String>) -> Vec<String> {
    let mut variables = Vec::new();
    if let Some(ref value) = msg.value {
        let mut vars = Vec::new();
        extract_variables_from_pattern_into(value, &mut vars);
        variables.extend(vars);
    }
    for attr in &msg.attributes {
        let mut vars = Vec::new();
        extract_variables_from_pattern_into(&attr.value, &mut vars);
        variables.extend(vars);
    }
    variables.sort();
    variables.dedup();
    variables
}

/// Extract variable names from a pattern.
fn extract_variables_from_pattern(pattern: &ast::Pattern<String>) -> Vec<String> {
    let mut variables = Vec::new();
    extract_variables_from_pattern_into(pattern, &mut variables);
    variables.sort();
    variables.dedup();
    variables
}

/// Extract variable names from a pattern into a vector.
fn extract_variables_from_pattern_into(
    pattern: &ast::Pattern<String>,
    variables: &mut Vec<String>,
) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            extract_variables_from_expression(expression, variables);
        }
    }
}

/// Extract variable names from an expression.
fn extract_variables_from_expression(
    expression: &ast::Expression<String>,
    variables: &mut Vec<String>,
) {
    match expression {
        ast::Expression::Inline(inline) => {
            extract_variables_from_inline(inline, variables);
        },
        ast::Expression::Select { selector, variants } => {
            extract_variables_from_inline(selector, variables);
            for variant in variants {
                extract_variables_from_pattern_into(&variant.value, variables);
            }
        },
    }
}

/// Extract variable names from an inline expression.
fn extract_variables_from_inline(
    inline: &ast::InlineExpression<String>,
    variables: &mut Vec<String>,
) {
    match inline {
        ast::InlineExpression::VariableReference { id } => {
            variables.push(id.name.clone());
        },
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for arg in &arguments.positional {
                extract_variables_from_inline(arg, variables);
            }
            for arg in &arguments.named {
                extract_variables_from_inline(&arg.value, variables);
            }
        },
        ast::InlineExpression::Placeable { expression } => {
            extract_variables_from_expression(expression, variables);
        },
        _ => {},
    }
}
