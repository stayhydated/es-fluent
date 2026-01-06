[![Docs](https://docs.rs/es-fluent-lang/badge.svg)](https://docs.rs/es-fluent-lang/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang.svg)](https://crates.io/crates/es-fluent-lang)

# es-fluent-lang

Runtime support for `es-fluent` language management.

This crate provides the core language types (re-exporting `unic-langid`) and the optional "Language Enum" generator macro.

## Features

### `#[es_fluent_language]`

Generates a strongly-typed enum of all available languages in your project. It automatically scans your `i18n.toml` assets directory to find supported locales.

```rust
use es_fluent_lang::es_fluent_language;
use es_fluent::EsFluent;

// Define an empty enum, and the macro fills it
#[es_fluent_language]
#[derive(Debug, Clone, Copy, PartialEq, Eq, EsFluent)]
pub enum Languages {}
```

If your `assets_dir` contains `en`, `fr`, and `de` folders, this generates:

```rust
pub enum Languages {
    En,
    Fr,
    De,
}
```

It also implements:

- `Default`: Uses the `fallback_language` from your config.
- `FromStr`: Parses string codes (e.g., "en-US") into the enum variant.
- `Into<LanguageIdentifier>`: Converts back to a standard locale ID.

## Standard Translations

The crate also includes a built-in module for translating language names themselves (e.g., "English", "Français", "Deutsch"). This means you can easily build a "Language Picker" UI without manually translating the names of every language.
