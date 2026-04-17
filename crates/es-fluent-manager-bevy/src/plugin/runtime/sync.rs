use super::super::state::update_global_bundle;
use crate::{I18nAssets, I18nBundle, I18nDomainBundles, I18nResource, LocaleChangedEvent};
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use unic_langid::LanguageIdentifier;

fn current_bundle_id(i18n_bundle: &I18nBundle, lang: &LanguageIdentifier) -> Option<usize> {
    i18n_bundle
        .0
        .get(lang)
        .map(|bundle| std::sync::Arc::as_ptr(bundle) as *const () as usize)
}

#[doc(hidden)]
pub(crate) fn sync_global_state(
    i18n_bundle: Res<I18nBundle>,
    i18n_domain_bundles: Res<I18nDomainBundles>,
    i18n_assets: Res<I18nAssets>,
    i18n_resource: Res<I18nResource>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut redraw_events: MessageWriter<RequestRedraw>,
    mut last_current_bundle: Local<Option<(LanguageIdentifier, Option<usize>)>>,
) {
    let current_lang = i18n_resource.current_language().clone();
    let current_bundle_id = current_bundle_id(&i18n_bundle, &current_lang);
    let locale_switched = matches!(
        last_current_bundle.as_ref(),
        Some((previous_lang, _)) if previous_lang != &current_lang
    );
    let current_bundle_changed = !matches!(
        last_current_bundle.as_ref(),
        Some((previous_lang, previous_bundle_id))
            if previous_lang == &current_lang && previous_bundle_id == &current_bundle_id
    );

    if i18n_bundle.is_changed() || i18n_domain_bundles.is_changed() {
        update_global_bundle((*i18n_bundle).clone(), (*i18n_domain_bundles).clone());

        if !locale_switched
            && current_bundle_changed
            && i18n_assets.is_language_loaded(&current_lang)
        {
            debug!("I18n bundle ready for current language: {}", current_lang);
            // Re-emit the active locale once its bundle becomes available or changes,
            // so `RefreshForLocale` registrations refresh after async loads complete
            // and current-locale hot reloads.
            locale_changed_events.write(LocaleChangedEvent(current_lang.clone()));
            // Request a redraw so that UI updates even when using WinitSettings::desktop_app()
            redraw_events.write(RequestRedraw);
        }
    }

    *last_current_bundle = Some((current_lang, current_bundle_id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::lock_bevy_global_state;
    use crate::{FluentText, FluentTextRegistration, RefreshForLocale, ToFluentString};
    use bevy::ecs::message::Messages;
    use es_fluent_manager_core::ResourceKey;
    use fluent_bundle::FluentResource;
    use std::sync::Arc;
    use unic_langid::{LanguageIdentifier, langid};

    #[derive(Clone, Component, Debug, Eq, PartialEq)]
    struct RefreshableMessage(String);

    impl RefreshForLocale for RefreshableMessage {
        fn refresh_for_locale(&mut self, lang: &LanguageIdentifier) {
            self.0 = lang.to_string();
        }
    }

    impl ToFluentString for RefreshableMessage {
        fn to_fluent_string(&self) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn sync_global_state_re_emits_locale_changed_when_current_bundle_becomes_ready() {
        let _guard = lock_bevy_global_state();
        let lang = langid!("en");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(lang.clone(), "app".to_string(), Handle::default());

        app.add_message::<LocaleChangedEvent>();
        app.add_message::<RequestRedraw>();
        app.insert_resource(i18n_assets);
        app.insert_resource(I18nBundle::default());
        app.insert_resource(I18nDomainBundles::default());
        app.insert_resource(I18nResource::new(lang.clone()));
        app.register_fluent_text_from_locale::<RefreshableMessage>();
        app.add_systems(Update, sync_global_state);

        let entity = app
            .world_mut()
            .spawn((
                FluentText::new(RefreshableMessage("initial".to_string())),
                Text::new("old"),
            ))
            .id();

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };
        let mut redraw_cursor = {
            let messages = app.world().resource::<Messages<RequestRedraw>>();
            messages.get_cursor_current()
        };

        app.update();
        assert_eq!(
            &app.world().get::<Text>(entity).expect("text").0,
            "old",
            "text should stay untouched until the language is ready"
        );

        let resource = Arc::new(FluentResource::try_new("hello = hi".to_string()).expect("ftl"));
        app.world_mut()
            .resource_mut::<I18nAssets>()
            .loaded_resources
            .insert((lang.clone(), ResourceKey::new("app")), resource.clone());

        let mut bundle = fluent_bundle::bundle::FluentBundle::new_concurrent(vec![lang.clone()]);
        bundle.add_resource(resource).expect("add resource");
        app.world_mut()
            .resource_mut::<I18nBundle>()
            .0
            .insert(lang.clone(), Arc::new(bundle));

        app.update();

        let locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(locale_changes, vec![lang.clone()]);

        let redraw_count = {
            let messages = app.world().resource::<Messages<RequestRedraw>>();
            redraw_cursor.read(&messages).count()
        };
        assert_eq!(redraw_count, 1);

        let fluent_text = app
            .world()
            .get::<FluentText<RefreshableMessage>>(entity)
            .expect("fluent text");
        assert_eq!(fluent_text.value.0, "en");
        assert_eq!(&app.world().get::<Text>(entity).expect("text").0, "en");
    }

    #[test]
    fn sync_global_state_ignores_unrelated_bundle_changes() {
        let _guard = lock_bevy_global_state();
        let current_lang = langid!("en");
        let other_lang = langid!("fr");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(current_lang.clone(), "app".to_string(), Handle::default());
        i18n_assets.add_asset(other_lang.clone(), "app".to_string(), Handle::default());

        app.add_message::<LocaleChangedEvent>();
        app.add_message::<RequestRedraw>();
        app.insert_resource(i18n_assets);
        app.insert_resource(I18nBundle::default());
        app.insert_resource(I18nDomainBundles::default());
        app.insert_resource(I18nResource::new(current_lang.clone()));
        app.add_systems(Update, sync_global_state);

        let current_resource =
            Arc::new(FluentResource::try_new("hello = hi".to_string()).expect("ftl"));
        app.world_mut()
            .resource_mut::<I18nAssets>()
            .loaded_resources
            .insert(
                (current_lang.clone(), ResourceKey::new("app")),
                current_resource.clone(),
            );

        let mut current_bundle =
            fluent_bundle::bundle::FluentBundle::new_concurrent(vec![current_lang.clone()]);
        current_bundle
            .add_resource(current_resource)
            .expect("add resource");
        app.world_mut()
            .resource_mut::<I18nBundle>()
            .0
            .insert(current_lang.clone(), Arc::new(current_bundle));

        let mut locale_cursor = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            messages.get_cursor_current()
        };
        let mut redraw_cursor = {
            let messages = app.world().resource::<Messages<RequestRedraw>>();
            messages.get_cursor_current()
        };

        app.update();

        let initial_locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(initial_locale_changes, vec![current_lang.clone()]);

        let initial_redraw_count = {
            let messages = app.world().resource::<Messages<RequestRedraw>>();
            redraw_cursor.read(&messages).count()
        };
        assert_eq!(initial_redraw_count, 1);

        let other_resource =
            Arc::new(FluentResource::try_new("bonjour = salut".to_string()).expect("ftl"));
        app.world_mut()
            .resource_mut::<I18nAssets>()
            .loaded_resources
            .insert(
                (other_lang.clone(), ResourceKey::new("app")),
                other_resource.clone(),
            );

        let mut other_bundle =
            fluent_bundle::bundle::FluentBundle::new_concurrent(vec![other_lang.clone()]);
        other_bundle
            .add_resource(other_resource)
            .expect("add resource");
        app.world_mut()
            .resource_mut::<I18nBundle>()
            .0
            .insert(other_lang.clone(), Arc::new(other_bundle));

        app.update();

        let locale_changes = {
            let messages = app.world().resource::<Messages<LocaleChangedEvent>>();
            locale_cursor
                .read(&messages)
                .map(|message| message.0.clone())
                .collect::<Vec<_>>()
        };
        assert!(locale_changes.is_empty());

        let redraw_count = {
            let messages = app.world().resource::<Messages<RequestRedraw>>();
            redraw_cursor.read(&messages).count()
        };
        assert_eq!(redraw_count, 0);
    }
}
