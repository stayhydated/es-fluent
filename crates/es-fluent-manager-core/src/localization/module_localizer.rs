//! Lookup over Fluent bundles built once for a selected locale.

use super::{
    FluentArgumentMap, LocalizationError, Localizer, SyncFluentBundle, build_sync_bundle,
    localize_with_bundles,
};
use crate::asset_localization::{LocaleLoadReport, ModuleData, ResourceKey};
use es_fluent_shared::{fluent::FluentDomain, registry::StaticFluentMessageKey};
use fluent_bundle::{FluentError, FluentResource};
use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::Arc,
};
use unic_langid::LanguageIdentifier;

/// Loaded resources for one locale, keyed by resource.
pub type LocaleResources = Vec<(ResourceKey, Arc<FluentResource>)>;

/// A module's Fluent bundles for one selected locale.
///
/// Creation walks the requested locale's fallback chain, loads each supported
/// candidate, and builds one bundle per domain and candidate. Lookups then try
/// those bundles in order, so a message missing from the requested locale
/// resolves from the nearest fallback that defines it.
pub struct ModuleLocalizer {
    data: &'static ModuleData,
    /// Bundles per domain, ordered from the requested locale to its last
    /// loadable fallback.
    bundles: HashMap<FluentDomain, Box<[SyncFluentBundle]>>,
}

impl ModuleLocalizer {
    /// Builds the bundles for `lang`, loading candidate locales with `load`.
    ///
    /// `load` returns the resources of one supported locale, or an error when
    /// that locale is not ready; unready candidates are skipped. Bundle
    /// assembly errors in the first loaded candidate fail creation. Errors in
    /// later fallback candidates skip that candidate's domain.
    pub fn new(
        data: &'static ModuleData,
        lang: &LanguageIdentifier,
        mut load: impl FnMut(&LanguageIdentifier) -> Result<LocaleResources, LocalizationError>,
    ) -> Result<Self, LocalizationError> {
        let mut chains = HashMap::<FluentDomain, Vec<SyncFluentBundle>>::new();
        let mut primary = true;
        let candidates = crate::fallback::locale_candidates(lang)
            .into_iter()
            .filter(|candidate| data.supported_languages.contains(candidate));

        for candidate in candidates {
            let Ok(resources) = load(&candidate) else {
                continue;
            };

            let mut resources_by_domain = BTreeMap::<FluentDomain, Vec<_>>::new();
            for (key, resource) in resources {
                resources_by_domain
                    .entry(key.domain_name())
                    .or_default()
                    .push(resource);
            }

            for (domain, resources) in resources_by_domain {
                let (bundle, add_errors) = build_sync_bundle(&candidate, resources);
                if !add_errors.is_empty() {
                    if primary {
                        let error =
                            BundleBuildError::from_add_errors(data.name, lang, &domain, add_errors);
                        tracing::error!("{error}");
                        return Err(io::Error::other(error).into());
                    }

                    tracing::warn!(
                        "Skipping fallback locale '{}' for requested locale '{}' in module '{}' domain '{}' because Fluent bundle assembly failed",
                        candidate,
                        lang,
                        data.name,
                        domain,
                    );
                    continue;
                }

                chains.entry(domain).or_default().push(bundle);
            }
            primary = false;
        }

        if chains.is_empty() {
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        }

        Ok(Self {
            data,
            bundles: chains
                .into_iter()
                .map(|(domain, bundles)| (domain, bundles.into_boxed_slice()))
                .collect(),
        })
    }

    /// Returns the bundles for `domain`, in fallback order.
    pub fn bundles(&self, domain: &str) -> Option<&[SyncFluentBundle]> {
        self.bundles.get(domain).map(AsRef::as_ref)
    }
}

impl Localizer for ModuleLocalizer {
    /// Returns `None` for keys owned by another module, messages missing from
    /// every selected locale, and messages that fail to format.
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String> {
        if key.owner() != self.data.owner || !self.data.owns_domain(key.domain()) {
            return None;
        }

        let bundles = self.bundles(key.domain().as_str())?;
        let (value, errors) = localize_with_bundles(bundles, key.id(), args)?;
        if !errors.is_empty() {
            tracing::error!(
                "Fluent formatting errors for id '{}' in module '{}': {:?}",
                key.id().as_str(),
                self.data.name,
                errors
            );
            return None;
        }

        Some(value)
    }
}

/// Logs why a locale's resources are not ready for a module.
#[doc(hidden)]
pub fn log_unready_locale(data: &ModuleData, lang: &LanguageIdentifier, report: &LocaleLoadReport) {
    let mut missing_required = report
        .missing_required_keys()
        .into_iter()
        .map(|key| key.to_string())
        .collect::<Vec<_>>();
    missing_required.sort();
    tracing::debug!(
        "Locale '{}' is not ready for module '{}': missing_required={:?}, errors={:?}",
        lang,
        data.name,
        missing_required,
        report.errors()
    );
}

/// Fluent bundle assembly failure for a module's requested locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleBuildError {
    module_name: String,
    language: LanguageIdentifier,
    domain: String,
    diagnostics: Vec<String>,
}

impl BundleBuildError {
    fn from_add_errors(
        module_name: &str,
        language: &LanguageIdentifier,
        domain: &FluentDomain,
        add_errors: Vec<Vec<FluentError>>,
    ) -> Self {
        let diagnostics = add_errors
            .into_iter()
            .enumerate()
            .map(|(resource_index, errors)| {
                let messages = errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("resource #{resource_index}: {messages}")
            })
            .collect();

        Self {
            module_name: module_name.to_string(),
            language: language.clone(),
            domain: domain.as_str().to_string(),
            diagnostics,
        }
    }

    /// Returns the module whose bundle failed.
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    /// Returns the requested locale.
    pub fn language(&self) -> &LanguageIdentifier {
        &self.language
    }

    /// Returns the domain whose bundle failed.
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Returns one diagnostic per failing resource.
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

impl std::fmt::Display for BundleBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to build a Fluent bundle for module '{}' domain '{}' and language '{}': {}",
            self.module_name,
            self.domain,
            self.language,
            self.diagnostics.join(" | ")
        )
    }
}

impl std::error::Error for BundleBuildError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_localization::ModuleDomain;
    use unic_langid::langid;

    static LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("fr"), langid!("fr-CA")];
    static DATA: ModuleData = ModuleData {
        name: "app",
        owner: crate::__macro::static_domain("app"),
        supported_languages: LANGUAGES,
        domains: &[
            ModuleDomain {
                domain: crate::__macro::static_domain("aaa"),
                namespaces: &[],
            },
            ModuleDomain {
                domain: crate::__macro::static_domain("zzz"),
                namespaces: &[],
            },
        ],
    };

    fn key(domain: &'static str, id: &'static str) -> StaticFluentMessageKey {
        crate::__macro::static_message_key(
            "app",
            crate::__macro::static_domain(domain),
            crate::__macro::static_entry_id(id),
        )
    }

    fn resources(entries: &[(&'static str, &str)]) -> LocaleResources {
        entries
            .iter()
            .map(|(key, source)| {
                (
                    ResourceKey::from_static_path(key),
                    Arc::new(FluentResource::try_new((*source).to_string()).expect("valid FTL")),
                )
            })
            .collect()
    }

    const DUPLICATE: &str = "hello = One\nhello = Two";

    #[test]
    fn bundle_errors_in_any_domain_of_the_selected_locale_fail_selection() {
        for broken_domain in ["aaa", "zzz"] {
            let error = ModuleLocalizer::new(&DATA, &langid!("en"), |_| {
                Ok(["aaa", "zzz"]
                    .into_iter()
                    .flat_map(|domain| {
                        let source = if domain == broken_domain {
                            DUPLICATE
                        } else {
                            "hello = Hello"
                        };
                        resources(&[(domain, source)])
                    })
                    .collect())
            })
            .err()
            .expect("a broken bundle in the selected locale should fail selection");

            let LocalizationError::IoError(error) = error else {
                panic!("expected a bundle build error for {broken_domain}");
            };
            let error = error
                .get_ref()
                .and_then(|error| error.downcast_ref::<BundleBuildError>())
                .expect("bundle build error");
            assert_eq!(error.domain(), broken_domain);
        }
    }

    #[test]
    fn broken_fallback_domains_are_skipped_and_messages_fall_back_per_domain() {
        let localizer = ModuleLocalizer::new(&DATA, &langid!("fr-CA"), |candidate| {
            Ok(match candidate.to_string().as_str() {
                "fr-CA" => resources(&[("aaa", "hello = Allo")]),
                "fr" => resources(&[("aaa", DUPLICATE), ("zzz", "bye = Salut")]),
                _ => unreachable!("en is not in the fr-CA fallback chain"),
            })
        })
        .expect("fr-CA should be selected");

        assert_eq!(
            localizer.localize(key("aaa", "hello"), None),
            Some("Allo".into())
        );
        assert_eq!(
            localizer.localize(key("zzz", "bye"), None),
            Some("Salut".into())
        );
        assert_eq!(localizer.bundles("aaa").map(<[_]>::len), Some(1));
    }

    #[test]
    fn unready_candidates_are_skipped() {
        let localizer = ModuleLocalizer::new(&DATA, &langid!("fr-CA"), |candidate| {
            if candidate == &langid!("fr-CA") {
                Err(LocalizationError::LanguageNotSupported(candidate.clone()))
            } else {
                Ok(resources(&[("aaa", "hello = Bonjour")]))
            }
        })
        .expect("fr should stand in for fr-CA");
        assert_eq!(
            localizer.localize(key("aaa", "hello"), None),
            Some("Bonjour".into())
        );

        let error = ModuleLocalizer::new(&DATA, &langid!("en"), |candidate| {
            Err(LocalizationError::LanguageNotSupported(candidate.clone()))
        })
        .err()
        .expect("no loadable candidate");
        assert!(matches!(error, LocalizationError::LanguageNotSupported(_)));
    }

    #[test]
    fn keys_of_other_modules_are_not_localized() {
        let localizer = ModuleLocalizer::new(&DATA, &langid!("en"), |_| {
            Ok(resources(&[("aaa", "hello = Hi")]))
        })
        .expect("en should be selected");

        let foreign = crate::__macro::static_message_key(
            "other",
            crate::__macro::static_domain("aaa"),
            crate::__macro::static_entry_id("hello"),
        );
        assert_eq!(localizer.localize(foreign, None), None);
        assert_eq!(localizer.localize(key("bbb", "hello"), None), None);
    }
}
