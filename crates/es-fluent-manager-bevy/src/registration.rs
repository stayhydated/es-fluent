use bevy::prelude::*;
use std::{any::TypeId, collections::HashSet};

#[derive(Default, Resource)]
pub(crate) struct RegisteredFluentTextTypes {
    text_systems: HashSet<TypeId>,
    locale_refresh_systems: HashSet<TypeId>,
}

impl RegisteredFluentTextTypes {
    fn register_text_systems<T: 'static>(&mut self) -> bool {
        self.text_systems.insert(TypeId::of::<T>())
    }

    fn register_locale_refresh<T: 'static>(&mut self) -> bool {
        self.locale_refresh_systems.insert(TypeId::of::<T>())
    }

    #[cfg(test)]
    pub(crate) fn text_system_count(&self) -> usize {
        self.text_systems.len()
    }

    #[cfg(test)]
    pub(crate) fn locale_refresh_system_count(&self) -> usize {
        self.locale_refresh_systems.len()
    }
}

fn mark_text_systems_registered<T: 'static>(app: &mut App) -> bool {
    app.init_resource::<RegisteredFluentTextTypes>();
    app.world_mut()
        .resource_mut::<RegisteredFluentTextTypes>()
        .register_text_systems::<T>()
}

fn mark_locale_refresh_registered<T: 'static>(app: &mut App) -> bool {
    app.init_resource::<RegisteredFluentTextTypes>();
    app.world_mut()
        .resource_mut::<RegisteredFluentTextTypes>()
        .register_locale_refresh::<T>()
}

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
        T: es_fluent::FluentMessage + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self;

    /// Registers the necessary systems for a `FluentText<T>` component that
    /// also implements `RefreshForLocale`.
    ///
    /// This ensures that the component's value is updated when the locale changes.
    fn register_fluent_text_from_locale<
        T: es_fluent::FluentMessage
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
        T: es_fluent::FluentMessage + Clone + Component + Send + Sync + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        if mark_text_systems_registered::<T>(self) {
            self.add_systems(
                PostUpdate,
                (
                    crate::systems::update_all_fluent_text_on_locale_change::<T>,
                    crate::systems::update_fluent_text_system::<T>,
                )
                    .chain(),
            );
        }
        self
    }

    fn register_fluent_text_from_locale<
        T: es_fluent::FluentMessage
            + Clone
            + Component
            + crate::RefreshForLocale
            + Send
            + Sync
            + 'static,
    >(
        &mut self,
    ) -> &mut Self {
        let should_register_locale_refresh = mark_locale_refresh_registered::<T>(self);
        let should_register_text_systems = mark_text_systems_registered::<T>(self);

        match (should_register_locale_refresh, should_register_text_systems) {
            (true, true) => {
                self.add_systems(
                    PostUpdate,
                    (
                        crate::update_values_on_locale_change::<T>,
                        crate::systems::update_all_fluent_text_on_locale_change::<T>,
                        crate::systems::update_fluent_text_system::<T>,
                    )
                        .chain(),
                );
            },
            (true, false) => {
                self.add_systems(
                    PostUpdate,
                    crate::update_values_on_locale_change::<T>
                        .before(crate::systems::update_all_fluent_text_on_locale_change::<T>),
                );
            },
            (false, true) => {
                self.add_systems(
                    PostUpdate,
                    (
                        crate::systems::update_all_fluent_text_on_locale_change::<T>,
                        crate::systems::update_fluent_text_system::<T>,
                    )
                        .chain(),
                );
            },
            (false, false) => {},
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RefreshForLocale;
    use es_fluent::{FluentMessage, FluentValue};
    use std::collections::HashMap;
    use unic_langid::{LanguageIdentifier, langid};

    #[derive(Clone, Component)]
    struct RefreshableMessage;

    impl RefreshForLocale for RefreshableMessage {
        fn refresh_for_locale(&mut self, _lang: &LanguageIdentifier) {}
    }

    impl FluentMessage for RefreshableMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize("registration-test", "refreshable", None)
        }
    }

    #[test]
    fn register_from_locale_adds_text_systems_when_locale_refresh_already_exists() {
        let mut app = App::new();
        let mut message = RefreshableMessage;
        let mut localize =
            |_domain: &str, _id: &str, _args: Option<&HashMap<&str, FluentValue<'_>>>| {
                "unused".to_string()
            };

        message.refresh_for_locale(&langid!("en-US"));
        assert_eq!(message.to_fluent_string_with(&mut localize), "unused");
        assert!(mark_locale_refresh_registered::<RefreshableMessage>(
            &mut app
        ));
        app.register_fluent_text_from_locale::<RefreshableMessage>();

        let registered = app.world().resource::<RegisteredFluentTextTypes>();
        assert_eq!(registered.text_system_count(), 1);
        assert_eq!(registered.locale_refresh_system_count(), 1);
    }
}
