use bevy::prelude::*;

/// Plugin that adds EsFluent support to Bevy applications.
/// This plugin automatically updates text components when the language changes.
pub struct EsFluentBevyPlugin;

impl Plugin for EsFluentBevyPlugin {
    fn build(&self, _app: &mut App) {
        // The plugin doesn't add any systems by default
        // Users can register specific types using the register_es_fluent_type method
        info!("EsFluentBevyPlugin initialized");
    }
}

/// A trait for registering specific type systems
pub trait EsFluentTypeRegistration {
    fn register_es_fluent_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;

    /// Register a type with parent-child text update support
    fn register_es_fluent_parent_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;
}

impl EsFluentTypeRegistration for App {
    fn register_es_fluent_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            Update,
            (
                crate::systems::update_all_es_fluent_text_on_locale_change::<T>,
                crate::systems::update_es_fluent_text_system::<T>,
            )
                .chain(),
        );
        self
    }

    fn register_es_fluent_parent_type<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            Update,
            (
                crate::systems::update_all_es_fluent_text_on_locale_change::<T>,
                crate::systems::update_es_fluent_text_parent_system::<T>,
            )
                .chain(),
        );
        self
    }
}
