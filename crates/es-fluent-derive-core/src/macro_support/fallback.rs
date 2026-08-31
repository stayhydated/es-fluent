use super::*;

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
    pub(super) const fn name(self) -> &'static str {
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
