# es-fluent-manager-bevy Architecture

This document details the architecture of the `es-fluent-manager-bevy` crate, which integrates `es-fluent` with the [Bevy](https://github.com/bevyengine/bevy) game engine.

## Overview

`es-fluent-manager-bevy` adapts the localization system to Bevy's ECS (Entity Component System) and asset infrastructure. It supports:

- **Asset Loading**: Loading `.ftl` files via `AssetServer`.
- **Macro-Driven Registration**: `#[derive(BevyFluentText)]` auto-registers components and locale refresh.
- **Reactivity**: Automatically updating UI text when the language changes.
- **Global Integration**: Seamlessly hooking into `es-fluent`'s global context.
- **Hot Reloading**: Reloading translations when files change on disk.

## Architecture

The system bridges the ECS world with the static global context required by `es-fluent`.

```mermaid
flowchart TD
    subgraph ASSETS["Asset System"]
        DISK[".ftl files"]
        LOADER[FtlAssetLoader]
        STORE[I18nAssets]
    end

    subgraph STATE["Resources"]
        RES["I18nResource (Current Lang)"]
        BUNDLE["I18nBundle (Compiled Bundles)"]
        GLOBAL["BevyI18nState (ArcSwap Global)"]
    end

    subgraph ECS["ECS World"]
        COMP["FluentText Component"]
        SYS["Update System"]
        EVENT["LocaleChangedEvent"]
    end

    subgraph GLOBAL_CTX["es-fluent Global"]
        CUSTOM["Custom Localizer"]
    end

    DISK -->|load| LOADER
    LOADER -->|produce| STORE
    STORE -->|compile| BUNDLE

    RES -->|change| EVENT
    EVENT -->|trigger| SYS
    SYS -->|read| COMP
    SYS -->|format using| BUNDLE
    SYS -->|update| COMP

    BUNDLE -->|syncs to| GLOBAL
    GLOBAL -->|hook| CUSTOM
```

## Key Components

### `I18nPlugin`

The entry point. It registers the `FtlAssetLoader`, resources, and—crucially—installs a **custom localizer** (`es_fluent::set_custom_localizer`). This custom localizer redirects all global `localize!` calls (used by `derive(EsFluent)` types) to the active Bevy resources, allowing standard Rust objects to stringify correctly even inside Bevy systems.

### `BevyFluentText` (derive macro)

The recommended path for Bevy components. This macro (re-exported from
`es-fluent-manager-macros`) submits a `BevyFluentTextRegistration` via
inventory, and `I18nPlugin` auto-registers those types at startup.

If any fields are marked with `#[locale]`, the macro generates a
`RefreshForLocale` implementation and registers the locale-aware systems
(`register_fluent_text_from_locale`). Otherwise it uses the standard
registration (`register_fluent_text`). This keeps locale-driven fields
(like `Languages` from `es_fluent_lang`) in sync automatically.

Manual registration via `FluentTextRegistration` remains available for types
that cannot derive the macro.

### `BevyI18nState` (Global Mirror)

A global static that mirrors the ECS state for use by the custom localizer. It uses `ArcSwap` for lock-free reads:

```rs
static BEVY_I18N_STATE: OnceLock<ArcSwap<BevyI18nState>> = OnceLock::new();
```

Using `ArcSwap` instead of `Arc<RwLock<...>>` enables lock-free access during localization calls. When the bundle or language changes, a new `BevyI18nState` is atomically swapped in, ensuring the hot path (`localize!` calls) never blocks on a lock.

### `FtlAssetLoader`

Implements `AssetLoader` to parse `.ftl` files into `FtlAsset`s.

### `I18nResource`

Holds the current active language. Setting this triggers the update pipeline.

### `FluentText<T>`

A component wrapper for localizable data. When registered (typically via
`BevyFluentText`), the update systems keep the rendered string in sync with
the current locale.

When `LocaleChangedEvent` fires, the `update_all_fluent_text_on_locale_change` system iterates over all `FluentText` components and re-renders the string data. Additionally, `update_fluent_text_system` handles initial rendering and updates when `FluentText` components are added or modified.

### `define_i18n_module!`

Re-exported from `es-fluent-manager-macros::define_bevy_i18n_module`. See the [es-fluent-manager-macros architecture](../../es-fluent-manager-macros/docs/ARCHITECTURE.md) for details on how the macro discovers languages and generates module data. This macro registers the crate's assets with the system so Bevy knows which domains to load.

## Flow

1. **Startup**: `I18nPlugin` initializes resources, registers the global custom localizer, and auto-registers any `BevyFluentText` types discovered via inventory.
1. **Loading**: Bevy loads all `.ftl` assets defined by registered modules.
1. **Compilation**: `I18nBundle` creates `FluentBundle`s from loaded assets.
1. **Localization**:
   - **Components**: `FluentText<T>` components update automatically via `update_all_fluent_text_on_locale_change`.
   - **Global**: `localize!("my-id")` works anywhere because the global hook calls back into the Bevy state.
