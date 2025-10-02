use crate::{LocaleChangedEvent, components::FluentText};
use bevy::prelude::*;
use es_fluent::ToFluentString;

/// System that updates `Text` components based on `FluentText` values.
/// This system handles both cases where `Text` is on the same entity
/// as `FluentText` or on a child entity.
pub fn update_fluent_text_system<T: ToFluentString + Clone + Component>(
    mut text_query: Query<&mut Text>,
    fluent_text_query: Query<(Entity, &FluentText<T>, Option<&Children>), Changed<FluentText<T>>>,
) {
    for (entity, fluent_text, children) in fluent_text_query.iter() {
        let new_text = fluent_text.value.to_fluent_string();

        // Try to update a Text component on the same entity
        if let Ok(mut text) = text_query.get_mut(entity) {
            info!("Updating direct text: {}", &new_text);
            // A text component is composed of sections.
            // We'll update the first section's value.
            *text = Text::from(new_text.clone());
        }

        // Try to update Text components on children
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

/// System that updates all `FluentText` components when the locale changes.
pub fn update_all_fluent_text_on_locale_change<T: ToFluentString + Clone + Component>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    mut query: Query<&mut FluentText<T>>,
) {
    for _event in locale_changed_events.read() {
        // When locale changes, we mark all FluentText components as changed
        // so they will be updated by the update_fluent_text_system
        for mut fluent_text in query.iter_mut() {
            // This is a workaround to force the Changed detection
            fluent_text.set_changed();
        }
    }
}
