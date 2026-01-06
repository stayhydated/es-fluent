# es-fluent-lang-macro

Procedural macro for finding and enumerating supported languages.

This crate helps you generate a standardized `Language` enum for your application, based on the actual translation files present in your project.

## Usage

You should usually access this macro via the `es-fluent-lang` crate.

```rust
use es_fluent_lang::es_fluent_language;

#[es_fluent_language]
pub enum MyLanguages {}
```

See the [es-fluent-lang documentation](https://docs.rs/es-fluent-lang) for full usage details.
