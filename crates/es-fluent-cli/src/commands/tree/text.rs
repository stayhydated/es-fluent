use super::{
    links::{RustLinkIndex, TreeLinkMode, file_url},
    source_map::{EntryLocation, FtlSourceMap, SourcePosition},
    validation::{validate_tree_locale_dir, validate_tree_locale_setup},
};

use crate::ftl::LocaleContext;

use anyhow::Result;

use anstream::println;

use colored::Colorize as _;

use fluent_syntax::ast;

use std::{fs, path::Path};

use terminal_link::Link;

use treelog::Tree;

#[derive(Clone, Copy)]
pub(super) struct TreeRenderer<'a> {
    show_attributes: bool,
    show_variables: bool,
    terminal_links: bool,
    link_mode: TreeLinkMode,
    rust_links: Option<&'a RustLinkIndex>,
}
impl<'a> TreeRenderer<'a> {
    pub(super) fn new(
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
    pub(super) fn build_file_tree(&self, relative_path: &str, abs_path: &Path) -> Result<Tree> {
        let file_label = self.path_link_label(relative_path.yellow().to_string(), abs_path, None);
        let resource = crate::ftl::parse_ftl_file(abs_path).map_err(|error| {
            anyhow::anyhow!("failed to parse FTL file '{relative_path}': {error}")
        })?;
        let source = fs::read_to_string(abs_path).map_err(|error| {
            anyhow::anyhow!("failed to read FTL file '{relative_path}': {error}")
        })?;
        let source_map = FtlSourceMap::new(&source);

        let entries: Vec<Tree> = resource
            .body
            .iter()
            .filter_map(|entry| match entry {
                ast::Entry::Message(msg) => Some(self.build_message_tree_with_source(
                    &msg.id.name,
                    msg,
                    Some(abs_path),
                    Some(&source_map),
                )),
                ast::Entry::Term(term) => Some(self.build_term_tree_with_source(
                    &term.id.name,
                    term,
                    Some(abs_path),
                    Some(&source_map),
                )),
                ast::Entry::Comment(_) => None,
                ast::Entry::GroupComment(_) => None,
                ast::Entry::ResourceComment(_) => None,
                ast::Entry::Junk { .. } => None,
            })
            .collect();

        Ok(Tree::Node(file_label, entries))
    }

    /// Build a tree for a message entry.
    #[cfg(test)]
    pub(super) fn build_message_tree(&self, id: &str, msg: &ast::Message<String>) -> Tree {
        self.build_message_tree_with_source(id, msg, None, None)
    }

    pub(super) fn build_message_tree_with_source(
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
    pub(super) fn build_term_tree(&self, id: &str, term: &ast::Term<String>) -> Tree {
        self.build_term_tree_with_source(id, term, None, None)
    }

    pub(super) fn build_term_tree_with_source(
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
    pub(super) fn build_entry_children(
        &self,
        attributes: &[ast::Attribute<String>],
        value: Option<&ast::Pattern<String>>,
    ) -> Vec<Tree> {
        self.build_entry_children_with_source(None, attributes, value, None, None, None)
    }

    pub(super) fn build_entry_children_with_source(
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
            let variable_attributes = if self.show_attributes {
                attributes
            } else {
                &[]
            };
            let mut variables: Vec<_> =
                crate::ftl::extract_variables_from_value_and_attributes(value, variable_attributes)
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

    pub(super) fn path_link_label(
        &self,
        label: String,
        path: &Path,
        position: Option<SourcePosition>,
    ) -> String {
        self.link_label(label, Some((path, position)))
    }

    pub(super) fn ftl_link_target<'b>(
        &self,
        ftl_path: Option<&'b Path>,
        ftl_position: Option<SourcePosition>,
    ) -> Option<(&'b Path, Option<SourcePosition>)> {
        ftl_path.map(|path| (path, ftl_position))
    }

    pub(super) fn entry_link_target<'b>(
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

    pub(super) fn variable_link_target<'b>(
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

    pub(super) fn link_label(
        &self,
        label: String,
        target: Option<(&Path, Option<SourcePosition>)>,
    ) -> String {
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
/// Print the tree for a single crate.
pub(super) fn print_crate_tree(
    krate: &crate::core::CrateInfo,
    all_locales: bool,
    show_attributes: bool,
    show_variables: bool,
    terminal_links: bool,
    link_mode: TreeLinkMode,
    rust_links: Option<&RustLinkIndex>,
) -> Result<()> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;
    validate_tree_locale_setup(&ctx, all_locales)?;
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
        validate_tree_locale_dir(locale, &locale_dir)?;

        let ftl_files = ctx.discover_files(locale)?;

        let file_trees: Vec<Tree> = ftl_files
            .iter()
            .map(|file_info| {
                renderer.build_file_tree(
                    &crate::utils::paths::slash_path(&file_info.relative_path),
                    &file_info.abs_path,
                )
            })
            .collect::<Result<_>>()?;

        let locale_label = renderer.path_link_label(locale.green().to_string(), &locale_dir, None);
        locale_trees.push(Tree::Node(locale_label, file_trees));
    }

    let crate_label = renderer.path_link_label(
        krate.name.as_str().bold().cyan().to_string(),
        &krate.manifest_dir,
        None,
    );
    let tree = Tree::Node(crate_label, locale_trees);
    println!("{}", tree.render_to_string());

    Ok(())
}
