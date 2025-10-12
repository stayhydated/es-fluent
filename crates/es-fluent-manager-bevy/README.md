# es-fluent-manager-bevy

The `es-fluent-manager-bevy` crate provides integration with the Bevy game engine for the `es-fluent` localization system. It offers a plugin, resources, and systems to manage loading and displaying translations for assets.

This crate is designed to work with asset-based localization, where translation files are loaded by Bevy's asset server.

## Compatible Bevy versions

The `master` branch is compatible with the latest Bevy release.

Compatibility of `es-fluent-manager-bevy` versions:

| `es-fluent-manager-bevy` | `bevy` |
| :----------------------- | :----- |
| `0.17`                   | `0.17` |

## Usage

1.  In each of your crates that has translations, define a bevy-specific i18n module:

```rs
// In my_crate/src/lib.rs
// This macro discovers languages from your `i18n` directory and registers
// the module for Bevy's asset system.
es_fluent_manager_bevy::define_i18n_module!();
```

2.  At the start of your application, add the plugin:

```rs
use bevy::prelude::*;
use es_fluent_manager_bevy::{EsFluentBevyPlugin, I18nPlugin};
use strum::EnumIter;
use unic_langid::{LanguageIdentifier, langid};

// This macro discovers languages from your `i18n` directory and registers
// the module for Bevy's asset system.
// In this case, for any EsFluent derived item included with your bevy application's entrypoint.
es_fluent_manager_bevy::define_i18n_module!();

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en-US")))
        // ... other app setup
        .add_systems(Startup, setup_system)
        .run();
}
```

## Example
- [bevy](../../examples/bevy-example)
