use bevy::prelude::*;
use crate::EsFluentText;

/// Extension trait for working with EsFluentText components.
pub trait EsFluentTextExtension {
    /// Adds an EsFluentText component to an entity and links it to a Text component.
    fn insert_es_fluent_text<T: Clone + Component>(&mut self, value: T) -> &mut Self;
}

impl EsFluentTextExtension for EntityCommands<'_> {
    fn insert_es_fluent_text<T: Clone + Component>(&mut self, value: T) -> &mut Self {
        self.insert(EsFluentText::new(value))
    }
}