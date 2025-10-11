# es-fluent-manager-bevy

The `es-fluent-manager-bevy` crate provides integration with the Bevy game engine for the `es-fluent` localization system. It offers a plugin, resources, and systems to manage loading and displaying translations for assets.

This crate is designed to work with asset-based localization, where translation files are loaded by Bevy's asset server.

## Usage

```rs
use bevy::prelude::*;
use es_fluent_manager_bevy::{EsFluentBevyPlugin, I18nPlugin};
use strum::EnumIter;
use unic_langid::{LanguageIdentifier, langid};

// This macro discovers languages from your `i18n` directory and registers
// the module for Bevy's asset system.
es_fluent_manager_bevy::define_i18n_module!();

#[derive(Clone, Copy, Debug, Default, EnumIter, EsFluent, PartialEq, Component)]
pub enum Languages {
    #[default]
    English,
    French,
    Chinese,
}

impl From<Languages> for LanguageIdentifier {
    fn from(val: Languages) -> Self {
        match val {
            Languages::English => langid!("en"),
            Languages::French => langid!("fr"),
            Languages::Chinese => langid!("cn"),
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(Languages::default().into()))
        // ... other app setup
        .add_systems(Startup, setup_system)
        .run();
}
```
