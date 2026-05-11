[![Docs](https://docs.rs/es-fluent-toml/badge.svg)](https://docs.rs/es-fluent-toml/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-toml.svg)](https://crates.io/crates/es-fluent-toml)

# es-fluent-toml

**Internal Crate**: Configuration parser and path resolver for `i18n.toml`.

`es-fluent-toml` is the single source of truth for workspace localization
configuration. It parses `i18n.toml`, resolves asset paths relative to the config
file, and discovers available locales for macros, the build-helper crate, and
custom tooling.

## Key API

- `I18nConfig`: raw deserialized configuration
- `ResolvedI18nLayout`: config plus resolved absolute paths and locale helpers
- `FluentFeature`: supports `fluent_feature = "name"` and
  `fluent_feature = ["name", "other"]`

## Typical direct use

Most applications use this crate indirectly through [`es-fluent`](../es-fluent/README.md),
[`es-fluent-build`](../es-fluent-build/README.md),
[`es-fluent-manager-macros`](../es-fluent-manager-macros/README.md), or
[`es-fluent-cli`](../es-fluent-cli/README.md). Depend on it directly only when
writing custom tools around `i18n.toml`.

```rust,no_run
fn main() -> Result<(), es_fluent_toml::I18nConfigError> {
    let layout = es_fluent_toml::ResolvedI18nLayout::from_manifest_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
    )?;
    println!("assets: {}", layout.assets_dir.display());
    Ok(())
}
```
