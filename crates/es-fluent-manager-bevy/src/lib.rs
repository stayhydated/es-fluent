use bevy::prelude::*;
use es_fluent_manager::I18N_MANAGER;
use i18n_embed::unic_langid::LanguageIdentifier;

pub use es_fluent_manager::{I18nManager, I18nManagerError, I18nModule};

#[cfg(feature = "macros")]
pub use es_fluent_manager::register_i18n_module;

#[derive(Default, Resource)]
pub struct I18nResource {
    pub requested_languages: Vec<String>,
    pub pending_locale_change: Option<String>,
}

#[derive(Event)]
pub struct LocaleChangeEvent {
    pub locale: String,
}

#[derive(Event)]
pub struct LocaleChangedEvent {
    pub locale: String,
}

pub struct I18nPlugin {
    pub default_languages: Vec<String>,
}

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(I18nResource {
            requested_languages: self.default_languages.clone(),
            pending_locale_change: None,
        })
        .add_event::<LocaleChangeEvent>()
        .add_event::<LocaleChangedEvent>()
        .add_systems(Startup, initialize_i18n_system)
        .add_systems(Update, handle_locale_changes_system);
    }
}

fn initialize_i18n_system(i18n_resource: Res<I18nResource>) {
    let language_ids: Result<Vec<LanguageIdentifier>, _> = i18n_resource
        .requested_languages
        .iter()
        .map(|lang| lang.parse())
        .collect();

    match language_ids {
        Ok(langs) => {
            if let Err(e) = I18N_MANAGER.init_all(&langs) {
                error!("Failed to initialize I18n modules: {}", e);
            } else {
                info!("I18n modules initialized successfully");
            }
        },
        Err(e) => {
            error!("Failed to parse language identifiers: {}", e);
        },
    }
}

fn handle_locale_changes_system(
    mut locale_change_events: EventReader<LocaleChangeEvent>,
    mut locale_changed_events: EventWriter<LocaleChangedEvent>,
    //mut i18n_resource: ResMut<I18nResource>,
) {
    for event in locale_change_events.read() {
        match I18N_MANAGER.change_locale_all(&event.locale) {
            Ok(()) => {
                info!("Locale changed to: {}", event.locale);
                locale_changed_events.write(LocaleChangedEvent {
                    locale: event.locale.clone(),
                });
            },
            Err(e) => {
                error!("Failed to change locale to {}: {}", event.locale, e);
            },
        }
    }
}

pub fn change_locale(locale: &str, event_writer: &mut EventWriter<LocaleChangeEvent>) {
    event_writer.write(LocaleChangeEvent {
        locale: locale.to_string(),
    });
}

pub fn get_current_locale() -> Option<String> {
    I18N_MANAGER.current_language()
}
