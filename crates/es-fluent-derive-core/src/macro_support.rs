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

pub fn fallback_validation(input: &syn::DeriveInput) -> FallbackValidation {
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
    if derive_requires_test(input)
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

fn derive_requires_test(input: &syn::DeriveInput) -> bool {
    if attributes_require_test(&input.attrs) {
        return true;
    }

    // Rustc evaluates and removes an active `cfg` attribute before invoking a
    // derive. Reparse the real source file to recover item and inline-module
    // guards, and keep ambiguous or unavailable source evidence strict.
    let Some(source_path) = input.ident.span().local_file() else {
        return false;
    };
    let Ok(source) = std::fs::read_to_string(source_path) else {
        return false;
    };
    let Ok(file) = syn::parse_file(&source) else {
        return false;
    };
    source_items_require_test(&file.items, &input.ident.to_string(), false)
}

fn source_items_require_test(
    items: &[syn::Item],
    target_ident: &str,
    parent_requires_test: bool,
) -> bool {
    let mut matching_items = Vec::new();
    collect_matching_item_cfgs(
        items,
        target_ident,
        parent_requires_test,
        &mut matching_items,
    );
    !matching_items.is_empty() && matching_items.into_iter().all(std::convert::identity)
}

fn collect_matching_item_cfgs(
    items: &[syn::Item],
    target_ident: &str,
    parent_requires_test: bool,
    matching_items: &mut Vec<bool>,
) {
    for item in items {
        match item {
            syn::Item::Enum(item) if item.ident == target_ident => {
                matching_items.push(parent_requires_test || attributes_require_test(&item.attrs))
            },
            syn::Item::Struct(item) if item.ident == target_ident => {
                matching_items.push(parent_requires_test || attributes_require_test(&item.attrs))
            },
            syn::Item::Union(item) if item.ident == target_ident => {
                matching_items.push(parent_requires_test || attributes_require_test(&item.attrs))
            },
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_matching_item_cfgs(
                        items,
                        target_ident,
                        parent_requires_test || attributes_require_test(&module.attrs),
                        matching_items,
                    );
                }
            },
            _ => {},
        }
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
    fn source_item_cfgs_include_inline_module_ancestors() {
        let file = syn::parse_file(
            r#"
                #[cfg(test)]
                mod tests {
                    struct NestedTestOnly;
                }

                #[cfg(any(test, feature = "demo"))]
                struct MaybeTest;

                #[cfg(test)]
                struct ReusedName;
                struct ReusedName;
            "#,
        )
        .expect("parse source");

        assert!(source_items_require_test(
            &file.items,
            "NestedTestOnly",
            false
        ));
        assert!(!source_items_require_test(&file.items, "MaybeTest", false));
        assert!(!source_items_require_test(&file.items, "ReusedName", false));
    }
}
