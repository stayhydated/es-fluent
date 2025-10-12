# es-fluent-manager-macros

The `es-fluent-manager-macros` crate provides the procedural macros that help automate the setup of translation modules for `es-fluent-manager`.

These macros read your `i18n.toml` configuration and scan your translation directories at compile time to generate the necessary static data structures for module discovery.

## Usage

You typically call one of these macros once in your `lib.rs` or `main.rs` to set up the translation module for your crate.

### For Embedded Translations:

```rs
// In lib.rs or main.rs
es_fluent_manager_macros::define_embedded_i18n_module!();
```

### For Bevy Asset-based Translations:

```rs
// In lib.rs or main.rs
es_fluent_manager_macros::define_bevy_i18n_module!();
```
