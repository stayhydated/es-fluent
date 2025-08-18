# es-fluent-manager-bevy

A Bevy-native internationalization (i18n) manager that integrates [es-fluent](../es-fluent/README.md) with [Bevy's asset system](https://bevy-engine.org/learn/book/assets/).

## Overview

This crate provides a Bevy plugin that enables runtime loading of Fluent (.ftl) translation files through Bevy's asset system, while maintaining full compatibility with the `#[derive(EsFluent)]` macros. This approach offers several advantages over compile-time embedding:

- 🔄 **Hot-reloading support** via Bevy's asset system
- 📦 **Runtime asset loading** instead of compile-time embedding  
- 🎯 **Native Bevy integration** with standard asset workflows
- 🔧 **Seamless derive macro support** - your code doesn't change
- 🌐 **Dynamic language switching** at runtime

## Quick Start

### 1. Add to Cargo.toml

```toml
[dependencies]
bevy = "0.16"
es-fluent = { version = "0.1", features = ["derive"] }
es-fluent-manager-bevy = "0.1"
unic-langid = "0.9"
```

### 2. Organize Your Assets

Place your .ftl files in your Bevy assets directory:

```
assets/
└── i18n/
    ├── en/
    │   └── main.ftl
    ├── fr/
    │   └── main.ftl
    └── cn/
        └── main.ftl
```

### 3. Set Up the Plugin

```rust
use bevy::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::{I18nPlugin, I18nPluginConfig};
use unic_langid::langid;

#[derive(EsFluent)]
enum MyMessages {
    HelloWorld,
    Welcome { name: String },
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_config(I18nPluginConfig {
            initial_language: langid!("en"),
            asset_path: "i18n".to_string(),
            domains: vec!["main".to_string()],
            supported_languages: vec![langid!("en"), langid!("fr"), langid!("cn")],
        }))
        .run();
}
```

### 4. Use EsFluent Derive Macros

The derive macros work exactly as before - no changes needed:

```rust
use es_fluent::{EsFluent, ToFluentString};

#[derive(EsFluent)]
enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

fn update_button_text(
    button_query: Query<&ButtonState, Changed<ButtonState>>,
    mut text_query: Query<&mut Text>,
) {
    if let Ok(button_state) = button_query.single() {
        if let Ok(mut text) = text_query.single_mut() {
            *text = Text::from(button_state.to_fluent_string());
        }
    }
}
```

### 5. Handle Language Changes

```rust
use es_fluent_manager_bevy::{LocaleChangeEvent, LocaleChangedEvent};

fn switch_language(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut locale_events: EventWriter<LocaleChangeEvent>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        locale_events.write(LocaleChangeEvent(langid!("fr")));
    }
}

fn handle_language_changed(
    mut events: EventReader<LocaleChangedEvent>,
    // ... update your UI components
) {
    for event in events.read() {
        info!("Language changed to: {}", event.0);
        // Update UI elements that need manual refresh
    }
}
```

## Asset Organization

### Directory Structure

The plugin expects assets to be organized as:
```
{asset_path}/{language_code}/{domain}.ftl
```

For example, with `asset_path = "i18n"` and domain `"main"`:
```
assets/i18n/en/main.ftl
assets/i18n/fr/main.ftl
assets/i18n/cn/main.ftl
```

### Multiple Domains

You can organize translations into multiple domains:

```rust
I18nPluginConfig {
    domains: vec!["ui".to_string(), "game".to_string(), "errors".to_string()],
    // ...
}
```

This creates:
```
assets/i18n/en/ui.ftl
assets/i18n/en/game.ftl  
assets/i18n/en/errors.ftl
```

## FTL File Format

Your .ftl files use standard [Fluent syntax](https://projectfluent.org/):

```fluent
# en/main.ftl
button_state-Normal = Normal
button_state-Hovered = Hovered  
button_state-Pressed = Pressed

my_messages-HelloWorld = Hello World!
my_messages-Welcome = Welcome, { $name }!
```

```fluent
# fr/main.ftl  
button_state-Normal = Normal
button_state-Hovered = Survolé
button_state-Pressed = Appuyé

my_messages-HelloWorld = Bonjour le monde !
my_messages-Welcome = Bienvenue, { $name } !
```

## Configuration Options

```rust
I18nPluginConfig {
    /// The initial language to use
    initial_language: langid!("en"),
    
    /// Asset path relative to the assets directory
    asset_path: "i18n".to_string(),
    
    /// Translation domains (corresponds to .ftl file names)
    domains: vec!["main".to_string()],
    
    /// All supported languages (must have corresponding asset files)
    supported_languages: vec![langid!("en"), langid!("fr")],
}
```

## Events

The plugin provides events for language management:

- **`LocaleChangeEvent`** - Send this to request a language change
- **`LocaleChangedEvent`** - Receive this when language has changed

```rust
// Request language change
locale_change_events.write(LocaleChangeEvent(langid!("fr")));

// Handle language change
fn on_locale_changed(mut events: EventReader<LocaleChangedEvent>) {
    for event in events.read() {
        println!("Language is now: {}", event.0);
    }
}
```

## Hot Reloading

Thanks to Bevy's asset system, translation files support hot-reloading in development builds. Simply edit your .ftl files and they'll be automatically reloaded!

## Migration from Compile-Time Approach

If you were previously using the compile-time embedding approach:

1. **Remove** `build.rs` and build dependencies
2. **Move** .ftl files from source directory to `assets/i18n/`  
3. **Replace** `I18nPlugin::new()` with `I18nPlugin::with_config()`
4. **Keep** all your `#[derive(EsFluent)]` code unchanged!

## Comparison with Other Approaches

| Feature | Asset-Based (This) | Compile-Time | Runtime File Loading |
|---------|-------------------|--------------|---------------------|
| Hot Reloading | ✅ | ❌ | ⚠️ Manual |
| Bundle Size | ✅ Smaller | ❌ Larger | ✅ Smaller |
| Startup Time | ⚠️ Async Load | ✅ Immediate | ⚠️ Sync Load |
| Bevy Integration | ✅ Native | ❌ External | ❌ External |
| Derive Macros | ✅ Full Support | ✅ Full Support | ❌ Limited |

## Advanced Usage

### Custom Asset Paths

```rust
// Load from a different assets subdirectory
I18nPlugin::with_config(I18nPluginConfig {
    asset_path: "localization/translations".to_string(),
    // ...
})
```

### Multiple Language Variants

```rust
// Support regional variants
supported_languages: vec![
    langid!("en-US"),
    langid!("en-GB"), 
    langid!("fr-FR"),
    langid!("fr-CA"),
],
```

### Runtime Language Detection

```rust
fn detect_system_language() -> LanguageIdentifier {
    // Implement your detection logic
    langid!("en")
}

let config = I18nPluginConfig {
    initial_language: detect_system_language(),
    // ...
};
```

## Examples

See the [bevy-example](../../examples/bevy-example/) for a complete working demonstration showing:

- UI text localization
- Dynamic language switching
- Multiple enum types with derived translations
- Button interaction with localized state changes

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.