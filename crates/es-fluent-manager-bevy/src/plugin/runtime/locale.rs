use super::super::state::try_update_global_language;
use crate::{
    BundleBuildFailures, CurrentLanguageId, I18nAssets, I18nBundle, I18nResource,
    LocaleChangeEvent, LocaleChangedEvent, PendingLanguageChange,
};
use bevy::prelude::*;
use es_fluent_manager_core::locale_candidates;
use std::collections::HashSet;
use unic_langid::LanguageIdentifier;

pub(super) fn apply_selected_language(
    resolved_language: LanguageIdentifier,
    i18n_resource: &mut I18nResource,
    current_language_id: &mut CurrentLanguageId,
    locale_changed_events: &mut MessageWriter<LocaleChangedEvent>,
) -> bool {
    if i18n_resource.current_language() == &resolved_language
        && current_language_id.0 == resolved_language
    {
        return false;
    }

    if let Err(error) = try_update_global_language(resolved_language.clone()) {
        warn!(
            "Skipping locale change to '{}' because the fallback manager rejected the switch: {}",
            resolved_language, error
        );
        return false;
    }

    i18n_resource.set_language(resolved_language.clone());
    current_language_id.0 = resolved_language.clone();
    locale_changed_events.write(LocaleChangedEvent(resolved_language));
    true
}

enum RequestedLanguageResolution {
    Ready(LanguageIdentifier),
    Pending(LanguageIdentifier),
    Blocked(LanguageIdentifier),
    Immediate(LanguageIdentifier),
}

fn resolve_requested_language(
    requested_language: &LanguageIdentifier,
    i18n_bundle: &I18nBundle,
    i18n_assets: &I18nAssets,
    bundle_build_failures: &BundleBuildFailures,
) -> RequestedLanguageResolution {
    let ready_languages = i18n_bundle.0.keys().cloned().collect::<HashSet<_>>();
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

    for candidate in locale_candidates(requested_language) {
        if blocked_languages.contains(&candidate) {
            let diagnostics = bundle_build_failures
                .0
                .get(&candidate)
                .map(|errors| errors.join(" | "))
                .unwrap_or_else(|| "unknown bundle build failure".to_string());
            warn!(
                "Skipping locale change to '{}' because Fluent bundle assembly failed for '{}': {}",
                requested_language, candidate, diagnostics
            );
            return RequestedLanguageResolution::Blocked(candidate);
        }

        if ready_languages.contains(&candidate) {
            if candidate != *requested_language {
                info!(
                    "Locale '{}' not found, falling back to '{}'",
                    requested_language, candidate
                );
            }

            return RequestedLanguageResolution::Ready(candidate);
        }

        if available_languages.contains(&candidate) {
            if candidate != *requested_language {
                info!(
                    "Locale '{}' is not ready yet, waiting for available fallback '{}'",
                    requested_language, candidate
                );
            }

            return RequestedLanguageResolution::Pending(candidate);
        }
    }

    RequestedLanguageResolution::Immediate(requested_language.clone())
}

#[doc(hidden)]
pub(crate) fn handle_locale_changes(
    mut locale_change_events: MessageReader<LocaleChangeEvent>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    bundle_build_failures: Res<BundleBuildFailures>,
    mut current_language_id: ResMut<CurrentLanguageId>,
    mut pending_language_change: ResMut<PendingLanguageChange>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);
        let resolution = resolve_requested_language(
            &event.0,
            &i18n_bundle,
            &i18n_assets,
            &bundle_build_failures,
        );

        match resolution {
            RequestedLanguageResolution::Ready(resolved_language)
            | RequestedLanguageResolution::Immediate(resolved_language) => {
                pending_language_change.0 = None;
                apply_selected_language(
                    resolved_language,
                    &mut i18n_resource,
                    &mut current_language_id,
                    &mut locale_changed_events,
                );
            },
            RequestedLanguageResolution::Pending(resolved_language) => {
                if pending_language_change.0.as_ref() != Some(&resolved_language) {
                    info!(
                        "Deferring locale change to '{}' until its Fluent bundle is ready",
                        resolved_language
                    );
                }
                pending_language_change.0 = Some(resolved_language);
            },
            RequestedLanguageResolution::Blocked(blocked_language) => {
                if let Some(pending_language) = pending_language_change.0.take() {
                    info!(
                        "Clearing deferred locale change to '{}' because a later request for blocked locale '{}' superseded it",
                        pending_language, blocked_language
                    );
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set_bevy_i18n_state;
    use crate::test_support::lock_bevy_global_state;
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
        app.insert_resource(I18nBundle(HashMap::from([(en.clone(), Arc::new(bundle))])));
        app.insert_resource(BundleBuildFailures(HashMap::from([(
            fr.clone(),
            vec!["resource 'app': duplicate message id 'hello'".to_string()],
        )])));
        app.insert_resource(I18nResource::new(en.clone()));
        app.insert_resource(CurrentLanguageId(en.clone()));
        app.insert_resource(PendingLanguageChange::default());
        app.add_systems(Update, handle_locale_changes);

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };

        app.world_mut().write_message(LocaleChangeEvent(fr));
        app.update();

        assert_eq!(app.world().resource::<CurrentLanguageId>().0, en);
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
        app.insert_resource(I18nBundle(HashMap::from([(en.clone(), Arc::new(bundle))])));
        app.insert_resource(BundleBuildFailures::default());
        app.insert_resource(I18nResource::new(en.clone()));
        app.insert_resource(CurrentLanguageId(en.clone()));
        app.insert_resource(PendingLanguageChange::default());
        app.add_systems(Update, handle_locale_changes);

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };

        app.world_mut().write_message(LocaleChangeEvent(fr.clone()));
        app.update();

        assert_eq!(
            app.world().resource::<I18nResource>().current_language(),
            &en
        );
        assert_eq!(app.world().resource::<CurrentLanguageId>().0, en);
        assert_eq!(
            app.world().resource::<PendingLanguageChange>().0.as_ref(),
            Some(&fr)
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
