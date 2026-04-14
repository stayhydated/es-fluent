[![Docs](https://docs.rs/es-fluent-toml/badge.svg)](https://docs.rs/es-fluent-toml/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-toml.svg)](https://crates.io/crates/es-fluent-toml)

# es-fluent-toml

**Internal Crate**: Configuration parser and build-script helpers for `i18n.toml`.

`es-fluent-toml` is the single source of truth for workspace localization
configuration. It parses `i18n.toml`, resolves asset paths relative to the config
file, discovers available locales, and exposes build-time helpers used by macros
and custom tooling.

## Key API

- `I18nConfig`: raw deserialized configuration
- `ResolvedI18nLayout`: config plus resolved absolute paths and locale helpers
- `FluentFeature`: supports `fluent_feature = "name"` and
  `fluent_feature = ["name", "other"]`
- `build::track_i18n_assets()`: emits `cargo:rerun-if-changed` directives for
  locale assets

## Typical direct use

Most applications use this crate indirectly through [`es-fluent`](../es-fluent/README.md),
[`es-fluent-manager-macros`](../es-fluent-manager-macros/README.md), or
[`es-fluent-cli`](../es-fluent-cli/README.md). Depend on it directly only when
writing custom build scripts or tools around `i18n.toml`.

```rs
fn main() {
    es_fluent_toml::build::track_i18n_assets();
}
```
