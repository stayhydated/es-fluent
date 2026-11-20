//! Build manifest written by [`crate::track_i18n_assets`] and read by es-fluent
//! macros.
//!
//! Macros expand from the manifest instead of reading `i18n.toml` and locale
//! directories themselves. Cargo reruns the build script when those inputs
//! change, and macros reference the manifest's content hash through `env!`, so
//! rustc and compiler caches track exactly the data the macros consumed.

use es_fluent_shared::LanguageIdentifier;
use es_fluent_shared::fluent::FluentDomain;
use es_fluent_shared::namespace::ResolvedNamespace;
use es_fluent_shared::resource::{FallbackCatalogIndex, ModuleResourceSpec, ResourcePlan};
use es_fluent_toml::{I18nConfig, I18nConfigError, ResolvedI18nLayout};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Manifest format written by this version of `es-fluent-build`.
///
/// Increment it whenever the serialized shape changes so macros reject a
/// manifest written by a mismatched `es-fluent-build` version.
const FORMAT: u32 = 1;

/// The i18n inputs es-fluent macros read for one package.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct BuildManifest {
    format: u32,
    /// `i18n.toml` contents.
    config: String,
    /// Locale directories under the assets directory, or the discovery error.
    languages: Result<Vec<String>, String>,
    /// Resources discovered for every package-owned domain, or the discovery error.
    resources: Result<SerializedResources, String>,
    /// Resolvable fallback messages by domain. `None` when catalog validation
    /// is deferred, as in CLI inventory builds.
    fallback_messages: Option<BTreeMap<String, BTreeSet<String>>>,
}

impl BuildManifest {
    /// Collects the manifest for `package` from its resolved layout.
    ///
    /// `with_fallback_catalog` controls whether fallback resources are parsed
    /// into the message catalog.
    pub fn collect(
        layout: &ResolvedI18nLayout,
        package: &str,
        with_fallback_catalog: bool,
    ) -> Result<Self, String> {
        layout
            .config
            .validate_for_package(package)
            .map_err(|error| error.to_string())?;
        let config = std::fs::read_to_string(&layout.config_path)
            .map_err(|error| format!("failed to read {}: {error}", layout.config_path.display()))?;

        let fallback_messages = if with_fallback_catalog {
            let catalog = crate::catalog::collect_fallback_catalog(layout, package)?;
            let mut messages = BTreeMap::<String, BTreeSet<String>>::new();
            for (domain, id) in catalog.resolvable_messages() {
                messages
                    .entry(domain.as_str().to_string())
                    .or_default()
                    .insert(id.as_str().to_string());
            }
            Some(messages)
        } else {
            None
        };

        Ok(Self {
            format: FORMAT,
            config,
            languages: discover_languages(&layout.config, &layout.manifest_dir)
                .map(|languages| languages.iter().map(ToString::to_string).collect()),
            resources: discover_resources(&layout.config, &layout.manifest_dir, package)
                .map(|resources| SerializedResources::from(&resources)),
            fallback_messages,
        })
    }

    /// Serializes the manifest.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("build manifest serialization cannot fail")
    }

    /// Parses a manifest, rejecting one written in another format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct Header {
            format: u32,
        }

        let header: Header = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid es-fluent build manifest: {error}"))?;
        if header.format != FORMAT {
            return Err(format!(
                "es-fluent build manifest format {} does not match the expected format {FORMAT}; use matching es-fluent and es-fluent-build versions",
                header.format
            ));
        }
        serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid es-fluent build manifest: {error}"))
    }

    /// Returns the package configuration.
    pub fn config(&self) -> Result<I18nConfig, I18nConfigError> {
        I18nConfig::from_toml_str(&self.config)
    }

    /// Returns the locales discovered under the assets directory.
    pub fn languages(&self) -> Result<Vec<LanguageIdentifier>, String> {
        let languages = self.languages.as_ref().map_err(Clone::clone)?;
        languages
            .iter()
            .map(|language| {
                language
                    .parse()
                    .map_err(|error| format!("invalid language '{language}': {error}"))
            })
            .collect()
    }

    /// Returns the resources discovered for every package-owned domain.
    pub fn resources(&self) -> Result<PackageResources, String> {
        self.resources
            .as_ref()
            .map_err(Clone::clone)?
            .clone()
            .try_into()
    }

    /// Returns the fallback-message index, or `None` when catalog validation
    /// was deferred.
    pub fn fallback_messages(&self) -> Option<FallbackCatalogIndex> {
        let messages = self.fallback_messages.as_ref()?;
        Some(
            messages
                .iter()
                .flat_map(|(domain, ids)| ids.iter().map(move |id| (domain.as_str(), id.as_str())))
                .collect(),
        )
    }
}

/// Resources discovered for every domain owned by a package.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageResources {
    /// Languages with at least one resource or locale directory, sorted.
    pub languages: Vec<LanguageIdentifier>,
    /// Package-owned domains with their discovered namespaces. The package
    /// domain comes first.
    pub domains: Vec<(FluentDomain, Vec<ResolvedNamespace>)>,
    /// Resources by language, sorted by language and locale-relative path.
    pub resource_specs_by_language: Vec<(LanguageIdentifier, Vec<ModuleResourceSpec>)>,
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
struct SerializedResources {
    languages: Vec<String>,
    domains: Vec<SerializedDomain>,
    resources_by_language: Vec<SerializedLanguageResources>,
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
struct SerializedDomain {
    domain: FluentDomain,
    namespaces: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
struct SerializedLanguageResources {
    language: String,
    resources: Vec<ModuleResourceSpec>,
}

impl From<&PackageResources> for SerializedResources {
    fn from(resources: &PackageResources) -> Self {
        Self {
            languages: resources
                .languages
                .iter()
                .map(ToString::to_string)
                .collect(),
            domains: resources
                .domains
                .iter()
                .map(|(domain, namespaces)| SerializedDomain {
                    domain: domain.clone(),
                    namespaces: namespaces
                        .iter()
                        .map(|namespace| namespace.as_str().to_string())
                        .collect(),
                })
                .collect(),
            resources_by_language: resources
                .resource_specs_by_language
                .iter()
                .map(|(language, resources)| SerializedLanguageResources {
                    language: language.to_string(),
                    resources: resources.clone(),
                })
                .collect(),
        }
    }
}

impl TryFrom<SerializedResources> for PackageResources {
    type Error = String;

    fn try_from(resources: SerializedResources) -> Result<Self, Self::Error> {
        let language = |language: &str| -> Result<LanguageIdentifier, String> {
            language
                .parse()
                .map_err(|error| format!("invalid language '{language}': {error}"))
        };

        Ok(Self {
            languages: resources
                .languages
                .iter()
                .map(|value| language(value))
                .collect::<Result<_, _>>()?,
            domains: resources
                .domains
                .into_iter()
                .map(|domain| {
                    let namespaces = domain
                        .namespaces
                        .into_iter()
                        .map(|namespace| {
                            ResolvedNamespace::new(namespace.clone()).map_err(|error| {
                                format!("invalid namespace '{namespace}': {error}")
                            })
                        })
                        .collect::<Result<_, _>>()?;
                    Ok((domain.domain, namespaces))
                })
                .collect::<Result<_, String>>()?,
            resource_specs_by_language: resources
                .resources_by_language
                .into_iter()
                .map(|entry| Ok((language(&entry.language)?, entry.resources)))
                .collect::<Result<_, String>>()?,
        })
    }
}

/// Discovers the locale directories under the configured assets directory.
///
/// Build scripts and macros without a build manifest share this discovery.
#[doc(hidden)]
pub fn discover_languages(
    config: &I18nConfig,
    manifest_dir: &Path,
) -> Result<Vec<LanguageIdentifier>, String> {
    config
        .available_languages_from_base(Some(manifest_dir))
        .map_err(|error| error.to_string())
}

/// Discovers resources for the package domain and every configured domain.
///
/// Build scripts and macros without a build manifest share this discovery.
#[doc(hidden)]
pub fn discover_resources(
    config: &I18nConfig,
    manifest_dir: &Path,
    package: &str,
) -> Result<PackageResources, String> {
    let assets_dir = config
        .assets_dir_from_base(Some(manifest_dir))
        .map_err(|error| {
            format!("Failed to resolve assets directory from configuration: {error}")
        })?;
    config
        .validated_assets_dir_from_base(Some(manifest_dir))
        .map_err(|error| format!("Assets directory validation failed: {error}"))?;

    let package_domain = FluentDomain::try_new(package.to_string()).map_err(|error| {
        format!("CARGO_PKG_NAME '{package}' is not a valid Fluent domain: {error}")
    })?;
    let mut languages = BTreeSet::new();
    let mut resource_specs_by_language = BTreeMap::<LanguageIdentifier, Vec<_>>::new();
    let mut domains = Vec::new();

    for domain in std::iter::once(&package_domain).chain(&config.domains) {
        let (domain_languages, namespaces, domain_specs) =
            ResourcePlan::sparse_from_assets(domain.as_str(), &assets_dir)
                .map_err(|error| error.to_string())?
                .into_parts();
        languages.extend(domain_languages);
        for (language, specs) in domain_specs {
            resource_specs_by_language
                .entry(language)
                .or_default()
                .extend(specs);
        }
        domains.push((domain.clone(), namespaces));
    }

    for specs in resource_specs_by_language.values_mut() {
        specs.sort_by(|left, right| left.locale_relative_path.cmp(&right.locale_relative_path));
    }

    Ok(PackageResources {
        languages: languages.into_iter().collect(),
        domains,
        resource_specs_by_language: resource_specs_by_language.into_iter().collect(),
    })
}
