use crate::{BundleBuildFailures, FtlAsset, I18nAssets, I18nBundle, I18nDomainBundles};
use bevy::asset::{AssetEvent, AssetId, AssetLoadFailedEvent};
use bevy::prelude::*;
use es_fluent_manager_core::{ResourceKey, SyncFluentBundle};
use fluent_bundle::{FluentError, FluentResource};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

type DomainBundleMap = HashMap<String, Arc<SyncFluentBundle>>;
type DomainResourceMap = HashMap<String, Vec<Arc<FluentResource>>>;

struct BundleCaches {
    bundle: Option<Arc<SyncFluentBundle>>,
    locale_resources: Option<Vec<Arc<FluentResource>>>,
    domain_bundles: DomainBundleMap,
    domain_locale_resources: DomainResourceMap,
    unscoped_diagnostics: Vec<String>,
}

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
        i18n_domain_bundles.remove(lang);
        bundle_build_failures.0.remove(lang);

        if i18n_assets
            .assets
            .keys()
            .any(|(language, _)| language == lang)
            && i18n_assets.is_language_loaded(lang)
        {
            i18n_bundle.mark_ready_without_unscoped_bundle(lang.clone());
            i18n_domain_bundles.set_locale_resources(lang.clone(), HashMap::new());
            i18n_domain_bundles.set_bundles(lang.clone(), HashMap::new());
            debug!("Marked empty ready fluent resource cache for {}", lang);
        } else {
            i18n_bundle.remove(lang);
            debug!("Removed fluent resource cache for {}", lang);
        }

        return;
    }

    match build_bundle_caches(lang, resources) {
        Ok(caches) => {
            let BundleCaches {
                bundle,
                locale_resources,
                domain_bundles,
                domain_locale_resources,
                unscoped_diagnostics,
            } = caches;

            if !unscoped_diagnostics.is_empty() {
                warn!(
                    "Unscoped Fluent lookup for {} is unavailable or ambiguous because the merged all-domain bundle could not be assembled: {}. Domain-scoped generated lookup remains available.",
                    lang,
                    unscoped_diagnostics.join(" | ")
                );
            }

            if let Some(locale_resources) = locale_resources {
                i18n_bundle.set_locale_resources(lang.clone(), locale_resources);
            } else {
                i18n_bundle.remove(lang);
            }
            i18n_domain_bundles.set_locale_resources(lang.clone(), domain_locale_resources);
            bundle_build_failures.0.remove(lang);

            if i18n_assets.is_language_loaded(lang) {
                if let Some(bundle) = bundle {
                    i18n_bundle.set_bundle(lang.clone(), bundle);
                } else {
                    i18n_bundle.mark_ready_without_unscoped_bundle(lang.clone());
                }
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
) -> Result<BundleCaches, Vec<String>> {
    let (domain_bundles, domain_locale_resources) = build_domain_bundles(lang, &resources)?;
    let (bundle, locale_resources, unscoped_diagnostics) =
        match build_bundle_from_resources(lang, resources) {
            Ok((bundle, accepted_resources)) => {
                let locale_resources = accepted_resources
                    .into_iter()
                    .map(|(_, resource)| resource)
                    .collect::<Vec<_>>();
                (Some(bundle), Some(locale_resources), Vec::new())
            },
            Err(diagnostics) => (None, None, diagnostics),
        };

    Ok(BundleCaches {
        bundle,
        locale_resources,
        domain_bundles,
        domain_locale_resources,
        unscoped_diagnostics,
    })
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
    let mut bundle =
        SyncFluentBundle::new_concurrent(es_fluent_manager_core::locale_candidates(lang));
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::Assets;
    use es_fluent_manager_core::ModuleResourceSpec;
    use unic_langid::langid;

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("valid FTL"))
    }

    fn spec(key: &str, required: bool) -> ModuleResourceSpec {
        ModuleResourceSpec {
            key: ResourceKey::new(key),
            locale_relative_path: format!("{key}.ftl"),
            required,
        }
    }

    fn empty_bundle(lang: &LanguageIdentifier) -> Arc<SyncFluentBundle> {
        Arc::new(SyncFluentBundle::new_concurrent(
            es_fluent_manager_core::locale_candidates(lang),
        ))
    }

    #[test]
    fn build_bundle_caches_creates_default_and_domain_scoped_bundles() {
        let lang = langid!("en");
        let caches = build_bundle_caches(
            &lang,
            vec![
                (ResourceKey::new("app"), resource("app-title = App")),
                (ResourceKey::new("admin"), resource("admin-title = Admin")),
            ],
        )
        .expect("valid resources should build caches");

        let bundle = caches.bundle.expect("unscoped bundle");
        assert_eq!(
            caches.locale_resources.expect("unscoped resources").len(),
            2
        );
        assert!(bundle.get_message("app-title").is_some());
        assert!(bundle.get_message("admin-title").is_some());
        assert!(
            caches.domain_bundles["app"]
                .get_message("app-title")
                .is_some()
        );
        assert!(
            caches.domain_bundles["app"]
                .get_message("admin-title")
                .is_none()
        );
        assert_eq!(caches.domain_locale_resources["app"].len(), 1);
        assert_eq!(caches.domain_locale_resources["admin"].len(), 1);
        assert!(caches.unscoped_diagnostics.is_empty());
    }

    #[test]
    fn build_bundle_caches_keeps_domain_bundles_when_unscoped_bundle_has_duplicates() {
        let caches = build_bundle_caches(
            &langid!("en"),
            vec![
                (ResourceKey::new("app"), resource("shared = First")),
                (ResourceKey::new("admin"), resource("shared = Second")),
            ],
        )
        .expect("cross-domain duplicates should still build domain caches");

        assert!(caches.bundle.is_none());
        assert!(caches.locale_resources.is_none());
        assert!(
            caches
                .unscoped_diagnostics
                .iter()
                .any(|message| message.contains("resource 'admin'"))
        );
        assert!(caches.domain_bundles["app"].get_message("shared").is_some());
        assert!(
            caches.domain_bundles["admin"]
                .get_message("shared")
                .is_some()
        );
        assert_eq!(caches.domain_locale_resources["app"].len(), 1);
        assert_eq!(caches.domain_locale_resources["admin"].len(), 1);
    }

    #[test]
    fn build_bundle_from_resources_reports_duplicate_message_ids() {
        let diagnostics = match build_bundle_from_resources(
            &langid!("en"),
            vec![
                (ResourceKey::new("app"), resource("shared = First")),
                (ResourceKey::new("admin"), resource("shared = Second")),
            ],
        ) {
            Ok(_) => panic!("duplicate message IDs should reject the cache rebuild"),
            Err(diagnostics) => diagnostics,
        };

        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("resource 'admin'"))
        );
    }

    #[test]
    fn build_domain_bundles_reports_domain_context_for_duplicate_message_ids() {
        let diagnostics = match build_domain_bundles(
            &langid!("en"),
            &[
                (ResourceKey::new("app/main"), resource("shared = First")),
                (ResourceKey::new("app/extra"), resource("shared = Second")),
            ],
        ) {
            Ok(_) => panic!("duplicate domain messages should reject the domain cache"),
            Err(diagnostics) => diagnostics,
        };

        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("domain 'app'"))
        );
    }

    #[test]
    fn build_fluent_bundles_rebuilds_added_i18n_assets_without_explicit_events() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), Handle::default());
        i18n_assets
            .loaded_resources
            .insert((lang.clone(), resource_spec.key), resource("hello = Hello"));

        let mut app = App::new();
        app.add_message::<AssetEvent<FtlAsset>>()
            .add_message::<AssetLoadFailedEvent<FtlAsset>>()
            .insert_resource(i18n_assets)
            .insert_resource(I18nBundle::default())
            .insert_resource(I18nDomainBundles::default())
            .insert_resource(BundleBuildFailures::default())
            .add_systems(Update, build_fluent_bundles);

        app.update();

        assert!(app.world().resource::<I18nBundle>().get(&lang).is_some());
        assert!(
            app.world()
                .resource::<I18nDomainBundles>()
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get("app"))
                .is_some()
        );
    }

    #[test]
    fn build_fluent_bundles_rebuilds_languages_from_asset_events() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut ftl_assets = Assets::<FtlAsset>::default();
        let handle = ftl_assets.add(FtlAsset {
            content: "hello = Hello".to_string(),
        });
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(lang.clone(), resource_spec.clone(), handle.clone());
        i18n_assets.loaded_resources.insert(
            (lang.clone(), resource_spec.key.clone()),
            resource("hello = Hello"),
        );

        let mut app = App::new();
        app.add_message::<AssetEvent<FtlAsset>>()
            .add_message::<AssetLoadFailedEvent<FtlAsset>>()
            .insert_resource(i18n_assets)
            .insert_resource(I18nBundle::default())
            .insert_resource(I18nDomainBundles::default())
            .insert_resource(BundleBuildFailures::default())
            .add_systems(Update, build_fluent_bundles);

        app.update();
        app.world_mut()
            .resource_mut::<I18nBundle>()
            .remove_bundle(&lang);
        app.world_mut()
            .write_message(AssetEvent::LoadedWithDependencies { id: handle.id() });
        app.update();

        assert!(app.world().resource::<I18nBundle>().get(&lang).is_some());
    }

    #[test]
    fn rebuild_bundle_for_language_removes_empty_language_cache() {
        let lang = langid!("en");
        let mut i18n_bundle = I18nBundle::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();
        let i18n_assets = I18nAssets::new();

        i18n_bundle.set_bundle(lang.clone(), empty_bundle(&lang));
        i18n_bundle.set_locale_resources(lang.clone(), vec![resource("old = Old")]);
        i18n_domain_bundles.set_bundles(
            lang.clone(),
            HashMap::from([("app".to_string(), empty_bundle(&lang))]),
        );
        i18n_domain_bundles.set_locale_resources(
            lang.clone(),
            HashMap::from([("app".to_string(), vec![resource("old = Old")])]),
        );
        bundle_build_failures
            .0
            .insert(lang.clone(), vec!["old failure".to_string()]);

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert!(i18n_bundle.get(&lang).is_none());
        assert!(!i18n_bundle.locale_resources.contains_key(&lang));
        assert!(!i18n_domain_bundles.bundles.contains_key(&lang));
        assert!(!i18n_domain_bundles.locale_resources.contains_key(&lang));
        assert!(!bundle_build_failures.0.contains_key(&lang));
    }

    #[test]
    fn rebuild_bundle_for_language_marks_optional_only_language_ready_without_resources() {
        let lang = langid!("en");
        let optional_spec = spec("app", false);
        let mut i18n_assets = I18nAssets::new();
        let mut i18n_bundle = I18nBundle::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();

        i18n_assets.add_optional_asset_spec(lang.clone(), optional_spec, Handle::default());
        i18n_bundle.set_bundle(lang.clone(), empty_bundle(&lang));
        i18n_bundle.set_locale_resources(lang.clone(), vec![resource("old = Old")]);
        i18n_domain_bundles.set_bundles(
            lang.clone(),
            HashMap::from([("app".to_string(), empty_bundle(&lang))]),
        );
        i18n_domain_bundles.set_locale_resources(
            lang.clone(),
            HashMap::from([("app".to_string(), vec![resource("old = Old")])]),
        );
        bundle_build_failures
            .0
            .insert(lang.clone(), vec!["old failure".to_string()]);

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert!(i18n_bundle.get(&lang).is_none());
        assert_eq!(i18n_bundle.languages().collect::<Vec<_>>(), vec![&lang]);
        assert!(!i18n_bundle.locale_resources.contains_key(&lang));
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .expect("ready empty domain bundle map should be published")
                .is_empty()
        );
        assert!(
            i18n_domain_bundles
                .locale_resources
                .get(&lang)
                .expect("ready empty domain resource map should be published")
                .is_empty()
        );
        assert!(!bundle_build_failures.0.contains_key(&lang));
    }

    #[test]
    fn rebuild_bundle_for_language_stores_partial_resources_without_ready_bundle() {
        let lang = langid!("en");
        let optional_spec = spec("app", false);
        let required_spec = spec("admin", true);
        let mut i18n_assets = I18nAssets::new();
        let mut i18n_bundle = I18nBundle::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();

        i18n_assets.add_optional_asset_spec(lang.clone(), optional_spec.clone(), Handle::default());
        i18n_assets.add_asset_spec(lang.clone(), required_spec, Handle::default());
        i18n_assets.loaded_resources.insert(
            (lang.clone(), optional_spec.key.clone()),
            resource("app-title = App"),
        );

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert!(i18n_bundle.get(&lang).is_none());
        assert_eq!(i18n_bundle.locale_resources[&lang].len(), 1);
        assert_eq!(i18n_domain_bundles.locale_resources[&lang]["app"].len(), 1);
        assert!(!bundle_build_failures.0.contains_key(&lang));
    }

    #[test]
    fn rebuild_bundle_for_language_commits_domain_cache_when_unscoped_bundle_fails() {
        let lang = langid!("en");
        let app_spec = spec("app", true);
        let admin_spec = spec("admin", true);
        let mut i18n_assets = I18nAssets::new();
        let mut i18n_bundle = I18nBundle::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();
        let old_bundle = empty_bundle(&lang);

        i18n_bundle.set_bundle(lang.clone(), old_bundle.clone());
        i18n_assets.add_asset_spec(lang.clone(), app_spec.clone(), Handle::default());
        i18n_assets.add_asset_spec(lang.clone(), admin_spec.clone(), Handle::default());
        i18n_assets
            .loaded_resources
            .insert((lang.clone(), app_spec.key), resource("shared = First"));
        i18n_assets
            .loaded_resources
            .insert((lang.clone(), admin_spec.key), resource("shared = Second"));

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert!(i18n_bundle.get(&lang).is_none());
        assert!(!i18n_bundle.locale_resources.contains_key(&lang));
        assert_eq!(i18n_bundle.languages().count(), 1);
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get("app"))
                .is_some()
        );
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get("admin"))
                .is_some()
        );
        assert!(!bundle_build_failures.0.contains_key(&lang));
    }

    #[test]
    fn rebuild_bundle_for_language_keeps_last_ready_cache_when_domain_rebuild_fails() {
        let lang = langid!("en");
        let main_spec = spec("app/main", true);
        let extra_spec = spec("app/extra", true);
        let old_resource = resource("hello = Old");
        let mut i18n_assets = I18nAssets::new();
        let mut i18n_bundle = I18nBundle::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();

        let (old_bundle, old_resources) = build_bundle_from_resources(
            &lang,
            vec![(ResourceKey::new("app"), old_resource.clone())],
        )
        .expect("old unscoped cache should build");
        let (old_domain_bundles, old_domain_resources) =
            build_domain_bundles(&lang, &[(ResourceKey::new("app"), old_resource)])
                .expect("old domain cache should build");
        i18n_bundle.set_bundle(lang.clone(), old_bundle);
        i18n_bundle.set_locale_resources(
            lang.clone(),
            old_resources
                .into_iter()
                .map(|(_, resource)| resource)
                .collect(),
        );
        i18n_domain_bundles.set_bundles(lang.clone(), old_domain_bundles);
        i18n_domain_bundles.set_locale_resources(lang.clone(), old_domain_resources);
        let old_ready_id = i18n_bundle
            .ready_cache_id(&lang)
            .expect("old cache should be marked ready");

        i18n_assets.add_asset_spec(lang.clone(), main_spec.clone(), Handle::default());
        i18n_assets.add_asset_spec(lang.clone(), extra_spec.clone(), Handle::default());
        i18n_assets
            .loaded_resources
            .insert((lang.clone(), main_spec.key), resource("shared = First"));
        i18n_assets
            .loaded_resources
            .insert((lang.clone(), extra_spec.key), resource("shared = Second"));

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert_eq!(i18n_bundle.ready_cache_id(&lang), Some(old_ready_id));
        assert!(
            i18n_bundle
                .get(&lang)
                .expect("last accepted unscoped bundle should remain")
                .get_message("hello")
                .is_some()
        );
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get("app"))
                .expect("last accepted domain bundle should remain")
                .get_message("hello")
                .is_some()
        );
        assert!(
            bundle_build_failures
                .0
                .get(&lang)
                .expect("failed rebuild should be retained as diagnostics")
                .iter()
                .any(|message| message.contains("domain 'app'"))
        );
    }
}
