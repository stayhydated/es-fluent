use crate::{
    ActiveLanguageId, BundleBuildFailures, I18nAssets, I18nBundle, I18nResource, LanguageSelection,
    LocaleChangeEvent, LocaleChangedEvent, PendingLanguageChange, RequestedLanguageId,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use es_fluent_manager_core::{FallbackChainAvailability, resolve_fallback_chain_availability};
use std::collections::HashSet;
use unic_langid::LanguageIdentifier;

pub(super) fn apply_selected_language(
    selection: &LanguageSelection,
    i18n_resource: &mut I18nResource,
    active_language_id: &mut ActiveLanguageId,
    locale_changed_events: &mut MessageWriter<LocaleChangedEvent>,
) -> bool {
    if i18n_resource.active_language() == &selection.requested
        && i18n_resource.resolved_language() == &selection.resolved
        && active_language_id.0 == selection.requested
    {
        return false;
    }

    if let Err(error) = i18n_resource.select_fallback_language(&selection.requested) {
        warn!(
            "Skipping locale change to '{}' because the fallback manager rejected the switch: {}",
            selection.requested, error
        );
        return false;
    }

    i18n_resource.set_active_language(selection.requested.clone(), selection.resolved.clone());
    active_language_id.0 = selection.requested.clone();
    locale_changed_events.write(LocaleChangedEvent(selection.requested.clone()));
    true
}

enum RequestedLanguageResolution {
    Ready(LanguageSelection),
    Pending(LanguageSelection),
    Blocked(LanguageSelection),
    Unavailable,
}

#[derive(SystemParam)]
pub(crate) struct LocaleChangeParams<'w> {
    locale_changed_events: MessageWriter<'w, LocaleChangedEvent>,
    i18n_resource: ResMut<'w, I18nResource>,
    i18n_bundle: Res<'w, I18nBundle>,
    i18n_assets: Res<'w, I18nAssets>,
    bundle_build_failures: Res<'w, BundleBuildFailures>,
    requested_language_id: ResMut<'w, RequestedLanguageId>,
    active_language_id: ResMut<'w, ActiveLanguageId>,
    pending_language_change: ResMut<'w, PendingLanguageChange>,
}

fn resolve_requested_language(
    requested_language: &LanguageIdentifier,
    i18n_bundle: &I18nBundle,
    i18n_assets: &I18nAssets,
    bundle_build_failures: &BundleBuildFailures,
) -> RequestedLanguageResolution {
    let ready_languages = i18n_bundle.languages().cloned().collect::<HashSet<_>>();
    let blocked_languages = bundle_build_failures
        .0
        .keys()
        .filter(|language| !ready_languages.contains(*language))
        .cloned()
        .collect::<HashSet<_>>();
    let available_languages = i18n_assets
        .available_languages()
        .into_iter()
        .filter(|language| !blocked_languages.contains(language))
        .collect::<HashSet<_>>();
    let ready_candidates = ready_languages.iter().cloned().collect::<Vec<_>>();
    let available_candidates = available_languages.iter().cloned().collect::<Vec<_>>();
    let blocked_candidates = blocked_languages.iter().cloned().collect::<Vec<_>>();

    match resolve_fallback_chain_availability(
        requested_language,
        &ready_candidates,
        &available_candidates,
        &blocked_candidates,
    ) {
        FallbackChainAvailability::Ready(candidate) => {
            if candidate != *requested_language {
                if let Some(errors) = bundle_build_failures.0.get(requested_language) {
                    warn!(
                        "Locale '{}' failed validation; using ready fallback '{}': {}",
                        requested_language,
                        candidate,
                        errors.join(" | ")
                    );
                } else {
                    info!(
                        "Locale '{}' is not ready yet; using ready fallback '{}'",
                        requested_language, candidate
                    );
                }
            }

            RequestedLanguageResolution::Ready(LanguageSelection::new(
                requested_language.clone(),
                candidate,
            ))
        },
        FallbackChainAvailability::Available(candidate) => {
            if candidate != *requested_language {
                info!(
                    "Locale '{}' is not ready yet, waiting for available fallback '{}'",
                    requested_language, candidate
                );
            }

            RequestedLanguageResolution::Pending(LanguageSelection::new(
                requested_language.clone(),
                candidate,
            ))
        },
        FallbackChainAvailability::Blocked(candidate) => {
            let diagnostics = bundle_build_failures
                .0
                .get(&candidate)
                .map(|errors| errors.join(" | "))
                .unwrap_or_else(|| "unknown bundle build failure".to_string());
            warn!(
                "Skipping locale change to '{}' because Fluent bundle assembly failed for '{}': {}",
                requested_language, candidate, diagnostics
            );
            RequestedLanguageResolution::Blocked(LanguageSelection::new(
                requested_language.clone(),
                candidate,
            ))
        },
        FallbackChainAvailability::Unavailable => RequestedLanguageResolution::Unavailable,
    }
}

#[doc(hidden)]
pub(crate) fn handle_locale_changes(
    mut locale_change_events: MessageReader<LocaleChangeEvent>,
    mut params: LocaleChangeParams,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);
        params.requested_language_id.0 = event.0.clone();
        let resolution = resolve_requested_language(
            &event.0,
            &params.i18n_bundle,
            &params.i18n_assets,
            &params.bundle_build_failures,
        );

        match resolution {
            RequestedLanguageResolution::Ready(selection) => {
                params.pending_language_change.0 = None;
                apply_selected_language(
                    &selection,
                    &mut params.i18n_resource,
                    &mut params.active_language_id,
                    &mut params.locale_changed_events,
                );
            },
            RequestedLanguageResolution::Pending(selection) => {
                if params.pending_language_change.0.as_ref() != Some(&selection) {
                    info!(
                        "Deferring locale change to '{}' until Fluent bundle '{}' is ready",
                        selection.requested, selection.resolved
                    );
                }
                params.pending_language_change.0 = Some(selection);
            },
            RequestedLanguageResolution::Blocked(selection) => {
                if let Some(pending_language) = params.pending_language_change.0.take() {
                    info!(
                        "Clearing deferred locale change to '{}' because a later request for blocked locale '{}' superseded it",
                        pending_language.requested, selection.requested
                    );
                }
            },
            RequestedLanguageResolution::Unavailable => {
                if let Some(pending_language) = params.pending_language_change.0.take() {
                    info!(
                        "Clearing deferred locale change to '{}' because a later request for unsupported locale '{}' superseded it",
                        pending_language.requested, event.0
                    );
                }
                info!(
                    "Keeping active locale '{}' because requested locale '{}' has no usable Bevy asset fallback chain",
                    params.active_language_id.0, event.0
                );
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_manager_core::SyncFluentBundle;
    use std::sync::Arc;
    use unic_langid::langid;

    #[derive(Default, Resource)]
    struct ObservedLocaleChanges(Vec<LanguageIdentifier>);

    fn observe_locale_changes(
        mut events: MessageReader<LocaleChangedEvent>,
        mut observed: ResMut<ObservedLocaleChanges>,
    ) {
        observed
            .0
            .extend(events.read().map(|event| event.0.clone()));
    }

    fn insert_ready_bundle(i18n_bundle: &mut I18nBundle, lang: LanguageIdentifier) {
        i18n_bundle.set_bundle(
            lang.clone(),
            Arc::new(SyncFluentBundle::new_concurrent(vec![lang])),
        );
    }

    fn app_with_locale_system(
        i18n_bundle: I18nBundle,
        i18n_assets: I18nAssets,
        failures: BundleBuildFailures,
        pending_language_change: PendingLanguageChange,
    ) -> App {
        let lang = langid!("en");
        let mut app = App::new();
        app.add_message::<LocaleChangeEvent>()
            .add_message::<LocaleChangedEvent>()
            .insert_resource(I18nResource::new(lang.clone()))
            .insert_resource(RequestedLanguageId(lang.clone()))
            .insert_resource(ActiveLanguageId(lang))
            .insert_resource(i18n_bundle)
            .insert_resource(i18n_assets)
            .insert_resource(failures)
            .insert_resource(pending_language_change)
            .insert_resource(ObservedLocaleChanges::default())
            .add_systems(
                Update,
                (handle_locale_changes, observe_locale_changes).chain(),
            );
        app
    }

    #[test]
    fn resolve_requested_language_returns_ready_exact_locale() {
        let lang = langid!("en");
        let mut i18n_bundle = I18nBundle::default();
        insert_ready_bundle(&mut i18n_bundle, lang.clone());

        match resolve_requested_language(
            &lang,
            &i18n_bundle,
            &I18nAssets::new(),
            &BundleBuildFailures::default(),
        ) {
            RequestedLanguageResolution::Ready(selection) => {
                assert_eq!(selection.requested, lang);
                assert_eq!(selection.resolved, lang);
            },
            _ => panic!("expected ready locale resolution"),
        }
    }

    #[test]
    fn resolve_requested_language_returns_ready_parent_fallback() {
        let mut i18n_bundle = I18nBundle::default();
        insert_ready_bundle(&mut i18n_bundle, langid!("en"));

        match resolve_requested_language(
            &langid!("en-US"),
            &i18n_bundle,
            &I18nAssets::new(),
            &BundleBuildFailures::default(),
        ) {
            RequestedLanguageResolution::Ready(selection) => {
                assert_eq!(selection.requested, langid!("en-US"));
                assert_eq!(selection.resolved, langid!("en"));
            },
            _ => panic!("expected ready fallback locale resolution"),
        }
    }

    #[test]
    fn resolve_requested_language_returns_pending_available_fallback() {
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(langid!("en"), "app".to_string(), Handle::default());

        match resolve_requested_language(
            &langid!("en-US"),
            &I18nBundle::default(),
            &i18n_assets,
            &BundleBuildFailures::default(),
        ) {
            RequestedLanguageResolution::Pending(selection) => {
                assert_eq!(selection.requested, langid!("en-US"));
                assert_eq!(selection.resolved, langid!("en"));
            },
            _ => panic!("expected pending locale resolution"),
        }
    }

    #[test]
    fn resolve_requested_language_returns_blocked_fallback() {
        let mut failures = BundleBuildFailures::default();
        failures
            .0
            .insert(langid!("en"), vec!["duplicate message".to_string()]);

        match resolve_requested_language(
            &langid!("en-US"),
            &I18nBundle::default(),
            &I18nAssets::new(),
            &failures,
        ) {
            RequestedLanguageResolution::Blocked(selection) => {
                assert_eq!(selection.requested, langid!("en-US"));
                assert_eq!(selection.resolved, langid!("en"));
            },
            _ => panic!("expected blocked locale resolution"),
        }
    }

    #[test]
    fn resolve_requested_language_returns_unavailable_without_any_candidate() {
        match resolve_requested_language(
            &langid!("de-AT"),
            &I18nBundle::default(),
            &I18nAssets::new(),
            &BundleBuildFailures::default(),
        ) {
            RequestedLanguageResolution::Unavailable => {},
            _ => panic!("expected unavailable locale resolution"),
        }
    }

    #[test]
    fn handle_locale_changes_applies_ready_locale_and_emits_change_event() {
        let fr = langid!("fr");
        let mut i18n_bundle = I18nBundle::default();
        insert_ready_bundle(&mut i18n_bundle, fr.clone());
        let mut app = app_with_locale_system(
            i18n_bundle,
            I18nAssets::new(),
            BundleBuildFailures::default(),
            PendingLanguageChange::default(),
        );

        app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
        app.update();

        assert_eq!(app.world().resource::<RequestedLanguageId>().0, fr);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, fr);
        assert_eq!(
            app.world().resource::<I18nResource>().active_language(),
            &fr
        );
        assert!(app.world().resource::<PendingLanguageChange>().0.is_none());
        assert_eq!(app.world().resource::<ObservedLocaleChanges>().0, vec![fr]);
    }

    #[test]
    fn handle_locale_changes_defers_available_locale_until_bundle_is_ready() {
        let fr = langid!("fr");
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(fr.clone(), "app".to_string(), Handle::default());
        let mut app = app_with_locale_system(
            I18nBundle::default(),
            i18n_assets,
            BundleBuildFailures::default(),
            PendingLanguageChange::default(),
        );

        app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
        app.update();

        assert_eq!(app.world().resource::<RequestedLanguageId>().0, fr);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, langid!("en"));
        assert_eq!(
            app.world().resource::<PendingLanguageChange>().0,
            Some(LanguageSelection::new(fr.clone(), fr))
        );
        assert!(app.world().resource::<ObservedLocaleChanges>().0.is_empty());
    }

    #[test]
    fn handle_locale_changes_clears_pending_for_blocked_and_unavailable_requests() {
        let fr = langid!("fr");
        let de = langid!("de");
        let mut failures = BundleBuildFailures::default();
        failures
            .0
            .insert(de.clone(), vec!["duplicate message".to_string()]);
        let mut app = app_with_locale_system(
            I18nBundle::default(),
            I18nAssets::new(),
            failures,
            PendingLanguageChange(Some(LanguageSelection::new(fr.clone(), fr.clone()))),
        );

        app.world_mut().write_message(LocaleChangeEvent(de.clone()));
        app.update();

        assert_eq!(app.world().resource::<RequestedLanguageId>().0, de);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, langid!("en"));
        assert!(app.world().resource::<PendingLanguageChange>().0.is_none());
        assert!(app.world().resource::<ObservedLocaleChanges>().0.is_empty());

        app.world_mut().resource_mut::<PendingLanguageChange>().0 =
            Some(LanguageSelection::new(fr.clone(), fr));
        app.world_mut()
            .write_message(LocaleChangeEvent(langid!("zh-Hant-TW")));
        app.update();

        assert_eq!(
            app.world().resource::<RequestedLanguageId>().0,
            langid!("zh-Hant-TW")
        );
        assert!(app.world().resource::<PendingLanguageChange>().0.is_none());
        assert!(app.world().resource::<ObservedLocaleChanges>().0.is_empty());
    }

    #[test]
    fn handle_locale_changes_keeps_current_locale_when_ready_selection_is_already_active() {
        let en = langid!("en");
        let mut i18n_bundle = I18nBundle::default();
        insert_ready_bundle(&mut i18n_bundle, en.clone());
        let mut app = app_with_locale_system(
            i18n_bundle,
            I18nAssets::new(),
            BundleBuildFailures::default(),
            PendingLanguageChange::default(),
        );

        app.world_mut().write_message(LocaleChangeEvent(en));
        app.update();

        assert!(app.world().resource::<ObservedLocaleChanges>().0.is_empty());
        assert!(app.world().resource::<PendingLanguageChange>().0.is_none());
    }
}
