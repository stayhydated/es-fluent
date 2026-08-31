use proc_macro2::TokenStream;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use syn::visit::Visit as _;

use super::super::FallbackValidationDerive;
use super::cfg::{
    attributes_create_test_context, attributes_enable_test_only_derive, attributes_require_test,
};

#[derive(Debug)]
pub(crate) struct SourceDeclaration {
    pub(crate) path: PathBuf,
    pub(crate) marked_source: String,
    pub(crate) marker_ident: String,
}

pub(crate) fn collect_source_evidence(
    path: &Path,
    module_dir: &Path,
    parent_requires_test: bool,
    target: &SourceDeclaration,
    derive: Option<FallbackValidationDerive>,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    evidence: &mut Vec<bool>,
) {
    let path = canonical_path(path);
    let module_dir = canonical_path(module_dir);
    if !visited.insert((path.clone(), module_dir.clone(), parent_requires_test)) {
        return;
    }
    let source = if path == target.path {
        target.marked_source.as_str()
    } else {
        let Ok(source) = std::fs::read_to_string(&path) else {
            return;
        };
        // The parsed syntax is used only for this call.
        return collect_source_evidence_from_source(
            &source,
            &path,
            &module_dir,
            parent_requires_test,
            target,
            derive,
            visited,
            evidence,
        );
    };
    collect_source_evidence_from_source(
        source,
        &path,
        &module_dir,
        parent_requires_test,
        target,
        derive,
        visited,
        evidence,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "source ownership evidence carries traversal state explicitly"
)]
fn collect_source_evidence_from_source(
    source: &str,
    path: &Path,
    module_dir: &Path,
    parent_requires_test: bool,
    target: &SourceDeclaration,
    derive: Option<FallbackValidationDerive>,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    evidence: &mut Vec<bool>,
) {
    let Ok(file) = syn::parse_file(source) else {
        return;
    };
    collect_item_evidence(
        &file.items,
        path,
        module_dir,
        parent_requires_test,
        target,
        derive,
        visited,
        evidence,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "source ownership evidence carries traversal state explicitly"
)]
fn collect_item_evidence(
    items: &[syn::Item],
    current_file: &Path,
    module_dir: &Path,
    parent_requires_test: bool,
    target: &SourceDeclaration,
    derive: Option<FallbackValidationDerive>,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    evidence: &mut Vec<bool>,
) {
    for item in items {
        let declaration = match item {
            syn::Item::Enum(item) => Some((&item.ident, &item.attrs)),
            syn::Item::Struct(item) => Some((&item.ident, &item.attrs)),
            syn::Item::Union(item) => Some((&item.ident, &item.attrs)),
            _ => None,
        };
        if let Some((ident, attributes)) = declaration
            && current_file == target.path
            && ident == target.marker_ident.as_str()
        {
            evidence.push(
                parent_requires_test
                    || attributes_require_test(attributes)
                    || attributes_enable_test_only_derive(attributes, derive),
            );
        }

        if let syn::Item::Macro(item_macro) = item
            && let Some(include_path) = literal_include_path(&item_macro.mac, current_file)
        {
            let include_requires_test =
                parent_requires_test || attributes_require_test(&item_macro.attrs);
            collect_source_evidence(
                &include_path,
                include_path.parent().unwrap_or(Path::new("")),
                include_requires_test,
                target,
                derive,
                visited,
                evidence,
            );
        }

        if let syn::Item::Macro(item_macro) = item
            && current_file == target.path
            && token_stream_contains_ident(&item_macro.mac.tokens, &target.marker_ident)
        {
            evidence.push(
                parent_requires_test
                    || attributes_require_test(&item_macro.attrs)
                    || attributes_enable_test_only_derive(&item_macro.attrs, derive),
            );
        }

        if let syn::Item::Fn(function) = item {
            let mut visitor = LocalItemEvidenceVisitor {
                current_file,
                module_dir: module_dir.to_path_buf(),
                parent_requires_test,
                target,
                derive,
                visited,
                evidence,
            };
            visitor.visit_item_fn(function);
        }

        let syn::Item::Mod(module) = item else {
            continue;
        };
        let module_requires_test = parent_requires_test || attributes_require_test(&module.attrs);
        if let Some((_, items)) = &module.content {
            let child_dir = module_dir.join(module.ident.to_string());
            collect_item_evidence(
                items,
                current_file,
                &child_dir,
                module_requires_test,
                target,
                derive,
                visited,
                evidence,
            );
            continue;
        }

        for (child_path, child_dir) in resolve_module_paths(module, module_dir) {
            if child_path == target.path || target.path.starts_with(&child_dir) {
                collect_source_evidence(
                    &child_path,
                    &child_dir,
                    module_requires_test,
                    target,
                    derive,
                    visited,
                    evidence,
                );
            }
        }
    }
}

pub(crate) fn literal_include_path(
    include_macro: &syn::Macro,
    current_file: &Path,
) -> Option<PathBuf> {
    if !include_macro.path.is_ident("include") {
        return None;
    }

    let path = syn::parse2::<syn::LitStr>(include_macro.tokens.clone()).ok()?;
    let parent = current_file.parent().unwrap_or(Path::new(""));
    let path = canonical_path(&parent.join(path.value()));
    path.is_file().then_some(path)
}

struct LocalItemEvidenceVisitor<'a> {
    current_file: &'a Path,
    module_dir: PathBuf,
    parent_requires_test: bool,
    target: &'a SourceDeclaration,
    derive: Option<FallbackValidationDerive>,
    visited: &'a mut HashSet<(PathBuf, PathBuf, bool)>,
    evidence: &'a mut Vec<bool>,
}

impl LocalItemEvidenceVisitor<'_> {
    fn record_declaration(&mut self, ident: &syn::Ident, attributes: &[syn::Attribute]) {
        if self.current_file == self.target.path && ident == self.target.marker_ident.as_str() {
            self.evidence.push(
                self.parent_requires_test
                    || attributes_require_test(attributes)
                    || attributes_enable_test_only_derive(attributes, self.derive),
            );
        }
    }

    fn with_test_context(&mut self, attributes: &[syn::Attribute], visit: impl FnOnce(&mut Self)) {
        let parent_requires_test = self.parent_requires_test;
        self.parent_requires_test |= attributes_create_test_context(attributes);
        visit(self);
        self.parent_requires_test = parent_requires_test;
    }

    fn collect_literal_include(
        &mut self,
        include_macro: &syn::Macro,
        attributes: &[syn::Attribute],
    ) {
        let Some(include_path) = literal_include_path(include_macro, self.current_file) else {
            return;
        };
        let include_requires_test =
            self.parent_requires_test || attributes_require_test(attributes);
        collect_source_evidence(
            &include_path,
            include_path.parent().unwrap_or(Path::new("")),
            include_requires_test,
            self.target,
            self.derive,
            self.visited,
            self.evidence,
        );
    }

    fn collect_literal_statement_include(
        &mut self,
        include_macro: &syn::Macro,
        attributes: &[syn::Attribute],
    ) {
        let Some(include_path) = literal_include_path(include_macro, self.current_file) else {
            return;
        };
        let include_requires_test =
            self.parent_requires_test || attributes_require_test(attributes);
        if !self.visited.insert((
            include_path.clone(),
            include_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            include_requires_test,
        )) {
            return;
        }
        let source = if include_path == self.target.path {
            self.target.marked_source.clone()
        } else {
            let Ok(source) = std::fs::read_to_string(&include_path) else {
                return;
            };
            source
        };
        let Ok(expression) = syn::parse_str::<syn::Expr>(&source) else {
            return;
        };
        let mut visitor = LocalItemEvidenceVisitor {
            current_file: &include_path,
            module_dir: include_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            parent_requires_test: include_requires_test,
            target: self.target,
            derive: self.derive,
            visited: self.visited,
            evidence: self.evidence,
        };
        visitor.visit_expr(&expression);
    }
}

impl<'ast> syn::visit::Visit<'ast> for LocalItemEvidenceVisitor<'_> {
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.record_declaration(&item.ident, &item.attrs);
        syn::visit::visit_item_enum(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.with_test_context(&item.attrs, |visitor| {
            syn::visit::visit_item_fn(visitor, item);
        });
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        self.collect_literal_include(&item.mac, &item.attrs);
        if self.current_file == self.target.path
            && token_stream_contains_ident(&item.mac.tokens, &self.target.marker_ident)
        {
            self.evidence.push(
                self.parent_requires_test
                    || attributes_require_test(&item.attrs)
                    || attributes_enable_test_only_derive(&item.attrs, self.derive),
            );
        }
        syn::visit::visit_item_macro(self, item);
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.collect_literal_statement_include(&statement.mac, &statement.attrs);
        syn::visit::visit_stmt_macro(self, statement);
    }

    fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
        self.collect_literal_statement_include(&expression.mac, &expression.attrs);
        syn::visit::visit_expr_macro(self, expression);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.with_test_context(&item.attrs, |visitor| {
            if let Some((_, items)) = &item.content {
                let child_module_dir = visitor.module_dir.join(item.ident.to_string());
                let previous_module_dir =
                    std::mem::replace(&mut visitor.module_dir, child_module_dir);
                for item in items {
                    visitor.visit_item(item);
                }
                visitor.module_dir = previous_module_dir;
                return;
            }

            for (child_path, child_dir) in resolve_module_paths(item, &visitor.module_dir) {
                if child_path == visitor.target.path || visitor.target.path.starts_with(&child_dir)
                {
                    collect_source_evidence(
                        &child_path,
                        &child_dir,
                        visitor.parent_requires_test,
                        visitor.target,
                        visitor.derive,
                        visitor.visited,
                        visitor.evidence,
                    );
                }
            }
        });
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.record_declaration(&item.ident, &item.attrs);
        syn::visit::visit_item_struct(self, item);
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.record_declaration(&item.ident, &item.attrs);
        syn::visit::visit_item_union(self, item);
    }
}

fn token_stream_contains_ident(tokens: &TokenStream, target: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        proc_macro2::TokenTree::Group(group) => {
            token_stream_contains_ident(&group.stream(), target)
        },
        proc_macro2::TokenTree::Ident(ident) => ident == target,
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

fn resolve_module_paths(module: &syn::ItemMod, module_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    if let Some(attribute) = module
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("path"))
    {
        let syn::Meta::NameValue(value) = &attribute.meta else {
            return Vec::new();
        };
        let syn::Expr::Lit(expression) = &value.value else {
            return Vec::new();
        };
        let syn::Lit::Str(path) = &expression.lit else {
            return Vec::new();
        };
        let path = canonical_path(&module_dir.join(path.value()));
        let child_dir = module_child_dir(&path, &module.ident.to_string());
        return path
            .is_file()
            .then_some((path, child_dir))
            .into_iter()
            .collect();
    }

    let name = module.ident.to_string();
    let flat = canonical_path(&module_dir.join(format!("{name}.rs")));
    let nested = canonical_path(&module_dir.join(&name).join("mod.rs"));
    [flat, nested]
        .into_iter()
        .filter(|path| path.is_file())
        .map(|path| {
            let child_dir = module_child_dir(&path, &name);
            (path, child_dir)
        })
        .collect()
}

fn module_child_dir(path: &Path, module_name: &str) -> PathBuf {
    if path.file_name().is_some_and(|name| name == "mod.rs") {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.parent()
            .map(|parent| parent.join(module_name))
            .unwrap_or_default()
    }
}

pub(super) fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(super) fn mark_source_declaration(
    source: &str,
    range: std::ops::Range<usize>,
    expected: Option<&str>,
) -> Option<(String, String)> {
    let actual = source.get(range.clone())?;
    if actual.is_empty() || expected.is_some_and(|expected| expected != actual) {
        return None;
    }
    let mut marker = "__EsFluentFallbackValidationTarget".to_string();
    while source.contains(&marker) {
        marker.push('_');
    }
    let mut marked = source.to_string();
    marked.replace_range(range, &marker);
    Some((marked, marker))
}

pub(super) fn source_range(
    source: &str,
    location: proc_macro2::LineColumn,
    expected: Option<&str>,
) -> Option<std::ops::Range<usize>> {
    let expected = expected?;
    let line_start = if location.line == 1 {
        0
    } else {
        source
            .match_indices('\n')
            .nth(location.line.checked_sub(2)?)
            .map(|(index, _)| index + 1)?
    };
    let line = source
        .get(line_start..)?
        .split_once('\n')
        .map_or_else(|| source.get(line_start..), |(line, _)| Some(line))?;
    let byte_column = line_start.checked_add(location.column)?;
    let character_column = line.char_indices().nth(location.column).map_or_else(
        || line_start.checked_add(line.len()),
        |(index, _)| line_start.checked_add(index),
    )?;

    [byte_column, character_column]
        .into_iter()
        .find_map(|start| {
            let end = start.checked_add(expected.len())?;
            (source.get(start..end) == Some(expected)).then_some(start..end)
        })
}
