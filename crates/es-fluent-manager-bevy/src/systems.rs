use crate::{I18nAssets, I18nBundle, I18nResource, LocaleChangedEvent, components::FluentText};
use bevy::prelude::*;
use es_fluent::ToFluentString;

/// Updates `Text` components based on changed `FluentText` values.
///
/// This system handles incremental updates when `FluentText` components change.
pub fn update_fluent_text_system<T: ToFluentString + Clone + Component>(
    mut text_query: Query<&mut Text>,
    fluent_text_query: Query<
        (Entity, &FluentText<T>, Option<&Children>),
        Or<(Added<FluentText<T>>, Changed<FluentText<T>>)>,
    >,
    i18n_assets: Res<I18nAssets>,
    i18n_resource: Res<I18nResource>,
) {
    if !i18n_assets.is_language_loaded(i18n_resource.current_language()) {
        return;
    }
    for (entity, fluent_text, children) in fluent_text_query.iter() {
        update_text_for_entity(&mut text_query, entity, children, &fluent_text.value);
    }
}

/// Marks all `FluentText<T>` components as changed when locale changes,
/// and performs a full refresh when the i18n bundle becomes ready.
pub fn update_all_fluent_text_on_locale_change<T: ToFluentString + Clone + Component>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    i18n_resource: Res<I18nResource>,
    mut text_query: Query<&mut Text>,
    fluent_text_query: Query<(Entity, &FluentText<T>, Option<&Children>)>,
    event_loop_proxy: Option<Res<bevy::winit::EventLoopProxyWrapper>>,
) {
    // Trigger update when locale changes via event OR when the bundle resource changes
    // (handles initial load where event may not propagate across schedule boundaries)
    let should_update = locale_changed_events.read().next().is_some() || i18n_bundle.is_changed();

    if should_update && i18n_assets.is_language_loaded(i18n_resource.current_language()) {
        // Perform a full update of all FluentText components
        for (entity, fluent_text, children) in fluent_text_query.iter() {
            update_text_for_entity(&mut text_query, entity, children, &fluent_text.value);
        }
        // Wake up the event loop to ensure UI updates are visible immediately,
        // especially when using WinitSettings::desktop_app() which only
        // redraws on input events.
        if let Some(proxy) = event_loop_proxy {
            let _ = proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
        }
    }
}

fn update_text_for_entity<T: ToFluentString>(
    text_query: &mut Query<&mut Text>,
    entity: Entity,
    children: Option<&Children>,
    value: &T,
) {
    let new_text = value.to_fluent_string();

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
