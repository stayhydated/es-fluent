use bevy::prelude::*;

/// A plugin that initializes the `es-fluent` Bevy integration.
pub struct EsFluentBevyPlugin;

impl Plugin for EsFluentBevyPlugin {
    fn build(&self, _app: &mut App) {
        debug!("EsFluentBevyPlugin initialized");
    }
}

/// Trait for auto-registering FluentText systems with Bevy.
///
/// This trait is implemented by the `#[derive(EsFluent)]` macro when using
/// `#[fluent(bevy)]` or `#[fluent(bevy_locale)]` attributes.
pub trait BevyFluentTextRegistration: Send + Sync {
    /// Registers the FluentText systems for this type with the Bevy app.
    fn register(&self, app: &mut App);
}

inventory::collect!(&'static dyn BevyFluentTextRegistration);

/// An extension trait for `App` to simplify the registration of `FluentText` components.
pub trait FluentTextRegistration {
    /// Registers the necessary systems for a `FluentText<T>` component.
    fn register_fluent_text<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;

    /// Registers the necessary systems for a `FluentText<T>` component that
    /// also implements `RefreshForLocale`.
    ///
    /// This ensures that the component's value is updated when the locale changes.
    fn register_fluent_text_from_locale<
        T: es_fluent::ToFluentString
            + Clone
            + Component
            + crate::RefreshForLocale
            + Send
            + Sync
            + 'static,
    >(
        &mut self,
    ) -> &mut Self;
}

impl FluentTextRegistration for App {
    fn register_fluent_text<
        T: es_fluent::ToFluentString + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            PostUpdate,
            (
                crate::systems::update_all_fluent_text_on_locale_change::<T>,
                crate::systems::update_fluent_text_system::<T>,
            )
                .chain(),
        );
        self
    }

    fn register_fluent_text_from_locale<
        T: es_fluent::ToFluentString
            + Clone
            + Component
            + crate::RefreshForLocale
            + Send
            + Sync
            + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_systems(
            PostUpdate,
            (
                crate::update_values_on_locale_change::<T>,
                crate::systems::update_all_fluent_text_on_locale_change::<T>,
                crate::systems::update_fluent_text_system::<T>,
            )
                .chain(),
        );
        self
    }
}
