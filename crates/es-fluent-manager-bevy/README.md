[![Docs](https://docs.rs/es-fluent-manager-bevy/badge.svg)](https://docs.rs/es-fluent-manager-bevy/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-bevy.svg)](https://crates.io/crates/es-fluent-manager-bevy)

# es-fluent-manager-bevy

[online demo](https://stayhydated.github.io/es-fluent/bevy-example/)

Seamless [Bevy](https://bevyengine.org/) integration for `es-fluent`.

This plugin connects `es-fluent`'s type-safe localization with Bevy's ECS and Asset system. It allows standard `#[derive(EsFluent)]` types to serve as components that automatically update when the app/game's language changes.

| `es-fluent-manager-bevy` | `bevy`   |
| :----------------------- | :------- |
| **crates.io**            |          |
| `0.18.x`                 | `0.18.x` |
| `0.17.x`                 | `0.17.x` |

## Features

- **Asset Loading**: Loads `.ftl` files via Bevy's `AssetServer`.
- **Hot Reloading**: Supports hot-reloading of translations during development.
- **Reactive UI**: The `FluentText` component automatically refreshes text when the locale changes.
- **Bevy-native Context**: Systems can request `BevyI18n` as a `SystemParam` for direct localization.
- **Explicit Context**: Localization comes from Bevy resources instead of a context-free bridge.

## Quick Start

### 1. Define the Module

In your crate root (`lib.rs` or `main.rs`), tell the manager to scan your assets:

```rs
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_bevy::define_i18n_module!();
```

### 2. Initialize & Use

Add the plugin to your `App` and define your I18n module:

```rs
use bevy::prelude::*;
use es_fluent_manager_bevy::I18nPlugin;
use unic_langid::langid;

es_fluent_manager_bevy::define_i18n_module!();

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(I18nPlugin::with_language(langid!("en")))
        .run();
}
```

Plugin startup uses strict module discovery, so invalid or duplicate
registrations are reported through `I18nPluginStartupError` instead of being
normalized silently. When setup fails, the plugin skips localization runtime
setup and leaves the error resource in the app world for diagnostics. Failed hot
reloads or locale switches keep the last accepted locale active instead of
publishing a broken update.

Use `RequestedLanguageId` to read the latest user intent and `ActiveLanguageId`
to read the currently published locale. `LocaleChangedEvent` refers to
`ActiveLanguageId`, not merely the latest request. When a requested locale
falls back to a resolved locale, Bevy publishes the requested locale for change
events and ECS resources while using the resolved locale for ready bundle
lookup. Runtime fallback managers are best-effort: Bevy asks them to select the
requested locale first, then the resolved locale, but rejection does not block
Bevy asset-backed locale publication. Only metadata-only Bevy registrations
create Bevy asset availability; runtime localizer registrations are reserved
for the fallback manager and do not make a locale wait on Bevy asset bundles.
When attached, runtime fallback selection uses `FluentManager`'s best-effort
behavior; generated embedded localizers are fallback-aware, while custom
runtime localizers should implement parent-locale fallback in
`select_language(...)` when they need it. Runtime fallback managers are attached
at startup only when they accept the requested or resolved locale, and are used
only after Bevy resolves a locale through asset or ready-bundle availability
during startup or a later `LocaleChangeEvent`; runtime-only locales do not by
themselves make a Bevy locale switch selectable.

For direct localization inside a system, request `BevyI18n` like any other
Bevy system parameter:

```rs
use es_fluent::ThisFtl as _;
use es_fluent_manager_bevy::BevyI18n;

fn update_title(i18n: BevyI18n) {
    let title = i18n.localize_message(&UiMessage::Settings);
    // `SettingsPanel` is any type that derives `EsFluentThis`.
    let section_title = SettingsPanel::this_ftl(&i18n);
    // apply `title` to your Bevy UI, window, or gameplay state
    // use `section_title` for an `EsFluentThis` type label
}
```

### 3. Define Localizable Components (Recommended)

Prefer the `BevyFluentText` derive macro. It auto-registers your type with
`I18nPlugin` via inventory, so you don't have to call any registration functions
manually.

If a field depends on the active locale (like the `Languages` enum from
[es_fluent_lang](../es-fluent-lang/README.md)), mark it with `#[locale]`. The
macro will generate `RefreshForLocale` and register the locale-aware systems for
you.
`#[locale]` is supported on named struct fields and named enum variant fields,
and multiple named fields in the same variant refresh together.

`RefreshForLocale` receives the originally requested locale, not the fallback
resource locale. For example, if `en-GB` falls back to `en` assets, locale-aware
fields still refresh with `en-GB`.

```rs
use bevy::prelude::Component;
use es_fluent::EsFluent;
use es_fluent_manager_bevy::BevyFluentText;

#[derive(BevyFluentText, Clone, Component, EsFluent)]
pub enum UiMessage {
    StartGame,
    Settings,
    LanguageHint {
        #[locale]
        current_language: Languages,
    },
}
```

### 4. Using in UI

Use the `FluentText` component wrapper for any type that implements
`FluentMessage` (which `#[derive(EsFluent)]` provides).

```rs
use es_fluent_manager_bevy::FluentText;

fn spawn_menu(mut commands: Commands) {
    commands.spawn((
        FluentText::new(UiMessage::StartGame),
        Text::new(""),
    ));
}
```

### Manual Registration

If you cannot derive `BevyFluentText` (for example, for external types), register manually:

```rs
app.register_fluent_text::<UiMessage>();
```
