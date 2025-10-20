# es-fluent-lang-macro

This crate provides the `#[es_fluent_language]` procedural macro for automatically generating a language selection enum based on your project's localization setup.

## Features

- **Automatic Enum Generation**: Reads your `i18n.toml` file to discover available languages and generates a Rust `enum` with a variant for each one.
- **Compile-Time Validation**: Ensures that all languages defined in your localization assets are valid and supported by the `es-fluent-lang` data crate.
- **Convenient Conversions**: Implements `From` conversions to and from `unic_langid::LanguageIdentifier` for seamless integration with Fluent.
- **Default Fallback**: Automatically implements `Default` for the enum, using the `fallback_language` specified in your `i18n.toml`.

## Usage

First, add the crate to your `Cargo.toml`:

```toml
[dependencies]
es-fluent-lang = "*"
es-fluent-lang-macro = "*"
unic-langid = "*"
```

Then, define your language enum in your code:

```rs
use es_fluent::EsFluent;
use es_fluent_lang_macro::es_fluent_language;
use unic_langid::LanguageIdentifier;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
pub enum Languages {}

// Now you can use the generated enum:
fn main() {
    let default_lang: Language = Language::default();
    let lang_id: LanguageIdentifier = default_lang.into();
    println!("Default language is: {}", lang_id);

    // Example: assuming 'fr' is an available language in your i18n setup
    // let french = Language::Fr;
}
```

The macro will expand the empty `Language` enum into one with variants corresponding to the languages found in the directory specified by `assets_dir` in your `i18n.toml`. For example, if you have `en` and `fr` directories, the enum will have `En` and `Fr` variants.
