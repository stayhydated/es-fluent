[![Docs](https://docs.rs/es-fluent-lang/badge.svg)](https://docs.rs/es-fluent-lang/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-lang.svg)](https://crates.io/crates/es-fluent-lang)

# es-fluent-lang

Runtime support for `es-fluent` language management.

This crate provides the core language types (re-exporting `unic-langid`) and the optional "Language Enum" generator macro.

## Features

### `#[es_fluent_language]`

Generates a strongly-typed enum of all available languages in your project. It automatically scans your `i18n.toml` assets directory to find supported locales.

```rs
use es_fluent_lang::es_fluent_language;
use es_fluent::EsFluent;
use strum::EnumIter;

// Define an empty enum, and the macro fills it
#[es_fluent_language]
#[derive(Debug, Clone, Copy, PartialEq, Eq, EsFluent, EnumIter)]
pub enum Languages {}
```

If your `assets_dir` contains the same locales as the executable README example
(`en`, `fr-FR`, and `zh-CN`), this generates:

```rs
pub enum Languages {
    En,
    FrFr,
    ZhCn,
}
```

It also implements:

- `Default`: Uses the `fallback_language` from your config.
- `FromStr`: Parses string codes (e.g., "en", "fr-FR", or "zh-CN") into the enum variant.
- `TryFrom<&LanguageIdentifier>` / `TryFrom<LanguageIdentifier>`: Converts from a locale ID and returns an error for unsupported locales.
- `Into<LanguageIdentifier>`: Converts back to a standard locale ID.

For user-facing labels, derive `EsFluent` on the generated enum and call
manager-backed `localize_message(...)` instead of relying on `Display`.

If you want to provide your own language-name translations, use
`#[es_fluent_language(custom)]`. Custom mode skips the built-in
`es-fluent-lang` runtime hook. When combined with `#[derive(EsFluent)]`, it
also leaves inventory registration enabled so your own FTL resources can
provide the labels.

### Feature Flags

- `macros` (default): Enables the `#[es_fluent_language]` macro.
- `localized-langs`: Format language names in the currently selected UI language instead of as autonyms.
- `bevy`: Backward-compatible feature for Bevy projects. The WASM force-link keepalive is no longer Bevy-specific.

## Standard Translations

The crate also includes a built-in module for translating language names themselves (e.g., "English", "Français", "Deutsch"). This means you can easily build a "Language Picker" UI without manually translating the names of every language.

By default, labels are formatted directly from ICU4X display-name data as autonyms, so `i18n.localize_message(&Languages::FrFr)` resolves to `français` and `i18n.localize_message(&Languages::ZhCn)` resolves to `中文`. With the `localized-langs` feature, the same ICU4X data is formatted in the currently selected UI language instead, so selecting English yields labels like `French` and `Chinese`.

For a language picker, iterate your generated enum, render each label through
the active manager, and pass the selected variant back to the manager:

```rs
use es_fluent::FluentMessage as _;
use strum::IntoEnumIterator as _;

for language in Languages::iter() {
    let label = i18n.localize_message(&language);
    println!("{language:?}: {label}");
}

i18n.select_language(Languages::FrFr)?;
```

The runtime resolves fallback locales through the shared ICU4X/CLDR fallback chain when a display locale is missing exact display-name data. If you need fully custom labels for project-specific or unsupported locale tags, use `#[es_fluent_language(custom)]` and ship your own translations.

The built-in language-name module follows successful manager locale switches
but does not count as application content support. A manager still reports an
unsupported locale when no application translation module can serve it.

For `wasm32` builds, default `#[es_fluent_language]` enums emit a small
force-link keepalive hook so the built-in language-name module is retained
across managers, including Dioxus and Bevy.
