use super::super::state::update_global_language;
use crate::{
    CurrentLanguageId, I18nAssets, I18nBundle, I18nResource, LocaleChangeEvent, LocaleChangedEvent,
};
use bevy::prelude::*;
use es_fluent_manager_core::resolve_ready_locale;
use unic_langid::LanguageIdentifier;

fn apply_selected_language(
    resolved_language: LanguageIdentifier,
    i18n_resource: &mut I18nResource,
    current_language_id: &mut CurrentLanguageId,
    locale_changed_events: &mut MessageWriter<LocaleChangedEvent>,
) {
    if i18n_resource.current_language() == &resolved_language
        && current_language_id.0 == resolved_language
    {
        return;
    }

    i18n_resource.set_language(resolved_language.clone());
    update_global_language(resolved_language.clone());
    current_language_id.0 = resolved_language.clone();
    locale_changed_events.write(LocaleChangedEvent(resolved_language));
}

fn resolve_requested_language(
    requested_language: &LanguageIdentifier,
    i18n_bundle: &I18nBundle,
    i18n_assets: &I18nAssets,
) -> LanguageIdentifier {
    let available_languages = i18n_assets.available_languages();
    let ready_languages = i18n_bundle.0.keys().cloned().collect::<Vec<_>>();
    let resolved_language =
        resolve_ready_locale(requested_language, &ready_languages, &available_languages)
            .unwrap_or_else(|| requested_language.clone());

    if resolved_language != *requested_language {
        info!(
            "Locale '{}' not found, falling back to '{}'",
            requested_language, resolved_language
        );
    }

    resolved_language
}

#[doc(hidden)]
pub(crate) fn handle_locale_changes(
    mut locale_change_events: MessageReader<LocaleChangeEvent>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    mut current_language_id: ResMut<CurrentLanguageId>,
) {
    for event in locale_change_events.read() {
        info!("Changing locale to: {}", event.0);
        let resolved_language = resolve_requested_language(&event.0, &i18n_bundle, &i18n_assets);
        apply_selected_language(
            resolved_language,
            &mut i18n_resource,
            &mut current_language_id,
            &mut locale_changed_events,
        );
    }
}
