# es-fluent-manager-macros

The `es-fluent-manager-macros` crate provides the procedural macros that help automate the setup of translation modules for `es-fluent-manager`.

These macros read your `i18n.toml` configuration and scan your translation directories at compile time to generate the necessary static data structures for module discovery.

## Macros

-   **`define_embedded_i18n_module!`**: This macro is used for translations that are embedded directly into the application binary. It generates a `RustEmbed` struct and an `EmbeddedI18nModule` instance that `es-fluent-manager-core` can automatically discover.

-   **`define_bevy_i18n_module!`**: This macro is tailored for Bevy applications. It scans the translation directories and generates an `AssetI18nModule` instance, which registers the crate's translations with the Bevy asset system.

## Usage

You typically call one of these macros once in your `lib.rs` or `main.rs` to set up the translation module for your crate.

### For Embedded Translations:

```rs,no_run
// In lib.rs or main.rs
use es_fluent_manager_macros::define_embedded_i18n_module;

define_embedded_i18n_module!();
```

### For Bevy Asset-based Translations:

```rs,no_run
// In lib.rs or main.rs
use es_fluent_manager_macros::define_bevy_i18n_module;

define_bevy_i18n_module!();
```
