# es-fluent-manager-bevy

Seamless [Bevy](https://bevyengine.org/) integration for `es-fluent`.

This plugin connects `es-fluent`'s type-safe localization with Bevy's ECS and Asset system. It allows you to use standard `#[derive(EsFluent)]` types as components that automatically update when the game's language changes.

## Features

- **Asset Loading**: Loads `.ftl` files via Bevy's `AssetServer`.
- **Hot Reloading**: Supports hot-reloading of translations during development.
- **Reactive UI**: The `FluentText` component automatically refreshes text when the locale changes.
- **Global Hook**: Integrates with `es-fluent`'s global state.

## Quick Start

### 1. Setup

Add the plugin to your `App` and define your I18n module:

```rust
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

```rust
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

```rust
app.register_fluent_text::<UiMessage>();
```
