# es-fluent-manager-bevy

The `es-fluent-manager-bevy` crate provides integration with the Bevy game engine for the `es-fluent` localization system. It offers a plugin, resources, and systems to manage loading and displaying translations for assets.

This crate is designed to work with asset-based localization, where translation files are loaded by Bevy's asset server.

## Features

-   **Bevy Plugin**: A simple plugin to initialize the localization system.
-   **Asset Loader**: An `AssetLoader` for `.ftl` files.
-   **Resources**: `I18nResource` for managing the current locale and `I18nBundle` for storing loaded Fluent bundles.
-   **Systems**: Systems to automatically update components that display localized text when the locale changes.
-   **Macro Helper**: A `define_bevy_i18n_module!` macro to automatically discover languages and set up the necessary module data for asset-based loading.

## Usage

1.  Add the `EsFluentBevyPlugin` to your Bevy app.
2.  Define your i18n module using the `define_bevy_i18n_module!` macro.
3.  Use the `I18nResource` and `localize` function to translate messages in your systems.
4.  Use components that implement `FluentText` to have their text automatically updated on locale change.

```rust,no_run
use bevy::prelude::*;
use es_fluent_manager_bevy::{EsFluentBevyPlugin, define_bevy_i18n_module, localize, I18nResource, I18nBundle};

// This macro discovers languages from your `i18n` directory and registers
// the module for Bevy's asset system.
define_bevy_i18n_module!();

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EsFluentBevyPlugin)
        // ... other app setup
        .add_systems(Startup, setup_system)
        .run();
}

fn setup_system(i18n_res: Res<I18nResource>, i18n_bundle: Res<I18nBundle>) {
    // Localize a message using the current locale
    let greeting = localize(&i18n_res, &i18n_bundle, "hello-world", None);
    println!("{}", greeting);
}
```
