use crate::{
    BundleBuildFailures, FluentResourceScope, FtlAsset, I18nAssets, I18nDomainBundles,
    I18nReadyLocales, I18nResourceKey,
};
use bevy::asset::{AssetEvent, AssetId, AssetLoadFailedEvent};
use bevy::prelude::*;
use es_fluent_manager_core::SyncFluentBundle;
use fluent_bundle::{FluentError, FluentResource};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

type DomainBundleMap = HashMap<FluentResourceScope, Arc<SyncFluentBundle>>;
type DomainResourceMap = HashMap<FluentResourceScope, Vec<Arc<FluentResource>>>;

struct BundleCaches {
    domain_bundles: DomainBundleMap,
    domain_locale_resources: DomainResourceMap,
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
        for (lang, _) in i18n_assets.resource_specs.keys() {
            dirty_languages.insert(lang.clone());
        }
    }

    dirty_languages
}

fn rebuild_bundle_for_language(
    i18n_bundle: &mut I18nReadyLocales,
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
            .resource_specs
            .keys()
            .any(|(language, _)| language == lang)
            && i18n_assets.is_language_loaded(lang)
        {
            i18n_bundle.mark_ready(lang.clone());
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
                domain_bundles,
                domain_locale_resources,
            } = caches;

            i18n_domain_bundles.set_locale_resources(lang.clone(), domain_locale_resources);
            bundle_build_failures.0.remove(lang);

            if i18n_assets.is_language_loaded(lang) {
                i18n_bundle.mark_ready(lang.clone());
                i18n_domain_bundles.set_bundles(lang.clone(), domain_bundles);
                debug!("Updated fluent bundle cache for {}", lang);
            } else {
                i18n_bundle.remove(lang);
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
    resources: Vec<(I18nResourceKey, Arc<FluentResource>)>,
) -> Result<BundleCaches, Vec<String>> {
    let (domain_bundles, domain_locale_resources) = build_domain_bundles(lang, &resources)?;

    Ok(BundleCaches {
        domain_bundles,
        domain_locale_resources,
    })
}

fn build_bundle_from_resources(
    lang: &LanguageIdentifier,
    resources: Vec<(I18nResourceKey, Arc<FluentResource>)>,
) -> Result<
    (
        Arc<SyncFluentBundle>,
        Vec<(I18nResourceKey, Arc<FluentResource>)>,
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
    accepted_resources: &[(I18nResourceKey, Arc<FluentResource>)],
) -> Result<(DomainBundleMap, DomainResourceMap), Vec<String>> {
    let mut grouped =
        HashMap::<FluentResourceScope, Vec<(I18nResourceKey, Arc<FluentResource>)>>::new();
    for (resource_key, resource) in accepted_resources.iter().cloned() {
        grouped
            .entry(resource_key.domain())
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

fn format_add_errors(resource_key: &I18nResourceKey, errors: Vec<FluentError>) -> String {
    let messages = errors
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "resource '{}:{}': {}",
        resource_key.owner(),
        resource_key.key(),
        messages
    )
}

#[doc(hidden)]
pub(crate) fn build_fluent_bundles(
    mut i18n_bundle: ResMut<I18nReadyLocales>,
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
    use es_fluent_manager_core::{
        LocaleRelativeFtlPath, ModuleResourceSpec, ResourceKey, StaticFluentDomain,
    };
    use unic_langid::langid;

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("valid FTL"))
    }

    fn owner() -> StaticFluentDomain {
        es_fluent_manager_core::__macro::static_domain("test-owner")
    }

    fn resource_key(key: &'static str) -> I18nResourceKey {
        I18nResourceKey::new(owner(), ResourceKey::from_static_path(key))
    }

    fn state_key(
        lang: LanguageIdentifier,
        spec: &ModuleResourceSpec,
    ) -> (LanguageIdentifier, I18nResourceKey) {
        (lang, I18nResourceKey::new(owner(), spec.key.clone()))
    }

    fn scope(domain: &'static str) -> FluentResourceScope {
        FluentResourceScope::new(
            owner(),
            es_fluent_manager_core::__macro::static_domain(domain),
        )
    }

    fn spec(key: &str, required: bool) -> ModuleResourceSpec {
        let resource_key = ResourceKey::try_new(key)
            .unwrap_or_else(|error| panic!("test resource key '{key}' should be valid: {error}"));
        let locale_relative_path = LocaleRelativeFtlPath::try_new(format!("{key}.ftl"))
            .unwrap_or_else(|error| panic!("test FTL path '{key}.ftl' should be valid: {error}"));
        ModuleResourceSpec::new(resource_key, locale_relative_path, required)
    }

    fn empty_bundle(lang: &LanguageIdentifier) -> Arc<SyncFluentBundle> {
        Arc::new(SyncFluentBundle::new_concurrent(
            es_fluent_manager_core::locale_candidates(lang),
        ))
    }

    #[test]
    fn build_bundle_caches_creates_independent_scoped_bundles() {
        let lang = langid!("en");
        let caches = build_bundle_caches(
            &lang,
            vec![
                (resource_key("app"), resource("app-title = App")),
                (resource_key("admin"), resource("admin-title = Admin")),
            ],
        )
        .expect("valid resources should build caches");

        assert!(
            caches.domain_bundles[&scope("app")]
                .get_message("app-title")
                .is_some()
        );
        assert!(
            caches.domain_bundles[&scope("app")]
                .get_message("admin-title")
                .is_none()
        );
        assert_eq!(caches.domain_locale_resources[&scope("app")].len(), 1);
        assert_eq!(caches.domain_locale_resources[&scope("admin")].len(), 1);
    }

    #[test]
    fn build_bundle_caches_allows_duplicate_ids_across_scopes() {
        let caches = build_bundle_caches(
            &langid!("en"),
            vec![
                (resource_key("app"), resource("shared = First")),
                (resource_key("admin"), resource("shared = Second")),
            ],
        )
        .expect("cross-domain duplicate IDs should build independent caches");

        assert!(
            caches.domain_bundles[&scope("app")]
                .get_message("shared")
                .is_some()
        );
        assert!(
            caches.domain_bundles[&scope("admin")]
                .get_message("shared")
                .is_some()
        );
        assert_eq!(caches.domain_locale_resources[&scope("app")].len(), 1);
        assert_eq!(caches.domain_locale_resources[&scope("admin")].len(), 1);
    }

    #[test]
    fn build_bundle_from_resources_reports_duplicate_message_ids() {
        let diagnostics = match build_bundle_from_resources(
            &langid!("en"),
            vec![
                (resource_key("app"), resource("shared = First")),
                (resource_key("admin"), resource("shared = Second")),
            ],
        ) {
            Ok(_) => panic!("duplicate message IDs should reject the cache rebuild"),
            Err(diagnostics) => diagnostics,
        };

        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("resource 'test-owner:admin'"))
        );
    }

    #[test]
    fn build_domain_bundles_reports_domain_context_for_duplicate_message_ids() {
        let diagnostics = match build_domain_bundles(
            &langid!("en"),
            &[
                (resource_key("app/main"), resource("shared = First")),
                (resource_key("app/extra"), resource("shared = Second")),
            ],
        ) {
            Ok(_) => panic!("duplicate domain messages should reject the domain cache"),
            Err(diagnostics) => diagnostics,
        };

        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("domain 'test-owner:app'"))
        );
    }

    #[test]
    fn build_fluent_bundles_rebuilds_added_i18n_assets_without_explicit_events() {
        let lang = langid!("en");
        let resource_spec = spec("app", true);
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset_spec(
            owner(),
            lang.clone(),
            resource_spec.clone(),
            Handle::default(),
        );
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &resource_spec),
            resource("hello = Hello"),
        );

        let mut app = App::new();
        app.add_message::<AssetEvent<FtlAsset>>()
            .add_message::<AssetLoadFailedEvent<FtlAsset>>()
            .insert_resource(i18n_assets)
            .insert_resource(I18nReadyLocales::default())
            .insert_resource(I18nDomainBundles::default())
            .insert_resource(BundleBuildFailures::default())
            .add_systems(Update, build_fluent_bundles);

        app.update();

        assert!(
            app.world()
                .resource::<I18nReadyLocales>()
                .ready_cache_id(&lang)
                .is_some()
        );
        assert!(
            app.world()
                .resource::<I18nDomainBundles>()
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get(&scope("app")))
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
        i18n_assets.add_asset_spec(owner(), lang.clone(), resource_spec.clone(), handle.clone());
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &resource_spec),
            resource("hello = Hello"),
        );

        let mut app = App::new();
        app.add_message::<AssetEvent<FtlAsset>>()
            .add_message::<AssetLoadFailedEvent<FtlAsset>>()
            .insert_resource(i18n_assets)
            .insert_resource(I18nReadyLocales::default())
            .insert_resource(I18nDomainBundles::default())
            .insert_resource(BundleBuildFailures::default())
            .add_systems(Update, build_fluent_bundles);

        app.update();
        app.world_mut()
            .resource_mut::<I18nReadyLocales>()
            .remove(&lang);
        app.world_mut()
            .write_message(AssetEvent::LoadedWithDependencies { id: handle.id() });
        app.update();

        assert!(
            app.world()
                .resource::<I18nReadyLocales>()
                .ready_cache_id(&lang)
                .is_some()
        );
    }

    #[test]
    fn rebuild_bundle_for_language_removes_empty_language_cache() {
        let lang = langid!("en");
        let mut i18n_bundle = I18nReadyLocales::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();
        let i18n_assets = I18nAssets::new();

        i18n_bundle.mark_ready(lang.clone());
        i18n_domain_bundles.set_bundles(
            lang.clone(),
            HashMap::from([(scope("app"), empty_bundle(&lang))]),
        );
        i18n_domain_bundles.set_locale_resources(
            lang.clone(),
            HashMap::from([(scope("app"), vec![resource("old = Old")])]),
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

        assert!(i18n_bundle.ready_cache_id(&lang).is_none());
        assert!(!i18n_domain_bundles.bundles.contains_key(&lang));
        assert!(!i18n_domain_bundles.locale_resources.contains_key(&lang));
        assert!(!bundle_build_failures.0.contains_key(&lang));
    }

    #[test]
    fn rebuild_bundle_for_language_marks_optional_only_language_ready_without_resources() {
        let lang = langid!("en");
        let optional_spec = spec("app", false);
        let mut i18n_assets = I18nAssets::new();
        let mut i18n_bundle = I18nReadyLocales::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();

        i18n_assets.add_optional_asset_spec(
            owner(),
            lang.clone(),
            optional_spec,
            Handle::default(),
        );
        i18n_bundle.mark_ready(lang.clone());
        i18n_domain_bundles.set_bundles(
            lang.clone(),
            HashMap::from([(scope("app"), empty_bundle(&lang))]),
        );
        i18n_domain_bundles.set_locale_resources(
            lang.clone(),
            HashMap::from([(scope("app"), vec![resource("old = Old")])]),
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

        assert!(i18n_bundle.ready_cache_id(&lang).is_some());
        assert_eq!(i18n_bundle.languages().collect::<Vec<_>>(), vec![&lang]);
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
        let mut i18n_bundle = I18nReadyLocales::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();

        i18n_assets.add_optional_asset_spec(
            owner(),
            lang.clone(),
            optional_spec.clone(),
            Handle::default(),
        );
        i18n_assets.add_asset_spec(owner(), lang.clone(), required_spec, Handle::default());
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &optional_spec),
            resource("app-title = App"),
        );

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert!(i18n_bundle.ready_cache_id(&lang).is_none());
        assert_eq!(
            i18n_domain_bundles.locale_resources[&lang][&scope("app")].len(),
            1
        );
        assert!(!bundle_build_failures.0.contains_key(&lang));
    }

    #[test]
    fn rebuild_bundle_for_language_accepts_duplicate_ids_across_scopes() {
        let lang = langid!("en");
        let app_spec = spec("app", true);
        let admin_spec = spec("admin", true);
        let mut i18n_assets = I18nAssets::new();
        let mut i18n_bundle = I18nReadyLocales::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();
        i18n_assets.add_asset_spec(owner(), lang.clone(), app_spec.clone(), Handle::default());
        i18n_assets.add_asset_spec(owner(), lang.clone(), admin_spec.clone(), Handle::default());
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &app_spec),
            resource("shared = First"),
        );
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &admin_spec),
            resource("shared = Second"),
        );

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert!(i18n_bundle.ready_cache_id(&lang).is_some());
        assert_eq!(i18n_bundle.languages().count(), 1);
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get(&scope("app")))
                .is_some()
        );
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get(&scope("admin")))
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
        let mut i18n_bundle = I18nReadyLocales::default();
        let mut i18n_domain_bundles = I18nDomainBundles::default();
        let mut bundle_build_failures = BundleBuildFailures::default();

        let (old_domain_bundles, old_domain_resources) =
            build_domain_bundles(&lang, &[(resource_key("app"), old_resource)])
                .expect("old domain cache should build");
        i18n_bundle.mark_ready(lang.clone());
        i18n_domain_bundles.set_bundles(lang.clone(), old_domain_bundles);
        i18n_domain_bundles.set_locale_resources(lang.clone(), old_domain_resources);
        let old_ready_id = i18n_bundle
            .ready_cache_id(&lang)
            .expect("old cache should be marked ready");

        i18n_assets.add_asset_spec(owner(), lang.clone(), main_spec.clone(), Handle::default());
        i18n_assets.add_asset_spec(owner(), lang.clone(), extra_spec.clone(), Handle::default());
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &main_spec),
            resource("shared = First"),
        );
        i18n_assets.loaded_resources.insert(
            state_key(lang.clone(), &extra_spec),
            resource("shared = Second"),
        );

        rebuild_bundle_for_language(
            &mut i18n_bundle,
            &mut i18n_domain_bundles,
            &mut bundle_build_failures,
            &i18n_assets,
            &lang,
        );

        assert_eq!(i18n_bundle.ready_cache_id(&lang), Some(old_ready_id));
        assert!(
            i18n_domain_bundles
                .bundles
                .get(&lang)
                .and_then(|bundles| bundles.get(&scope("app")))
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
                .any(|message| message.contains("domain 'test-owner:app'"))
        );
    }
}
