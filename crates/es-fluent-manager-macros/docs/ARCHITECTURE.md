# es-fluent-manager-macros Architecture

This document details the architecture of the `es-fluent-manager-macros` crate, which automates the discovery and registration of localization assets.

## Overview

This crate provides helper macros that scan the project's filesystem at compile time to identify available languages and generate the necessary boilerplate to register an `I18nModule`.

## Macros

### `define_embedded_i18n_module!`

Used by `es-fluent-manager-embedded`.

1. **Scans**: `i18n/` directory defined in `i18n.toml`.
1. **Generates**:
   - A struct deriving `RustEmbed` (from `rust-embed` crate), embedding files into the binary.
   - Static `ModuleData` listing supported languages from canonical locale files. Non-canonical locale directory names are rejected at macro expansion time instead of being normalized at runtime:
     `{lang}/{crate}.ftl` for non-namespaced modules, or recursively discovered
     `{lang}/{crate}/**/*.ftl` namespace files for namespaced modules.
   - `inventory::submit!` block to register the module.

### `define_bevy_i18n_module!`

Used by `es-fluent-manager-bevy`.

1. **Scans**: `assets/` directory (or configured path).
1. **Generates**:
   - Static `ModuleData` listing supported languages from canonical locale directories.
   - `inventory::submit!` block to register an `I18nModuleRegistration`.
   - A per-language resource plan manifest (`resource_plan_for_language`) so each locale only queues the canonical resource files that actually exist, including nested namespace paths like `{crate}/ui/button.ftl`.
   - For namespaced modules, the generated plan contains only `{crate}/{namespace}.ftl` entries. A stray `{crate}.ftl` base file is ignored for that namespaced locale plan.
   - _Note_: Does not embed files; Bevy still loads assets at runtime.

## Rationale

These macros exist to:

1. **Reduce Boilerplate**: Users don't need to manually list every supported language in code.
1. **Single Source of Truth**: The file system (presence of `.ftl` files) determines availability.
1. **Compile-Time Safety**: Verifies that `i18n.toml` exists and the assets directory is valid.
