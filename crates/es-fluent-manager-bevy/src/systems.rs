use bevy::prelude::*;
use es_fluent::ToFluentString;
use crate::LocaleChangedEvent;

/// A component that holds a value implementing the EsFluent trait.
/// This component automatically updates the associated Text component
/// when the language changes.
#[derive(Component, Clone)]
pub struct EsFluentText<T: Clone> {
    pub value: T,
}

impl<T: Clone> EsFluentText<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

/// System that updates Text components based on EsFluentText values.
/// This system runs when EsFluentText components are modified.
pub fn update_es_fluent_text_system<T: ToFluentString + Clone + Component>(
    mut query: Query<(&EsFluentText<T>, &mut Text), Changed<EsFluentText<T>>>,
) {
    for (es_fluent_text, mut text) in query.iter_mut() {
        let new_text = es_fluent_text.value.to_fluent_string();
        info!("Updating direct text: {}", new_text);
        text.0 = new_text;
    }
}

/// System that updates Text components based on EsFluentText values in parent entities.
/// This system is useful when the EsFluentText component is on a parent entity and the
/// Text component is on a child entity.
pub fn update_es_fluent_text_parent_system<T: ToFluentString + Clone + Component>(
    mut parent_query: Query<(&EsFluentText<T>, &Children), Changed<EsFluentText<T>>>,
    mut text_query: Query<&mut Text>,
) {
    for (es_fluent_text, children) in parent_query.iter_mut() {
        let new_text = es_fluent_text.value.to_fluent_string();
        info!("Updating parent-child text: {}", new_text);
        for child in children {
            if let Ok(mut text) = text_query.get_mut(*child) {
                text.0 = new_text.clone();
            }
        }
    }
}

/// System that updates all EsFluentText components when the locale changes.
pub fn update_all_es_fluent_text_on_locale_change<T: ToFluentString + Clone + Component>(
    mut locale_changed_events: EventReader<LocaleChangedEvent>,
    mut query: Query<&mut EsFluentText<T>>,
) {
    for _event in locale_changed_events.read() {
        // When locale changes, we mark all EsFluentText components as changed
        // so they will be updated by the update_es_fluent_text_system
        for mut es_fluent_text in query.iter_mut() {
            // This is a workaround to force the Changed detection
            es_fluent_text.set_changed();
        }
    }
}