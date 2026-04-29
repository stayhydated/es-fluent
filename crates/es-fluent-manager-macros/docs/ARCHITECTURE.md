# es-fluent-manager-macros Architecture

This document details the architecture of the `es-fluent-manager-macros` crate, which automates the discovery and registration of localization assets.

## Overview

This crate provides helper macros that scan the project's filesystem at compile time to identify available languages and generate the necessary boilerplate to register an `I18nModule`.

## Macros

### `define_embedded_i18n_module!`

Used by `es-fluent-manager-embedded`.

1. **Scans**: The configured `assets_dir` from `i18n.toml`.
1. **Generates**:
   - A struct deriving `RustEmbed` (from `rust-embed` crate), embedding files into the binary.
   - Static `ModuleData` listing supported canonical locale directories and discovered namespace paths. Non-canonical locale directory names are rejected at macro expansion time instead of being normalized at runtime.
   - For non-namespaced modules, the generated resource plan requires `{lang}/{crate}.ftl`.
   - For namespaced modules, namespace files such as `{lang}/{crate}/ui.ftl` or `{lang}/{crate}/ui/button.ftl` are required, while `{lang}/{crate}.ftl` remains an optional mixed-mode base resource.
   - `inventory::submit!` block to register the module.

### `define_dioxus_i18n_module!`

Used by `es-fluent-manager-dioxus`.

1. **Scans**: The configured `assets_dir` from `i18n.toml`.
1. **Generates**:
   - The same `RustEmbed`-backed module registration shape used by the embedded
     manager.
   - Static `ModuleData` listing supported canonical locale directories and discovered namespace paths.
   - The same embedded resource-plan semantics as `define_embedded_i18n_module!`.
   - `inventory::submit!` block to register an embedded runtime localizer for
     Dioxus client and SSR integrations.

### `define_bevy_i18n_module!`

Used by `es-fluent-manager-bevy`.

1. **Scans**: The configured `assets_dir` from `i18n.toml`.
1. **Generates**:
   - Static `ModuleData` listing supported languages from canonical locale directories.
   - `inventory::submit!` block to register an `I18nModuleRegistration`.
   - A per-language resource plan manifest (`resource_plan_for_language`) so each locale only queues the canonical resource files that actually exist, including nested namespace paths like `{crate}/ui/button.ftl`.
   - For namespaced modules, an exact per-language plan contains the discovered namespace resources for that locale. A stray `{crate}.ftl` base file is ignored by that Bevy asset plan.
   - _Note_: Does not embed files; Bevy still loads assets at runtime.

## Rationale

These macros exist to:

1. **Reduce Boilerplate**: Users don't need to manually list every supported language in code.
1. **Single Source of Truth**: The file system (presence of `.ftl` files) determines availability.
1. **Compile-Time Safety**: Verifies that `i18n.toml` exists and the assets directory is valid.
