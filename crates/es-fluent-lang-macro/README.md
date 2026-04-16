[![Docs](https://docs.rs/es-fluent-lang-macro/badge.svg)](https://docs.rs/es-fluent-lang-macro/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang-macro.svg)](https://crates.io/crates/es-fluent-lang-macro)

# es-fluent-lang-macro

Procedural macro for finding and enumerating supported languages.

This crate helps you generate a standardized `Language` enum for your application, based on the actual translation files present in your project.

## Usage

You should usually access this macro via the `es-fluent-lang` crate.

```rs
use es_fluent_lang::es_fluent_language;

#[es_fluent_language]
pub enum MyLanguages {}
```

Use `#[es_fluent_language(custom)]` when your app provides its own language-name
translations. In that mode, the generated enum is inventory-visible and may
include locale folders that are not covered by the bundled `es-fluent-lang`
locale table.

See the [es-fluent-lang documentation](https://docs.rs/es-fluent-lang) for full usage details.
