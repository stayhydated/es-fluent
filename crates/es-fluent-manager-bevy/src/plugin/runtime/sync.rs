use super::locale::apply_selected_language;
use crate::{
    ActiveLanguageId, I18nBundle, I18nDomainBundles, I18nResource, LocaleChangedEvent,
    PendingLanguageChange,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use unic_langid::LanguageIdentifier;

fn current_bundle_id(i18n_bundle: &I18nBundle, lang: &LanguageIdentifier) -> Option<usize> {
    i18n_bundle
        .get(lang)
        .map(|bundle| std::sync::Arc::as_ptr(bundle) as *const () as usize)
}

#[derive(SystemParam)]
pub(crate) struct SyncGlobalStateParams<'w, 's> {
    i18n_bundle: Res<'w, I18nBundle>,
    i18n_domain_bundles: Res<'w, I18nDomainBundles>,
    i18n_resource: ResMut<'w, I18nResource>,
    active_language_id: ResMut<'w, ActiveLanguageId>,
    pending_language_change: ResMut<'w, PendingLanguageChange>,
    locale_changed_events: MessageWriter<'w, LocaleChangedEvent>,
    redraw_events: MessageWriter<'w, RequestRedraw>,
    last_current_bundle: Local<'s, Option<(LanguageIdentifier, LanguageIdentifier, Option<usize>)>>,
}

#[doc(hidden)]
pub(crate) fn sync_global_state(mut params: SyncGlobalStateParams) {
    let current_lang = params.i18n_resource.active_language().clone();
    let current_resolved_lang = params.i18n_resource.resolved_language().clone();
    let current_bundle_ptr_id = current_bundle_id(&params.i18n_bundle, &current_resolved_lang);
    let current_bundle_present = current_bundle_ptr_id.is_some();
    let locale_switched = matches!(
        params.last_current_bundle.as_ref(),
        Some((previous_lang, previous_resolved_lang, _))
            if previous_lang != &current_lang
                || previous_resolved_lang != &current_resolved_lang
    );
    let current_bundle_changed = !matches!(
        params.last_current_bundle.as_ref(),
        Some((previous_lang, previous_resolved_lang, previous_bundle_id))
            if previous_lang == &current_lang
                && previous_resolved_lang == &current_resolved_lang
                && previous_bundle_id == &current_bundle_ptr_id
    );

    if params.i18n_bundle.is_changed() || params.i18n_domain_bundles.is_changed() {
        if let Some(pending_language) = params.pending_language_change.0.clone() {
            let pending_bundle_id =
                current_bundle_id(&params.i18n_bundle, &pending_language.resolved);
            if pending_bundle_id.is_some() {
                let published = apply_selected_language(
                    &pending_language,
                    &mut params.i18n_resource,
                    &mut params.active_language_id,
                    &mut params.locale_changed_events,
                );
                params.pending_language_change.0 = None;
                if published {
                    params.redraw_events.write(RequestRedraw);
                    *params.last_current_bundle = Some((
                        pending_language.requested,
                        pending_language.resolved,
                        pending_bundle_id,
                    ));
                } else {
                    *params.last_current_bundle =
                        Some((current_lang, current_resolved_lang, current_bundle_ptr_id));
                }
                return;
            }
        }

        if !locale_switched && current_bundle_changed && current_bundle_present {
            debug!("I18n bundle ready for current language: {}", current_lang);
            // Re-emit the active locale only after an accepted bundle exists for it,
            // so `RefreshForLocale` registrations refresh after async loads complete
            // and current-locale hot reloads, but not after rejected rebuilds.
            params
                .locale_changed_events
                .write(LocaleChangedEvent(current_lang.clone()));
            // Request a redraw so that UI updates even when using WinitSettings::desktop_app()
            params.redraw_events.write(RequestRedraw);
        }
    }

    *params.last_current_bundle =
        Some((current_lang, current_resolved_lang, current_bundle_ptr_id));
}
