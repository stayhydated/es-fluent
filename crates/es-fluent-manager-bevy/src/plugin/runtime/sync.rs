use super::super::state::update_global_bundle;
use crate::{I18nAssets, I18nBundle, I18nResource, LocaleChangedEvent};
use bevy::prelude::*;
use bevy::window::RequestRedraw;

#[doc(hidden)]
pub(crate) fn sync_global_state(
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    i18n_resource: Res<I18nResource>,
    mut locale_changed_events: MessageWriter<LocaleChangedEvent>,
    mut redraw_events: MessageWriter<RequestRedraw>,
) {
    if i18n_bundle.is_changed() {
        update_global_bundle((*i18n_bundle).clone());

        if i18n_assets.is_language_loaded(i18n_resource.current_language()) {
            let lang = i18n_resource.current_language().clone();
            debug!("I18n bundle ready for current language: {}", lang);
            // Re-emit the active locale once its bundle is usable so
            // `RefreshForLocale` registrations refresh after async loads complete.
            locale_changed_events.write(LocaleChangedEvent(lang));
            // Request a redraw so that UI updates even when using WinitSettings::desktop_app()
            redraw_events.write(RequestRedraw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let lang = langid!("en");
        let mut app = App::new();
        let mut i18n_assets = I18nAssets::new();
        i18n_assets.add_asset(lang.clone(), "app".to_string(), Handle::default());

        app.add_message::<LocaleChangedEvent>();
        app.add_message::<RequestRedraw>();
        app.insert_resource(i18n_assets);
        app.insert_resource(I18nBundle::default());
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
}
