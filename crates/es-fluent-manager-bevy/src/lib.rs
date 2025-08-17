// In: crates/es-fluent-manager-bevy/src/lib.rs

use bevy::prelude::*;
use es_fluent::localization::{self, LocalizationContext};
use unic_langid::LanguageIdentifier;

#[derive(Event, Clone)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

#[derive(Event, Clone)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

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
        let mut context = LocalizationContext::new_with_discovered_modules();
        context.select_language(&self.initial_language);

        // This is the most important step for the Bevy integration.
        localization::set_context(context);

        info!("es-fluent context initialized and set for language '{}'", self.initial_language);

        app.add_event::<LocaleChangeEvent>()
           .add_event::<LocaleChangedEvent>()
           .add_systems(Update, handle_locale_changes);
    }
}

fn handle_locale_changes(
    mut reader: EventReader<LocaleChangeEvent>,
    mut writer: EventWriter<LocaleChangedEvent>,
) {
    for event in reader.read() {
        // Use the public `with_context` function to mutate the global state.
        localization::with_context(|ctx| {
            ctx.select_language(&event.0);
        });
        writer.write(LocaleChangedEvent(event.0.clone()));
        info!("Locale changed to '{}'", event.0);
    }
}
