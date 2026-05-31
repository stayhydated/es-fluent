//! Tree command for displaying FTL structure.
//!
//! This module provides functionality to display a tree view of FTL items
//! for each FTL file associated with a crate.

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, WorkspaceInfo};
use crate::ftl::{CrateFtlLayout, LocaleContext};
use crate::generation::MonolithicExecutor;
use crate::utils::ui;
use anyhow::Result;
use clap::{ArgAction, Parser, ValueEnum};
use colored::Colorize as _;
use fluent_syntax::ast;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use terminal_link::Link;
use treelog::Tree;

#[derive(Clone, Copy)]
struct TreeRenderer<'a> {
    show_attributes: bool,
    show_variables: bool,
    terminal_links: bool,
    link_mode: TreeLinkMode,
    rust_links: Option<&'a RustLinkIndex>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourcePosition {
    line: usize,
    column: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EntryLocation {
    id_position: SourcePosition,
    start: usize,
    end: usize,
}

struct FtlSourceMap<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> FtlSourceMap<'a> {
    fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' && offset + 1 < source.len() {
                line_starts.push(offset + 1);
            }
        }

        Self {
            source,
            line_starts,
        }
    }

    fn find_message(&self, id: &str) -> Option<EntryLocation> {
        self.find_entry(id, EntryKind::Message)
    }

    fn find_term(&self, id: &str) -> Option<EntryLocation> {
        self.find_entry(id, EntryKind::Term)
    }

    fn find_attribute(&self, entry: EntryLocation, id: &str) -> Option<SourcePosition> {
        let first_line = self.line_index(entry.start);
        let last_line = self.line_index(entry.end.saturating_sub(1));

        for line_index in first_line..=last_line {
            let line = self.line(line_index);
            let trimmed = line.trim_start();
            let leading = line.len() - trimmed.len();
            let Some(rest) = trimmed.strip_prefix('.') else {
                continue;
            };
            let Some(after_id) = rest.strip_prefix(id) else {
                continue;
            };
            if after_id.trim_start().starts_with('=') {
                return Some(self.position(self.line_starts[line_index] + leading));
            }
        }

        None
    }

    fn find_variable(&self, entry: EntryLocation, name: &str) -> Option<SourcePosition> {
        let needle = format!("${name}");
        let mut offset = entry.start;

        while offset < entry.end {
            let relative = self.source[offset..entry.end].find(&needle)?;
            let candidate = offset + relative;
            let after = candidate + needle.len();
            if self.is_variable_boundary(after) {
                return Some(self.position(candidate));
            }
            offset = after;
        }

        None
    }

    fn find_entry(&self, id: &str, kind: EntryKind) -> Option<EntryLocation> {
        for line_index in 0..self.line_starts.len() {
            let line = self.line(line_index);
            let trimmed = line.trim_start();
            let leading = line.len() - trimmed.len();

            let id_offset = match kind {
                EntryKind::Message => message_id_offset(trimmed, id),
                EntryKind::Term => term_id_offset(trimmed, id),
            };

            if let Some(id_offset) = id_offset {
                let start = self.line_starts[line_index] + leading;
                let id_start = start + id_offset;
                return Some(EntryLocation {
                    id_position: self.position(id_start),
                    start,
                    end: self.entry_end(line_index),
                });
            }
        }

        None
    }

    fn entry_end(&self, start_line: usize) -> usize {
        for line_index in start_line + 1..self.line_starts.len() {
            let line = self.line(line_index);
            let trimmed = line.trim_start();
            if line.len() == trimmed.len() && top_level_entry_start(trimmed) {
                return self.line_starts[line_index];
            }
        }

        self.source.len()
    }

    fn position(&self, offset: usize) -> SourcePosition {
        let line_index = self.line_index(offset);
        SourcePosition {
            line: line_index + 1,
            column: offset.saturating_sub(self.line_starts[line_index]) + 1,
        }
    }

    fn line_index(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
    }

    fn line(&self, index: usize) -> &str {
        let start = self.line_starts[index];
        let end = self
            .line_starts
            .get(index + 1)
            .map_or(self.source.len(), |next| next.saturating_sub(1));
        &self.source[start..end]
    }

    fn is_variable_boundary(&self, offset: usize) -> bool {
        self.source[offset..]
            .chars()
            .next()
            .is_none_or(|ch| !is_identifier_continue(ch))
    }
}

#[derive(Clone, Copy)]
enum EntryKind {
    Message,
    Term,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum TreeLinkMode {
    /// Link message and variable rows to Rust source locations when available.
    #[default]
    Rust,
    /// Link message, attribute, and variable rows to FTL source locations.
    Ftl,
}

#[derive(Clone, Debug)]
struct RustEntryLink {
    path: PathBuf,
    position: Option<SourcePosition>,
    variables: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
struct RustLinkIndex {
    entries: HashMap<String, RustEntryLink>,
}

impl RustLinkIndex {
    fn from_inventory(manifest_dir: &Path, inventory: es_fluent_runner::InventoryData) -> Self {
        let entries = inventory
            .expected_keys
            .into_iter()
            .filter_map(|key| {
                let source_file = key.source_file?;
                let path = absolute_source_path(manifest_dir, source_file.as_str());
                let position = key.source_line.map(|line| SourcePosition {
                    line: line.get() as usize,
                    column: 1,
                });

                Some((
                    key.key.into_string(),
                    RustEntryLink {
                        path,
                        position,
                        variables: key
                            .variables
                            .into_iter()
                            .map(|variable| variable.into_string())
                            .collect(),
                    },
                ))
            })
            .collect();

        Self { entries }
    }

    fn get(&self, key: &str) -> Option<&RustEntryLink> {
        self.entries.get(key)
    }
}

impl<'a> TreeRenderer<'a> {
    fn new(
        show_attributes: bool,
        show_variables: bool,
        terminal_links: bool,
        link_mode: TreeLinkMode,
        rust_links: Option<&'a RustLinkIndex>,
    ) -> Self {
        Self {
            show_attributes,
            show_variables,
            terminal_links,
            link_mode,
            rust_links,
        }
    }

    /// Build a tree for a single FTL file.
    fn build_file_tree(&self, relative_path: &str, abs_path: &Path) -> Tree {
        let file_label = self.path_link_label(relative_path.yellow().to_string(), abs_path, None);
        let source = fs::read_to_string(abs_path).ok();
        let source_map = source.as_deref().map(FtlSourceMap::new);
        let resource = match crate::ftl::parse_ftl_file(abs_path) {
            Ok(res) => res,
            Err(_) => {
                return Tree::Node(
                    file_label,
                    vec![Tree::Leaf(vec!["<parse error>".red().to_string()])],
                );
            },
        };

        let entries: Vec<Tree> = resource
            .body
            .iter()
            .filter_map(|entry| match entry {
                ast::Entry::Message(msg) => Some(self.build_message_tree_with_source(
                    &msg.id.name,
                    msg,
                    Some(abs_path),
                    source_map.as_ref(),
                )),
                ast::Entry::Term(term) => Some(self.build_term_tree_with_source(
                    &term.id.name,
                    term,
                    Some(abs_path),
                    source_map.as_ref(),
                )),
                ast::Entry::Comment(_) => None,
                ast::Entry::GroupComment(_) => None,
                ast::Entry::ResourceComment(_) => None,
                ast::Entry::Junk { .. } => None,
            })
            .collect();

        Tree::Node(file_label, entries)
    }

    /// Build a tree for a message entry.
    #[cfg(test)]
    fn build_message_tree(&self, id: &str, msg: &ast::Message<String>) -> Tree {
        self.build_message_tree_with_source(id, msg, None, None)
    }

    fn build_message_tree_with_source(
        &self,
        id: &str,
        msg: &ast::Message<String>,
        abs_path: Option<&Path>,
        source_map: Option<&FtlSourceMap<'_>>,
    ) -> Tree {
        let entry_location = source_map.and_then(|map| map.find_message(id));
        let children = self.build_entry_children_with_source(
            Some(id),
            &msg.attributes,
            msg.value.as_ref(),
            abs_path,
            source_map,
            entry_location,
        );
        let label = self.link_label(
            id.to_string(),
            self.entry_link_target(
                id,
                abs_path,
                entry_location.map(|location| location.id_position),
            ),
        );

        if children.is_empty() {
            Tree::Leaf(vec![label])
        } else {
            Tree::Node(label, children)
        }
    }

    /// Build a tree for a term entry.
    #[cfg(test)]
    fn build_term_tree(&self, id: &str, term: &ast::Term<String>) -> Tree {
        self.build_term_tree_with_source(id, term, None, None)
    }

    fn build_term_tree_with_source(
        &self,
        id: &str,
        term: &ast::Term<String>,
        abs_path: Option<&Path>,
        source_map: Option<&FtlSourceMap<'_>>,
    ) -> Tree {
        let entry_location = source_map.and_then(|map| map.find_term(id));
        let term_key = format!("-{id}");
        let children = self.build_entry_children_with_source(
            Some(&term_key),
            &term.attributes,
            Some(&term.value),
            abs_path,
            source_map,
            entry_location,
        );
        let label = format!("-{}", id);
        let label = self.link_label(
            label.dimmed().to_string(),
            self.entry_link_target(
                &term_key,
                abs_path,
                entry_location.map(|location| location.id_position),
            ),
        );

        if children.is_empty() {
            Tree::Leaf(vec![label])
        } else {
            Tree::Node(label, children)
        }
    }

    /// Build child nodes for an entry (attributes and variables).
    #[cfg(test)]
    fn build_entry_children(
        &self,
        attributes: &[ast::Attribute<String>],
        value: Option<&ast::Pattern<String>>,
    ) -> Vec<Tree> {
        self.build_entry_children_with_source(None, attributes, value, None, None, None)
    }

    fn build_entry_children_with_source(
        &self,
        current_key: Option<&str>,
        attributes: &[ast::Attribute<String>],
        value: Option<&ast::Pattern<String>>,
        abs_path: Option<&Path>,
        source_map: Option<&FtlSourceMap<'_>>,
        entry_location: Option<EntryLocation>,
    ) -> Vec<Tree> {
        let mut children: Vec<Tree> = Vec::new();

        if self.show_attributes {
            for attr in attributes {
                let attr_label = format!("@{}", attr.id.name);
                let position = source_map.and_then(|map| {
                    entry_location.and_then(|location| map.find_attribute(location, &attr.id.name))
                });
                let attr_label = self.link_label(
                    attr_label.dimmed().to_string(),
                    self.ftl_link_target(abs_path, position),
                );
                children.push(Tree::Leaf(vec![attr_label]));
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
                    .map(|v| {
                        let position = source_map.and_then(|map| {
                            entry_location.and_then(|location| map.find_variable(location, v))
                        });
                        self.link_label(
                            format!("${v}").magenta().to_string(),
                            self.variable_link_target(current_key, v, abs_path, position),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(&", ".magenta().to_string());
                children.push(Tree::Leaf(vec![vars_str]));
            }
        }

        children
    }

    fn path_link_label(
        &self,
        label: String,
        path: &Path,
        position: Option<SourcePosition>,
    ) -> String {
        self.link_label(label, Some((path, position)))
    }

    fn ftl_link_target<'b>(
        &self,
        ftl_path: Option<&'b Path>,
        ftl_position: Option<SourcePosition>,
    ) -> Option<(&'b Path, Option<SourcePosition>)> {
        ftl_path.map(|path| (path, ftl_position))
    }

    fn entry_link_target<'b>(
        &'b self,
        key: &str,
        ftl_path: Option<&'b Path>,
        ftl_position: Option<SourcePosition>,
    ) -> Option<(&'b Path, Option<SourcePosition>)> {
        if self.link_mode == TreeLinkMode::Rust
            && let Some(rust_link) = self.rust_links.and_then(|links| links.get(key))
        {
            return Some((rust_link.path.as_path(), rust_link.position));
        }

        self.ftl_link_target(ftl_path, ftl_position)
    }

    fn variable_link_target<'b>(
        &'b self,
        key: Option<&str>,
        variable: &str,
        ftl_path: Option<&'b Path>,
        ftl_position: Option<SourcePosition>,
    ) -> Option<(&'b Path, Option<SourcePosition>)> {
        if self.link_mode == TreeLinkMode::Rust
            && let Some(rust_link) = key
                .and_then(|key| self.rust_links.and_then(|links| links.get(key)))
                .filter(|link| link.variables.contains(variable))
        {
            return Some((rust_link.path.as_path(), rust_link.position));
        }

        self.ftl_link_target(ftl_path, ftl_position)
    }

    fn link_label(&self, label: String, target: Option<(&Path, Option<SourcePosition>)>) -> String {
        if !self.terminal_links {
            return label;
        }

        let Some((path, position)) = target else {
            return label;
        };

        let url = file_url(path, position);
        Link::new(&label, &url).to_string()
    }
}

fn file_url(path: &Path, position: Option<SourcePosition>) -> String {
    match position {
        Some(position) => format!(
            "file://{}:{}:{}",
            path.display(),
            position.line,
            position.column
        ),
        None => format!("file://{}", path.display()),
    }
}

fn absolute_source_path(manifest_dir: &Path, source_file: &str) -> PathBuf {
    let source_path = Path::new(source_file);
    if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        manifest_dir.join(source_path)
    }
}

fn message_id_offset(line: &str, id: &str) -> Option<usize> {
    let rest = line.strip_prefix(id)?;
    rest.trim_start().starts_with('=').then_some(0)
}

fn term_id_offset(line: &str, id: &str) -> Option<usize> {
    let rest = line.strip_prefix('-')?.strip_prefix(id)?;
    rest.trim_start().starts_with('=').then_some(0)
}

fn top_level_entry_start(line: &str) -> bool {
    if line.is_empty() || line.starts_with('}') {
        return false;
    }
    line.starts_with('#')
        || term_entry_start(line)
        || line
            .chars()
            .next()
            .is_some_and(|ch| is_identifier_start(ch) && line.contains('='))
}

fn term_entry_start(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('-') else {
        return false;
    };
    rest.chars()
        .next()
        .is_some_and(|ch| is_identifier_start(ch) && line.contains('='))
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}

/// Arguments for the tree command.
#[derive(Debug, Parser)]
pub struct TreeArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Show all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Hide attributes under each message.
    #[arg(long = "no-attributes", action = ArgAction::SetFalse, default_value_t = true)]
    pub attributes: bool,

    /// Hide variables used in each message.
    #[arg(long = "no-variables", action = ArgAction::SetFalse, default_value_t = true)]
    pub variables: bool,

    /// Link target mode for message and variable rows.
    #[arg(long = "link-mode", value_enum, default_value_t = TreeLinkMode::default())]
    pub link_mode: TreeLinkMode,

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
    let terminal_links = show_text && ui::Ui::terminal_links_enabled();

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

    let rust_link_indexes = collect_rust_link_indexes(&workspace, args.link_mode, terminal_links)?;

    for krate in &workspace.crates {
        print_crate_tree(
            krate,
            args.all,
            args.attributes,
            args.variables,
            terminal_links,
            args.link_mode,
            rust_link_indexes.get(&krate.name),
        )?;
    }

    Ok(())
}

fn collect_rust_link_indexes(
    workspace: &WorkspaceCrates,
    link_mode: TreeLinkMode,
    terminal_links: bool,
) -> Result<HashMap<String, RustLinkIndex>, CliError> {
    if !terminal_links || link_mode != TreeLinkMode::Rust || workspace.valid.is_empty() {
        return Ok(HashMap::new());
    }

    let runner_workspace = WorkspaceInfo {
        root_dir: workspace.workspace_info.root_dir.clone(),
        target_dir: workspace.workspace_info.target_dir.clone(),
        crates: workspace.valid.clone(),
    };

    crate::generation::prepare_monolithic_runner_crate(&runner_workspace)
        .map_err(|error| CliError::Other(error.to_string()))?;

    let temp_store =
        es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&runner_workspace.root_dir);
    let executor = MonolithicExecutor::new(&runner_workspace);
    let mut indexes = HashMap::new();

    for krate in &workspace.valid {
        executor
            .execute_request(&krate.check_request(), false)
            .map_err(|error| CliError::Other(error.to_string()))?;

        let inventory = temp_store
            .read_inventory(&krate.name)
            .map_err(|error| CliError::Other(error.to_string()))?;
        indexes.insert(
            krate.name.clone(),
            RustLinkIndex::from_inventory(&krate.manifest_dir, inventory),
        );
    }

    Ok(indexes)
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
    terminal_links: bool,
    link_mode: TreeLinkMode,
    rust_links: Option<&RustLinkIndex>,
) -> Result<()> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;
    let renderer = TreeRenderer::new(
        show_attributes,
        show_variables,
        terminal_links,
        link_mode,
        rust_links,
    );

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

        let locale_label = renderer.path_link_label(locale.green().to_string(), &locale_dir, None);
        locale_trees.push(Tree::Node(locale_label, file_trees));
    }

    let crate_label = renderer.path_link_label(
        krate.name.bold().cyan().to_string(),
        &krate.manifest_dir,
        None,
    );
    let tree = Tree::Node(crate_label, locale_trees);
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

    fn renderer(show_attributes: bool, show_variables: bool) -> TreeRenderer<'static> {
        TreeRenderer::new(
            show_attributes,
            show_variables,
            ui::Ui::terminal_links_enabled(),
            TreeLinkMode::Rust,
            None,
        )
    }

    fn position(line: usize, column: usize) -> SourcePosition {
        SourcePosition { line, column }
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
    fn tree_args_show_attributes_and_variables_by_default() {
        let default = TreeArgs::try_parse_from(["tree"]).expect("default tree args parse");
        assert!(default.attributes);
        assert!(default.variables);
        assert_eq!(default.link_mode, TreeLinkMode::Rust);

        let hidden = TreeArgs::try_parse_from(["tree", "--no-attributes", "--no-variables"])
            .expect("negative detail flags parse");
        assert!(!hidden.attributes);
        assert!(!hidden.variables);

        let ftl_links =
            TreeArgs::try_parse_from(["tree", "--link-mode", "ftl"]).expect("ftl link mode parses");
        assert_eq!(ftl_links.link_mode, TreeLinkMode::Ftl);

        assert!(TreeArgs::try_parse_from(["tree", "--attributes"]).is_err());
        assert!(TreeArgs::try_parse_from(["tree", "--variables"]).is_err());
    }

    #[test]
    fn ftl_source_map_finds_entry_attribute_and_variable_positions() {
        let content = "greeting = Hello { $name }\n    .title = Title for { $name }\ncount = { $num ->\n    [one] One\n   *[other] { $num }\n}\n-term = Term { $value }\n";
        let source_map = FtlSourceMap::new(content);

        let greeting = source_map.find_message("greeting").unwrap();
        assert_eq!(greeting.id_position, position(1, 1));
        assert_eq!(
            source_map.find_attribute(greeting, "title"),
            Some(position(2, 5))
        );
        assert_eq!(
            source_map.find_variable(greeting, "name"),
            Some(position(1, 20))
        );

        let count = source_map.find_message("count").unwrap();
        assert_eq!(count.id_position, position(3, 1));
        assert_eq!(
            source_map.find_variable(count, "num"),
            Some(position(3, 11))
        );

        let term = source_map.find_term("term").unwrap();
        assert_eq!(term.id_position, position(7, 1));
        assert_eq!(
            source_map.find_variable(term, "value"),
            Some(position(7, 16))
        );
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
    #[serial_test::serial(process)]
    fn build_file_tree_adds_terminal_links_for_entries_and_variables() {
        temp_env::with_var("FORCE_HYPERLINK", Some("1"), || {
            let temp = tempfile::tempdir().expect("tempdir");
            let ftl_path = temp.path().join("test-app.ftl");
            fs::write(&ftl_path, "greeting = Hello { $name }\n").expect("write ftl");

            let tree = renderer(false, true).build_file_tree("test-app.ftl", &ftl_path);
            let output = tree.render_to_string();

            assert!(output.contains("\u{1b}]8;;file://"));
            assert!(output.contains(&format!("file://{}", ftl_path.display())));
            assert!(output.contains(&format!("file://{}:1:1", ftl_path.display())));
            assert!(output.contains(&format!("file://{}:1:20", ftl_path.display())));
        });
    }

    #[test]
    fn build_file_tree_link_mode_selects_rust_or_ftl_targets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ftl_path = temp.path().join("test-app.ftl");
        let rust_path = temp.path().join("src/lib.rs");
        fs::create_dir_all(rust_path.parent().unwrap()).expect("create src dir");
        fs::write(&ftl_path, "greeting = Hello { $name }\n").expect("write ftl");
        fs::write(&rust_path, "pub struct Greeting;\n").expect("write rust");

        let rust_links = RustLinkIndex::from_inventory(
            temp.path(),
            es_fluent_runner::InventoryData {
                expected_keys: vec![es_fluent_runner::ExpectedKey {
                    key: es_fluent_shared::fluent::FluentEntryId::try_new("greeting").expect("key"),
                    variables: vec![
                        es_fluent_shared::fluent::FluentArgumentName::try_new("name")
                            .expect("variable"),
                    ],
                    source_file: es_fluent_shared::source::SourceFile::new("src/lib.rs"),
                    source_line: Some(es_fluent_shared::source::SourceLine::new(42)),
                }],
            },
        );

        let rust_renderer =
            TreeRenderer::new(false, true, true, TreeLinkMode::Rust, Some(&rust_links));
        let rust_output = rust_renderer
            .build_file_tree("test-app.ftl", &ftl_path)
            .render_to_string();

        assert!(rust_output.contains(&format!("file://{}:42:1", rust_path.display())));
        assert!(!rust_output.contains(&format!("file://{}:1:1", ftl_path.display())));
        assert!(!rust_output.contains(&format!("file://{}:1:20", ftl_path.display())));

        let ftl_renderer =
            TreeRenderer::new(false, true, true, TreeLinkMode::Ftl, Some(&rust_links));
        let ftl_output = ftl_renderer
            .build_file_tree("test-app.ftl", &ftl_path)
            .render_to_string();

        assert!(ftl_output.contains(&format!("file://{}:1:1", ftl_path.display())));
        assert!(ftl_output.contains(&format!("file://{}:1:20", ftl_path.display())));
        assert!(!ftl_output.contains(&format!("file://{}:42:1", rust_path.display())));
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
        let printed = print_crate_tree(&krate, false, true, true, false, TreeLinkMode::Rust, None);
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
            link_mode: TreeLinkMode::Rust,
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
            link_mode: TreeLinkMode::Rust,
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
            link_mode: TreeLinkMode::Ftl,
            output: OutputFormat::Text,
        });
        assert!(text.is_ok());
    }
}
