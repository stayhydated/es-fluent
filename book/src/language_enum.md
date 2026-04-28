# Language Enum

Most apps need to know which languages are available — for a settings screen, a language picker, or to select the right locale at startup. The `#[es_fluent_language]` macro generates a strongly-typed enum from the folders in your `assets_dir`, so you never hardcode locale strings.

## Setup

Add the `es-fluent-lang` crate:

```toml
[dependencies]
es-fluent-lang = "0.15"
```

## Usage

Define an empty enum and annotate it with `#[es_fluent_language]`:

```rust
use es_fluent_lang::es_fluent_language;
use es_fluent::EsFluent;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Debug, Clone, Copy, PartialEq, EsFluent, EnumIter)]
pub enum Languages {}
```

If your `assets_dir` contains `en`, `fr`, and `de` folders, the macro expands this into:

```rust
pub enum Languages {
    De,
    En,
    Fr,
}
```

The macro also generates these trait implementations:

| Trait                          | Description                                                           |
| ------------------------------ | --------------------------------------------------------------------- |
| `Default`                      | Returns the variant matching `fallback_language` from `i18n.toml`     |
| `FromStr`                      | Parses `"en-US"`, `"fr"`, or `"de-DE-1901"` into the matching variant |
| `TryFrom<&LanguageIdentifier>` | Converts from a `unic-langid` identifier                              |
| `Into<LanguageIdentifier>`     | Converts back to a `unic-langid` identifier                           |

## Using with Managers

The `Languages` enum plugs directly into manager initialization:

```rust
use es_fluent_manager_embedded as manager;

let i18n = manager::EmbeddedI18n::try_new_with_language(Languages::En)?;
```

Since it implements `Into<LanguageIdentifier>`, you can pass variants anywhere a `LanguageIdentifier` is expected.

## Language Name Labels

By deriving `EsFluent` alongside `#[es_fluent_language]`, each variant can be rendered through an explicit manager with `i18n.localize_message(&language)`. The crate formats those labels directly from ICU4X display-name data, so a language picker works out of the box:

```rust
use es_fluent::FluentMessage;

// Prints the language name in its native script
println!("{}", i18n.localize_message(&Languages::Fr)); // → "français"
```

## Custom Mode

By default, the macro links to the built-in `es-fluent-lang` runtime and skips inventory registration. If you want to provide your own translations for language names (for example, project-specific labels or exact wording control), use **custom mode**:

```rust
#[es_fluent_language(custom)]
#[derive(Debug, Clone, Copy, EsFluent, EnumIter)]
pub enum Languages {}
```

In custom mode:

- The macro stops injecting the built-in `es-fluent-lang` resource attributes.
- When you also derive `EsFluent`, `cargo es-fluent generate` will create keys for the enum in your FTL files.
- You provide your own translations instead of using ICU4X-backed labels.
- Use this when your app ships custom language-name translations for project-specific or otherwise unsupported locale tags.
