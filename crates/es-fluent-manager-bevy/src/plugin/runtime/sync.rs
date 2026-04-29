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
pub(crate) struct SyncLocaleStateParams<'w, 's> {
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
pub(crate) fn sync_locale_state(mut params: SyncLocaleStateParams) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_manager_core::SyncFluentBundle;
    use std::sync::Arc;
    use unic_langid::langid;

    #[derive(Default, Resource)]
    struct ObservedSyncEvents {
        locale_changed: Vec<LanguageIdentifier>,
        redraw_count: usize,
    }

    fn bundle_for(lang: &LanguageIdentifier) -> Arc<SyncFluentBundle> {
        Arc::new(SyncFluentBundle::new_concurrent(vec![lang.clone()]))
    }

    fn observe_sync_events(
        mut locale_changed_events: MessageReader<LocaleChangedEvent>,
        mut redraw_events: MessageReader<RequestRedraw>,
        mut observed: ResMut<ObservedSyncEvents>,
    ) {
        observed
            .locale_changed
            .extend(locale_changed_events.read().map(|event| event.0.clone()));
        observed.redraw_count += redraw_events.read().count();
    }

    fn app_with_sync_systems(
        i18n_bundle: I18nBundle,
        i18n_domain_bundles: I18nDomainBundles,
        i18n_resource: I18nResource,
        active_language_id: ActiveLanguageId,
        pending_language_change: PendingLanguageChange,
    ) -> App {
        let mut app = App::new();
        app.add_message::<LocaleChangedEvent>()
            .add_message::<RequestRedraw>()
            .insert_resource(i18n_bundle)
            .insert_resource(i18n_domain_bundles)
            .insert_resource(i18n_resource)
            .insert_resource(active_language_id)
            .insert_resource(pending_language_change)
            .insert_resource(ObservedSyncEvents::default())
            .add_systems(Update, (sync_locale_state, observe_sync_events).chain());
        app
    }

    #[test]
    fn current_bundle_id_tracks_present_bundle_identity() {
        let lang = langid!("en");
        let mut i18n_bundle = I18nBundle::default();
        assert_eq!(current_bundle_id(&i18n_bundle, &lang), None);

        let bundle = Arc::new(SyncFluentBundle::new_concurrent(vec![lang.clone()]));
        let expected_id = Arc::as_ptr(&bundle) as *const () as usize;
        i18n_bundle.set_bundle(lang.clone(), bundle);

        assert_eq!(current_bundle_id(&i18n_bundle, &lang), Some(expected_id));
    }

    #[test]
    fn sync_locale_state_emits_locale_changed_and_redraw_when_current_bundle_becomes_ready() {
        let lang = langid!("en");
        let mut i18n_bundle = I18nBundle::default();
        i18n_bundle.set_bundle(lang.clone(), bundle_for(&lang));

        let mut app = app_with_sync_systems(
            i18n_bundle,
            I18nDomainBundles::default(),
            I18nResource::new(lang.clone()),
            ActiveLanguageId(lang.clone()),
            PendingLanguageChange::default(),
        );

        app.update();

        let observed = app.world().resource::<ObservedSyncEvents>();
        assert_eq!(observed.locale_changed, vec![lang]);
        assert_eq!(observed.redraw_count, 1);
    }

    #[test]
    fn sync_locale_state_publishes_pending_language_once_bundle_is_ready() {
        let en = langid!("en");
        let fr = langid!("fr");
        let mut i18n_bundle = I18nBundle::default();
        i18n_bundle.set_bundle(fr.clone(), bundle_for(&fr));

        let mut app = app_with_sync_systems(
            i18n_bundle,
            I18nDomainBundles::default(),
            I18nResource::new(en.clone()),
            ActiveLanguageId(en),
            PendingLanguageChange(Some(crate::LanguageSelection::new(fr.clone(), fr.clone()))),
        );

        app.update();

        assert_eq!(
            app.world().resource::<I18nResource>().active_language(),
            &fr
        );
        assert_eq!(app.world().resource::<ActiveLanguageId>().0, fr);
        assert!(app.world().resource::<PendingLanguageChange>().0.is_none());
        let observed = app.world().resource::<ObservedSyncEvents>();
        assert_eq!(observed.locale_changed, vec![fr]);
        assert_eq!(observed.redraw_count, 1);
    }
}
