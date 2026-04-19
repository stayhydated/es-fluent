[![Docs](https://docs.rs/es-fluent-manager-macros/badge.svg)](https://docs.rs/es-fluent-manager-macros/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-macros.svg)](https://crates.io/crates/es-fluent-manager-macros)

# es-fluent-manager-macros

The `es-fluent-manager-macros` crate provides the compile-time macros used by `es-fluent-manager-embedded` and `es-fluent-manager-bevy`.

These macros read your `i18n.toml` configuration, scan the configured `assets_dir` at compile time, generate module metadata, and expose the `BevyFluentText` derive used by the Bevy integration.

For `BevyFluentText`, `#[locale]` may be applied to named struct fields and named enum variant fields. Multiple named locale fields in the same variant are supported and refresh together.

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

## Incremental Builds and Asset Changes

The macros scan your configured locale assets at compile time. If you rename or add locale
folders, Cargo may not automatically rebuild the crate, which can leave cached
language lists behind.

To make asset changes trigger rebuilds, add a `build.rs` to your crate:

```rs
fn main() {
    es_fluent_toml::build::track_i18n_assets();
}
```

And add this to your `Cargo.toml`:

```toml
[build-dependencies]
es-fluent-toml = "*"
```
