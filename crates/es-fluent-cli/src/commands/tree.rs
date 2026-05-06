//! Tree command for displaying FTL structure.
//!
//! This module provides functionality to display a tree view of FTL items
//! for each FTL file associated with a crate.

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::CliError;
use crate::ftl::{CrateFtlLayout, LocaleContext};
use crate::utils::ui;
use anyhow::Result;
use clap::Parser;
use colored::Colorize as _;
use fluent_syntax::ast;
use serde::Serialize;
use std::path::Path;
use treelog::Tree;

#[derive(Clone, Copy)]
struct TreeRenderer {
    show_attributes: bool,
    show_variables: bool,
}

impl TreeRenderer {
    fn new(show_attributes: bool, show_variables: bool) -> Self {
        Self {
            show_attributes,
            show_variables,
        }
    }

    /// Build a tree for a single FTL file.
    fn build_file_tree(&self, relative_path: &str, abs_path: &Path) -> Tree {
        let resource = match crate::ftl::parse_ftl_file(abs_path) {
            Ok(res) => res,
            Err(_) => {
                return Tree::Node(
                    relative_path.yellow().to_string(),
                    vec![Tree::Leaf(vec!["<parse error>".red().to_string()])],
                );
            },
        };

        let entries: Vec<Tree> = resource
            .body
            .iter()
            .filter_map(|entry| match entry {
                ast::Entry::Message(msg) => Some(self.build_message_tree(&msg.id.name, msg)),
                ast::Entry::Term(term) => Some(self.build_term_tree(&term.id.name, term)),
                ast::Entry::Comment(_) => None,
                ast::Entry::GroupComment(_) => None,
                ast::Entry::ResourceComment(_) => None,
                ast::Entry::Junk { .. } => None,
            })
            .collect();

        Tree::Node(relative_path.yellow().to_string(), entries)
    }

    /// Build a tree for a message entry.
    fn build_message_tree(&self, id: &str, msg: &ast::Message<String>) -> Tree {
        let children = self.build_entry_children(&msg.attributes, msg.value.as_ref());

        if children.is_empty() {
            Tree::Leaf(vec![id.to_string()])
        } else {
            Tree::Node(id.to_string(), children)
        }
    }

    /// Build a tree for a term entry.
    fn build_term_tree(&self, id: &str, term: &ast::Term<String>) -> Tree {
        let children = self.build_entry_children(&term.attributes, Some(&term.value));
        let label = format!("-{}", id);

        if children.is_empty() {
            Tree::Leaf(vec![label.dimmed().to_string()])
        } else {
            Tree::Node(label.dimmed().to_string(), children)
        }
    }

    /// Build child nodes for an entry (attributes and variables).
    fn build_entry_children(
        &self,
        attributes: &[ast::Attribute<String>],
        value: Option<&ast::Pattern<String>>,
    ) -> Vec<Tree> {
        let mut children: Vec<Tree> = Vec::new();

        if self.show_attributes {
            for attr in attributes {
                let attr_label = format!("@{}", attr.id.name);
                children.push(Tree::Leaf(vec![attr_label.dimmed().to_string()]));
            }
        }

        if self.show_variables {
            let mut variables: Vec<_> =
                crate::ftl::extract_variables_from_value_and_attributes(value, attributes)
                    .into_iter()
                    .collect();

            if !variables.is_empty() {
                variables.sort();
                let vars_str = variables
                    .iter()
                    .map(|v| format!("${}", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                children.push(Tree::Leaf(vec![vars_str.magenta().to_string()]));
            }
        }

        children
    }
}

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

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

#[derive(Serialize)]
struct TreeJsonReport {
    crates: Vec<TreeCrateJson>,
}

#[derive(Serialize)]
struct TreeCrateJson {
    name: String,
    locales: Vec<TreeLocaleJson>,
}

#[derive(Serialize)]
struct TreeLocaleJson {
    locale: String,
    files: Vec<TreeFileJson>,
}

#[derive(Serialize)]
struct TreeFileJson {
    path: String,
    parse_error: bool,
    entries: Vec<TreeEntryJson>,
}

#[derive(Serialize)]
struct TreeEntryJson {
    id: String,
    kind: &'static str,
    attributes: Vec<String>,
    variables: Vec<String>,
}

/// Run the tree command.
pub fn run_tree(args: TreeArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;
    let show_text = !args.output.is_json();

    if show_text {
        ui::Ui::print_tree_header();
    }

    if workspace.crates.is_empty() {
        if show_text {
            ui::Ui::print_no_crates_found();
        }
        return Ok(());
    }

    if args.output.is_json() {
        let crates = workspace
            .crates
            .iter()
            .map(|krate| build_crate_tree_json(krate, args.all))
            .collect::<Result<Vec<_>>>()?;
        args.output.print_json(&TreeJsonReport { crates })?;
        return Ok(());
    }

    for krate in &workspace.crates {
        print_crate_tree(krate, args.all, args.attributes, args.variables)?;
    }

    Ok(())
}

fn build_crate_tree_json(
    krate: &crate::core::CrateInfo,
    all_locales: bool,
) -> Result<TreeCrateJson> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;
    let mut locales = Vec::new();

    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        if !locale_dir.exists() {
            continue;
        }

        let ftl_files = CrateFtlLayout::from_assets_dir(&ctx.assets_dir, locale, &ctx.crate_name)
            .discover_files()?;
        let files = ftl_files
            .iter()
            .map(|file_info| {
                build_file_tree_json(
                    &file_info.relative_path.display().to_string(),
                    &file_info.abs_path,
                )
            })
            .collect::<Vec<_>>();

        locales.push(TreeLocaleJson {
            locale: locale.clone(),
            files,
        });
    }

    Ok(TreeCrateJson {
        name: krate.name.clone(),
        locales,
    })
}

fn build_file_tree_json(relative_path: &str, abs_path: &Path) -> TreeFileJson {
    let Ok(resource) = crate::ftl::parse_ftl_file(abs_path) else {
        return TreeFileJson {
            path: relative_path.to_string(),
            parse_error: true,
            entries: Vec::new(),
        };
    };

    let entries = resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(message) => Some(TreeEntryJson {
                id: message.id.name.clone(),
                kind: "message",
                attributes: message
                    .attributes
                    .iter()
                    .map(|attribute| attribute.id.name.clone())
                    .collect(),
                variables: {
                    let mut variables = crate::ftl::extract_variables_from_value_and_attributes(
                        message.value.as_ref(),
                        &message.attributes,
                    )
                    .into_iter()
                    .collect::<Vec<_>>();
                    variables.sort();
                    variables
                },
            }),
            ast::Entry::Term(term) => Some(TreeEntryJson {
                id: format!("-{}", term.id.name),
                kind: "term",
                attributes: term
                    .attributes
                    .iter()
                    .map(|attribute| attribute.id.name.clone())
                    .collect(),
                variables: {
                    let mut variables = crate::ftl::extract_variables_from_value_and_attributes(
                        Some(&term.value),
                        &term.attributes,
                    )
                    .into_iter()
                    .collect::<Vec<_>>();
                    variables.sort();
                    variables
                },
            }),
            _ => None,
        })
        .collect();

    TreeFileJson {
        path: relative_path.to_string(),
        parse_error: false,
        entries,
    }
}

/// Print the tree for a single crate.
fn print_crate_tree(
    krate: &crate::core::CrateInfo,
    all_locales: bool,
    show_attributes: bool,
    show_variables: bool,
) -> Result<()> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;
    let renderer = TreeRenderer::new(show_attributes, show_variables);

    let mut locale_trees: Vec<Tree> = Vec::new();

    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        if !locale_dir.exists() {
            continue;
        }

        let ftl_files = CrateFtlLayout::from_assets_dir(&ctx.assets_dir, locale, &ctx.crate_name)
            .discover_files()?;

        if ftl_files.is_empty() {
            continue;
        }

        let file_trees: Vec<Tree> = ftl_files
            .iter()
            .map(|file_info| {
                renderer.build_file_tree(
                    &file_info.relative_path.display().to_string(),
                    &file_info.abs_path,
                )
            })
            .collect();

        locale_trees.push(Tree::Node(locale.green().to_string(), file_trees));
    }

    let tree = Tree::Node(krate.name.bold().cyan().to_string(), locale_trees);
    println!("{}", tree.render_to_string());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CrateInfo;
    use fluent_syntax::parser;
    use std::fs;

    fn parse_ftl(content: &str) -> ast::Resource<String> {
        parser::parse(content.to_string()).unwrap()
    }

    fn get_message<'a>(
        resource: &'a ast::Resource<String>,
        id: &str,
    ) -> Option<&'a ast::Message<String>> {
        resource.body.iter().find_map(|entry| {
            if let ast::Entry::Message(msg) = entry
                && msg.id.name == id
            {
                return Some(msg);
            }
            None
        })
    }

    fn renderer(show_attributes: bool, show_variables: bool) -> TreeRenderer {
        TreeRenderer::new(show_attributes, show_variables)
    }

    fn create_workspace_with_tree_data() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create i18n dirs");
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"[package]
    name = "test-app"
    version = "0.1.0"
    edition = "2024"
    "#,
        )
        .expect("write Cargo.toml");
        fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");
        fs::write(
            temp.path().join("i18n/en/test-app.ftl"),
            "hello = Hello { $name }\n-term = Term Value\n",
        )
        .expect("write main ftl");
        fs::write(
            temp.path().join("i18n/en/test-app/ui.ftl"),
            "button = Click\n",
        )
        .expect("write namespaced ftl");
        temp
    }

    fn crate_info_from_temp(temp: &tempfile::TempDir) -> CrateInfo {
        CrateInfo {
            name: "test-app".to_string(),
            manifest_dir: temp.path().to_path_buf(),
            src_dir: temp.path().join("src"),
            i18n_config_path: temp.path().join("i18n.toml"),
            ftl_output_dir: temp.path().join("i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    #[test]
    fn test_extract_variables_simple() {
        let content = "hello = Hello { $name }!";
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "hello").unwrap();

        let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
            msg.value.as_ref(),
            &msg.attributes,
        )
        .into_iter()
        .collect();
        variables.sort();

        assert_eq!(variables, vec!["name"]);
    }

    #[test]
    fn test_extract_variables_multiple() {
        let content = "greeting = Hello { $name }, you have { $count } messages";
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "greeting").unwrap();

        let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
            msg.value.as_ref(),
            &msg.attributes,
        )
        .into_iter()
        .collect();
        variables.sort();

        assert_eq!(variables, vec!["count", "name"]);
    }

    #[test]
    fn test_extract_variables_select() {
        let content = r#"count = { $num ->
    [one] One item
       *[other] { $num } items
    }"#;
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "count").unwrap();

        let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
            msg.value.as_ref(),
            &msg.attributes,
        )
        .into_iter()
        .collect();
        variables.sort();

        assert_eq!(variables, vec!["num"]);
    }

    #[test]
    fn test_extract_variables_nested() {
        let content = r#"message = Hello { $user }, today is { DATETIME($date) }"#;
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "message").unwrap();

        let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
            msg.value.as_ref(),
            &msg.attributes,
        )
        .into_iter()
        .collect();
        variables.sort();

        assert_eq!(variables, vec!["date", "user"]);
    }

    #[test]
    fn test_build_message_tree_simple() {
        let content = "hello = Hello World";
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "hello").unwrap();

        let tree = renderer(false, false).build_message_tree("hello", msg);

        match tree {
            Tree::Leaf(lines) => assert_eq!(lines, vec!["hello"]),
            _ => panic!("Expected leaf node"),
        }
    }

    #[test]
    fn test_build_message_tree_with_attributes() {
        let content = r#"button = Button
    .tooltip = Click me
    .aria-label = Submit"#;
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "button").unwrap();

        let tree = renderer(true, false).build_message_tree("button", msg);

        match tree {
            Tree::Node(label, children) => {
                assert_eq!(label, "button");
                assert_eq!(children.len(), 2);
            },
            _ => panic!("Expected node with children"),
        }
    }

    #[test]
    fn test_build_message_tree_with_variables() {
        let content = "greeting = Hello { $name }";
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "greeting").unwrap();

        let tree = renderer(false, true).build_message_tree("greeting", msg);

        match tree {
            Tree::Node(label, children) => {
                assert_eq!(label, "greeting");
                assert_eq!(children.len(), 1);
            },
            _ => panic!("Expected node with children"),
        }
    }

    #[test]
    fn test_build_entry_children_no_attributes_no_variables() {
        let children = renderer(false, false).build_entry_children(&[], None);
        assert!(children.is_empty());
    }

    #[test]
    fn test_build_entry_children_attributes_only() {
        let content = r#"button = Button
    .tooltip = Click me"#;
        let resource = parse_ftl(content);
        let msg = get_message(&resource, "button").unwrap();

        let children =
            renderer(true, false).build_entry_children(&msg.attributes, msg.value.as_ref());

        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_build_file_tree_nonexistent() {
        let tree =
            renderer(false, false).build_file_tree("test.ftl", Path::new("/nonexistent/path.ftl"));

        match tree {
            Tree::Node(label, children) => {
                assert!(label.contains("test.ftl"));
                assert!(
                    children.is_empty(),
                    "nonexistent file should produce empty tree"
                );
            },
            _ => panic!("Expected node"),
        }
    }

    #[test]
    fn test_tree_render_basic() {
        let tree = Tree::Node(
            "root".to_string(),
            vec![
                Tree::Leaf(vec!["item1".to_string()]),
                Tree::Leaf(vec!["item2".to_string()]),
            ],
        );

        let output = tree.render_to_string();
        assert!(output.contains("root"));
        assert!(output.contains("item1"));
        assert!(output.contains("item2"));
    }

    #[test]
    fn test_tree_render_nested() {
        let tree = Tree::Node(
            "crate".to_string(),
            vec![Tree::Node(
                "en".to_string(),
                vec![Tree::Leaf(vec!["message".to_string()])],
            )],
        );

        let output = tree.render_to_string();
        assert!(output.contains("crate"));
        assert!(output.contains("en"));
        assert!(output.contains("message"));
    }

    #[test]
    fn test_build_term_tree_and_print_crate_tree() {
        let temp = create_workspace_with_tree_data();
        let krate = crate_info_from_temp(&temp);

        // Exercise print path for crate tree.
        let printed = print_crate_tree(&krate, false, true, true);
        assert!(printed.is_ok());

        let resource = parse_ftl("-term = Term\n");
        let term = resource
            .body
            .iter()
            .find_map(|entry| match entry {
                ast::Entry::Term(term) => Some(term),
                _ => None,
            })
            .expect("term exists");
        let tree = renderer(false, false).build_term_tree(&term.id.name, term);
        match tree {
            Tree::Leaf(lines) => assert!(lines[0].contains("-term")),
            _ => panic!("expected leaf term tree"),
        }
    }

    #[test]
    fn run_tree_returns_ok_for_missing_package_filter() {
        let temp = create_workspace_with_tree_data();
        let result = run_tree(TreeArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-package".to_string()),
            },
            all: false,
            attributes: false,
            variables: false,
            output: OutputFormat::Text,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn build_file_tree_json_reports_messages_terms_variables_and_parse_errors() {
        let temp = create_workspace_with_tree_data();
        let valid = build_file_tree_json("test-app.ftl", &temp.path().join("i18n/en/test-app.ftl"));

        assert!(!valid.parse_error);
        assert_eq!(valid.path, "test-app.ftl");
        assert!(valid.entries.iter().any(|entry| {
            entry.id == "hello" && entry.kind == "message" && entry.variables == ["name"]
        }));
        assert!(
            valid
                .entries
                .iter()
                .any(|entry| { entry.id == "-term" && entry.kind == "term" })
        );

        let invalid = temp.path().join("i18n/en/broken.ftl");
        fs::write(&invalid, "broken = {").expect("write invalid ftl");
        let broken = build_file_tree_json("broken.ftl", &invalid);
        assert!(broken.parse_error);
        assert!(broken.entries.is_empty());
    }

    #[test]
    fn build_crate_tree_json_collects_locale_files_and_skips_missing_locales() {
        let temp = create_workspace_with_tree_data();
        fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr locale");
        let krate = crate_info_from_temp(&temp);

        let json = build_crate_tree_json(&krate, true).expect("tree json should build");

        assert_eq!(json.name, "test-app");
        assert!(json.locales.iter().any(|locale| locale.locale == "en"));
        assert!(
            json.locales
                .iter()
                .any(|locale| { locale.locale == "fr" && locale.files.is_empty() })
        );
        assert!(json.locales.iter().any(|locale| {
            locale
                .files
                .iter()
                .any(|file| file.path.contains("test-app.ftl"))
        }));
    }

    #[test]
    fn run_tree_covers_json_and_text_command_paths() {
        let temp = create_workspace_with_tree_data();

        let json = run_tree(TreeArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: true,
            attributes: true,
            variables: true,
            output: OutputFormat::Json,
        });
        assert!(json.is_ok());

        let text = run_tree(TreeArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            attributes: true,
            variables: true,
            output: OutputFormat::Text,
        });
        assert!(text.is_ok());
    }
}
