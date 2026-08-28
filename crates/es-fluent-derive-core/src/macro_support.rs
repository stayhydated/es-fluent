//! Shared helpers for proc-macro crates built on `es-fluent-derive-core`.

use crate::error::EsFluentCoreError;
use es_fluent_shared::fluent::{
    FluentArgumentName, FluentDomain, FluentMessageId, FluentVariantKey,
};
use es_fluent_shared::resource::{
    FALLBACK_CATALOG_ENV, INVENTORY_RUNNER_ENV, fallback_catalog_contains,
};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ResolvedCratePath {
    tokens: TokenStream,
    rust_path: String,
}

impl ResolvedCratePath {
    pub fn resolve(package_name: &str, fallback_crate_ident: &str) -> Self {
        match crate_name(package_name) {
            // Rustdoc compiles a crate's examples in a synthetic crate while
            // retaining the documented package manifest. Resolve through the
            // passed `--extern` facade instead of that synthetic `crate` root.
            Ok(FoundCrate::Itself) if std::env::var_os("UNSTABLE_RUSTDOC_TEST_PATH").is_some() => {
                Self::fallback(fallback_crate_ident)
            },
            Ok(FoundCrate::Itself) => Self {
                tokens: quote! { crate },
                rust_path: "crate".to_string(),
            },
            Ok(FoundCrate::Name(name)) => {
                let ident = format_ident!("{name}");
                Self {
                    tokens: quote! { ::#ident },
                    rust_path: format!("::{name}"),
                }
            },
            Err(_) => Self::fallback(fallback_crate_ident),
        }
    }

    pub fn fallback(crate_ident: &str) -> Self {
        let ident = format_ident!("{crate_ident}");
        Self {
            tokens: quote! { ::#ident },
            rust_path: format!("::{crate_ident}"),
        }
    }

    pub fn tokens(&self) -> &TokenStream {
        &self.tokens
    }

    pub fn rust_path(&self) -> &str {
        &self.rust_path
    }
}

pub fn resolve_crate_path(package_name: &str, fallback_crate_ident: &str) -> TokenStream {
    ResolvedCratePath::resolve(package_name, fallback_crate_ident)
        .tokens()
        .clone()
}

#[derive(Clone, Debug)]
enum FallbackCatalog {
    Unconfigured,
    CoverageExempt,
    ConfigurationError {
        package: String,
        path: String,
        details: String,
    },
    Missing {
        package: String,
        fallback_root: String,
    },
    Unreadable {
        package: String,
        path: String,
        details: String,
    },
    Contents {
        package: String,
        fallback_root: String,
        contents: Vec<u8>,
    },
}

#[derive(Clone, Debug)]
pub struct FallbackValidation {
    policy: es_fluent_toml::MissingMessagePolicy,
    catalog: FallbackCatalog,
}

impl FallbackValidation {
    pub const fn unconfigured() -> Self {
        Self::unconfigured_with_policy(es_fluent_toml::MissingMessagePolicy::Strict)
    }

    const fn unconfigured_with_policy(policy: es_fluent_toml::MissingMessagePolicy) -> Self {
        Self {
            policy,
            catalog: FallbackCatalog::Unconfigured,
        }
    }

    pub const fn fallback_str_unconfigured() -> Self {
        Self::unconfigured_with_policy(es_fluent_toml::MissingMessagePolicy::FallbackStr)
    }

    pub const fn fallback_for_generated_key<'a>(&self, fallback: &'a str) -> Option<&'a str> {
        match self.policy {
            es_fluent_toml::MissingMessagePolicy::Strict => None,
            es_fluent_toml::MissingMessagePolicy::FallbackStr => Some(fallback),
        }
    }

    pub const fn is_setup_error(&self) -> bool {
        matches!(
            &self.catalog,
            FallbackCatalog::ConfigurationError { .. }
                | FallbackCatalog::Missing { .. }
                | FallbackCatalog::Unreadable { .. }
        )
    }

    pub fn diagnostic(
        &self,
        domain: Option<&FluentDomain>,
        id: &FluentMessageId,
        source_name: &str,
    ) -> Option<String> {
        match &self.catalog {
            FallbackCatalog::Unconfigured | FallbackCatalog::CoverageExempt => None,
            FallbackCatalog::ConfigurationError {
                package,
                path,
                details,
            } => Some(format!(
                "failed to read es-fluent configuration for package `{package}` at `{path}`: {details}"
            )),
            FallbackCatalog::Missing {
                package,
                fallback_root,
            } => Some(format!(
                "es-fluent fallback catalog is unavailable for configured package `{package}`\nexpected fallback resources under `{fallback_root}`\nhelp: add `es-fluent-build` under `[build-dependencies]`, call `es_fluent_build::track_i18n_assets()` from Cargo's custom build target, then run `cargo es-fluent doctor --package {package}`"
            )),
            FallbackCatalog::Unreadable {
                package,
                path,
                details,
            } => Some(format!(
                "failed to read the es-fluent fallback catalog for package `{package}` at `{path}`: {details}\nhelp: rerun the package build or run `cargo es-fluent doctor --package {package}`"
            )),
            FallbackCatalog::Contents {
                package,
                fallback_root,
                contents,
            } if self.policy == es_fluent_toml::MissingMessagePolicy::Strict => {
                let domain = domain.map_or(package.as_str(), FluentDomain::as_str);
                (!fallback_catalog_contains(contents, domain, id.as_str())).then(|| {
                    format!(
                        "missing fallback Fluent message `{}` in domain `{domain}` for Rust item `{source_name}`\nexpected a message value under `{fallback_root}`\nhelp: run `cargo es-fluent generate --package {package}` or add the fallback value manually",
                        id.as_str()
                    )
                })
            },
            FallbackCatalog::Contents { .. } => None,
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackValidationDerive {
    EsFluent,
    EsFluentLabel,
    EsFluentVariants,
    EsFluentChoice,
}

impl FallbackValidationDerive {
    const fn name(self) -> &'static str {
        match self {
            Self::EsFluent => "EsFluent",
            Self::EsFluentLabel => "EsFluentLabel",
            Self::EsFluentVariants => "EsFluentVariants",
            Self::EsFluentChoice => "EsFluentChoice",
        }
    }
}

pub fn fallback_validation(input: &syn::DeriveInput) -> FallbackValidation {
    fallback_validation_impl(input, None)
}

#[doc(hidden)]
pub fn fallback_validation_for_derive(
    input: &syn::DeriveInput,
    derive: FallbackValidationDerive,
) -> FallbackValidation {
    fallback_validation_impl(input, Some(derive))
}

fn fallback_validation_impl(
    input: &syn::DeriveInput,
    derive: Option<FallbackValidationDerive>,
) -> FallbackValidation {
    let Ok(package) = std::env::var("CARGO_PKG_NAME") else {
        return FallbackValidation::unconfigured();
    };
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let config_path = manifest_dir.join("i18n.toml");
    let layout = match es_fluent_toml::ResolvedI18nLayout::from_manifest_dir(&manifest_dir) {
        Ok(layout) => layout,
        Err(es_fluent_toml::I18nConfigError::NotFound) => {
            return FallbackValidation::unconfigured();
        },
        Err(error) => {
            return FallbackValidation {
                policy: es_fluent_toml::MissingMessagePolicy::Strict,
                catalog: FallbackCatalog::ConfigurationError {
                    package,
                    path: config_path.to_string_lossy().into_owned(),
                    details: error.to_string(),
                },
            };
        },
    };
    let policy = layout.missing_message_policy();
    if let Err(error) = layout.config.validate_for_package(&package) {
        return FallbackValidation {
            policy,
            catalog: FallbackCatalog::ConfigurationError {
                package,
                path: config_path.to_string_lossy().into_owned(),
                details: error.to_string(),
            },
        };
    }
    if derive_requires_test(input, derive)
        || std::env::var_os(INVENTORY_RUNNER_ENV).is_some()
        || std::env::var_os("UNSTABLE_RUSTDOC_TEST_PATH").is_some()
    {
        return FallbackValidation {
            policy,
            catalog: FallbackCatalog::CoverageExempt,
        };
    }

    let fallback_root = layout
        .output_dir
        .strip_prefix(&layout.manifest_dir)
        .unwrap_or(&layout.output_dir)
        .to_string_lossy()
        .into_owned();
    let Some(catalog_path) = std::env::var_os(FALLBACK_CATALOG_ENV).map(std::path::PathBuf::from)
    else {
        return FallbackValidation {
            policy,
            catalog: FallbackCatalog::Missing {
                package,
                fallback_root,
            },
        };
    };

    let catalog = match std::fs::read(&catalog_path) {
        Ok(contents) => FallbackCatalog::Contents {
            package,
            fallback_root,
            contents,
        },
        Err(error) => FallbackCatalog::Unreadable {
            package,
            path: catalog_path.to_string_lossy().into_owned(),
            details: error.to_string(),
        },
    };
    FallbackValidation { policy, catalog }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestDisabledCfg {
    True,
    False,
    Unknown,
}

fn attributes_require_test(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| attribute.parse_args::<syn::Meta>().ok())
        .any(|predicate| cfg_with_test_disabled(&predicate) == TestDisabledCfg::False)
}

fn attributes_enable_test_only_derive(
    attributes: &[syn::Attribute],
    derive: Option<FallbackValidationDerive>,
) -> bool {
    let Some(derive) = derive else {
        return false;
    };
    attributes.iter().any(|attribute| {
        let syn::Meta::List(list) = &attribute.meta else {
            return false;
        };
        if !list.path.is_ident("cfg_attr") {
            return false;
        }
        let Some(arguments) = cfg_predicates(list) else {
            return false;
        };
        let Some((predicate, applied_attributes)) = arguments.split_first() else {
            return false;
        };
        cfg_with_test_disabled(predicate) == TestDisabledCfg::False
            && applied_attributes
                .iter()
                .any(|attribute| meta_derives(attribute, derive))
    })
}

fn meta_derives(meta: &syn::Meta, derive: FallbackValidationDerive) -> bool {
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("derive") {
        return false;
    }

    use syn::parse::Parser as _;

    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .is_some_and(|paths| {
            paths.iter().any(|path| {
                path.segments
                    .last()
                    .is_some_and(|segment| segment.ident == derive.name())
            })
        })
}

fn derive_requires_test(
    input: &syn::DeriveInput,
    derive: Option<FallbackValidationDerive>,
) -> bool {
    if attributes_require_test(&input.attrs)
        || attributes_enable_test_only_derive(&input.attrs, derive)
    {
        return true;
    }

    // Rustc removes active `cfg` and `cfg_attr` attributes before invoking a
    // derive. Follow only Cargo target roots and module branches that can own
    // this source file, then match the declaration by its stable source
    // location. Unresolved and macro-generated evidence remains strict.
    let Some(source_path) = input.ident.span().local_file() else {
        return false;
    };
    let source_path = canonical_path(&source_path);
    let Ok(source) = std::fs::read_to_string(&source_path) else {
        return false;
    };
    let source_text = input.ident.span().source_text();
    let Some(range) = source_range(&source, input.ident.span().start(), source_text.as_deref())
    else {
        return false;
    };
    let Some((marked_source, marker_ident)) =
        mark_source_declaration(&source, range, source_text.as_deref())
    else {
        return false;
    };
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let target = SourceDeclaration {
        path: source_path.clone(),
        marked_source,
        marker_ident,
    };
    let mut evidence = Vec::new();
    let mut visited = HashSet::new();
    for root in cargo_source_roots(&manifest_dir) {
        let module_dir = root.path.parent().unwrap_or(Path::new(""));
        collect_source_evidence(
            &root.path,
            module_dir,
            root.test_only,
            &target,
            derive,
            &mut visited,
            &mut evidence,
        );
    }

    if evidence.is_empty() {
        let module_dir = source_path.parent().unwrap_or(Path::new(""));
        collect_source_evidence(
            &source_path,
            module_dir,
            false,
            &target,
            derive,
            &mut visited,
            &mut evidence,
        );
    }

    !evidence.is_empty() && evidence.into_iter().all(std::convert::identity)
}

#[derive(Debug)]
struct SourceDeclaration {
    path: PathBuf,
    marked_source: String,
    marker_ident: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SourceRoot {
    path: PathBuf,
    test_only: bool,
}

fn collect_source_evidence(
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
            && current_file == target.path
            && token_stream_contains_ident(&item_macro.mac.tokens, &target.marker_ident)
        {
            evidence.push(
                parent_requires_test
                    || attributes_require_test(&item_macro.attrs)
                    || attributes_enable_test_only_derive(&item_macro.attrs, derive),
            );
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

fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn mark_source_declaration(
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

fn source_range(
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

fn cargo_source_roots(manifest_dir: &Path) -> Vec<SourceRoot> {
    let manifest = std::fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .ok()
        .and_then(|source| toml::from_str::<toml::Value>(&source).ok());
    let package = manifest
        .as_ref()
        .and_then(|manifest| manifest.get("package"))
        .and_then(toml::Value::as_table);
    let mut roots = Vec::new();

    if let Some(library) = manifest
        .as_ref()
        .and_then(|manifest| manifest.get("lib"))
        .and_then(toml::Value::as_table)
    {
        add_source_root(
            &mut roots,
            manifest_dir,
            library.get("path").and_then(toml::Value::as_str),
            "src/lib.rs",
            false,
        );
    } else if package_bool(package, "autolib") {
        add_existing_root(&mut roots, manifest_dir.join("src/lib.rs"), false);
    }

    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "bin",
        "src/bin",
        false,
    );
    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "test",
        "tests",
        true,
    );
    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "example",
        "examples",
        false,
    );
    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "bench",
        "benches",
        false,
    );

    if package_bool(package, "autobins") {
        add_existing_root(&mut roots, manifest_dir.join("src/main.rs"), false);
        add_auto_target_roots(&mut roots, &manifest_dir.join("src/bin"), false);
    }
    if package_bool(package, "autotests") {
        add_auto_target_roots(&mut roots, &manifest_dir.join("tests"), true);
    }
    if package_bool(package, "autoexamples") {
        add_auto_target_roots(&mut roots, &manifest_dir.join("examples"), false);
    }
    if package_bool(package, "autobenches") {
        add_auto_target_roots(&mut roots, &manifest_dir.join("benches"), false);
    }

    let mut seen = HashSet::new();
    roots.retain(|root| seen.insert(root.clone()));
    roots
}

fn package_bool(package: Option<&toml::Table>, key: &str) -> bool {
    package
        .and_then(|package| package.get(key))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true)
}

fn add_declared_target_roots(
    roots: &mut Vec<SourceRoot>,
    manifest: Option<&toml::Value>,
    manifest_dir: &Path,
    table: &str,
    default_dir: &str,
    test_only: bool,
) {
    let Some(targets) = manifest
        .and_then(|manifest| manifest.get(table))
        .and_then(toml::Value::as_array)
    else {
        return;
    };
    for target in targets {
        let Some(target) = target.as_table() else {
            continue;
        };
        if let Some(path) = target.get("path").and_then(toml::Value::as_str) {
            add_existing_root(roots, manifest_dir.join(path), test_only);
            continue;
        }
        let Some(name) = target.get("name").and_then(toml::Value::as_str) else {
            continue;
        };
        add_existing_root(
            roots,
            manifest_dir.join(default_dir).join(format!("{name}.rs")),
            test_only,
        );
        add_existing_root(
            roots,
            manifest_dir.join(default_dir).join(name).join("main.rs"),
            test_only,
        );
        if table == "bin" {
            add_existing_root(roots, manifest_dir.join("src/main.rs"), test_only);
        }
    }
}

fn add_source_root(
    roots: &mut Vec<SourceRoot>,
    manifest_dir: &Path,
    configured_path: Option<&str>,
    default_path: &str,
    test_only: bool,
) {
    add_existing_root(
        roots,
        manifest_dir.join(configured_path.unwrap_or(default_path)),
        test_only,
    );
}

fn add_auto_target_roots(roots: &mut Vec<SourceRoot>, directory: &Path, test_only: bool) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            add_existing_root(roots, path, test_only);
        } else if path.is_dir() {
            add_existing_root(roots, path.join("main.rs"), test_only);
        }
    }
}

fn add_existing_root(roots: &mut Vec<SourceRoot>, path: PathBuf, test_only: bool) {
    if path.is_file() {
        roots.push(SourceRoot {
            path: canonical_path(&path),
            test_only,
        });
    }
}

fn cfg_with_test_disabled(predicate: &syn::Meta) -> TestDisabledCfg {
    match predicate {
        syn::Meta::Path(path) if path.is_ident("test") => TestDisabledCfg::False,
        syn::Meta::Path(_) | syn::Meta::NameValue(_) => TestDisabledCfg::Unknown,
        syn::Meta::List(list) if list.path.is_ident("all") => {
            let Some(predicates) = cfg_predicates(list) else {
                return TestDisabledCfg::Unknown;
            };
            if predicates
                .iter()
                .any(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::False)
            {
                TestDisabledCfg::False
            } else if predicates
                .iter()
                .all(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::True)
            {
                TestDisabledCfg::True
            } else {
                TestDisabledCfg::Unknown
            }
        },
        syn::Meta::List(list) if list.path.is_ident("any") => {
            let Some(predicates) = cfg_predicates(list) else {
                return TestDisabledCfg::Unknown;
            };
            if predicates
                .iter()
                .any(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::True)
            {
                TestDisabledCfg::True
            } else if predicates
                .iter()
                .all(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::False)
            {
                TestDisabledCfg::False
            } else {
                TestDisabledCfg::Unknown
            }
        },
        syn::Meta::List(list) if list.path.is_ident("not") => {
            let Some(predicates) = cfg_predicates(list) else {
                return TestDisabledCfg::Unknown;
            };
            let [predicate] = predicates.as_slice() else {
                return TestDisabledCfg::Unknown;
            };
            match cfg_with_test_disabled(predicate) {
                TestDisabledCfg::True => TestDisabledCfg::False,
                TestDisabledCfg::False => TestDisabledCfg::True,
                TestDisabledCfg::Unknown => TestDisabledCfg::Unknown,
            }
        },
        syn::Meta::List(_) => TestDisabledCfg::Unknown,
    }
}

fn cfg_predicates(list: &syn::MetaList) -> Option<Vec<syn::Meta>> {
    use syn::parse::Parser as _;

    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .map(IntoIterator::into_iter)
        .map(Iterator::collect)
}

pub fn static_domain_tokens(
    facade_path: &TokenStream,
    domain_override: Option<&FluentDomain>,
) -> TokenStream {
    match domain_override {
        Some(domain) => {
            let domain = domain.as_str();
            quote! { #facade_path::registry::__macro::static_domain(#domain) }
        },
        None => quote! {
            #facade_path::registry::StaticFluentDomain::from_package_name(env!("CARGO_PKG_NAME"))
        },
    }
}

pub fn static_entry_id_tokens(
    facade_path: &TokenStream,
    entry_id: &FluentMessageId,
) -> TokenStream {
    let entry_id = entry_id.as_str();
    quote! {
        #facade_path::registry::__macro::static_entry_id(#entry_id)
    }
}

pub fn static_message_key_tokens(
    facade_path: &TokenStream,
    domain_override: Option<&FluentDomain>,
    entry_id: &FluentMessageId,
    fallback: Option<&str>,
) -> TokenStream {
    let domain = static_domain_tokens(facade_path, domain_override);
    let entry_id = static_entry_id_tokens(facade_path, entry_id);
    match fallback {
        Some(fallback) => quote! {
            #facade_path::registry::__macro::static_message_key_with_fallback(
                env!("CARGO_PKG_NAME"),
                #domain,
                #entry_id,
                #fallback,
            )
        },
        None => quote! {
            #facade_path::registry::__macro::static_message_key(
                env!("CARGO_PKG_NAME"),
                #domain,
                #entry_id,
            )
        },
    }
}

pub fn static_argument_name_tokens(
    facade_path: &TokenStream,
    argument_name: &FluentArgumentName,
) -> TokenStream {
    let argument_name = argument_name.as_str();
    quote! {
        #facade_path::registry::__macro::static_argument_name(#argument_name)
    }
}

pub fn static_variant_key_tokens(
    facade_path: &TokenStream,
    variant_key: &FluentVariantKey,
) -> TokenStream {
    let variant_key = variant_key.as_str();
    quote! {
        #facade_path::registry::__macro::static_variant_key(#variant_key)
    }
}

pub fn core_error_to_compile_error(error: EsFluentCoreError) -> TokenStream {
    if let EsFluentCoreError::StructuredAttributeErrors(errors) = error {
        let errors = errors.into_iter().map(|error| {
            let message = error.to_string();
            match error.span {
                Some(span) => quote_spanned! { span=> compile_error!(#message); },
                None => quote! { compile_error!(#message); },
            }
        });
        return quote! { #(#errors)* };
    }

    let message = error.to_string();
    match error.span() {
        Some(span) => quote_spanned! { span=> compile_error!(#message); },
        None => quote! { compile_error!(#message); },
    }
}

#[cfg(test)]
#[serial_test::serial(manifest)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn rustdoc_synthetic_crate_bypasses_strict_fallback_coverage() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write config");
        let id = FluentMessageId::try_new("doctest_only").expect("valid message id");

        temp_env::with_vars(
            [
                ("CARGO_MANIFEST_DIR", Some(temp.path().as_os_str())),
                ("CARGO_PKG_NAME", Some(OsStr::new("test-package"))),
                (INVENTORY_RUNNER_ENV, None),
                (FALLBACK_CATALOG_ENV, None),
                ("UNSTABLE_RUSTDOC_TEST_PATH", Some(OsStr::new("doctest.rs"))),
            ],
            || {
                assert_eq!(
                    fallback_validation(&syn::parse_quote!(
                        struct DoctestOnly;
                    ))
                    .diagnostic(None, &id, "DoctestOnly"),
                    None
                );
            },
        );
    }

    #[test]
    fn cfg_predicates_only_exempt_items_that_require_test() {
        let test_only: syn::DeriveInput = syn::parse_quote! {
            #[cfg(all(unix, test))]
            struct TestOnly;
        };
        let maybe_test: syn::DeriveInput = syn::parse_quote! {
            #[cfg(any(unix, test))]
            struct MaybeTest;
        };
        let double_negative: syn::DeriveInput = syn::parse_quote! {
            #[cfg(not(not(test)))]
            struct DoubleNegative;
        };

        assert!(attributes_require_test(&test_only.attrs));
        assert!(!attributes_require_test(&maybe_test.attrs));
        assert!(attributes_require_test(&double_negative.attrs));
    }

    #[test]
    fn cfg_attr_exemption_is_specific_to_the_test_only_derive() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[cfg_attr(test, derive(es_fluent::EsFluent))]
            #[cfg_attr(any(test, feature = "demo"), derive(es_fluent::EsFluentLabel))]
            struct TestOnly;
        };

        assert!(attributes_enable_test_only_derive(
            &input.attrs,
            Some(FallbackValidationDerive::EsFluent)
        ));
        assert!(!attributes_enable_test_only_derive(
            &input.attrs,
            Some(FallbackValidationDerive::EsFluentLabel)
        ));
        assert!(!attributes_enable_test_only_derive(
            &input.attrs,
            Some(FallbackValidationDerive::EsFluentVariants)
        ));
    }
}
