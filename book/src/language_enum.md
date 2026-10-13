# Build a language picker

Use `iso-types`' `#[iso_locale]` to declare your application's supported
locales as an enum. The macro reads `i18n.toml` and the canonical locale
directories, includes the fallback locale, and supplies manager conversions.

## Configure the dependency

Enable `locales` and `fluent` for an es-fluent application:

```toml
[dependencies]
iso-types = { git = "https://github.com/stayhydated/iso-types", features = ["locales", "fluent"] }
```

Keep the package's `es-fluent-build` build script tracking its locale assets.
Define an empty enum in the package's library:

```rust
use iso_types::iso_locale;

#[iso_locale]
pub enum Languages {}
```

For directories `en`, `fr-FR`, and `zh-CN`, the generated variants are
`En`, `FrFr`, and `ZhCn`. The macro supplies `Clone`, `Copy`, `Debug`, `Eq`,
`Hash`, `PartialEq`, and Strum's `VariantArray`. `Languages::VARIANTS` provides a
constant slice of every locale. Add `iso_types::strum::EnumIter` when a caller
needs `IntoEnumIterator`.

`Default` selects the configured fallback. `FromStr` and `TryFrom` accept
supported Fluent or ICU language identifiers, preserving distinct errors
for malformed input, excluded data, and valid but unselected locales. Owned and
borrowed enum values convert into either identifier type. Parsing normalizes
current ISO representations with ICU4X; `eng-US` selects a configured `en-US`
locale. Deprecated aliases such as `iw-IL` return `LocaleConversionError::OutOfScope`.
Use ICU canonical tags for the fallback and locale directories.

The picker supports current living languages with CLDR modern locale coverage,
recommended modern scripts, and current regions. Locale variants and historical,
deprecated, or uncommon components are excluded. The macro checks this scope at
compile time, including for custom labels.

## Select and display a locale

Pass a variant directly to manager APIs that accept
`Into<LanguageIdentifier>`:

```rust,ignore
use iso_types::VariantArray as _;

let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(Languages::En)?;
for language in Languages::VARIANTS {
    println!("{}: {}", language.code(), i18n.localize_message(language));
}
i18n.select_language(Languages::FrFr)?;
```

Both `Display` and built-in Fluent labels use English. `FrFr` renders as
`French (France)`; a script is included when present. These labels
come from ICU4X's English name data. `EnUs` renders as `American English`, and
`ZhHantTw` renders as `Traditional Chinese (Taiwan)`. The macro reports a compile
error if ICU4X cannot name a configured locale; choose a locale covered by ICU.

The ISO runtime module follows manager locale selection while keeping its
English labels. Application translation modules determine which locales are
supported. The macro keeps its runtime registration linked in optimized WASM
applications.

## Own the language labels

Use custom mode to generate locale label keys in your package's FTL:

```rust
#[iso_types::iso_locale(custom)]
pub enum Languages {}
```

`cargo es-fluent generate` discovers this enum through inventory. The keys use
the exact locale tags, such as `en` and `fr-FR`, and belong to the declaring
package. Edit those messages to control the picker wording. `Display` continues
to render ICU4X's English label.
