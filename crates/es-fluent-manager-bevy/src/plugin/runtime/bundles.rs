use crate::{BundleBuildFailures, FtlAsset, I18nAssets, I18nBundle, I18nDomainBundles};
use bevy::asset::{AssetEvent, AssetId, AssetLoadFailedEvent};
use bevy::prelude::*;
use es_fluent_manager_core::{ResourceKey, SyncFluentBundle, locale_candidates};
use fluent_bundle::{FluentError, FluentResource};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

type DomainBundleMap = HashMap<String, Arc<SyncFluentBundle>>;
type DomainResourceMap = HashMap<String, Vec<Arc<FluentResource>>>;

fn dirty_asset_ids(
    asset_events: &mut MessageReader<AssetEvent<FtlAsset>>,
    asset_failed_events: &mut MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) -> Vec<AssetId<FtlAsset>> {
    let mut ids = asset_events
        .read()
        .map(|event| match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::Removed { id }
            | AssetEvent::Unused { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
        })
        .copied()
        .collect::<Vec<_>>();
    ids.extend(asset_failed_events.read().map(|event| event.id));
    ids
}

fn dirty_languages_for_assets(
    i18n_assets: &Res<I18nAssets>,
    dirty_asset_ids: Vec<AssetId<FtlAsset>>,
) -> HashSet<LanguageIdentifier> {
    let mut dirty_languages = dirty_asset_ids
        .into_iter()
        .filter_map(|id| {
            i18n_assets
                .assets
                .iter()
                .find(|(_, handle)| handle.id() == id)
                .map(|((lang, _), _)| lang.clone())
        })
        .collect::<HashSet<_>>();

    if i18n_assets.is_added() {
        for (lang, _) in i18n_assets.assets.keys() {
            dirty_languages.insert(lang.clone());
        }
    }

    dirty_languages
}

fn rebuild_bundle_for_language(
    i18n_bundle: &mut I18nBundle,
    i18n_domain_bundles: &mut I18nDomainBundles,
    bundle_build_failures: &mut BundleBuildFailures,
    i18n_assets: &I18nAssets,
    lang: &LanguageIdentifier,
) {
    let resources = i18n_assets.get_language_resource_entries(lang);
    if resources.is_empty() {
        i18n_bundle.remove(lang);
        i18n_domain_bundles.remove(lang);
        bundle_build_failures.0.remove(lang);
        debug!("Removed fluent resource cache for {}", lang);
        return;
    }

    match build_bundle_caches(lang, resources) {
        Ok((bundle, accepted_resources, domain_bundles, domain_locale_resources)) => {
            i18n_bundle.set_locale_resources(lang.clone(), accepted_resources);
            i18n_domain_bundles.set_locale_resources(lang.clone(), domain_locale_resources);
            bundle_build_failures.0.remove(lang);

            if i18n_assets.is_language_loaded(lang) {
                i18n_bundle.set_bundle(lang.clone(), bundle);
                i18n_domain_bundles.set_bundles(lang.clone(), domain_bundles);
                debug!("Updated fluent bundle cache for {}", lang);
            } else {
                i18n_bundle.remove_bundle(lang);
                i18n_domain_bundles.remove_bundles(lang);
                debug!(
                    "Stored partial fluent resource cache for {} while waiting on required resources",
                    lang
                );
            }
        },
        Err(diagnostics) => {
            error!(
                "Skipping fluent bundle cache replacement for {} because bundle assembly failed: {}",
                lang,
                diagnostics.join(" | ")
            );
            bundle_build_failures.0.insert(lang.clone(), diagnostics);
        },
    }
}

fn build_bundle_caches(
    lang: &LanguageIdentifier,
    resources: Vec<(ResourceKey, Arc<FluentResource>)>,
) -> Result<
    (
        Arc<SyncFluentBundle>,
        Vec<Arc<FluentResource>>,
        DomainBundleMap,
        DomainResourceMap,
    ),
    Vec<String>,
> {
    let (bundle, accepted_resources) = build_bundle_from_resources(lang, resources)?;
    let (domain_bundles, domain_locale_resources) =
        build_domain_bundles(lang, &accepted_resources)?;
    let locale_resources = accepted_resources
        .iter()
        .map(|(_, resource)| resource.clone())
        .collect::<Vec<_>>();
    Ok((
        bundle,
        locale_resources,
        domain_bundles,
        domain_locale_resources,
    ))
}

fn build_bundle_from_resources(
    lang: &LanguageIdentifier,
    resources: Vec<(ResourceKey, Arc<FluentResource>)>,
) -> Result<
    (
        Arc<SyncFluentBundle>,
        Vec<(ResourceKey, Arc<FluentResource>)>,
    ),
    Vec<String>,
> {
    let mut bundle = SyncFluentBundle::new_concurrent(locale_candidates(lang));
    let mut accepted_resources = Vec::with_capacity(resources.len());
    let mut diagnostics = Vec::new();

    for (resource_key, resource) in resources {
        match bundle.add_resource(resource.clone()) {
            Ok(()) => accepted_resources.push((resource_key, resource)),
            Err(errors) => diagnostics.push(format_add_errors(&resource_key, errors)),
        }
    }

    if diagnostics.is_empty() {
        Ok((Arc::new(bundle), accepted_resources))
    } else {
        Err(diagnostics)
    }
}

fn build_domain_bundles(
    lang: &LanguageIdentifier,
    accepted_resources: &[(ResourceKey, Arc<FluentResource>)],
) -> Result<(DomainBundleMap, DomainResourceMap), Vec<String>> {
    let mut grouped = HashMap::<String, Vec<(ResourceKey, Arc<FluentResource>)>>::new();
    for (resource_key, resource) in accepted_resources.iter().cloned() {
        grouped
            .entry(resource_key.domain().to_string())
            .or_default()
            .push((resource_key, resource));
    }

    let mut domain_bundles = HashMap::with_capacity(grouped.len());
    let mut domain_locale_resources = HashMap::with_capacity(grouped.len());
    for (domain, mut resources) in grouped {
        resources.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
        let (bundle, accepted_resources) =
            build_bundle_from_resources(lang, resources).map_err(|diagnostics| {
                diagnostics
                    .into_iter()
                    .map(|diagnostic| format!("domain '{}': {}", domain, diagnostic))
                    .collect::<Vec<_>>()
            })?;
        domain_bundles.insert(domain.clone(), bundle);
        domain_locale_resources.insert(
            domain,
            accepted_resources
                .into_iter()
                .map(|(_, resource)| resource)
                .collect(),
        );
    }

    Ok((domain_bundles, domain_locale_resources))
}

fn format_add_errors(resource_key: &ResourceKey, errors: Vec<FluentError>) -> String {
    let messages = errors
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join("; ");
    format!("resource '{}': {}", resource_key, messages)
}

#[doc(hidden)]
pub(crate) fn build_fluent_bundles(
    mut i18n_bundle: ResMut<I18nBundle>,
    mut i18n_domain_bundles: ResMut<I18nDomainBundles>,
    mut bundle_build_failures: ResMut<BundleBuildFailures>,
    i18n_assets: Res<I18nAssets>,
    mut asset_events: MessageReader<AssetEvent<FtlAsset>>,
    mut asset_failed_events: MessageReader<AssetLoadFailedEvent<FtlAsset>>,
) {
    let dirty_asset_ids = dirty_asset_ids(&mut asset_events, &mut asset_failed_events);
    let dirty_languages = dirty_languages_for_assets(&i18n_assets, dirty_asset_ids);

    for lang in dirty_languages {
        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );
    }
}
