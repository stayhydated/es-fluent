[![Docs](https://docs.rs/es-fluent-manager-bevy/badge.svg)](https://docs.rs/es-fluent-manager-bevy/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-bevy.svg)](https://crates.io/crates/es-fluent-manager-bevy)

# es-fluent-manager-bevy

Seamless [Bevy](https://bevyengine.org/) integration for `es-fluent`.

This plugin connects `es-fluent`'s type-safe localization with Bevy's ECS and Asset system. It allows you to use standard `#[derive(EsFluent)]` types as components that automatically update when the app/game's language changes.

| `es-fluent-manager-bevy` | `bevy`   |
| :----------------------- | :------- |
| **crates.io**            |          |
| `0.18.x`                 | `0.18.x` |
| `0.17.x`                 | `0.17.x` |

## Features

- **Asset Loading**: Loads `.ftl` files via Bevy's `AssetServer`.
- **Hot Reloading**: Supports hot-reloading of translations during development.
- **Reactive UI**: The `FluentText` component automatically refreshes text when the locale changes.
- **Global Hook**: Integrates with `es-fluent`'s global state.

## Quick Start

### 1. Setup

Add the plugin to your `App` and define your I18n module:

```rs
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

// a i18n.toml file must exist in the root of the crate
es_fluent_manager_bevy::define_i18n_module!();

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Initialize with default language
        .add_plugins(I18nPlugin::with_language(langid!("en-US")))
        .run();
}
```

### 2. Using in UI

Use the `FluentText` component wrapper for any type that implements `ToFluentString` (which `#[derive(EsFluent)]` provides).

```rs
use es_fluent::EsFluent;
use es_fluent_manager_bevy::FluentText;

#[derive(EsFluent, Clone, Component)]
pub enum UiMessage {
    StartGame,
    Settings,
}

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        // This text will automatically update if language changes
        FluentText::new(UiMessage::StartGame),
        Text::new(""),
    ));
}
```

### 3. Registering Components

For `FluentText` to work, you must register the specific inner type with the app so the plugin knows to update it:

```rs
app.register_fluent_text::<UiMessage>();
```

## es-fluent-lang integration

If your type contains fields that use [es_fluent_lang](../es-fluent-lang/README.md) (for the `Languages` enum), you must implement `RefreshForLocale` and use `register_fluent_text_from_locale` instead:

```rs
use es_fluent_manager_bevy::RefreshForLocale;

#[derive(EsFluent, Clone, Component)]
pub enum UiMessage {
    LanguageHint { current_language: Languages },
}

impl RefreshForLocale for UiMessage {
    fn refresh_for_locale(&mut self, lang: &unic_langid::LanguageIdentifier) {
        match self {
            UiMessage::LanguageHint { current_language } => {
                *current_language = Languages::from(lang);
            }
        }
    }
}

// Register with:
app.register_fluent_text_from_locale::<UiMessage>();
```
