[![Docs](https://docs.rs/es-fluent-manager-macros/badge.svg)](https://docs.rs/es-fluent-manager-macros/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-macros.svg)](https://crates.io/crates/es-fluent-manager-macros)

# es-fluent-manager-macros

The `es-fluent-manager-macros` crate provides the compile-time macros used by
`es-fluent-manager-embedded`, `es-fluent-manager-dioxus`, and
`es-fluent-manager-bevy`.

These macros read your `i18n.toml` configuration, scan the configured `assets_dir` at compile time, generate module metadata, and expose the `BevyFluentText` derive used by the Bevy integration.

Locale discovery is per-locale and resource-aware. A locale that contains only
some namespace files is still registered with the files it has, so runtime
fallback can use translated messages from that locale and follow the ICU4X
locale fallback chain for missing messages.

Most applications should call the re-exported macros from the concrete manager
crate they use. Depend on this crate directly only when building a custom
integration around the manager macro surface.

For `BevyFluentText`, `#[locale]` may be applied to named struct fields and named enum variant fields. Multiple named locale fields in the same variant are supported and refresh together.

## Usage

You typically call one of these macros once from a library-reachable module,
usually `src/i18n.rs` declared from `src/lib.rs`, to set up the translation
module for your crate. Calling the macro only from `src/main.rs` is runtime-only;
CLI generation still discovers localizable types through library targets.

### For Embedded Translations

```rs
// In src/i18n.rs, declared from src/lib.rs
es_fluent_manager_embedded::define_i18n_module!();
```

### For Dioxus Client or SSR Translations

```rs
// In src/i18n.rs, declared from src/lib.rs
es_fluent_manager_dioxus::define_i18n_module!();
```

### For Bevy Asset-based Translations

```rs
// In src/i18n.rs, declared from src/lib.rs
es_fluent_manager_bevy::define_i18n_module!();
```

## Incremental Builds and Asset Changes

The macros scan your configured locale assets at compile time. If you rename or add locale
folders, Cargo may not automatically rebuild the crate, which can leave cached
language lists behind.

To make asset changes trigger rebuilds, add a `build.rs` to your crate:

```rs
fn main() {
    es_fluent::build::track_i18n_assets();
}
```

And add this to your `Cargo.toml`:

```toml
[build-dependencies]
es-fluent = { version = "0.16", features = ["build"] }
```
