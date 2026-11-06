# Language enum

The [`iso-types`](https://github.com/stayhydated/iso-types) `#[iso_locale]`
macro generates a typed enum from the locale directories in `assets_dir`. Use it
to initialize a manager, switch locales, or build a language picker without
maintaining a second list of locale strings.

## Setup

Add `iso-types` with its `locales` and `fluent` features:

```toml
[dependencies]
iso-types = { git = "https://github.com/stayhydated/iso-types", default-features = false, features = ["locales", "fluent"] }
```

Feature flags:

- `locales` provides `#[iso_locale]`, locale parsing, and ICU4X locale names.
- `fluent` implements `FluentMessage` for the generated enum and registers the
  iso-types runtime module.
- `localized-langs` formats language names in the currently selected UI
  language instead of as autonyms.

The macro reads locales through the package's `es-fluent-build` manifest, so
compiler caches rebuild the enum when locale directories or `i18n.toml` change.

## Usage

Define an empty enum and annotate it with `#[iso_locale]`:

```rust
use iso_types::iso_locale;

#[iso_locale]
pub enum Languages {}
```

The macro derives `Clone`, `Copy`, `Debug`, `Eq`, `Hash`, and `PartialEq`, plus
Strum's `VariantArray`, `EnumIter`, `EnumCount`, `FromRepr`, and
`IntoStaticStr`. It implements `FluentMessage` itself, so do not derive
`EsFluent` on the enum.

If your `assets_dir` contains the same locales as the executable README example
(`en`, `fr-FR`, and `zh-CN`), the macro expands this into:

```rust
pub enum Languages {
    En,
    FrFr,
    ZhCn,
}
```

Locale directories and `fallback_language` must use ICU canonical tags, such as
`he-IL` rather than `iw-IL`. Any locale with complete ICU4X English names is
accepted, including variants such as `ca-ES-valencia`.

The macro also generates these implementations:

| Item | Description |
| --- | --- |
| `Default` | Returns the variant matching `fallback_language` from `i18n.toml` |
| `FromStr` | Canonicalizes and parses `"fr-FR"`, `"fr-fr"`, or `"fre-FR"` |
| `TryFrom<LanguageIdentifier>` | Converts from owned or borrowed `unic-langid` and ICU4X identifiers |
| `Into<LanguageIdentifier>` | Converts back to `unic-langid` and ICU4X identifiers |
| `code()`, `english_name()`, `autonym()` | Constant locale tag, English name, and self-name |
| `Display` | Writes the English name |
| `FluentMessage` | Renders language labels through a manager |

If the configured fallback language is not present as a locale directory, the
macro still adds it to the enum so `Default` always has a valid variant.
Parsing canonicalizes deprecated forms before matching, so `iw-IL` matches a
configured `he-IL`; valid tags outside the configured set return
`LocaleConversionError::Unsupported`.

## Use the enum with managers

The `Languages` enum plugs directly into manager initialization:

```rust
use es_fluent_manager_embedded as manager;

let i18n = manager::EmbeddedI18n::try_new_with_language(Languages::En)?;
```

Pass a variant directly to manager methods that accept
`Into<LanguageIdentifier>`. Use `.into()` where an API requires a concrete
`LanguageIdentifier`.

## Render language-name labels

Each variant can be rendered through an explicit manager with
`i18n.localize_message(&language)`:

```rust
println!("{}", i18n.localize_message(&Languages::FrFr)); // → "français (France)"
```

By default, labels are autonyms computed from ICU4X data at compile time:
`FrFr` renders as `français (France)` and `ZhCn` as `中文（中国）`. These labels
need no runtime module. With the `localized-langs` feature, the iso-types
runtime module formats each label in the currently selected UI language, so an
English UI renders `French (France)` and `Chinese (China)`. Missing names in the
selected language fall back to English.

For a language picker, iterate your generated enum, render each label through
the active manager, and pass the selected variant back to the manager:

```rust
use iso_types::IntoEnumIterator as _;

for language in Languages::iter() {
    let label = i18n.localize_message(&language);
    println!("{language:?}: {label}");
}

i18n.select_language(Languages::FrFr)?;
```

The iso-types runtime module follows successful manager locale switches but
does not count as application content support. A manager still reports an
unsupported locale when no application translation module can serve it.

## Custom mode

Use custom mode when the application owns the language-name translations, for
example for project-specific labels or exact wording control:

```rust
#[iso_locale(custom)]
pub enum Languages {}
```

In custom mode:

- The enum registers its variants with the package's FTL inventory, using each
  exact locale tag as the message ID.
- `cargo es-fluent generate` creates those keys in your FTL files.
- You provide the translations instead of using ICU4X-backed labels.
