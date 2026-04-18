use super::super::state::try_update_global_language_selection;
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

    if let Err(error) = try_update_global_language_selection(selection.requested.clone()) {
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
    use crate::set_bevy_i18n_state;
    use crate::test_support::lock_bevy_global_state;
    use crate::{ActiveLanguageId, RequestedLanguageId};
    use bevy::ecs::message::Messages;
    use std::collections::HashMap;
    use std::sync::Arc;
    use unic_langid::langid;

    #[test]
    fn handle_locale_changes_keeps_current_locale_when_requested_bundle_failed_to_build() {
        let _guard = lock_bevy_global_state();
        set_bevy_i18n_state(crate::BevyI18nState::new(langid!("en")));

        let en = langid!("en");
        let fr = langid!("fr");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(en.clone(), "app".to_string(), Handle::default());
        i18n_assets.add_asset(fr.clone(), "app".to_string(), Handle::default());

        let resource = Arc::new(
            fluent_bundle::FluentResource::try_new("hello = hi".to_string()).expect("ftl"),
        );
        let mut bundle = es_fluent_manager_core::SyncFluentBundle::new_concurrent(vec![en.clone()]);
        bundle.add_resource(resource).expect("add resource");

        app.add_message::<LocaleChangeEvent>();
        app.add_message::<LocaleChangedEvent>();
        app.insert_resource(i18n_assets);
        let mut i18n_bundle = I18nBundle::default();
        i18n_bundle.insert(en.clone(), Arc::new(bundle), Vec::new());
        app.insert_resource(i18n_bundle);
        app.insert_resource(BundleBuildFailures(HashMap::from([(
            fr.clone(),
            vec!["resource 'app': duplicate message id 'hello'".to_string()],
        )])));
        app.insert_resource(I18nResource::new(en.clone()));
        app.insert_resource(RequestedLanguageId(en.clone()));
        app.insert_resource(ActiveLanguageId(en.clone()));
        app.insert_resource(PendingLanguageChange::default());
        app.add_systems(Update, handle_locale_changes);

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };

        app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
        app.update();

        assert_eq!(app.world().resource::<RequestedLanguageId>().0, fr);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, en);
        let locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert!(locale_changes.is_empty());
    }

    #[test]
    fn handle_locale_changes_uses_ready_fallback_when_requested_locale_is_only_available() {
        let _guard = lock_bevy_global_state();
        set_bevy_i18n_state(crate::BevyI18nState::new(langid!("en")));

        let en = langid!("en");
        let en_us = langid!("en-US");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(en.clone(), "app".to_string(), Handle::default());
        i18n_assets.add_asset(en_us.clone(), "app".to_string(), Handle::default());

        let resource = Arc::new(
            fluent_bundle::FluentResource::try_new("hello = hi".to_string()).expect("ftl"),
        );
        let mut bundle = es_fluent_manager_core::SyncFluentBundle::new_concurrent(vec![en.clone()]);
        bundle.add_resource(resource).expect("add resource");

        app.add_message::<LocaleChangeEvent>();
        app.add_message::<LocaleChangedEvent>();
        app.insert_resource(i18n_assets);
        let mut i18n_bundle = I18nBundle::default();
        i18n_bundle.insert(en.clone(), Arc::new(bundle), Vec::new());
        app.insert_resource(i18n_bundle);
        app.insert_resource(BundleBuildFailures::default());
        app.insert_resource(I18nResource::new(en.clone()));
        app.insert_resource(RequestedLanguageId(en.clone()));
        app.insert_resource(ActiveLanguageId(en.clone()));
        app.insert_resource(PendingLanguageChange::default());
        app.add_systems(Update, handle_locale_changes);

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };

        app.world_mut()
            .write_message(LocaleChangeEvent(en_us.clone()));
        app.update();

        let i18n_resource = app.world().resource::<I18nResource>();
        assert_eq!(i18n_resource.active_language(), &en_us);
        assert_eq!(i18n_resource.resolved_language(), &en);
        assert_eq!(app.world().resource::<RequestedLanguageId>().0, en_us);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, en_us);
        assert_eq!(app.world().resource::<PendingLanguageChange>().0, None);
        let locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(locale_changes, vec![langid!("en-US")]);
    }

    #[test]
    fn handle_locale_changes_uses_ready_fallback_when_requested_bundle_failed_to_build() {
        let _guard = lock_bevy_global_state();
        set_bevy_i18n_state(crate::BevyI18nState::new(langid!("en")));

        let en = langid!("en");
        let en_us = langid!("en-US");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(en.clone(), "app".to_string(), Handle::default());
        i18n_assets.add_asset(en_us.clone(), "app".to_string(), Handle::default());

        let resource = Arc::new(
            fluent_bundle::FluentResource::try_new("hello = hi".to_string()).expect("ftl"),
        );
        let mut bundle = es_fluent_manager_core::SyncFluentBundle::new_concurrent(vec![en.clone()]);
        bundle.add_resource(resource).expect("add resource");

        app.add_message::<LocaleChangeEvent>();
        app.add_message::<LocaleChangedEvent>();
        app.insert_resource(i18n_assets);
        let mut i18n_bundle = I18nBundle::default();
        i18n_bundle.insert(en.clone(), Arc::new(bundle), Vec::new());
        app.insert_resource(i18n_bundle);
        app.insert_resource(BundleBuildFailures(HashMap::from([(
            en_us.clone(),
            vec!["resource 'app': duplicate message id 'hello'".to_string()],
        )])));
        app.insert_resource(I18nResource::new(en.clone()));
        app.insert_resource(RequestedLanguageId(en.clone()));
        app.insert_resource(ActiveLanguageId(en.clone()));
        app.insert_resource(PendingLanguageChange::default());
        app.add_systems(Update, handle_locale_changes);

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };

        app.world_mut()
            .write_message(LocaleChangeEvent(en_us.clone()));
        app.update();

        let i18n_resource = app.world().resource::<I18nResource>();
        assert_eq!(i18n_resource.active_language(), &en_us);
        assert_eq!(i18n_resource.resolved_language(), &en);
        assert_eq!(app.world().resource::<RequestedLanguageId>().0, en_us);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, en_us);
        assert_eq!(app.world().resource::<PendingLanguageChange>().0, None);
        let locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(locale_changes, vec![langid!("en-US")]);
    }

    #[test]
    fn handle_locale_changes_defers_available_locale_until_bundle_is_ready() {
        let _guard = lock_bevy_global_state();
        set_bevy_i18n_state(crate::BevyI18nState::new(langid!("en")));

        let en = langid!("en");
        let fr = langid!("fr");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(en.clone(), "app".to_string(), Handle::default());
        i18n_assets.add_asset(fr.clone(), "app".to_string(), Handle::default());

        let resource = Arc::new(
            fluent_bundle::FluentResource::try_new("hello = hi".to_string()).expect("ftl"),
        );
        let mut bundle = es_fluent_manager_core::SyncFluentBundle::new_concurrent(vec![en.clone()]);
        bundle.add_resource(resource).expect("add resource");

        app.add_message::<LocaleChangeEvent>();
        app.add_message::<LocaleChangedEvent>();
        app.insert_resource(i18n_assets);
        let mut i18n_bundle = I18nBundle::default();
        i18n_bundle.insert(en.clone(), Arc::new(bundle), Vec::new());
        app.insert_resource(i18n_bundle);
        app.insert_resource(BundleBuildFailures::default());
        app.insert_resource(I18nResource::new(en.clone()));
        app.insert_resource(RequestedLanguageId(en.clone()));
        app.insert_resource(ActiveLanguageId(en.clone()));
        app.insert_resource(PendingLanguageChange::default());
        app.add_systems(Update, handle_locale_changes);

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };

        app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
        app.update();

        assert_eq!(
            app.world().resource::<I18nResource>().active_language(),
            &en
        );
        assert_eq!(app.world().resource::<RequestedLanguageId>().0, fr);
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, en);
        assert_eq!(
            app.world()
                .resource::<PendingLanguageChange>()
                .0
                .as_ref()
                .map(|selection| (&selection.requested, &selection.resolved)),
            Some((&fr, &fr))
        );
        let locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert!(locale_changes.is_empty());
    }
}
