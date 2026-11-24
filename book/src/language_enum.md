# Build a language picker

Use `iso-types`' `#[iso_locale]` to declare your application's supported
locales as an enum. The macro reads `i18n.toml` and the canonical locale
directories, includes the fallback locale, and supplies manager conversions.

## Configure the dependency

This source migration targets the next incompatible release after `0.21`.
The ISO companion uses the unreleased manager and build-manifest APIs; it is
not a standalone release compatible with registry-only `es-fluent` dependencies.
Keep its revision and the es-fluent source overrides together.

For an external application using the embedded manager, this pinned setup
matches the integration in this repository:

```toml
[dependencies]
es-fluent = "0.21.2"
es-fluent-manager-embedded = "0.21.2"
iso-types = { git = "https://github.com/stayhydated/iso-types", rev = "1820793487f47ac93536195f35b0940780eb1473", features = ["locales", "fluent"] }

[build-dependencies]
es-fluent-build = "0.21.2"

[patch.crates-io]
es-fluent = { git = "https://github.com/stayhydated/es-fluent", rev = "8ff7e6852ba08081dce125656a32ee42125d5804" }
es-fluent-build = { git = "https://github.com/stayhydated/es-fluent", rev = "8ff7e6852ba08081dce125656a32ee42125d5804" }
es-fluent-derive-core = { git = "https://github.com/stayhydated/es-fluent", rev = "8ff7e6852ba08081dce125656a32ee42125d5804" }
es-fluent-manager-core = { git = "https://github.com/stayhydated/es-fluent", rev = "8ff7e6852ba08081dce125656a32ee42125d5804" }
es-fluent-manager-embedded = { git = "https://github.com/stayhydated/es-fluent", rev = "8ff7e6852ba08081dce125656a32ee42125d5804" }
es-fluent-toml = { git = "https://github.com/stayhydated/es-fluent", rev = "8ff7e6852ba08081dce125656a32ee42125d5804" }
```

Place the patches in the consuming workspace root: patches declared by a
dependency do not propagate to its consumers. This repository instead patches
the same contracts to its local workspace crates. If using the Dioxus or Bevy
manager, select it from the same es-fluent revision too. Replace this setup with
matching released versions when the incompatible migration is published.

Keep the package's `es-fluent-build` build script tracking its locale assets.
Define an empty enum in the package's library:

```rust
use iso_types::iso_locale;

#[iso_locale]
pub enum Languages {}
```

For directories `en`, `fr-FR`, and `zh-CN`, the generated variants are
`En`, `FrFr`, and `ZhCn`. The macro supplies `Clone`, `Copy`, `Debug`, `Eq`,
`Hash`, `PartialEq`, and Strum's `VariantArray`, `EnumIter`, `EnumCount`,
`FromRepr`, and `IntoStaticStr`. Import `iso_types::{VariantArray, IntoEnumIterator,
EnumCount}` for `Languages::VARIANTS`, `Languages::iter()`, and `Languages::COUNT`.
The slice and iterator follow canonical locale order. `code()` and `into_str()`
return the canonical tag in const contexts. `from_repr()` accepts a zero-based
ordinal in declaration order; use canonical tags for persistence.

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
use iso_types::IntoEnumIterator as _;

let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new_with_language(Languages::En)?;
for language in Languages::iter() {
    println!("{}: {}", language.code(), i18n.localize_message(&language));
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
