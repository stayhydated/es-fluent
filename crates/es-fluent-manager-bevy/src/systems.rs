use crate::{BevyI18n, I18nAssets, LocaleChangedEvent, components::FluentText};
use bevy::prelude::*;
use es_fluent::FluentMessage;

/// Updates `Text` components based on changed `FluentText` values.
///
/// This system handles incremental updates when `FluentText` components change.
#[doc(hidden)]
pub fn update_fluent_text_system<T: FluentMessage + Clone + Send + Sync + 'static>(
    mut text_query: Query<&mut Text>,
    fluent_text_query: Query<
        (Entity, &FluentText<T>, Option<&Children>),
        Or<(Added<FluentText<T>>, Changed<FluentText<T>>)>,
    >,
    i18n_assets: Res<I18nAssets>,
    i18n: BevyI18n,
) {
    if !i18n_assets.is_language_loaded(i18n.resolved_language()) {
        return;
    }
    for (entity, fluent_text, children) in fluent_text_query.iter() {
        update_text_for_entity(&mut text_query, entity, children, &fluent_text.value, &i18n);
    }
}

/// Marks all `FluentText<T>` components as changed when locale changes,
/// and performs a full refresh when the i18n bundle becomes ready.
#[doc(hidden)]
pub fn update_all_fluent_text_on_locale_change<T: FluentMessage + Clone + Send + Sync + 'static>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    i18n: BevyI18n,
    i18n_assets: Res<I18nAssets>,
    mut text_query: Query<&mut Text>,
    fluent_text_query: Query<(Entity, &FluentText<T>, Option<&Children>)>,
    event_loop_proxy: Option<Res<bevy::winit::EventLoopProxyWrapper>>,
) {
    // Trigger update when locale changes via event OR when the bundle resource changes
    // (handles initial load where event may not propagate across schedule boundaries)
    let should_update = locale_changed_events.read().next().is_some() || i18n.is_bundle_changed();

    if should_update && i18n_assets.is_language_loaded(i18n.resolved_language()) {
        // Perform a full update of all FluentText components
        for (entity, fluent_text, children) in fluent_text_query.iter() {
            update_text_for_entity(&mut text_query, entity, children, &fluent_text.value, &i18n);
        }
        // Wake up the event loop to ensure UI updates are visible immediately,
        // especially when using WinitSettings::desktop_app() which only
        // redraws on input events.
        if let Some(proxy) = event_loop_proxy {
            let _ = proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
        }
    }
}

#[doc(hidden)]
fn update_text_for_entity<T: FluentMessage>(
    text_query: &mut Query<&mut Text>,
    entity: Entity,
    children: Option<&Children>,
    value: &T,
    i18n: &BevyI18n<'_>,
) {
    let new_text = i18n.localize_message(value);

    if let Ok(mut text) = text_query.get_mut(entity) {
        trace!("Updating direct text on {:?}: {}", entity, &new_text);
        **text = new_text.clone();
    }

    if let Some(children) = children {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                trace!("Updating child text on {:?}: {}", child, &new_text);
                **text = new_text.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActiveLanguageId, FtlAsset, I18nBundle, I18nDomainBundles, I18nResource,
        RequestedLanguageId,
    };
    use es_fluent_manager_core::{ResourceKey, SyncFluentBundle};
    use fluent_bundle::FluentResource;
    use std::collections::HashMap;
    use std::sync::Arc;
    use unic_langid::langid;

    #[derive(Clone)]
    struct FakeMessage(&'static str);

    impl FluentMessage for FakeMessage {
        fn to_fluent_string_with(
            &self,
            _localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&std::collections::HashMap<&str, es_fluent::FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            self.0.to_string()
        }
    }

    #[derive(Clone)]
    struct DomainLookupMessage {
        domain: &'static str,
        id: &'static str,
    }

    impl FluentMessage for DomainLookupMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&std::collections::HashMap<&str, es_fluent::FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize(self.domain, self.id, None)
        }
    }

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("valid FTL"))
    }

    fn bundle_for(
        lang: &unic_langid::LanguageIdentifier,
        resource: Arc<FluentResource>,
    ) -> Arc<SyncFluentBundle> {
        let mut bundle = SyncFluentBundle::new_concurrent(vec![lang.clone()]);
        bundle
            .add_resource(resource)
            .expect("resource should be valid");
        Arc::new(bundle)
    }

    fn loaded_assets_for(lang: unic_langid::LanguageIdentifier) -> I18nAssets {
        let mut assets = I18nAssets::new();
        assets.add_asset(
            lang.clone(),
            "app".to_string(),
            Handle::<FtlAsset>::default(),
        );
        assets
            .loaded_resources
            .insert((lang, ResourceKey::new("app")), resource("hello = hi"));
        assets
    }

    #[test]
    fn update_fluent_text_system_updates_direct_and_child_text() {
        let lang = langid!("en-US");
        let mut app = App::new();
        app.insert_resource(loaded_assets_for(lang.clone()));
        app.insert_resource(I18nResource::new(lang));
        app.insert_resource(I18nBundle::default());
        app.insert_resource(RequestedLanguageId(langid!("en-US")));
        app.insert_resource(ActiveLanguageId(langid!("en-US")));
        app.insert_resource(I18nDomainBundles::default());
        app.add_systems(Update, update_fluent_text_system::<FakeMessage>);

        let child = app.world_mut().spawn(Text::new("old child")).id();
        let parent = app
            .world_mut()
            .spawn((
                FluentText::new(FakeMessage("new text")),
                Text::new("old parent"),
            ))
            .add_child(child)
            .id();

        app.update();

        let parent_text = &app.world().get::<Text>(parent).expect("parent text").0;
        let child_text = &app.world().get::<Text>(child).expect("child text").0;
        assert_eq!(parent_text, "new text");
        assert_eq!(child_text, "new text");
    }

    #[test]
    fn update_all_fluent_text_on_locale_change_updates_all_entities() {
        let lang = langid!("en-US");
        let mut app = App::new();
        app.insert_resource(loaded_assets_for(lang.clone()));
        app.insert_resource(I18nResource::new(lang.clone()));
        app.insert_resource(RequestedLanguageId(lang.clone()));
        app.insert_resource(ActiveLanguageId(lang.clone()));
        app.insert_resource(I18nBundle::default());
        app.insert_resource(I18nDomainBundles::default());
        app.add_message::<LocaleChangedEvent>();
        app.add_systems(
            Update,
            update_all_fluent_text_on_locale_change::<FakeMessage>,
        );

        let entity = app
            .world_mut()
            .spawn((FluentText::new(FakeMessage("updated")), Text::new("old")))
            .id();

        app.world_mut().write_message(LocaleChangedEvent(lang));
        app.update();

        let text = &app.world().get::<Text>(entity).expect("text").0;
        assert_eq!(text, "updated");
    }

    #[test]
    fn fluent_text_uses_domain_aware_generated_message_lookup() {
        let requested = langid!("en-US");
        let parent = langid!("en");
        let app_exact = resource("title = App exact");
        let admin_exact = resource("title = Admin exact");
        let app_parent = resource("subtitle = Parent fallback");
        let mut assets = I18nAssets::new();

        for (lang, domain, resource) in [
            (requested.clone(), "app", app_exact.clone()),
            (requested.clone(), "admin", admin_exact.clone()),
            (parent.clone(), "app", app_parent.clone()),
        ] {
            assets.add_asset(
                lang.clone(),
                domain.to_string(),
                Handle::<FtlAsset>::default(),
            );
            assets
                .loaded_resources
                .insert((lang, ResourceKey::new(domain)), resource);
        }

        let mut domain_bundles = I18nDomainBundles::default();
        domain_bundles.set_bundles(
            requested.clone(),
            HashMap::from([
                ("app".to_string(), bundle_for(&requested, app_exact.clone())),
                (
                    "admin".to_string(),
                    bundle_for(&requested, admin_exact.clone()),
                ),
            ]),
        );
        domain_bundles.set_locale_resources(
            requested.clone(),
            HashMap::from([
                ("app".to_string(), vec![app_exact]),
                ("admin".to_string(), vec![admin_exact]),
            ]),
        );
        domain_bundles.set_bundles(
            parent.clone(),
            HashMap::from([("app".to_string(), bundle_for(&parent, app_parent.clone()))]),
        );
        domain_bundles.set_locale_resources(
            parent,
            HashMap::from([("app".to_string(), vec![app_parent])]),
        );

        let mut app = App::new();
        app.insert_resource(assets);
        app.insert_resource(I18nResource::new_with_resolved_language(
            requested.clone(),
            requested.clone(),
        ));
        app.insert_resource(I18nBundle::default());
        app.insert_resource(RequestedLanguageId(requested.clone()));
        app.insert_resource(ActiveLanguageId(requested));
        app.insert_resource(domain_bundles);
        app.add_systems(Update, update_fluent_text_system::<DomainLookupMessage>);

        let app_title = app
            .world_mut()
            .spawn((
                FluentText::new(DomainLookupMessage {
                    domain: "app",
                    id: "title",
                }),
                Text::new("old"),
            ))
            .id();
        let admin_title = app
            .world_mut()
            .spawn((
                FluentText::new(DomainLookupMessage {
                    domain: "admin",
                    id: "title",
                }),
                Text::new("old"),
            ))
            .id();
        let parent_fallback = app
            .world_mut()
            .spawn((
                FluentText::new(DomainLookupMessage {
                    domain: "app",
                    id: "subtitle",
                }),
                Text::new("old"),
            ))
            .id();
        let missing = app
            .world_mut()
            .spawn((
                FluentText::new(DomainLookupMessage {
                    domain: "app",
                    id: "missing",
                }),
                Text::new("old"),
            ))
            .id();

        app.update();

        assert_eq!(
            &app.world().get::<Text>(app_title).expect("app text").0,
            "App exact"
        );
        assert_eq!(
            &app.world().get::<Text>(admin_title).expect("admin text").0,
            "Admin exact"
        );
        assert_eq!(
            &app.world()
                .get::<Text>(parent_fallback)
                .expect("fallback text")
                .0,
            "Parent fallback"
        );
        assert_eq!(
            &app.world().get::<Text>(missing).expect("missing text").0,
            "missing"
        );
    }
}
