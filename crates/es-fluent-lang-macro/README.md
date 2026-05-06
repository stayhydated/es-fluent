[![Docs](https://docs.rs/es-fluent-lang-macro/badge.svg)](https://docs.rs/es-fluent-lang-macro/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang-macro.svg)](https://crates.io/crates/es-fluent-lang-macro)

# es-fluent-lang-macro

Procedural macro for finding and enumerating supported languages.

This crate helps you generate a standardized `Language` enum for your application, based on the canonical locale folders present in your configured `assets_dir`.

## Usage

You should usually access this macro via the `es-fluent-lang` crate.

```rs
use es_fluent_lang::es_fluent_language;

#[es_fluent_language]
pub enum MyLanguages {}
```

Use `#[es_fluent_language(custom)]` when your app provides its own language-name
translations. In that mode, the generated enum is inventory-visible and your
FTL files become the source of truth for the display labels.

For example, locale folders named `en`, `fr-FR`, and `zh-CN` generate enum
variants `En`, `FrFr`, and `ZhCn`.

See the [es-fluent-lang documentation](https://docs.rs/es-fluent-lang) for full usage details.
