use bevy::prelude::*;
use es_fluent;
use es_fluent_manager_core::FluentManager;
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

#[derive(Event, Clone)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

#[derive(Event, Clone)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

/// A Bevy resource that holds the `FluentManager` for this application.
/// This resource is used for localization within Bevy systems.
#[derive(Resource)]
pub struct I18nResource {
    manager: Arc<RwLock<FluentManager>>,
}

impl I18nResource {
    /// Creates a new `I18nResource` with a `FluentManager` that has discovered modules.
    pub fn new_with_discovered_modules() -> Self {
        Self {
            manager: Arc::new(RwLock::new(FluentManager::new_with_discovered_modules())),
        }
    }

    /// Selects the language for all registered localizers.
    /// Errors from individual localizers are logged as warnings.
    pub fn select_language(&mut self, lang: &LanguageIdentifier) {
        let mut manager = self.manager.write()
            .expect("Failed to acquire write lock on FluentManager");
        manager.select_language(lang);
    }

    /// Attempts to localize a message ID with optional arguments.
    /// Returns the first successful localization from any localizer, or `None` if not found.
    pub fn localize<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String> {
        let manager = self.manager.read()
            .expect("Failed to acquire read lock on FluentManager");
        manager.localize(id, args)
    }
}

#[derive(Default)]
pub struct I18nPlugin {
    initial_language: LanguageIdentifier,
}

impl I18nPlugin {
    pub fn new(initial_language: LanguageIdentifier) -> Self {
        Self { initial_language }
    }
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        // Create and insert the Bevy resource
        let mut i18n_resource = I18nResource::new_with_discovered_modules();
        i18n_resource.select_language(&self.initial_language);

        // Set the global context for derive macros (shared reference)
        es_fluent::set_shared_context(i18n_resource.manager.clone());

        app.insert_resource(i18n_resource);

        info!("es-fluent I18nResource initialized for language '{}'", self.initial_language);

        app.add_event::<LocaleChangeEvent>()
           .add_event::<LocaleChangedEvent>()
           .add_systems(Update, handle_locale_changes);
    }
}

/// A system function that localizes a message ID with optional arguments using the `I18nResource`.
/// This function is intended to be used in Bevy systems that need to perform localization.
///
/// # Example
///
/// ```rust,ignore
/// use bevy::prelude::*;
/// use es_fluent_manager_bevy::{localize, I18nResource};
/// use fluent_bundle::FluentValue;
/// use std::collections::HashMap;
///
/// fn my_system(i18n_resource: Res<I18nResource>) {
///     let mut args = HashMap::new();
///     args.insert("name", FluentValue::from("Alice"));
///     let localized = localize(&i18n_resource, "greeting", Some(&args));
///     println!("Localized message: {}", localized);
/// }
/// ```
pub fn localize<'a>(
    i18n_resource: &I18nResource,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> String {
    i18n_resource.localize(id, args)
        .unwrap_or_else(|| {
            log::warn!("Translation for '{}' not found.", id);
            id.to_string()
        })
}

fn handle_locale_changes(
    mut reader: EventReader<LocaleChangeEvent>,
    mut writer: EventWriter<LocaleChangedEvent>,
    mut i18n_resource: ResMut<I18nResource>,
) {
    for event in reader.read() {
        // Use the Bevy resource to mutate the state.
        i18n_resource.select_language(&event.0);

        // The global context for derive macros is automatically updated since we're using
        // a shared Arc<RwLock<FluentManager>> that was set with set_shared_context

        writer.write(LocaleChangedEvent(event.0.clone()));
        info!("Locale changed to '{}'", event.0);
    }
}
