use crate::asset_localization::{ModuleDomain, ModuleResourceSpec, ResourcePlan};
use rust_embed::RustEmbed;
use std::collections::BTreeSet;
use unic_langid::LanguageIdentifier;

pub trait EmbeddedAssets: RustEmbed + Send + Sync + 'static {
    /// Returns every package-local domain and its canonical namespace list.
    fn domains() -> &'static [ModuleDomain];

    /// Returns the exact resource plan for a locale when the embedded asset tree
    /// can prove that only part of the module's global namespace set exists for
    /// that locale.
    fn resource_plan_for_language(lang: &LanguageIdentifier) -> Option<Vec<ModuleResourceSpec>> {
        let mut specs = Vec::new();
        for configured_domain in Self::domains() {
            let mut has_base_file = false;
            let mut found_namespaces = BTreeSet::new();

            for file_path in Self::iter() {
                let file_path_str = file_path.as_ref();
                let Some((file_lang, namespace)) = embedded_resource_from_asset_path(
                    file_path_str,
                    configured_domain.domain.as_str(),
                    configured_domain.namespaces,
                ) else {
                    continue;
                };
                if &file_lang != lang {
                    continue;
                }

                match namespace {
                    Some(namespace) => {
                        found_namespaces.insert(namespace);
                    },
                    None => {
                        has_base_file = true;
                    },
                }
            }

            if !has_base_file && found_namespaces.is_empty() {
                continue;
            }
            let resolved_namespaces = found_namespaces
                .into_iter()
                .map(|namespace| {
                    es_fluent_shared::namespace::ResolvedNamespace::new(namespace)
                        .expect("embedded namespace was prevalidated from module metadata")
                })
                .collect::<Vec<_>>();
            specs.extend(
                ResourcePlan::sparse_for_static_domain(
                    configured_domain.domain,
                    has_base_file,
                    &resolved_namespaces,
                    false,
                )
                .into_specs(),
            );
        }

        (!specs.is_empty()).then_some(specs)
    }
}

pub(super) fn embedded_resource_from_asset_path(
    file_path: &str,
    domain: &str,
    namespaces: &[&str],
) -> Option<(LanguageIdentifier, Option<String>)> {
    let mut segments = file_path.split('/');
    let language = segments.next()?;
    let next = segments.next()?;

    if next == format!("{domain}.ftl") && segments.next().is_none() {
        return parse_embedded_language_identifier(language).map(|lang| (lang, None));
    }

    if next != domain {
        return None;
    }

    let namespace_path = segments.collect::<Vec<_>>().join("/");
    let namespace = namespace_path.strip_suffix(".ftl")?;
    if namespace.is_empty() {
        return None;
    }

    namespaces
        .iter()
        .any(|configured| configured == &namespace)
        .then(|| {
            parse_embedded_language_identifier(language)
                .map(|lang| (lang, Some(namespace.to_string())))
        })
        .flatten()
}

pub(super) fn parse_embedded_language_identifier(raw: &str) -> Option<LanguageIdentifier> {
    es_fluent_shared::parse_canonical_language_identifier(raw).ok()
}
