use crate::{BevyI18n, I18nAssets, LocaleChangedMessage, components::FluentText};
use bevy::{prelude::*, window::RequestRedraw};
use es_fluent::FluentMessage;

/// Refreshes localized text after message changes, locale changes, and bundle reloads.
pub(crate) fn update_fluent_text_system<T: FluentMessage + Send + Sync + 'static>(
    mut text_query: Query<&mut Text>,
    fluent_text_query: Query<(Entity, Ref<FluentText<T>>, Option<&Children>)>,
    mut locale_changed_messages: MessageReader<LocaleChangedMessage>,
    i18n_assets: Res<I18nAssets>,
    i18n: BevyI18n,
    mut redraw_requests: Option<ResMut<Messages<RequestRedraw>>>,
) {
    let locale_changed = !locale_changed_messages.is_empty();
    locale_changed_messages.clear();

    if !i18n_assets.is_language_loaded(i18n.resolved_language()) {
        return;
    }

    let refresh_all = locale_changed || i18n.is_bundle_changed();
    let mut text_changed = false;
    for (entity, fluent_text, children) in &fluent_text_query {
        if refresh_all || fluent_text.is_changed() {
            text_changed |= update_text_for_entity(
                &mut text_query,
                entity,
                children,
                &fluent_text.value,
                &i18n,
            );
        }
    }

    if text_changed && let Some(redraw_requests) = redraw_requests.as_mut() {
        redraw_requests.write(RequestRedraw);
    }
}

fn update_text_for_entity<T: FluentMessage>(
    text_query: &mut Query<&mut Text>,
    entity: Entity,
    children: Option<&Children>,
    value: &T,
    i18n: &BevyI18n<'_>,
) -> bool {
    let new_text = i18n.localize_message(value);
    let mut changed = false;

    if let Ok(mut text) = text_query.get_mut(entity) {
        changed |= text.set_if_neq(Text::new(new_text.clone()));
    }

    if let Some(children) = children {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                changed |= text.set_if_neq(Text::new(new_text.clone()));
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActiveLanguageId, FluentResourceScope, FluentTextRegistration as _, FtlAsset,
        I18nDomainBundles, I18nReadyLocales, I18nResource, I18nResourceKey, RequestedLanguageId,
    };
    use es_fluent_manager_core::{
        ModuleResourceSpec, ResourceKey, StaticFluentDomain, SyncFluentBundle,
    };
    use fluent_bundle::FluentResource;
    use std::collections::HashMap;
    use std::sync::Arc;
    use unic_langid::langid;

    #[derive(Clone)]
    struct FakeMessage(&'static str);

    impl FluentMessage for FakeMessage {
        fn to_fluent_string_with(
            &self,
            _localize: &mut es_fluent::FluentMessageLookup<'_>,
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
            localize: &mut es_fluent::FluentMessageLookup<'_>,
        ) -> String {
            localize(
                es_fluent::registry::__macro::static_message_key(
                    "app",
                    es_fluent::registry::__macro::static_domain(self.domain),
                    es_fluent::registry::__macro::static_entry_id(self.id),
                ),
                None,
            )
        }
    }

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).expect("valid FTL"))
    }

    fn static_domain(value: &'static str) -> StaticFluentDomain {
        es_fluent_manager_core::__macro::static_domain(value)
    }

    fn scope(domain: &'static str) -> FluentResourceScope {
        FluentResourceScope::new(static_domain("app"), static_domain(domain))
    }

    fn resource_key(key: &'static str) -> I18nResourceKey {
        I18nResourceKey::new(static_domain("app"), ResourceKey::from_static_path(key))
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
            .insert((lang, resource_key("app")), resource("hello = hi"));
        assets
    }

    fn text_app() -> App {
        let lang = langid!("en-US");
        let mut app = App::new();
        app.insert_resource(loaded_assets_for(lang.clone()));
        app.insert_resource(I18nResource::new(lang.clone()));
        app.insert_resource(RequestedLanguageId(lang.clone()));
        app.insert_resource(ActiveLanguageId(lang));
        app.init_resource::<I18nReadyLocales>();
        app.init_resource::<I18nDomainBundles>();
        app.add_message::<LocaleChangedMessage>();
        app.add_message::<RequestRedraw>();
        app.register_fluent_text::<FakeMessage>();
        app
    }

    #[test]
    fn unchanged_output_preserves_text_change_ticks_and_does_not_request_redraw() {
        let mut app = text_app();
        let child = app.world_mut().spawn(Text::default()).id();
        let parent = app
            .world_mut()
            .spawn((FluentText::new(FakeMessage("hello")), Text::default()))
            .add_child(child)
            .id();
        app.update();
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        let parent_tick = app
            .world()
            .entity(parent)
            .get_ref::<Text>()
            .unwrap()
            .last_changed();
        let child_tick = app
            .world()
            .entity(child)
            .get_ref::<Text>()
            .unwrap()
            .last_changed();

        app.world_mut()
            .write_message(LocaleChangedMessage(langid!("en-US")));
        app.world_mut()
            .get_mut::<FluentText<FakeMessage>>(parent)
            .unwrap()
            .value = FakeMessage("hello");
        app.update();

        assert_eq!(
            app.world()
                .entity(parent)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            parent_tick
        );
        assert_eq!(
            app.world()
                .entity(child)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            child_tick
        );
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());

        app.world_mut()
            .get_mut::<FluentText<FakeMessage>>(parent)
            .unwrap()
            .value = FakeMessage("bonjour");
        app.update();
        assert_eq!(app.world().get::<Text>(parent).unwrap().0, "bonjour");
        assert_eq!(app.world().get::<Text>(child).unwrap().0, "bonjour");
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }

    #[test]
    fn locale_notifications_are_consumed_together_without_next_frame_refresh() {
        let mut app = text_app();
        let entity = app
            .world_mut()
            .spawn((FluentText::new(FakeMessage("hello")), Text::default()))
            .id();
        app.update();
        app.world_mut()
            .write_message(LocaleChangedMessage(langid!("fr")));
        app.world_mut()
            .write_message(LocaleChangedMessage(langid!("en-US")));
        app.update();

        app.world_mut().get_mut::<Text>(entity).unwrap().0 = "external override".to_owned();
        app.update();
        assert_eq!(
            app.world().get::<Text>(entity).unwrap().0,
            "external override"
        );
    }

    #[test]
    fn update_fluent_text_system_updates_direct_and_child_text() {
        let lang = langid!("en-US");
        let mut app = App::new();
        app.insert_resource(loaded_assets_for(lang.clone()));
        app.insert_resource(I18nResource::new(lang));
        app.insert_resource(I18nReadyLocales::default());
        app.insert_resource(RequestedLanguageId(langid!("en-US")));
        app.insert_resource(ActiveLanguageId(langid!("en-US")));
        app.insert_resource(I18nDomainBundles::default());
        app.add_message::<LocaleChangedMessage>();
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
    fn locale_change_refreshes_all_fluent_text_entities() {
        let lang = langid!("en-US");
        let mut app = App::new();
        app.insert_resource(loaded_assets_for(lang.clone()));
        app.insert_resource(I18nResource::new(lang.clone()));
        app.insert_resource(RequestedLanguageId(lang.clone()));
        app.insert_resource(ActiveLanguageId(lang.clone()));
        app.insert_resource(I18nReadyLocales::default());
        app.insert_resource(I18nDomainBundles::default());
        app.add_message::<LocaleChangedMessage>();
        app.add_systems(Update, update_fluent_text_system::<FakeMessage>);

        let entity = app
            .world_mut()
            .spawn((FluentText::new(FakeMessage("updated")), Text::new("old")))
            .id();

        app.world_mut().write_message(LocaleChangedMessage(lang));
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
            assets.add_asset_spec(
                static_domain("app"),
                lang.clone(),
                ModuleResourceSpec::base(domain, true),
                Handle::<FtlAsset>::default(),
            );
            assets
                .loaded_resources
                .insert((lang, resource_key(domain)), resource);
        }

        let mut domain_bundles = I18nDomainBundles::default();
        domain_bundles.set_bundles(
            requested.clone(),
            HashMap::from([
                (scope("app"), bundle_for(&requested, app_exact.clone())),
                (scope("admin"), bundle_for(&requested, admin_exact.clone())),
            ]),
        );
        domain_bundles.set_locale_resources(
            requested.clone(),
            HashMap::from([
                (scope("app"), vec![app_exact]),
                (scope("admin"), vec![admin_exact]),
            ]),
        );
        domain_bundles.set_bundles(
            parent.clone(),
            HashMap::from([(scope("app"), bundle_for(&parent, app_parent.clone()))]),
        );
        domain_bundles
            .set_locale_resources(parent, HashMap::from([(scope("app"), vec![app_parent])]));

        let mut app = App::new();
        app.insert_resource(assets);
        app.insert_resource(I18nResource::new_with_resolved_language(
            requested.clone(),
            requested.clone(),
        ));
        app.insert_resource(I18nReadyLocales::default());
        app.insert_resource(RequestedLanguageId(requested.clone()));
        app.insert_resource(ActiveLanguageId(requested));
        app.insert_resource(domain_bundles);
        app.add_message::<LocaleChangedMessage>();
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
    }
}
