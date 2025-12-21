# es-fluent-manager-bevy

The `es-fluent-manager-bevy` crate provides integration with the Bevy game engine for the `es-fluent` localization system. It offers a plugin, resources, and systems to manage loading and displaying translations for assets.

This crate is designed to work with asset-based localization, where translation files are loaded by Bevy's asset server.

## Compatible Bevy versions

The `master` branch is compatible with the latest Bevy release.

Compatibility of `es-fluent-manager-bevy` versions:

| `es-fluent-manager-bevy` | `bevy`   |
| :----------------------- | :------- |
| `0.17.x`                 | `0.17.x` |

## Usage

Here's a step-by-step guide to integrating `es-fluent-manager-bevy` into your application.

### 1. Define I18n Modules

In each of your crates that contains `EsFluent`-derived types, you need to define an i18n module. This macro discovers the languages from your `i18n` directory and registers the module with Bevy's asset system.

```rs
// In my_crate/src/lib.rs
use es_fluent::EsFluent;

// This macro discovers languages from your `i18n` directory and registers
// the module for Bevy's asset system.
es_fluent_manager_bevy::define_i18n_module!();

#[derive(Clone, Component, Copy, Debug, EsFluent)]
pub enum MyMessages {
    HelloWorld,
}
```

### 2. Add the Plugin to Your App

In your application's entry point, add the `I18nPlugin` and provide an initial language.

```rs
// In main.rs
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

// Also define a module for any EsFluent-derived items in your main crate.
es_fluent_manager_bevy::define_i18n_module!();

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en")))
        // ... other app setup
        .run();
}
```

### 3. Register Your Localized Components

For each component `T : es_fluent::ToFluentString` that you want to display using `FluentText<T>`, you need to register it with the app.

```rs
use es_fluent_manager_bevy::FluentTextRegistration as _;

// ... inside main()
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en")));

    // Register the component.
    app.register_fluent_text::<MyMessages>();

    app.add_systems(Startup, setup_system)
        .run();
}
```

If your component needs to be rebuilt when the language changes (e.g., it contains language-dependent data), you can implement `RefreshForLocale` and use `register_fluent_text_from_locale`.

### 4. Use `FluentText` in Your UI

Now you can use the `FluentText<T>` component to automatically display localized text. When the value of your component changes, the text will be updated automatically.

```rs
use es_fluent_manager_bevy::FluentText;

fn setup_system(mut commands: Commands) {
    commands.spawn((
        FluentText::new(ScreenMessages::Hello),
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}
```

## Example

- [bevy](https://github.com/stayhydated/es-fluent/tree/master/examples/bevy-example)
