use crate::LocaleChangedEvent;
use bevy::prelude::*;
use es_fluent::ToFluentString;

/// A component that holds a value that can be localized.
/// This component automatically updates the associated Text component
/// when the language changes or its value changes.
#[derive(Clone, Component)]
pub struct Localized<T: ToFluentString + Clone> {
    pub value: T,
}

impl<T: ToFluentString + Clone> Localized<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

/// System that updates Text components based on Localized values.
/// This system runs when Localized components are modified.
pub fn update_localized_text_system<T: ToFluentString + Clone + Component>(
    mut query: Query<(&Localized<T>, &mut Text), Changed<Localized<T>>>,
) {
    for (localized, mut text) in query.iter_mut() {
        let new_text = localized.value.to_fluent_string();
        info!("Updating direct text: {}", new_text);
        *text = Text::new(new_text);
    }
}

/// System that updates Text components based on Localized values in parent entities.
/// This system is useful when the Localized component is on a parent entity and the
/// Text component is on a child entity.
pub fn update_localized_text_parent_system<T: ToFluentString + Clone + Component>(
    mut parent_query: Query<(&Localized<T>, &Children), Changed<Localized<T>>>,
    mut text_query: Query<&mut Text>,
) {
    for (localized, children) in parent_query.iter_mut() {
        let new_text = localized.value.to_fluent_string();
        info!("Updating parent-child text: {}", new_text);
        for child in children {
            if let Ok(mut text) = text_query.get_mut(*child) {
                *text = Text::new(new_text.clone());
            }
        }
    }
}

/// System that updates all Localized components when the locale changes.
pub fn update_all_localized_text_on_locale_change<T: ToFluentString + Clone + Component>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    mut query: Query<&mut Localized<T>>,
) {
    for _event in locale_changed_events.read() {
        // When locale changes, we mark all Localized components as changed
        // so they will be updated by the update_localized_text_system
        for mut localized in query.iter_mut() {
            // This is a workaround to force the Changed detection
            localized.set_changed();
        }
    }
}
