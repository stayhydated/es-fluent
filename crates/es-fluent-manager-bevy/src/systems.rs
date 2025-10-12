use crate::{I18nAssets, I18nResource, LocaleChangedEvent, components::FluentText};
use bevy::prelude::*;
use es_fluent::ToFluentString;

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
        let new_text = fluent_text.value.to_fluent_string();

        if let Ok(mut text) = text_query.get_mut(entity) {
            info!("Updating direct text: {}", &new_text);
            *text = Text::from(new_text.clone());
        }

        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    info!("Updating child text: {}", &new_text);
                    *text = Text::from(new_text.clone());
                }
            }
        }
    }
}

pub fn update_all_fluent_text_on_locale_change<T: ToFluentString + Clone + Component>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    mut query: Query<&mut FluentText<T>>,
) {
    for _event in locale_changed_events.read() {
        for mut fluent_text in query.iter_mut() {
            fluent_text.set_changed();
        }
    }
}
