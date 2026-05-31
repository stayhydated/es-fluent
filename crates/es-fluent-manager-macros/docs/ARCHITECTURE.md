# es-fluent-manager-macros Architecture

This document details the architecture of the `es-fluent-manager-macros` crate, which automates the discovery and registration of localization assets.

## Overview

This crate provides helper macros that scan the project's filesystem at compile time to identify available languages and generate the necessary boilerplate to register an `I18nModule`.
The filesystem scan is delegated to `es-fluent-shared::resource::ResourcePlan::sparse_from_assets`,
so embedded, Dioxus, and Bevy macro expansion all use the same canonical
language, namespace, and `ModuleResourceSpec` planning logic as other manager
and tooling crates.

## Macros

### `define_embedded_i18n_module!`

Used by `es-fluent-manager-embedded`.

1. **Scans**: The configured `assets_dir` from `i18n.toml`.
1. **Generates**:
   - A struct deriving `RustEmbed` (from `rust-embed` crate), embedding files into the binary.
   - Static `ModuleData` listing supported canonical locale directories and discovered namespace paths from the shared sparse asset plan. Non-canonical locale directory names are rejected at macro expansion time instead of being normalized at runtime.
   - For non-namespaced modules, the generated resource plan requires `{lang}/{crate}.ftl`.
   - For namespaced modules, namespace files such as `{lang}/{crate}/ui.ftl` or `{lang}/{crate}/ui/button.ftl` are required, while `{lang}/{crate}.ftl` remains an optional mixed-mode base resource.
   - `inventory::submit!` block to register the module.

### `define_dioxus_i18n_module!`

Used by `es-fluent-manager-dioxus`.

1. **Scans**: The configured `assets_dir` from `i18n.toml`.
1. **Generates**:
   - Static `ModuleData` with the same supported-language and namespace metadata.
   - One `DioxusI18nAssetResource` entry per discovered FTL file, using Dioxus
     `asset!` to register package-local files with the Dioxus asset pipeline.
   - A static `DioxusI18nAssetModule`, a static module-set slice exposed through
     generated `dioxus_i18n_asset_modules()`, and generated
     `load_dioxus_i18n_assets(...)` helpers that asynchronously read resources
     with `dioxus::asset_resolver::read_asset_bytes`.
   - No `inventory::submit!` block; this path is loaded explicitly because
     Dioxus asset resolution is asynchronous.

### `define_bevy_i18n_module!`

Used by `es-fluent-manager-bevy`.

1. **Scans**: The configured `assets_dir` from `i18n.toml`.
1. **Generates**:
   - Static `ModuleData` listing supported languages from canonical locale directories.
   - `inventory::submit!` block to register an `I18nModuleRegistration`.
   - A shared per-language resource plan manifest (`resource_plan_for_language`) so each locale only queues the canonical resource files that actually exist, including nested namespace paths like `{crate}/ui/button.ftl`.
   - For namespaced modules, an exact per-language plan contains the discovered namespace resources for that locale. If `{crate}.ftl` exists for that locale, the plan includes it as an optional mixed-mode base resource.
   - _Note_: Does not embed files; Bevy still loads assets at runtime.

## Rationale

These macros exist to:

1. **Reduce Boilerplate**: Users don't need to manually list every supported language in code.
1. **Single Source of Truth**: The file system (presence of `.ftl` files) determines availability.
1. **Compile-Time Safety**: Verifies that `i18n.toml` exists and the assets directory is valid.
